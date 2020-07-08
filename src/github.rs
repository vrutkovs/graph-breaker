use anyhow::Error;
use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{
  Commit, Cred, FetchOptions, IndexAddOption, ObjectType, Oid, RemoteCallbacks, Repository,
  ResetType, Signature,
};
use github_rs::client::Github;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const FORK_REMOTE: &str = "origin";
const FORK_BRANCH: &str = "master";
const UPSTREAM_REMOTE: &str = "upstream";
const UPSTREAM_BRANCH: &str = "master";
const SIGNATURE_AUTHOR: &str = "Openshift OTA Bot";
const SIGNATURE_EMAIL: &str = "vrutkovs@redhat.com";

fn clone_repo(org: String, repo: String) -> Result<PathBuf, Error> {
  // Authentication
  let mut builder = RepoBuilder::new();
  let mut callbacks = RemoteCallbacks::new();
  let mut fetch_options = FetchOptions::new();

  callbacks.credentials(|_, _, _| {
    let ssh_pubkey_path = dirs::home_dir().unwrap().join(".ssh").join("id_rsa.pub");
    let ssh_secretkey_path = dirs::home_dir().unwrap().join(".ssh").join("id_rsa");
    let credentials = Cred::ssh_key(
      "git",
      Some(ssh_pubkey_path.as_path()),
      ssh_secretkey_path.as_path(),
      None,
    )
    .expect("Could not create credentials object");

    Ok(credentials)
  });

  fetch_options.remote_callbacks(callbacks);
  builder.fetch_options(fetch_options);

  let url = format!("git@github.com:{}/{}.git", org, repo);
  let tmpdir = tempdir()?;
  builder.clone(&url, tmpdir.path())?;
  Ok(tmpdir.path().to_owned())
}

fn add_fetch_remote(git_repo: &Repository, org: String, repo: String) -> Result<(), Error> {
  let url = format!("https://github.com/{}/{}.git", org, repo);
  git_repo
    .remote_add_fetch(UPSTREAM_REMOTE, &url)
    .map_err(|e| anyhow!(e.message().to_string()))
}

fn fetch_from_upstream(repo: &Repository) -> Result<(), Error> {
  repo
    .find_remote(UPSTREAM_REMOTE)?
    .fetch(&[UPSTREAM_BRANCH], None, None)?;
  let remote_refspec = format!("{}/{}", UPSTREAM_REMOTE, UPSTREAM_BRANCH);
  let fetch_head = repo.revparse_single(&remote_refspec)?;
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
) -> Result<(PathBuf, Repository), Error> {
  let path = clone_repo(forked_org, forked_user)?;
  let repo = Repository::open(path.clone())?;
  add_fetch_remote(&repo, target_org, target_user)?;
  fetch_from_upstream(&repo)?;
  Ok((path.to_owned(), repo))
}

pub fn commit(repo: &Repository, message: String) -> Result<Oid, Error> {
  // Stage all files
  let mut index = repo.index()?;
  index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
  let oid = index.write_tree()?;
  // Prepare commit metadata
  let signature = Signature::now(SIGNATURE_AUTHOR, SIGNATURE_EMAIL)?;
  let parent_commit = find_last_commit(&repo)?;
  let tree = repo.find_tree(oid)?;
  // Create a new HEAD commit
  repo
    .commit(
      Some("HEAD"),
      &signature,
      &signature,
      &message,
      &tree,
      &[&parent_commit],
    )
    .map_err(|e| anyhow!(e.message().to_string()))
}

pub fn create_pr(
  client: &Github,
  git_repo: &Repository,
  org: String,
  repo: String,
) -> Result<String, Error> {
  git_repo
    .find_remote(FORK_REMOTE)?
    .push(&[FORK_BRANCH], None)?;
  // client.get().orgs().org(&org).repos().repo(&repo);
  // todo!();
  Ok("https://github.com/foo".to_string())
}

pub fn destroy_repo(path: &Path) -> Result<(), Error> {
  fs::remove_dir_all(path).map_err(|e| anyhow!(e.to_string()))
}
