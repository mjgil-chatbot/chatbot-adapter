//! Config file loading and parsing.
//!
//! This module loads TOML config files for provider configurations.

use crate::{CwdBehavior, PromptMode, ProviderConfig, ProviderKind};
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
impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            config_paths: Vec::new(),
            env_overrides: true,
            explicit_path: None,
        }
    }}
impl ConfigLoader {
    pub fn add_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_paths.push(path.as_ref().to_path_buf());
        self
    }}
impl ConfigLoader {
    /// Set an explicit config file path. When set, only this path is loaded
    /// and all discovery/defaults are skipped.
    pub fn with_explicit_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.explicit_path = Some(path.as_ref().to_path_buf());
        self
    }}
impl ConfigLoader {
    /// Enable environment variable overrides (enabled by default).
    pub fn with_env_overrides(mut self, enabled: bool) -> Self {
        self.env_overrides = enabled;
        self
    }}
impl ConfigLoader {
    /// Get the default config directory path.
    pub fn default_config_dir() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("agent-adapter")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join("agent-adapter")
        } else {
            PathBuf::from(".agent-adapter")
        }
    }}
impl ConfigLoader {
    /// Get the default config file path.
    pub fn default_config_path() -> PathBuf {
        Self::default_config_dir().join("config.toml")
    }}
impl ConfigLoader {
    /// Get project-level config paths to check.
    pub fn project_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        // Check for .agent-adapter.toml in current directory
        if std::env::current_dir()
            .map(|p| p.join(".agent-adapter.toml").exists())
            .unwrap_or(false)
        {
            paths.push(std::env::current_dir().unwrap().join(".agent-adapter.toml"));
        }
        // Check for agent-adapter.toml in current directory
        if std::env::current_dir()
            .map(|p| p.join("agent-adapter.toml").exists())
            .unwrap_or(false)
        {
            paths.push(std::env::current_dir().unwrap().join("agent-adapter.toml"));
        }
        paths
    }}
impl ConfigLoader {
    pub fn load(&self) -> Result<Vec<ProviderConfig>, ConfigError> {
        let mut configs = Vec::new();

        // If explicit path is set, only load from that path (skip all discovery)
        if let Some(ref explicit_path) = self.explicit_path {
            if explicit_path.exists() {
                let content = std::fs::read_to_string(explicit_path)?;
                let parsed: toml::Table = toml::from_str(&content)?;
                for (name, value) in parsed {
                    let provider: ProviderConfig = value
                        .try_into()
                        .map_err(|e| ConfigError::InvalidProvider(format!("{}: {}", name, e)))?;
                    configs.push(provider);
                }
            }
            // Apply environment overrides even for explicit path
            if self.env_overrides {
                self.apply_env_overrides(&mut configs);
            }
            return Ok(configs);
        }

        // Load built-in defaults first
        configs.extend(Self::built_in_defaults());

        // Load project-level configs
        for path in Self::project_config_paths() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let parsed: toml::Table = toml::from_str(&content)?;
                for (name, value) in parsed {
                    let provider: ProviderConfig = value
                        .try_into()
                        .map_err(|e| ConfigError::InvalidProvider(format!("{}: {}", name, e)))?;
                    // Project configs override built-ins by name
                    configs.retain(|c| c.name != provider.name);
                    configs.push(provider);
                }
            }
        }

        // Load user-level configs
        for path in &self.config_paths {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let parsed: toml::Table = toml::from_str(&content)?;
                for (name, value) in parsed {
                    let provider: ProviderConfig = value
                        .try_into()
                        .map_err(|e| ConfigError::InvalidProvider(format!("{}: {}", name, e)))?;
                    // User configs override project and built-in
                    configs.retain(|c| c.name != provider.name);
                    configs.push(provider);
                }
            }
        }

        // Apply environment overrides if enabled
        if self.env_overrides {
            self.apply_env_overrides(&mut configs);
        }

        Ok(configs)
    }}
