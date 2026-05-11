//! Claude provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::ProviderId;
use crate::config::ProviderConfig;
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
