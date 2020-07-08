use anyhow::Error;
use std::path::Path;

const BRANCH: &str = "master";
const FORK_REMOTE: &str = "origin";
const UPSTREAM_REMOTE: &str = "upstream";

fn clone_repo(org: String, user: String, remote: String) -> Result<&'static Path, Error> {
  todo!();
}

fn add_remote(org: String, user: String, remote: String) -> Result<&'static Path, Error> {
  todo!();
}

fn fetch_remote(path: &Path, remote: String) -> Result<String, Error> {
  todo!();
}

fn stage_all_changes(path: &Path) -> Result<(), Error> {
  todo!();
}

fn push_to_remote(path: &Path, remote: &str) -> Result<(), Error> {
  todo!();
}

pub fn refresh_forked_repo(
  target_org: String,
  target_user: String,
  forked_org: String,
  forked_user: String,
) -> Result<&'static Path, Error> {
  let path = clone_repo(forked_org, forked_user, FORK_REMOTE.to_string())?;
  add_remote(target_org, target_user, UPSTREAM_REMOTE.to_string())?;
  fetch_remote(path, UPSTREAM_REMOTE.to_string())?;
  Ok(path)
}

pub fn commit(path: &Path, _title: String, _body: String) -> Result<(), Error> {
  stage_all_changes(path)?;
  todo!();
}

pub fn create_pr(path: &Path) -> Result<String, Error> {
  push_to_remote(path, FORK_REMOTE)?;
  todo!();
}

pub fn destroy_repo(_path: &Path) -> Result<(), Error> {
  todo!();
}
