//! Provider traits.

use agent_adapter_config::ProviderConfig;
use agent_adapter_core::{AdapterCapabilities, AdapterHealth, AdapterResult};
use async_trait::async_trait;
use std::path::PathBuf;

/// Transport kinds supported by a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    JsonlCommand,
    OneShotCommand,
    AcpStdio,
    PtyFallback,
}

/// Policy support declaration for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicySupport {
    pub supports_safe: bool,
    pub supports_dangerous: bool,
    pub requires_explicit_unsafe_args: bool,
}

impl PolicySupport {
    pub fn safe_only() -> Self {
        Self {
            supports_safe: true,
            supports_dangerous: false,
            requires_explicit_unsafe_args: false,
        }
    }

    pub fn dangerous_only() -> Self {
        Self {
            supports_safe: false,
            supports_dangerous: true,
            requires_explicit_unsafe_args: true,
        }
    }

    pub fn both() -> Self {
        Self {
            supports_safe: true,
            supports_dangerous: true,
            requires_explicit_unsafe_args: false,
        }
    }

    pub fn neither() -> Self {
        Self {
            supports_safe: false,
            supports_dangerous: false,
            requires_explicit_unsafe_args: false,
        }
    }
}

#[async_trait]
pub trait ProviderFactory: Send + Sync {
    /// Returns the provider identifier string.
    fn id(&self) -> &str;

    /// Returns the path to the provider documentation.
    fn docs_path(&self) -> PathBuf;

    /// Returns the default configuration for this provider.
    fn default_config(&self) -> ProviderConfig;

    /// Returns the capabilities supported by this provider.
    fn capabilities(&self) -> AdapterCapabilities;

    /// Returns the transport kinds supported by this provider.
    fn supported_transports(&self) -> Vec<TransportKind>;

    /// Returns the policy support for this provider.
    fn policy_support(&self) -> PolicySupport;

    /// Detects whether the provider binary is available.
    async fn detect(&self) -> AdapterResult<AdapterHealth>;

    /// Builds an adapter instance with the given configuration.
    async fn build(
        &self,
        config: ProviderConfig,
    ) -> AdapterResult<Box<dyn agent_adapter_core::AgentAdapter>>;
}
