//! Kimi provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
use crate::{build_command, BuildError, CommandSpec};
pub struct KimiAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}
impl KimiAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            jsonl: true,
            acp: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }}
impl KimiAdapter {
    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, BuildError> {
        // Kimi safe mode returns error for --print mode (AFK behavior)
        if request.approval_policy.is_safe() {
            return Err(BuildError {
                message: "Kimi safe mode not fully supported for --print mode (implies AFK)"
                    .to_string(),
            });
        }

        build_command(
            ProviderId::Kimi,
            request.approval_policy,
            &self.config,
            request,
        )
    }}
