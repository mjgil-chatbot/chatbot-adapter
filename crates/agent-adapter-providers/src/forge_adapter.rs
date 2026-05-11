//! Forge provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use crate::config::ProviderConfig;
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
