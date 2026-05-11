//! Error types for the agent adapter.

use crate::types::ProviderId;
use serde::{Deserialize, Serialize};

/// Result type for adapter operations.
pub type AdapterResult<T> = Result<T, AdapterError>;

/// Core error type for adapter operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdapterError {
    /// Provider binary not found.
    ProviderNotFound {
        provider: ProviderId,
        install_hint: Option<String>,
    },

    /// Provider binary exists but is not functional.
    ProviderNotFunctional {
        provider: ProviderId,
        message: String,
    },

    /// Provider requires authentication.
    NeedsAuth {
        provider: ProviderId,
        login_hint: String,
    },

    /// Provider is misconfigured.
    Misconfigured {
        provider: ProviderId,
        message: String,
    },

    /// Unsafe mode was requested but is not available for this provider.
    UnsafeModeUnavailable {
        provider: ProviderId,
        binary_version: Option<String>,
        missing_setting: String,
        remediation: String,
    },

    /// Safe mode is not available for this provider.
    SafeModeUnavailable {
        provider: ProviderId,
        reason: String,
    },

    /// The provider does not support a requested capability.
    UnsupportedCapability {
        provider: ProviderId,
        capability: String,
    },

    /// Transport failed.
    TransportError { message: String },

    /// JSON parsing failed.
    ParseError {
        message: String,
        raw: Option<String>,
    },

    /// Operation timed out.
    Timeout { run_id: String },

    /// Operation was cancelled.
    Cancelled { run_id: String },

    /// Session not found.
    SessionNotFound {
        provider: ProviderId,
        session_id: String,
    },

    /// Invalid configuration.
    InvalidConfig { message: String },
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::ProviderNotFound {
                provider,
                install_hint,
            } => {
                write!(f, "Provider '{}' not found", provider)?;
                if let Some(hint) = install_hint {
                    write!(f, ". {}", hint)?;
                }
                Ok(())
            }
            AdapterError::ProviderNotFunctional { provider, message } => {
                write!(f, "Provider '{}' not functional: {}", provider, message)
            }
            AdapterError::NeedsAuth {
                provider,
                login_hint,
            } => {
                write!(f, "Provider '{}' needs auth: {}", provider, login_hint)
            }
            AdapterError::Misconfigured { provider, message } => {
                write!(f, "Provider '{}' misconfigured: {}", provider, message)
            }
            AdapterError::UnsafeModeUnavailable {
                provider,
                missing_setting,
                remediation,
                ..
            } => {
                write!(
                    f,
                    "Unsafe mode unavailable for '{}': {} ({})",
                    provider, missing_setting, remediation
                )
            }
            AdapterError::SafeModeUnavailable { provider, reason } => {
                write!(f, "Safe mode unavailable for '{}': {}", provider, reason)
            }
            AdapterError::UnsupportedCapability {
                provider,
                capability,
            } => {
                write!(
                    f,
                    "Provider '{}' does not support capability: {}",
                    provider, capability
                )
            }
            AdapterError::TransportError { message } => {
                write!(f, "Transport error: {}", message)
            }
            AdapterError::ParseError { message, raw } => {
                write!(f, "Parse error: {}", message)?;
                if let Some(r) = raw {
                    write!(f, " (raw: {})", r)?;
                }
                Ok(())
            }
            AdapterError::Timeout { run_id } => {
                write!(f, "Operation timed out: {}", run_id)
            }
            AdapterError::Cancelled { run_id } => {
                write!(f, "Operation cancelled: {}", run_id)
            }
            AdapterError::SessionNotFound {
                provider,
                session_id,
            } => {
                write!(
                    f,
                    "Session '{}' not found for provider '{}'",
                    session_id, provider
                )
            }
            AdapterError::InvalidConfig { message } => {
                write!(f, "Invalid configuration: {}", message)
            }
        }
    }
}

impl std::error::Error for AdapterError {}

impl AdapterError {
    /// Returns the exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            AdapterError::ProviderNotFound { .. } => 2,
            AdapterError::NeedsAuth { .. } => 3,
            AdapterError::Misconfigured { .. } => 4,
            AdapterError::UnsafeModeUnavailable { .. } => 5,
            AdapterError::SafeModeUnavailable { .. } => 6,
            AdapterError::UnsupportedCapability { .. } => 7,
            AdapterError::TransportError { .. } => 8,
            AdapterError::ParseError { .. } => 9,
            AdapterError::Timeout { .. } => 10,
            AdapterError::Cancelled { .. } => 11,
            AdapterError::SessionNotFound { .. } => 12,
            AdapterError::ProviderNotFunctional { .. } => 1,
            AdapterError::InvalidConfig { .. } => 4,
        }
    }
}

