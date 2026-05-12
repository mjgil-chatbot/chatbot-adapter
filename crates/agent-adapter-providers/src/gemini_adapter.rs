//! Gemini provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::AdapterError;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
use crate::{build_command, CommandSpec};
pub struct GeminiAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}
impl GeminiAdapter {
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
impl GeminiAdapter {
    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, AdapterError> {
        build_command(
            ProviderId::Gemini,
            request.approval_policy,
            &self.config,
            request,
        )
        .map_err(|e| AdapterError::InvalidConfig { message: e.message })
    }}
