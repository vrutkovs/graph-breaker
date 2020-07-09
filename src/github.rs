use anyhow::Error;
use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{
  Commit, Cred, FetchOptions, IndexAddOption, ObjectType, Oid, PushOptions, Remote,
  RemoteCallbacks, Repository, ResetType, Signature,
};
use github_rs::client::Github;
use log::debug;
use std::env;
use std::path::PathBuf;

const FORK_REMOTE: &str = "origin";
const UPSTREAM_REMOTE: &str = "upstream";
const UPSTREAM_BRANCH: &str = "master";
const SIGNATURE_AUTHOR: &str = "Openshift OTA Bot";
const SIGNATURE_EMAIL: &str = "vrutkovs@redhat.com";

fn clone_repo(org: String, repo: String, path: &PathBuf) -> Result<Repository, Error> {
  // Authentication
  let mut builder = RepoBuilder::new();
  let mut callbacks = RemoteCallbacks::new();
  let mut fetch_options = FetchOptions::new();

  callbacks.credentials(|_url, username_from_url, _allowed_types| {
    Cred::ssh_key(
      username_from_url.unwrap(),
      None,
      std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
      None,
    )
  });

  fetch_options.remote_callbacks(callbacks);
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

  git_repo
    .remote(UPSTREAM_REMOTE, &url)
    .map_err(|e| anyhow!(e.message().to_string()))
}

fn fetch_from_upstream(
  repo: &Repository,
  target_org: String,
  target_user: String,
) -> Result<(), Error> {
  debug!("fetch_from_upstream+");
  let mut remote = add_fetch_remote(&repo, target_org, target_user)?;

  let mut callbacks = RemoteCallbacks::new();
  let mut fetch_options = FetchOptions::new();
  callbacks.credentials(|_url, username_from_url, _allowed_types| {
    Cred::ssh_key(
      username_from_url.unwrap(),
      None,
      std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
      None,
    )
  });

  fetch_options.remote_callbacks(callbacks);
  remote.fetch(&[UPSTREAM_BRANCH], Some(&mut fetch_options), None)?;
  let remote_refspec = format!("{}/{}", UPSTREAM_REMOTE, UPSTREAM_BRANCH);
  debug!("fetch_from_upstream: refspec {}", remote_refspec);
  let fetch_head = repo.revparse_single(&remote_refspec)?;
  debug!("fetch_from_upstream: fetch_head {}", fetch_head.id());
  let mut cb = CheckoutBuilder::new();
  repo
    .reset(&fetch_head, ResetType::Hard, Some(cb.force()))
    .map_err(|e| anyhow!(e.message().to_string()))
}

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
  let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
  obj
    .into_commit()
    .map_err(|_| git2::Error::from_str("Couldn't find commit"))
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
  repo
    .set_head(&refname)
    .map_err(|e| anyhow!(e.message().to_string()))
}

pub fn commit(repo: &Repository, branch: String, message: String) -> Result<Oid, Error> {
  // Stage all files
  let mut index = repo.index()?;
  index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
  let oid = index.write_tree()?;
  // Prepare commit metadata
  let signature = Signature::now(SIGNATURE_AUTHOR, SIGNATURE_EMAIL)?;
  let parent_commit = find_last_commit(&repo)?;
  let tree = repo.find_tree(oid)?;
  // Create a new HEAD commit
  let refname = format!("refs/heads/{}", &branch);
  repo
    .commit(
      Some(&refname),
      &signature,
      &signature,
      &message,
      &tree,
      &[&parent_commit],
    )
    .map_err(|e| anyhow!(e.message().to_string()))
}

pub fn push_to_remote(repo: &Repository, branch: String) -> Result<(), Error> {
  let mut callbacks = RemoteCallbacks::new();
  let mut push_options = PushOptions::new();

  callbacks.credentials(|_url, username_from_url, _allowed_types| {
    Cred::ssh_key(
      username_from_url.unwrap(),
      None,
      std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
      None,
    )
  });

  push_options.remote_callbacks(callbacks);

  let push_refspec = format!("refs/heads/{}", &branch);
  debug!(
    "create_pr: pushing refspec {} to {}",
    push_refspec.clone(),
    FORK_REMOTE
  );

  repo
    .find_remote(FORK_REMOTE)?
    .push(&[push_refspec.clone()], Some(&mut push_options))
    .map_err(|e| anyhow!(e.message().to_string()))
}

pub fn create_pr(
  client: &Github,
  git_repo: &Repository,
  org: String,
  repo: String,
  branch: String,
) -> Result<String, Error> {
  // client.get().orgs().org(&org).repos().repo(&repo);
  // todo!();
  Ok("https://github.com/foo".to_string())
}
