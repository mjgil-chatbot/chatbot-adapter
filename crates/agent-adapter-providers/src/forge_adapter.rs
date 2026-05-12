//! Forge provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
use crate::{build_command, BuildError, CommandSpec};
pub struct ForgeAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}
impl ForgeAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            tools: true,
            oneshot: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }}
impl ForgeAdapter {
    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, BuildError> {
        build_command(
            ProviderId::Forge,
            request.approval_policy,
            &self.config,
            request,
        )
    }}
