use anyhow::Error;
use std::fs::{remove_file, File};
use std::path::{Path, PathBuf};

const BLOCKED_DIR: &str = "blocked-edges";
const ALL_VERSIONS_REGEXP: &str = ".*";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct BlockedEdge {
  to: String,
  from: String,
}

pub fn block_edge(path: &Path, version: String) -> Result<(), Error> {
  let new_edge = BlockedEdge {
    to: version.clone(),
    from: ALL_VERSIONS_REGEXP.to_string(),
  };
  let edge_path: PathBuf = [
    path.to_str().unwrap().to_string(),
    BLOCKED_DIR.to_string(),
    format!("{}.yaml", version),
  ]
  .iter()
  .collect();

  let f = File::create(edge_path)?;
  serde_yaml::to_writer(f, &new_edge).map_err(|e| anyhow!(e.to_string()))
}

pub fn unblock_edge(path: &Path, version: String) -> Result<(), Error> {
  let edge_path: PathBuf = [
    path.to_str().unwrap().to_string(),
    BLOCKED_DIR.to_string(),
    format!("{}.yaml", version),
  ]
  .iter()
  .collect();
  match edge_path.exists() {
    true => remove_file(edge_path).map_err(|e| anyhow!(e.to_string())),
    false => Ok(()),
  }
}