/// Error returned when unsafe mode is requested but not available.
#[derive(Debug, Clone)]
pub struct UnsafeModeUnavailable {
    pub provider: ProviderId,
    pub binary_version: Option<String>,
    pub missing_setting: String,
    pub remediation: String,
}

impl UnsafeModeUnavailable {
    pub fn new(
        provider: ProviderId,
        missing_setting: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            binary_version: None,
            missing_setting: missing_setting.into(),
            remediation: remediation.into(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.binary_version = Some(version.into());
        self
    }
}

impl From<UnsafeModeUnavailable> for AdapterError {
    fn from(e: UnsafeModeUnavailable) -> Self {
        AdapterError::UnsafeModeUnavailable {
            provider: e.provider,
            binary_version: e.binary_version,
            missing_setting: e.missing_setting,
            remediation: e.remediation,
        }
    }
}

impl std::fmt::Display for UnsafeModeUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unsafe mode unavailable for '{}': {} ({}",
            self.provider, self.missing_setting, self.remediation
        )?;
        if let Some(v) = &self.binary_version {
            write!(f, "; version: {}", v)?;
        }
        write!(f, ")")
    }
}

impl std::error::Error for UnsafeModeUnavailable {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProviderId;

    #[test]
    fn test_error_exit_codes() {
        assert_eq!(
            AdapterError::ProviderNotFound {
                provider: ProviderId::Claude,
                install_hint: None,
            }
            .exit_code(),
            2
        );

        assert_eq!(
            AdapterError::NeedsAuth {
                provider: ProviderId::Claude,
                login_hint: "Run `claude auth login`".to_string(),
            }
            .exit_code(),
            3
        );

        assert_eq!(
            AdapterError::Misconfigured {
                provider: ProviderId::Forge,
                message: "missing unsafe_args".to_string(),
            }
            .exit_code(),
            4
        );

        assert_eq!(
            AdapterError::UnsafeModeUnavailable {
                provider: ProviderId::Forge,
                binary_version: None,
                missing_setting: "unsafe_args".to_string(),
                remediation: "Configure unsafe_args in config".to_string(),
            }
            .exit_code(),
            5
        );

        assert_eq!(
            AdapterError::SafeModeUnavailable {
                provider: ProviderId::Kimi,
                reason: "--print mode doesn't support safe mode".to_string(),
            }
            .exit_code(),
            6
        );

        assert_eq!(
            AdapterError::UnsupportedCapability {
                provider: ProviderId::Claude,
                capability: "image_input".to_string(),
            }
            .exit_code(),
            7
        );

        assert_eq!(
            AdapterError::TransportError {
                message: "Broken pipe".to_string(),
            }
            .exit_code(),
            8
        );

        assert_eq!(
            AdapterError::ParseError {
                message: "Unexpected token".to_string(),
                raw: None,
            }
            .exit_code(),
            9
        );

        assert_eq!(
            AdapterError::Timeout {
                run_id: "run-123".to_string(),
            }
            .exit_code(),
            10
        );

        assert_eq!(
            AdapterError::Cancelled {
                run_id: "run-456".to_string(),
            }
            .exit_code(),
            11
        );

        assert_eq!(
            AdapterError::SessionNotFound {
                provider: ProviderId::Claude,
                session_id: "abc123".to_string(),
            }
            .exit_code(),
            12
        );

        assert_eq!(
            AdapterError::ProviderNotFunctional {
                provider: ProviderId::Codex,
                message: "connection refused".to_string(),
            }
            .exit_code(),
            1
        );

        assert_eq!(
            AdapterError::InvalidConfig {
                message: "missing provider exe".to_string(),
            }
            .exit_code(),
            4
        );
    }

    #[test]
    fn test_error_display() {
        let err = AdapterError::ProviderNotFound {
            provider: ProviderId::Claude,
            install_hint: Some("Install from https://claude.ai/code".to_string()),
        };
        assert_eq!(
            err.to_string(),
            "Provider 'claude' not found. Install from https://claude.ai/code"
        );

        let err = AdapterError::UnsafeModeUnavailable {
            provider: ProviderId::Forge,
            binary_version: Some("1.0".to_string()),
            missing_setting: "unsafe_args".to_string(),
            remediation: "Set unsafe_args in config".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Unsafe mode unavailable for 'forge': unsafe_args (Set unsafe_args in config)"
        );
    }

    #[test]
    fn test_error_serde() {
        let err = AdapterError::ProviderNotFound {
            provider: ProviderId::Claude,
            install_hint: Some("Install".to_string()),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("ProviderNotFound"));

        let parsed: AdapterError = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, AdapterError::ProviderNotFound { .. }));
    }
}
