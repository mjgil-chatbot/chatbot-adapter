//! Config types.

use serde::{Deserialize, Serialize};

/// Current config schema version.
pub const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub kind: ProviderKind,
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub prompt_mode: PromptMode,
    #[serde(default)]
    pub mapper: String,
    #[serde(default)]
    pub unsafe_args: Vec<String>,
    #[serde(default)]
    pub safe_args: Vec<String>,
    #[serde(default)]
    pub env_allowlist: Vec<String>,
    #[serde(default)]
    pub cwd_behavior: CwdBehavior,
    #[serde(default)]
    pub schema_version: Option<i32>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: ProviderKind::JsonlCommand,
            exe: String::new(),
            args: Vec::new(),
            prompt_mode: PromptMode::Arg,
            mapper: String::new(),
            unsafe_args: Vec::new(),
            safe_args: Vec::new(),
            env_allowlist: Vec::new(),
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }
}

impl ProviderConfig {
    /// Create a config for a custom provider with the given name.
    pub fn for_custom(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ProviderKind::Custom,
            exe: String::new(),
            args: Vec::new(),
            prompt_mode: PromptMode::Arg,
            mapper: String::new(),
            unsafe_args: Vec::new(),
            safe_args: Vec::new(),
            env_allowlist: Vec::new(),
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.name.is_empty() {
            return Err(ConfigValidationError::MissingField("name".to_string()));
        }
        if self.exe.is_empty() {
            return Err(ConfigValidationError::MissingField("exe".to_string()));
        }

        // Validate name contains only valid characters
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ConfigValidationError::InvalidFieldValue {
                field: "name".to_string(),
                value: self.name.clone(),
                reason: "Name must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            });
        }

        // Validate mapper is not empty for non-builtin providers
        if self.mapper.is_empty() && self.kind != ProviderKind::Custom {
            return Err(ConfigValidationError::MissingField("mapper".to_string()));
        }

        // Validate that args don't contain shell injection characters
        // (placeholder expansion is handled separately)
        for arg in &self.args {
            if arg.contains('\x00') {
                return Err(ConfigValidationError::InvalidFieldValue {
                    field: "args".to_string(),
                    value: arg.clone(),
                    reason: "Args must not contain null bytes".to_string(),
                });
            }
        }

        // Validate schema version if present
        if let Some(version) = self.schema_version {
            if version < 0 {
                return Err(ConfigValidationError::InvalidFieldValue {
                    field: "schema_version".to_string(),
                    value: version.to_string(),
                    reason: "Schema version must be non-negative".to_string(),
                });
            }
            if version > CURRENT_SCHEMA_VERSION {
                return Err(ConfigValidationError::UnsupportedSchemaVersion(version));
            }
        }

        // Validate env_allowlist entries are valid env var names
        for var in &self.env_allowlist {
            if !var.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(ConfigValidationError::InvalidFieldValue {
                    field: "env_allowlist".to_string(),
                    value: var.clone(),
                    reason: "Environment variable names must contain only alphanumeric characters and underscores".to_string(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    JsonlCommand,
    OneShotCommand,
    AcpStdio,
    PtyFallback,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PromptMode {
    Stdin,
    #[default]
    Arg,
    File,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CwdBehavior {
    #[default]
    UseProvided,
    CreateTemp,
    CreateWorktree,
}

#[derive(Debug, Clone)]
pub enum ConfigValidationError {
    MissingField(String),
    UnsupportedSchemaVersion(i32),
    InvalidEnumValue {
        field: String,
        value: String,
    },
    InvalidFieldValue {
        field: String,
        value: String,
        reason: String,
    },
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValidationError::MissingField(name) => {
                write!(f, "Missing required field: {}", name)
            }
            ConfigValidationError::UnsupportedSchemaVersion(v) => {
                write!(f, "Unsupported schema version: {}", v)
            }
            ConfigValidationError::InvalidEnumValue { field, value } => {
                write!(f, "Invalid value '{}' for field '{}'", value, field)
            }
            ConfigValidationError::InvalidFieldValue {
                field,
                value,
                reason,
            } => {
                write!(
                    f,
                    "Invalid value '{}' for field '{}': {}",
                    value, field, reason
                )
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_kind_serde() {
        let kind = ProviderKind::JsonlCommand;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"jsonl_command\"");
    }

    #[test]
    fn test_provider_config_validate_success() {
        let config = ProviderConfig {
            name: "claude".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claude".to_string(),
            mapper: "claude".to_string(),
            schema_version: Some(1),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_provider_config_validate_missing_name() {
        let config = ProviderConfig {
            name: "".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claude".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ConfigValidationError::MissingField(n) if n == "name")
        );
    }

    #[test]
    fn test_provider_config_validate_missing_exe() {
        let config = ProviderConfig {
            name: "claude".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ConfigValidationError::MissingField(n) if n == "exe")
        );
    }

    #[test]
    fn test_provider_config_validate_future_version() {
        let config = ProviderConfig {
            name: "claude".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claude".to_string(),
            mapper: "claude".to_string(),
            schema_version: Some(999),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ConfigValidationError::UnsupportedSchemaVersion(v) if v == 999)
        );
    }

    #[test]
    fn test_current_schema_version() {
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_provider_config_default_schema_version() {
        let config = ProviderConfig::default();
        assert!(config.schema_version.is_none());
    }

    #[test]
    fn test_provider_config_validate_invalid_name_chars() {
        let config = ProviderConfig {
            name: "claude@provider".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claude".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ConfigValidationError::InvalidFieldValue { field, .. } if field == "name")
        );
    }

    #[test]
    fn test_provider_config_validate_missing_mapper() {
        // Non-custom provider needs a mapper
        let config = ProviderConfig {
            name: "test".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "test".to_string(),
            mapper: "".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ConfigValidationError::MissingField(n) if n == "mapper")
        );
    }

    #[test]
    fn test_provider_config_validate_custom_no_mapper() {
        // Custom provider doesn't require mapper
        let config = ProviderConfig {
            name: "custom".to_string(),
            kind: ProviderKind::Custom,
            exe: "custom".to_string(),
            mapper: "".to_string(),
            ..Default::default()
        };
        // Should pass even without mapper for custom kind
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_provider_config_validate_invalid_env_var_name() {
        let config = ProviderConfig {
            name: "test".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "test".to_string(),
            mapper: "test".to_string(),
            env_allowlist: vec!["INVALID-NAME".to_string()],
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ConfigValidationError::InvalidFieldValue { field, .. } if field == "env_allowlist")
        );
    }

    #[test]
    fn test_provider_config_validate_null_byte_in_args() {
        let config = ProviderConfig {
            name: "test".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "test".to_string(),
            mapper: "test".to_string(),
            args: vec!["normal".to_string(), "has\x00null".to_string()],
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ConfigValidationError::InvalidFieldValue { field, .. } if field == "args")
        );
    }

    #[test]
    fn test_provider_config_validate_negative_schema_version() {
        let config = ProviderConfig {
            name: "test".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "test".to_string(),
            mapper: "test".to_string(),
            schema_version: Some(-1),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ConfigValidationError::InvalidFieldValue { field, .. } if field == "schema_version")
        );
    }

    #[test]
    fn test_provider_config_validate_valid_custom_provider() {
        // Custom provider with all fields
        let config = ProviderConfig {
            name: "my-provider".to_string(),
            kind: ProviderKind::Custom,
            exe: "/usr/local/bin/my-provider".to_string(),
            args: vec!["--mode".to_string(), "interactive".to_string()],
            prompt_mode: PromptMode::Stdin,
            mapper: "custom".to_string(),
            unsafe_args: vec!["--dangerous".to_string()],
            safe_args: vec!["--safe".to_string()],
            env_allowlist: vec!["PATH".to_string(), "HOME".to_string()],
            cwd_behavior: CwdBehavior::CreateTemp,
            schema_version: Some(1),
        };
        assert!(config.validate().is_ok());
    }
}
