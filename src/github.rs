use futures::prelude::*;
use log::debug;
use std::env;
use std::path::PathBuf;

use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{
  Commit, Cred, Error, FetchOptions, IndexAddOption, ObjectType, Oid, PushOptions, Remote,
  RemoteCallbacks, Repository, ResetType, Signature,
};
use hubcaps::comments::CommentOptions;
use hubcaps::labels::{Label, LabelOptions};
use hubcaps::pulls::PullOptions;
use hubcaps::Github;

const FORK_REMOTE: &str = "origin";
const UPSTREAM_REMOTE: &str = "upstream";
const UPSTREAM_BRANCH: &str = "master";
const SIGNATURE_AUTHOR: &str = "Openshift OTA Bot";
const SIGNATURE_EMAIL: &str = "vrutkovs@redhat.com";
const VERSION_LABEL_COLOR: &str = "0e8a16";
const ACTION_LABEL_COLOR: &str = "0052cc";

fn get_ssh_auth_callbacks<'cb>() -> RemoteCallbacks<'cb> {
  let mut callbacks = RemoteCallbacks::new();
  callbacks.credentials(|_url, username_from_url, _allowed_types| {
    Cred::ssh_key(
      username_from_url.unwrap(),
      None,
      std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
      None,
    )
  });
  callbacks
}

fn clone_repo(org: String, repo: String, path: &PathBuf) -> Result<Repository, Error> {
  // Authentication
  let mut builder = RepoBuilder::new();
  let mut fetch_options = FetchOptions::new();
  fetch_options.remote_callbacks(get_ssh_auth_callbacks());
  builder.fetch_options(fetch_options);

  let url = format!("git@github.com:{}/{}.git", org, repo);
  debug!("clone_repo: cloning {}", url);
  let repo = builder.clone(&url, &path)?;
  debug!("clone_repo: done");
  Ok(repo)
}

fn add_fetch_remote(git_repo: &Repository, org: String, repo: String) -> Result<Remote, Error> {
  debug!("add_fetch_remote: adding {}", UPSTREAM_REMOTE);
  let url = format!("https://github.com/{}/{}.git", org, repo);
  debug!("add_fetch_remote: url {}", url);
  git_repo.remote(UPSTREAM_REMOTE, &url)
}

fn fetch_from_upstream(
  repo: &Repository,
  target_org: String,
  target_user: String,
) -> Result<(), Error> {
  debug!("fetch_from_upstream+");
  let mut fetch_options = FetchOptions::new();
  fetch_options.remote_callbacks(get_ssh_auth_callbacks());

  let mut remote = add_fetch_remote(&repo, target_org, target_user)?;
  remote.fetch(&[UPSTREAM_BRANCH], Some(&mut fetch_options), None)?;

  let remote_refspec = format!("{}/{}", UPSTREAM_REMOTE, UPSTREAM_BRANCH);
  debug!("fetch_from_upstream: refspec {}", remote_refspec);
  let fetch_head = repo.revparse_single(&remote_refspec)?;
  debug!("fetch_from_upstream: fetch_head {}", fetch_head.id());

  let mut cb = CheckoutBuilder::new();
  repo.reset(&fetch_head, ResetType::Hard, Some(cb.force()))
}

fn find_last_commit(repo: &Repository) -> Result<Commit, Error> {
  let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
  obj
    .into_commit()
    .map_err(|_| Error::from_str("Couldn't find commit"))
}

pub fn refresh_forked_repo(
  target_org: String,
  target_user: String,
  forked_org: String,
  forked_user: String,
  path: &PathBuf,
) -> Result<Repository, Error> {
  let repo = clone_repo(forked_org, forked_user, path)?;
  fetch_from_upstream(&repo, target_org, target_user)?;
  Ok(repo)
}

pub fn switch_to(repo: &Repository, branch: String) -> Result<(), Error> {
  let commit = repo.head()?.peel_to_commit()?;
  repo.branch(&branch, &commit, true)?;
  let refname = format!("refs/heads/{}", &branch);
  let obj = repo.revparse_single(&refname)?;
  repo.checkout_tree(&obj, None)?;
  repo.set_head(&refname)
}

