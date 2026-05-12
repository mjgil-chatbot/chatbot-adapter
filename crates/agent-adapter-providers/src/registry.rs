//! Factory registry for built-in providers.

use agent_adapter_config::ProviderConfig;
use agent_adapter_core::{AdapterHealth, AdapterResult, AgentAdapter, ProviderId};
use std::collections::HashMap;
use std::sync::Arc;
use crate::traits::ProviderFactory;
/// Registry of all built-in provider factories.
pub struct FactoryRegistry {
    factories: HashMap<String, Arc<dyn ProviderFactory>>,
}
impl FactoryRegistry {
    /// Get a factory by provider name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn ProviderFactory>> {
        self.factories.get(name)
    }}
impl FactoryRegistry {
    /// Get all registered provider names.
    pub fn provider_names(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }}
impl FactoryRegistry {
    /// Check if a provider is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }}
impl FactoryRegistry {
    /// Build an adapter for a given provider and config.
    pub async fn build_adapter(
        &self,
        name: &str,
        config: ProviderConfig,
    ) -> AdapterResult<Box<dyn AgentAdapter>> {
        let factory = self.factories.get(name).ok_or_else(|| {
            agent_adapter_core::AdapterError::ProviderNotFound {
                provider: ProviderId::Custom(name.to_string()),
                install_hint: None,
            }
        })?;

        factory.build(config).await
    }}
impl FactoryRegistry {
    /// Build an adapter with healthcheck for a given provider.
    pub async fn build_with_healthcheck(
        &self,
        name: &str,
        config: ProviderConfig,
    ) -> AdapterResult<(Box<dyn AgentAdapter>, AdapterHealth)> {
        let factory = self.factories.get(name).ok_or_else(|| {
            agent_adapter_core::AdapterError::ProviderNotFound {
                provider: ProviderId::Custom(name.to_string()),
                install_hint: None,
            }
        })?;

        let health = factory.detect().await?;
        let adapter = factory.build(config).await?;

        Ok((adapter, health))
    }}
impl FactoryRegistry {
    /// Detect health status for all registered providers.
    pub async fn detect_all_health(&self) -> HashMap<String, AdapterHealth> {
        let mut healths = HashMap::new();
        for (name, factory) in &self.factories {
            let health = factory
                .detect()
                .await
                .unwrap_or(AdapterHealth::Misconfigured {
                    message: "Health check failed".to_string(),
                });
            healths.insert(name.clone(), health);
        }
        healths
    }}
