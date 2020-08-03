//! Available service actions

use crate::anyhow::Context;
use crate::{config, git_repo, github, graph_schema};

use anyhow::Error;
use log::debug;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tempfile::tempdir;

const HASH_LENGTH: usize = 6;

#[derive(Debug, Serialize, Deserialize, std::cmp::PartialEq)]
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

impl Action {
  /// Return necessary data for PR - title, body, version label, action label
  pub fn to_pr_tuple(&self) -> (&str, &str, &str, String) {
    let action_str_json = serde_json::to_string(&self.r#type)
      .context("Failed to serialize action type")
      .unwrap();
    // json serialization adds quotes around the value
    let action_str = action_str_json.trim_matches('"');
    return (
      self.title.as_str(),
      self.body.as_str(),
      self.version.as_str(),
      action_str.to_string(),
    );
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
  action: Action,
  settings: config::GithubSettings,
) -> Result<String, Error> {
  debug!("perform_action+");

  let mut github_repo = github::GithubRepo::new(
    settings.token,
    settings.target_organization.as_str(),
    settings.target_repo.as_str(),
  );

  debug!("Looking for similar pull requests");
  let maybe_pr_id = github_repo.has_open_pr_for(action.version.as_str()).await?;
  if maybe_pr_id.is_some() {
    let pr_id = maybe_pr_id.unwrap();
    debug!("Updating existing PR ID {:?}", pr_id);
    let pr_url = github_repo
      .comment_in_pr(pr_id, action.body.as_str())
      .await?;
    // Close the pr if actions don't match
    let pr_action = github_repo.get_action_from_pr_labels(pr_id).await?;
    let pr_action_type: Result<ActionType, serde_json::Error> =
      serde_json::from_value(serde_json::Value::String(pr_action));
    match pr_action_type {
      Err(_) => {
        // No action found in the original PR
        return github_repo
          .close_pr(pr_id)
          .await
          .map_err(|e| anyhow!("Couldn't close PR: {}", e));
      }
      Ok(action_type) => {
        // Close PR if actions don't match
        if action_type != action.r#type {
          return github_repo
            .close_pr(pr_id)
            .await
            .map_err(|e| anyhow!("Couldn't close PR: {}", e));
        } else {
          return Ok(pr_url);
        }
      }
    }
  }

  let tmpdir = tempdir().context("Failed to create tempdir")?;
  let path = tmpdir.path().to_path_buf();

  debug!(
    "Cloning {}/{} to {}",
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
    path
      .to_str()
      .context("Failed to convert path to string")?
      .clone(),
  );
  let repo = git_repo::refresh_forked_repo(
    settings.target_organization.clone(),
    settings.target_repo.clone(),
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
    &path,
  )?;

  debug!("Calculating action");
  match action.r#type {
    ActionType::Disable => graph_schema::block_edge(&path, action.version.clone()),
    ActionType::Enable => graph_schema::unblock_edge(&path, action.version.clone()),
  }
  .context("Failed to perform action")?;

  let branch = generate_branch_name(action.title.clone());
  debug!("Generated branch {}", branch.clone());
  git_repo::switch_to(&repo, &branch.to_string()).context("Failed to switch to branch")?;

  debug!(
    "Pushing to {}/{}",
    settings.fork_organization.clone(),
    settings.fork_repo.clone(),
  );
  let commit_message = format!("{}\n{}", action.title, action.body);
  git_repo::commit(&repo, &branch, commit_message).context("Failed to commit changes")?;
  git_repo::push_to_remote(&repo, &branch).context("Failed to push to remote")?;

  debug!("Creating new PR");
  github_repo
    .create_pr(settings.fork_organization.as_str(), &branch, action)
    .await
    .map_err(|e| anyhow!("Couldn't create PR: {}", e))
}