pub fn commit(repo: &Repository, branch: String, message: String) -> Result<Oid, Error> {
  // Stage all files
  let mut index = repo.index()?;
  index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
  // No files included - skip
  if index.len() == 0 {
    return Err(git2::Error::from_str("empty commit detected"));
  }
  let oid = index.write_tree()?;
  // Prepare commit metadata
  let signature = Signature::now(SIGNATURE_AUTHOR, SIGNATURE_EMAIL)?;
  let parent_commit = find_last_commit(&repo)?;
  let tree = repo.find_tree(oid)?;
  // Create a new HEAD commit
  let refname = format!("refs/heads/{}", &branch);
  repo.commit(
    Some(&refname),
    &signature,
    &signature,
    &message,
    &tree,
    &[&parent_commit],
  )
}

pub fn push_to_remote(repo: &Repository, branch: String) -> Result<(), Error> {
  let mut push_options = PushOptions::new();
  push_options.remote_callbacks(get_ssh_auth_callbacks());

  let push_refspec = format!("refs/heads/{}", &branch);
  debug!(
    "create_pr: pushing refspec {} to {}",
    push_refspec.clone(),
    FORK_REMOTE
  );

  repo
    .find_remote(FORK_REMOTE)?
    .push(&[push_refspec.clone()], Some(&mut push_options))
}

pub async fn create_pr(
  github: &Github,
  org: &str,
  repo_name: &str,
  fork_org: String,
  fork_branch: String,
  title: String,
  body: String,
  version: &str,
  action: &str,
) -> Result<String, hubcaps::Error> {
  let repo = github.repo(org, repo_name);
  let pr = PullOptions {
    base: UPSTREAM_BRANCH.to_string(),
    head: format!("{}:{}", fork_org, fork_branch),
    title: title,
    body: Some(body),
  };
  let pull = repo.pulls().create(&pr).await?;
  // Add version label
  let _ = get_or_create_label(github, org, repo_name, version, VERSION_LABEL_COLOR).await?;
  // Add action label
  let _ = get_or_create_label(github, org, repo_name, action, ACTION_LABEL_COLOR).await?;
  let pull_request = repo.pulls().get(pull.number);
  pull_request.labels().set(vec![version, action]).await?;
  Ok(pull.html_url.clone())
}

pub async fn get_or_create_label(
  github: &Github,
  org: &str,
  repo_name: &str,
  label_name: &str,
  color: &str,
) -> Result<Label, hubcaps::Error> {
  let repo = github.repo(org, repo_name);
  let mut lbl_stream = repo.labels().iter();
  while let Some(item) = lbl_stream.next().await {
    if item.is_err() {
      continue;
    }
    let lbl = item.unwrap();
    if lbl.name == label_name {
      return Ok(lbl);
    }
  }

  let lbl_opts = LabelOptions {
    name: label_name.to_string(),
    color: color.to_string(),
    description: label_name.to_string(),
  };
  repo.labels().create(&lbl_opts).await
}

pub async fn has_open_pr_for(
  github: &Github,
  org: String,
  repo_name: String,
  version: String,
) -> Result<Option<u64>, hubcaps::Error> {
  let repo = github.repo(org, repo_name);
  let pulls = repo.pulls();
  let mut pr_stream = pulls.iter(&Default::default());
  while let Some(item) = pr_stream.next().await {
    if item.is_err() {
      continue;
    }
    let pr = item.unwrap();
    if pr.labels.iter().find(|l| l.name == version).is_some()
      && pr.base.commit_ref == UPSTREAM_BRANCH
    {
      return Ok(Some(pr.number));
    }
  }
  Ok(None)
}

pub async fn comment_pr(
  github: &Github,
  org: String,
  repo_name: String,
  id: u64,
  comment: String,
) -> Result<String, hubcaps::Error> {
  let repo = github.repo(org, repo_name);
  let pr = repo.pulls().get(id);
  let comment_opts = CommentOptions { body: comment };
  let _ = pr.comments().create(&comment_opts).await;
  Ok(pr.get().await?.html_url.clone())
}

pub async fn get_pr_action(
  github: &Github,
  org: String,
  repo_name: String,
  id: u64,
) -> Result<String, hubcaps::Error> {
  let repo = github.repo(org, repo_name);
  let pr = repo.pulls().get(id);
  Ok(
    pr.get()
      .await?
      .labels
      .iter()
      .find(|l| l.color == ACTION_LABEL_COLOR)
      .map(|l| l.name.clone())
      .unwrap(),
  )
}

pub async fn close_pr(
  github: &Github,
  org: String,
  repo_name: String,
  id: u64,
) -> Result<String, hubcaps::Error> {
  let repo = github.repo(org, repo_name);
  let pr = repo.pulls().get(id);
  let _ = pr.close().await;
  Ok(pr.get().await?.html_url.clone())
}
