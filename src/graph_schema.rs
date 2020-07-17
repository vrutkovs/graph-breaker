use anyhow::Error;
use std::fs;
use std::path::{Path, PathBuf};

const BLOCKED_DIR: &str = "blocked-edges";
const ALL_VERSIONS_REGEXP: &str = ".*";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct BlockedEdge {
  to: String,
  from: String,
}

fn generate_yml_path(path: &Path, version: String) -> PathBuf {
  [
    path.to_str().unwrap().to_string(),
    BLOCKED_DIR.to_string(),
    format!("{}.yaml", version),
  ]
  .iter()
  .collect()
}

pub fn block_edge(path: &Path, version: String) -> Result<(), Error> {
  let new_edge = BlockedEdge {
    to: version.clone(),
    from: ALL_VERSIONS_REGEXP.to_string(),
  };
  let f = fs::File::create(generate_yml_path(path, version))?;
  serde_yaml::to_writer(f, &new_edge).map_err(|e| anyhow!(e.to_string()))
}

pub fn unblock_edge(path: &Path, version: String) -> Result<(), Error> {
  let edge_path = generate_yml_path(path, version);
  match edge_path.exists() {
    true => fs::remove_file(edge_path).map_err(|e| anyhow!(e.to_string())),
    false => Ok(()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;
  use tempfile::tempdir;

  #[test]
  fn block_edge_new_file() {
    let tmpdir = tempdir().unwrap();
    let base_path = Path::new(tmpdir.path());
    std::fs::create_dir(base_path.join(BLOCKED_DIR)).unwrap();
    let version = "0.0.0".to_string();

    let result = block_edge(base_path, version.clone());
    assert!(result.is_ok());

    let f = fs::File::open(generate_yml_path(base_path, version.clone()));
    let edge: BlockedEdge = serde_yaml::from_reader(f.unwrap()).unwrap();
    assert_eq!(edge.to, version.clone());
    assert_eq!(edge.from, ALL_VERSIONS_REGEXP);
  }

  #[test]
  fn block_edge_existing_file() {
    let tmpdir = tempdir().unwrap();
    let base_path = Path::new(tmpdir.path());
    std::fs::create_dir(base_path.join(BLOCKED_DIR)).unwrap();
    let version = "0.0.0".to_string();
    let expected_path = generate_yml_path(base_path, version.clone());

    let mut f = fs::File::create(expected_path.clone()).unwrap();
    f.write_all(b"Hello, world!").unwrap();
    drop(f);

    let result = block_edge(base_path, version.clone());
    assert!(result.is_ok());

    f = fs::File::open(expected_path.clone()).unwrap();
    let edge: BlockedEdge = serde_yaml::from_reader(f).unwrap();
    assert_eq!(edge.to, version.clone());
    assert_eq!(edge.from, ALL_VERSIONS_REGEXP);
  }

  #[test]
  fn unblock_edge_new_file() {
    let tmpdir = tempdir().unwrap();
    let base_path = Path::new(tmpdir.path());
    std::fs::create_dir(base_path.join(BLOCKED_DIR)).unwrap();
    let version = "0.0.0".to_string();

    let result = unblock_edge(base_path, version.clone());
    assert!(result.is_ok());
    assert!(!generate_yml_path(base_path, version.clone()).exists());
  }

  #[test]
  fn unblock_edge_existing_file() {
    let tmpdir = tempdir().unwrap();
    let base_path = Path::new(tmpdir.path());
    std::fs::create_dir(base_path.join(BLOCKED_DIR)).unwrap();
    let version = "0.0.0".to_string();
    let expected_path = generate_yml_path(base_path, version.clone());

    let mut f = fs::File::create(expected_path.clone()).unwrap();
    f.write_all(b"Hello, world!").unwrap();
    drop(f);

    let result = unblock_edge(base_path, version.clone());
    assert!(result.is_ok());
    assert!(!expected_path.exists());
  }
}
