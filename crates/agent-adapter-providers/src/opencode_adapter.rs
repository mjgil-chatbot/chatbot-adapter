//! OpenCode provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
use crate::{build_command, BuildError, CommandSpec};
pub struct OpenCodeAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}
impl OpenCodeAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            acp: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }}
impl OpenCodeAdapter {
    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, BuildError> {
        build_command(
            ProviderId::OpenCode,
            request.approval_policy,
            &self.config,
            request,
        )
    }}
