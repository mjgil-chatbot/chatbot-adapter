//! Claude provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::AdapterError;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
use crate::{build_command, CommandSpec};
/// Claude provider adapter.
pub struct ClaudeAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
    provider_id: ProviderId,
}
impl ClaudeAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        Self::new_for_provider(config, ProviderId::Claude)
    }}
impl ClaudeAdapter {
    pub fn new_for_provider(config: ProviderConfig, provider_id: ProviderId) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
            provider_id,
        }
    }}
impl ClaudeAdapter {
    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, AdapterError> {
        build_command(
            self.provider_id.clone(),
            request.approval_policy,
            &self.config,
            request,
        )
        .map_err(|e| AdapterError::InvalidConfig { message: e.message })
    }}
impl From<super::BuildError> for AdapterError {
    fn from(e: super::BuildError) -> Self {
        AdapterError::InvalidConfig { message: e.message }
    }
}
