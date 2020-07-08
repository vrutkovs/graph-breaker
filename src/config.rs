//! Application settings for graph-breaker.
use anyhow::{Context, Result};
pub use smart_default::SmartDefault;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::{fs, io, path};
use structopt::StructOpt;

#[macro_export]
/// Assign to destination if source value is `Some`.
macro_rules! assign_if_some {
  ( $dst:expr, $src:expr ) => {{
    if let Some(x) = $src {
      $dst = x.into();
    };
  }};
}

/// Deserialize a log-level from a numerical value.
pub fn de_loglevel<'de, D>(deserializer: D) -> Result<Option<log::LevelFilter>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  use serde::Deserialize;
  let occurrences = String::deserialize(deserializer)?;

  let verbosity = match occurrences.as_str() {
    "" => log::LevelFilter::Warn,
    "v" => log::LevelFilter::Info,
    "vv" => log::LevelFilter::Debug,
    _ => log::LevelFilter::Trace,
  };
  Ok(Some(verbosity))
}

/// Try to merge configuration options into runtime settings.
///
/// This consumes a generic configuration object, trying to merge its options
/// into runtime settings. It only overlays populated values from config,
/// leaving unset ones preserved as-is from existing settings.
pub trait MergeOptions<T> {
  /// MergeOptions values from `options` into current settings.
  fn try_merge(&mut self, options: T) -> Result<()>;
}

// Runtime application settings (validated config).
#[derive(Debug, SmartDefault, Clone)]
pub struct AppSettings {
  /// Global log level.
  #[default(log::LevelFilter::Warn)]
  pub verbosity: log::LevelFilter,

  /// App options.
  pub service: ServiceSettings,

  /// Github options.
  pub github: GithubSettings,
}

impl AppSettings {
  /// Lookup all optional configs, merge them with defaults, and
  /// transform into valid runtime settings.
  pub fn assemble() -> Result<Self> {
    let defaults = Self::default();
    let mut cfg = defaults;

    // Source options.
    let cli_opts = CliOptions::from_args();

    // File options are required
    let file_opts = match FileOptions::read_filepath(cli_opts.config_path.clone()) {
      Ok(opts) => opts,
      Err(e) => return Err(e),
    };

    // Combine options into a single config.
    cfg.try_merge(cli_opts)?;
    cfg.try_merge(file_opts.clone())?;

    cfg.service = file_opts.service;
    cfg.github = file_opts.github;

    // Validate and convert to settings.
    Ok(cfg)
  }
}

/// CLI configuration flags
#[derive(Debug, StructOpt)]
pub struct CliOptions {
  /// Verbosity level
  #[structopt(short = "v", parse(from_occurrences))]
  pub verbosity: u8,

  /// Path to configuration file
  #[structopt(short = "c")]
  pub config_path: String,
}

impl MergeOptions<CliOptions> for AppSettings {
  fn try_merge(&mut self, opts: CliOptions) -> Result<()> {
    self.verbosity = match opts.verbosity {
      0 => self.verbosity,
      1 => log::LevelFilter::Info,
      2 => log::LevelFilter::Debug,
      _ => log::LevelFilter::Trace,
    };

    Ok(())
  }
}

/// File configuration schema
#[derive(Debug, Deserialize, Clone)]
pub struct FileOptions {
  /// Verbosity level.
  #[serde(default = "Option::default", deserialize_with = "de_loglevel")]
  pub verbosity: Option<log::LevelFilter>,

  /// App options.
  pub service: ServiceSettings,

  /// Github options.
  pub github: GithubSettings,
}

/// Service settings
#[derive(Debug, SmartDefault, Deserialize, Clone)]
pub struct ServiceSettings {
  /// Listening address for the main service.
  #[default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
  pub address: IpAddr,

  /// Listening port for the main service.
  #[default(8080)]
  pub port: u16,

  /// Client auth token
  pub client_auth_token: String,
}

/// Github settings
#[derive(Debug, SmartDefault, Deserialize, Clone)]
pub struct GithubSettings {
  /// Path to github token
  pub token: String,

  /// Target github org
  #[default("openshift")]
  pub target_organization: String,

  /// Target github repo
  #[default("cincinnati-graph-data")]
  pub target_repo: String,

  /// Fork github org/user
  #[default("vrutkovs")]
  pub fork_organization: String,

  /// Fork github repo
  #[default("cincinnati-graph-data")]
  pub fork_repo: String,
}

impl FileOptions {
  pub fn read_filepath<P>(cfg_path: P) -> Result<Self>
  where
    P: AsRef<path::Path>,
  {
    let cfg_file = fs::File::open(&cfg_path).context(format!(
      "failed to open config path {:?}",
      cfg_path.as_ref()
    ))?;
    let mut bufrd = io::BufReader::new(cfg_file);

    let mut content = vec![];
    bufrd.read_to_end(&mut content)?;
    let cfg = toml::from_slice(&content).context(format!(
      "failed to parse config file {}:\n{}",
      cfg_path.as_ref().display(),
      std::str::from_utf8(&content).unwrap_or("file not decodable")
    ))?;

    Ok(cfg)
  }
}

impl MergeOptions<FileOptions> for AppSettings {
  fn try_merge(&mut self, opts: FileOptions) -> Result<()> {
    assign_if_some!(self.verbosity, opts.verbosity);
    Ok(())
  }
}
