//! Claw provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
use crate::{build_command, BuildError, CommandSpec};
pub struct ClawAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}
impl ClawAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            tools: true,
            pty: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }}
impl ClawAdapter {
    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, BuildError> {
        build_command(
            ProviderId::Claw,
            request.approval_policy,
            &self.config,
            request,
        )
    }}
