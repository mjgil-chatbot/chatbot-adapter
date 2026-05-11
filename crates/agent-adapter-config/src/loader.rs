//! Config file loading and parsing.
//!
//! This module loads TOML config files for provider configurations.

use std::path::{Path, PathBuf};
use thiserror::Error;
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Invalid provider config: {0}")]
    InvalidProvider(String),
}
pub struct ConfigLoader {
    config_paths: Vec<PathBuf>,
    env_overrides: bool,
    /// When Some, only load from this single path (skip discovery and defaults)
    explicit_path: Option<PathBuf>,
}
