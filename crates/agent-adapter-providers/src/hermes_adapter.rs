//! Hermes provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
use crate::{build_command, BuildError, CommandSpec};
pub struct HermesAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}
impl HermesAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            acp: true,
            oneshot: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }}
impl HermesAdapter {
    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, BuildError> {
        build_command(
            ProviderId::Hermes,
            request.approval_policy,
            &self.config,
            request,
        )
    }}
