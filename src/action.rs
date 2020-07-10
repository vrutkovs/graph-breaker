//! Available service actions

use crate::{config, errors, github, graph_schema};

use anyhow::Error;
use log::debug;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tempfile::tempdir;

use hubcaps::{Credentials, Github};

const HASH_LENGTH: usize = 6;

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

/// Generate a new branch name
fn generate_branch_name(title: String) -> String {
  let rand_string: String = thread_rng()
    .sample_iter(&Alphanumeric)
    .take(HASH_LENGTH)
    .collect();

  let ascii_title = title
    .to_lowercase()
    .replace(|c: char| !c.is_ascii(), "")
    .replace(|c: char| c == ' ', "-");
  format!("{}-{}", ascii_title, rand_string)
}

/// Create a PR from specified action
pub async fn perform_action(
  action_type: ActionType,
  version: String,
  settings: config::GithubSettings,
) -> Result<String, Error> {
  debug!("perform_action+");
  let client = Github::new(
    "my-cool-user-agent/0.1.0",
    Credentials::Token(settings.token.clone()),
  )?;

  let tmpdir = tempdir()?;
  let path = tmpdir.path().to_path_buf();

  debug!(
    "Cloning {}/{} to {}",
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
    path.to_str().unwrap().clone(),
  );
  let repo = github::refresh_forked_repo(
    settings.target_organization.clone(),
    settings.target_repo.clone(),
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
    &path,
  )?;
  debug!("Calculating action");
  match action_type {
    ActionType::Disable => graph_schema::block_edge(&path, version.clone())?,
    ActionType::Enable => graph_schema::unblock_edge(&path, version.clone())?,
  };

  let branch = generate_branch_name(action.title.clone());
  debug!("Generated branch {}", branch.clone());
  github::switch_to(&repo, branch.clone())?;

  let pull_request_title = format!("Unblock edge {}", version.clone());
  let description = "6 clusters currently failing ( 8%),  8 gone (11%), and 61 successful (80%), out of 76 who attempted the update over 7d";
  let commit_message = format!("{}\n{}", pull_request_title, description);
  github::commit(&repo, branch.clone(), commit_message)?;
  github::push_to_remote(&repo, branch.clone())?;
  let pr_url = github::create_pr(
    &client,
    settings.target_organization.clone(),
    settings.target_repo.clone(),
    settings.fork_organization.clone(),
    branch.clone(),
    pull_request_title,
    description.to_string(),
  )
  .await?;
  debug!("Created PR {}", pr_url);

  Ok(pr_url)
}
