//! Gemini provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use crate::config::ProviderConfig;
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
