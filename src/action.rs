//! Available service actions

use crate::{config, errors, github, graph_schema};
use anyhow::Error;
use log::debug;
use std::path::PathBuf;
use tempfile::tempdir;

use github_rs::client::Github;

pub enum ActionType {
  Enable,
  Disable,
}

/// Check that valid action type is specified
pub fn ensure_valid_action_type(value: &str) -> Result<ActionType, errors::AppError> {
  match value {
    "enable" => return Ok(ActionType::Enable),
    "disable" => return Ok(ActionType::Disable),
    a => return Err(errors::AppError::InvalidAction(a.to_string())),
  }
}

/// Create a PR from specified action
pub fn perform_action(
  action_type: ActionType,
  version: String,
  settings: config::GithubSettings,
) -> Result<String, Error> {
  debug!("perform_action+");
  let client =
    Github::new(settings.token.clone()).map_err(|_| errors::AppError::InvalidGithubToken())?;

  let tmpdir = tempdir()?;
  let path = tmpdir.path().to_path_buf();

  debug!(
    "Cloning {}/{} to {}",
    settings.target_organization.clone(),
    settings.target_repo.clone(),
    path.to_str().unwrap().clone(),
  );
  let repo = github::refresh_forked_repo(
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
    settings.target_organization.clone(),
    settings.target_repo.clone(),
    &path,
  )?;
  debug!("Calculating action");
  // match action_type {
  //   ActionType::Disable => graph_schema::block_edge(version.clone())?,
  //   ActionType::Enable => graph_schema::unblock_edge(version.clone())?,
  // };

  let branch = String::from("jul-8");
  github::switch_to(&repo, branch.clone())?;

  let commit_message = format!("Block edge {}

  2 clusters currently failing (10%), 5 gone (25%), and 13 successful (65%), out of 20 who attempted the update over 7d", version.clone());
  github::commit(&repo, branch.clone(), commit_message)?;
  github::push_to_remote(&repo, branch.clone())?;
  let pr_url = github::create_pr(
    &client,
    &repo,
    settings.target_organization.clone(),
    settings.target_repo.clone(),
    branch.clone(),
  )?;
  debug!("Created PR {}", pr_url);
  debug!("perform_action-");

  Ok(pr_url)
}
