use crate::{action, git_repo};

use futures::prelude::*;
use log::debug;

use hubcaps::comments::CommentOptions;
use hubcaps::pulls::PullOptions;
use hubcaps::repositories::Repository;
use hubcaps::{Credentials, Github};

pub struct GithubRepo {
  repo: Repository,
}

impl GithubRepo {
  pub fn new(token: String, org_name: &str, repo_name: &str) -> Self {
    let client = Github::new("graph-breaker/0.1.0", Credentials::Token(token)).unwrap();
    let repo = client.repo(org_name, repo_name);
    GithubRepo { repo: repo }
  }

  pub async fn create_pr(
    &mut self,
    fork_org: &str,
    fork_branch: &str,
    action: action::Action,
  ) -> Result<String, hubcaps::Error> {
    let (title, body) = action.to_pr_tuple();

    let pr = PullOptions {
      base: git_repo::UPSTREAM_BRANCH.to_string(),
      head: format!("{}:{}", fork_org, fork_branch),
      title: title.to_string(),
      body: Some(body.to_string()),
    };
    let pull = self.repo.pulls().create(&pr).await?;
    Ok(pull.html_url.clone())
  }

  pub async fn has_open_pr_for(&mut self, version: &str) -> Result<Option<u64>, hubcaps::Error> {
    debug!("Looking for similar pull requests");
    let mut pr_stream = self.repo.pulls().iter(&Default::default());
    while let Some(item) = pr_stream.next().await {
      if item.is_err() {
        continue;
      }
      let pr = item.unwrap();
      debug!("Checking #{}: {}", pr.number, pr.title);
      // Check base branch
      if pr.base.commit_ref != git_repo::UPSTREAM_BRANCH {
        debug!("Wrong commit_ref: {}", pr.base.commit_ref);
        continue;
      }
      // Check PR title
      let title_iter = pr.title.split_whitespace();
      if title_iter.last() == Some(version) {
        debug!("Found matching PR");
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

  pub async fn get_action_from_pr_title(&mut self, id: u64) -> Result<String, hubcaps::Error> {
    let pr = self.repo.pulls().get(id);
    Ok(
      pr.get()
        .await?
        .title
        .split_whitespace()
        .next()
        .unwrap()
        .to_string(),
    )
  }

  pub async fn close_pr(&mut self, id: u64) -> Result<String, hubcaps::Error> {
    let pr = self.repo.pulls().get(id);
    let _ = pr.close().await;
    Ok(pr.get().await?.html_url.clone())
  }
}
