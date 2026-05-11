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
    }}
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
    }}
impl UnsafeModeUnavailable {
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.binary_version = Some(version.into());
        self
    }}
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
