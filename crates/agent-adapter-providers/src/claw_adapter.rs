//! Claw provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use crate::config::ProviderConfig;
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
