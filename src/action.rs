//! Available service actions

use crate::{config, errors, github, graph_schema};
use anyhow::Error;

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
  let path = github::refresh_forked_repo(
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
    settings.target_organization.clone(),
    settings.target_repo.clone(),
  )?;
  match action_type {
    ActionType::Disable => graph_schema::block_edge(version.clone())?,
    ActionType::Enable => graph_schema::unblock_edge(version.clone())?,
  };

  let commit_title = format!("Block edge {}", version.clone());
  let commit_body = "2 clusters currently failing (10%), 5 gone (25%), and 13 successful (65%), out of 20 who attempted the update over 7d";
  github::commit(path, commit_title, commit_body.to_string())?;
  let pr_url = github::create_pr(path)?;
  github::destroy_repo(path)?;

  Ok(pr_url)
}