impl ConfigLoader {
    /// Apply environment variable overrides.
    ///
    /// Environment variables for provider overrides:
    /// - `AGENT_ADAPTER_DEFAULT_PROVIDER`: Override the default provider name
    /// - `AGENT_ADAPTER_<PROVIDER>_EXE`: Override the executable for a specific provider
    /// - `AGENT_ADAPTER_<PROVIDER>_ARGS`: Override args for a specific provider (JSON array)
    fn apply_env_overrides(&self, configs: &mut Vec<ProviderConfig>) {
        // AGENT_ADAPTER_DEFAULT_PROVIDER: Override the default provider name
        if let Ok(default_provider) = std::env::var("AGENT_ADAPTER_DEFAULT_PROVIDER") {
            if !default_provider.is_empty() && !configs.is_empty() {
                let new_name = default_provider.clone();

                // Rename first config to the new provider name
                if let Some(config) = configs.first_mut() {
                    config.name = new_name.clone();
                }

                // Remove any OTHER configs that now have the same name as the renamed first config
                // Build a new Vec to avoid borrow issues with retain
                let mut result = vec![configs[0].clone()];
                for config in configs.iter().skip(1) {
                    if config.name != new_name {
                        result.push(config.clone());
                    }
                }
                *configs = result;
            }
        }

        // AGENT_ADAPTER_<PROVIDER>_EXE: Override exe for specific provider
        for config in configs.iter_mut() {
            let env_var = format!(
                "AGENT_ADAPTER_{}_EXE",
                config.name.to_uppercase().replace('-', "_")
            );
            if let Ok(exe) = std::env::var(&env_var) {
                if !exe.is_empty() {
                    config.exe = exe;
                }
            }

            // AGENT_ADAPTER_<PROVIDER>_ARGS: Override args for specific provider (JSON array)
            let args_var = format!(
                "AGENT_ADAPTER_{}_ARGS",
                config.name.to_uppercase().replace('-', "_")
            );
            if let Ok(args_json) = std::env::var(&args_var) {
                if let Ok(args) = serde_json::from_str::<Vec<String>>(&args_json) {
                    config.args = args;
                }
            }
        }
    }}
