use log::debug;
use std::env;
use std::path::PathBuf;

use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{
  Cred, Error, FetchOptions, IndexAddOption, ObjectType, Oid, PushOptions, RemoteCallbacks,
  Repository, ResetType, Signature,
};

const FORK_REMOTE: &str = "origin";
const UPSTREAM_REMOTE: &str = "upstream";
pub const UPSTREAM_BRANCH: &str = "master";
const SIGNATURE_AUTHOR: &str = "Openshift OTA Bot";
const SIGNATURE_EMAIL: &str = "vrutkovs@redhat.com";

pub struct GitRepo {
  repo: Repository,
}

impl GitRepo {
  pub fn new(org: &str, repo: &str, path: &PathBuf) -> Result<Self, Error> {
    debug!("new: cloning {}/{} to {}", org, repo, path.display());
    // Authentication
    let mut builder = RepoBuilder::new();
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(get_ssh_auth_callbacks());
    builder.fetch_options(fetch_options);

    let url = format!("git@github.com:{}/{}.git", org, repo);
    let repo = builder.clone(&url, &path)?;
    debug!("new: done");
    Ok(GitRepo { repo })
  }

  pub fn fetch_from_upstream(&mut self, org_name: &str, repo_name: &str) -> Result<(), Error> {
    let url = format!("https://github.com/{}/{}.git", org_name, repo_name);
    debug!("fetch_from_upstream: {}", url);
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(get_ssh_auth_callbacks());

    let mut remote = self.repo.remote(UPSTREAM_REMOTE, &url)?;
    remote.fetch(&[UPSTREAM_BRANCH], Some(&mut fetch_options), None)?;

    let remote_refspec = format!("{}/{}", UPSTREAM_REMOTE, UPSTREAM_BRANCH);
    debug!("fetch_from_upstream: refspec {}", remote_refspec);
    let fetch_head = self.repo.revparse_single(&remote_refspec)?;
    debug!("fetch_from_upstream: fetch_head {}", fetch_head.id());

    let mut cb = CheckoutBuilder::new();
    self
      .repo
      .reset(&fetch_head, ResetType::Hard, Some(cb.force()))
  }

  pub fn switch_to(&mut self, branch: &str) -> Result<(), Error> {
    let commit = self.repo.head()?.peel_to_commit()?;
    self.repo.branch(&branch, &commit, true)?;
    let refname = format!("refs/heads/{}", &branch);
    let obj = self.repo.revparse_single(&refname)?;
    self.repo.checkout_tree(&obj, None)?;
    self.repo.set_head(&refname)
  }

  pub fn commit(&mut self, branch: &str, message: String) -> Result<Oid, Error> {
    // Stage all files
    let mut index = self.repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    // No files included - skip
    if index.len() == 0 {
      return Err(git2::Error::from_str("empty commit detected"));
    }
    let oid = index.write_tree()?;
    // Prepare commit metadata
    let signature = Signature::now(SIGNATURE_AUTHOR, SIGNATURE_EMAIL)?;
    let obj = self.repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    let parent_commit = obj
      .into_commit()
      .map_err(|_| Error::from_str("Couldn't find commit"))?;
    let tree = self.repo.find_tree(oid)?;
    // Create a new HEAD commit
    let refname = format!("refs/heads/{}", &branch);
    self.repo.commit(
      Some(&refname),
      &signature,
      &signature,
      &message,
      &tree,
      &[&parent_commit],
    )
  }

  pub fn push_to_remote(&mut self, branch: &str) -> Result<(), Error> {
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(get_ssh_auth_callbacks());

    let push_refspec = format!("refs/heads/{}", &branch);
    debug!(
      "create_pr: pushing refspec {} to {}",
      push_refspec.clone(),
      FORK_REMOTE
    );

    self
      .repo
      .find_remote(FORK_REMOTE)?
      .push(&[push_refspec.clone()], Some(&mut push_options))
  }
}

fn get_ssh_auth_callbacks<'cb>() -> RemoteCallbacks<'cb> {
  let mut callbacks = RemoteCallbacks::new();
  callbacks.credentials(|_url, username_from_url, _allowed_types| {
    Cred::ssh_key(
      username_from_url.unwrap(),
      None,
      std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
      None,
    )
  });
  callbacks
}
