use crate::{action, git_repo};

use futures::prelude::*;
use hubcaps::comments::CommentOptions;
use hubcaps::labels::{Label, LabelOptions};
use hubcaps::pulls::PullOptions;
use hubcaps::repositories::Repository;
use hubcaps::{Credentials, Github};

const VERSION_LABEL_COLOR: &str = "0e8a16";
const ACTION_LABEL_COLOR: &str = "0052cc";

pub struct GithubRepo {
  repo: Repository,
}

impl GithubRepo {
  pub fn new(token: String, org_name: &str, repo_name: &str) -> Self {
    let client = Github::new("graph-breaker/0.1.0", Credentials::Token(token)).unwrap();
    let repo = client.repo(org_name, repo_name);
    GithubRepo { repo: repo }
  }

  async fn get_or_create_label(
    &mut self,
    name: &str,
    color: &str,
  ) -> Result<Label, hubcaps::Error> {
    let mut lbl_stream = self.repo.labels().iter();
    while let Some(item) = lbl_stream.next().await {
      if item.is_err() {
        continue;
      }
      let lbl = item.unwrap();
      if lbl.name == name {
        return Ok(lbl);
      }
    }

    let lbl_opts = LabelOptions {
      name: name.to_string(),
      color: color.to_string(),
      description: name.to_string(),
    };
    self.repo.labels().create(&lbl_opts).await
  }

  pub async fn create_pr(
    &mut self,
    fork_org: &str,
    fork_branch: &str,
    action: action::Action,
  ) -> Result<String, hubcaps::Error> {
    let (title, body, version_label, action_label) = action.to_pr_tuple();

    let pr = PullOptions {
      base: git_repo::UPSTREAM_BRANCH.to_string(),
      head: format!("{}:{}", fork_org, fork_branch),
      title: title.to_string(),
      body: Some(body.to_string()),
    };
    let pull = self.repo.pulls().create(&pr).await?;
    // Add version label
    let _ = self
      .get_or_create_label(version_label, VERSION_LABEL_COLOR)
      .await?;
    // Add action label
    let _ = self
      .get_or_create_label(action_label.as_str(), ACTION_LABEL_COLOR)
      .await?;
    let pull_request = self.repo.pulls().get(pull.number);
    pull_request
      .labels()
      .set(vec![version_label, action_label.as_str()])
      .await?;
    Ok(pull.html_url.clone())
  }

  pub async fn has_open_pr_for(&mut self, version: &str) -> Result<Option<u64>, hubcaps::Error> {
    let mut pr_stream = self.repo.pulls().iter(&Default::default());
    while let Some(item) = pr_stream.next().await {
      if item.is_err() {
        continue;
      }
      let pr = item.unwrap();
      if pr.labels.iter().find(|l| l.name == version).is_some()
        && pr.base.commit_ref == git_repo::UPSTREAM_BRANCH
      {
        return Ok(Some(pr.number));
      }
    }
    Ok(None)
  }

  pub async fn comment_in_pr(&mut self, id: u64, comment: &str) -> Result<String, hubcaps::Error> {
    let pr = self.repo.pulls().get(id);
    let comment_opts = CommentOptions {
      body: comment.to_string(),
    };
    let _ = pr.comments().create(&comment_opts).await;
    Ok(pr.get().await?.html_url.clone())
  }

  pub async fn get_action_from_pr_labels(&mut self, id: u64) -> Result<String, hubcaps::Error> {
    let pr = self.repo.pulls().get(id);
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

  pub async fn close_pr(&mut self, id: u64) -> Result<String, hubcaps::Error> {
    let pr = self.repo.pulls().get(id);
    let _ = pr.close().await;
    Ok(pr.get().await?.html_url.clone())
  }
}