impl ConfigLoader {
    /// Built-in provider defaults.
    fn built_in_defaults() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                name: "claude".to_string(),
                kind: ProviderKind::JsonlCommand,
                exe: "claude".to_string(),
                args: vec![
                    "-p".to_string(),
                    "{prompt}".to_string(),
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--input-format".to_string(),
                    "text".to_string(),
                ],
                prompt_mode: PromptMode::Arg,
                mapper: "claude".to_string(),
                unsafe_args: vec!["--dangerously-skip-permissions".to_string()],
                safe_args: vec!["--permission-mode".to_string(), "default".to_string()],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
            ProviderConfig {
                name: "minimax".to_string(),
                kind: ProviderKind::JsonlCommand,
                exe: "claude".to_string(),
                args: vec![
                    "-p".to_string(),
                    "{prompt}".to_string(),
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--input-format".to_string(),
                    "text".to_string(),
                ],
                prompt_mode: PromptMode::Arg,
                mapper: "claude".to_string(),
                unsafe_args: vec!["--dangerously-skip-permissions".to_string()],
                safe_args: vec!["--permission-mode".to_string(), "default".to_string()],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
            ProviderConfig {
                name: "codex".to_string(),
                kind: ProviderKind::JsonlCommand,
                exe: "codex".to_string(),
                args: vec![
                    "exec".to_string(),
                    "--json".to_string(),
                    "--cd".to_string(),
                    "{cwd}".to_string(),
                    "-".to_string(),
                ],
                prompt_mode: PromptMode::Stdin,
                mapper: "jsonl".to_string(),
                unsafe_args: vec!["--bypass".to_string(), "yolo".to_string()],
                safe_args: vec![],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
            ProviderConfig {
                name: "gemini".to_string(),
                kind: ProviderKind::JsonlCommand,
                exe: "gemini".to_string(),
                args: vec![
                    "-p".to_string(),
                    "{prompt}".to_string(),
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                ],
                prompt_mode: PromptMode::Arg,
                mapper: "jsonl".to_string(),
                unsafe_args: vec!["--approval-mode".to_string(), "yolo".to_string()],
                safe_args: vec![],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
            ProviderConfig {
                name: "kimi".to_string(),
                kind: ProviderKind::AcpStdio,
                exe: "kimi".to_string(),
                args: vec!["acp".to_string()],
                prompt_mode: PromptMode::Stdin,
                mapper: "claude".to_string(),
                unsafe_args: vec![],
                safe_args: vec![],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
            ProviderConfig {
                name: "opencode".to_string(),
                kind: ProviderKind::JsonlCommand,
                exe: "opencode".to_string(),
                args: vec![
                    "run".to_string(),
                    "--format".to_string(),
                    "json".to_string(),
                    "--dir".to_string(),
                    "{cwd}".to_string(),
                    "{prompt}".to_string(),
                ],
                prompt_mode: PromptMode::Arg,
                mapper: "jsonl".to_string(),
                unsafe_args: vec!["--allow".to_string()],
                safe_args: vec!["--deny".to_string()],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
            ProviderConfig {
                name: "hermes".to_string(),
                kind: ProviderKind::AcpStdio,
                exe: "hermes".to_string(),
                args: vec!["acp".to_string()],
                prompt_mode: PromptMode::Stdin,
                mapper: "claude".to_string(),
                unsafe_args: vec!["yolo".to_string()],
                safe_args: vec![],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::CreateWorktree,
                schema_version: None,
            },
            ProviderConfig {
                name: "forge".to_string(),
                kind: ProviderKind::OneShotCommand,
                exe: "forge".to_string(),
                args: vec!["-p".to_string(), "{prompt}".to_string()],
                prompt_mode: PromptMode::Arg,
                mapper: "plain".to_string(),
                unsafe_args: vec![],
                safe_args: vec![],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
            ProviderConfig {
                name: "claw".to_string(),
                kind: ProviderKind::Custom,
                exe: "claw".to_string(),
                args: vec![],
                prompt_mode: PromptMode::Stdin,
                mapper: "claude".to_string(),
                unsafe_args: vec![],
                safe_args: vec![],
                env_allowlist: vec![],
                cwd_behavior: CwdBehavior::UseProvided,
                schema_version: None,
            },
        ]
    }}
impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(serde::Deserialize)]
struct ProviderConfigToml {
    name: String,
    kind: String,
    #[serde(default)]
    exe: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    prompt_mode: Option<String>,
    #[serde(default)]
    mapper: Option<String>,
    #[serde(default)]
    unsafe_args: Vec<String>,
    #[serde(default)]
    safe_args: Vec<String>,
    #[serde(default)]
    env_allowlist: Vec<String>,
    #[serde(default)]
    cwd_behavior: Option<String>,
    #[serde(default)]
    schema_version: Option<i32>,
}
fn parse_prompt_mode(s: &str) -> PromptMode {
    match s {
        "arg" => PromptMode::Arg,
        "file" => PromptMode::File,
        _ => PromptMode::Stdin,
    }
}
fn parse_cwd_behavior(s: &str) -> CwdBehavior {
    match s {
        "create_temp" => CwdBehavior::CreateTemp,
        "create_worktree" => CwdBehavior::CreateWorktree,
        _ => CwdBehavior::UseProvided,
    }
}
impl TryFrom<toml::Value> for ProviderConfig {
    type Error = String;

    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        let toml_config: ProviderConfigToml = value
            .try_into()
            .map_err(|e| format!("TOML parse error: {}", e))?;

        let kind = match toml_config.kind.as_str() {
            "jsonl_command" => ProviderKind::JsonlCommand,
            "oneshot_command" => ProviderKind::OneShotCommand,
            "acp_stdio" => ProviderKind::AcpStdio,
            "pty_fallback" => ProviderKind::PtyFallback,
            "custom" => ProviderKind::Custom,
            other => return Err(format!("Unknown provider kind: {}", other)),
        };

        let exe = toml_config.exe.unwrap_or_else(|| toml_config.name.clone());
        let prompt_mode = toml_config
            .prompt_mode
            .map(|s| parse_prompt_mode(&s))
            .unwrap_or_else(|| ProviderConfig::default().prompt_mode);
        let cwd_behavior = toml_config
            .cwd_behavior
            .map(|s| parse_cwd_behavior(&s))
            .unwrap_or_else(|| ProviderConfig::default().cwd_behavior);

        Ok(ProviderConfig {
            name: toml_config.name,
            kind,
            exe,
            args: toml_config.args,
            prompt_mode,
            mapper: toml_config.mapper.unwrap_or_default(),
            unsafe_args: toml_config.unsafe_args,
            safe_args: toml_config.safe_args,
            env_allowlist: toml_config.env_allowlist,
            cwd_behavior,
            schema_version: toml_config.schema_version,
        })
    }
}
