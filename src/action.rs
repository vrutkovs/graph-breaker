//! Available service actions

use crate::{config, github, graph_schema};

use anyhow::Error;
use log::debug;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tempfile::tempdir;

use hubcaps::{Credentials, Github};

const HASH_LENGTH: usize = 6;

#[derive(Debug, Serialize, Deserialize)]
pub enum ActionType {
  #[serde(alias = "enable")]
  Enable,
  #[serde(alias = "disable")]
  Disable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Action {
  r#type: ActionType,
  version: String,
  title: String,
  body: String,
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
  action: Action,
  settings: config::GithubSettings,
) -> Result<String, Error> {
  debug!("perform_action+");
  let client = Github::new(
    "graph-breaker/0.1.0",
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
  match action.r#type {
    ActionType::Disable => graph_schema::block_edge(&path, action.version.clone())?,
    ActionType::Enable => graph_schema::unblock_edge(&path, action.version.clone())?,
  };

  let branch = generate_branch_name(action.title.clone());
  debug!("Generated branch {}", branch.clone());
  github::switch_to(&repo, branch.clone())?;

  debug!(
    "Pushing to {}/{}",
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
  );
  let commit_message = format!("{}\n{}", action.title, action.body);
  github::commit(&repo, branch.clone(), commit_message)?;
  github::push_to_remote(&repo, branch.clone())?;

  debug!("Preparing pull request");
  let pr_url = github::create_pr(
    &client,
    settings.target_organization.clone(),
    settings.target_repo.clone(),
    settings.fork_organization.clone(),
    branch.clone(),
    action.title,
    action.body,
  )
  .await?;
  debug!("Created PR {}", pr_url);

  Ok(pr_url)
}
