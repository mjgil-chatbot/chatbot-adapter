//! Factory registry for built-in providers.

use agent_adapter_config::ProviderConfig;
use agent_adapter_core::{AdapterHealth, AdapterResult, AgentAdapter, ProviderId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::traits::ProviderFactory;
use crate::{
    ClaudeFactory, ClawFactory, CodexFactory, ForgeFactory, GeminiFactory, HermesFactory,
    KimiFactory, MiniMaxFactory, OpenCodeFactory,
};

/// Registry of all built-in provider factories.
pub struct FactoryRegistry {
    factories: HashMap<String, Arc<dyn ProviderFactory>>,
}

impl FactoryRegistry {
    /// Create a new registry with all built-in factories registered.
    pub fn new() -> Self {
        let mut factories = HashMap::new();

        let built_ins: Vec<Arc<dyn ProviderFactory>> = vec![
            Arc::new(ClaudeFactory::new()),
            Arc::new(MiniMaxFactory::new()),
            Arc::new(CodexFactory::new()),
            Arc::new(GeminiFactory::new()),
            Arc::new(KimiFactory::new()),
            Arc::new(OpenCodeFactory::new()),
            Arc::new(HermesFactory::new()),
            Arc::new(ForgeFactory::new()),
            Arc::new(ClawFactory::new()),
        ];

        for factory in built_ins {
            factories.insert(factory.id().to_string(), factory);
        }

        Self { factories }
    }

    /// Get a factory by provider name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn ProviderFactory>> {
        self.factories.get(name)
    }

    /// Get all registered provider names.
    pub fn provider_names(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }

    /// Check if a provider is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

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
    }

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
    }

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
    }
}

impl Default for FactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_contains_all_builtins() {
        let registry = FactoryRegistry::new();
        assert!(registry.contains("claude"));
        assert!(registry.contains("minimax"));
        assert!(registry.contains("codex"));
        assert!(registry.contains("gemini"));
        assert!(registry.contains("kimi"));
        assert!(registry.contains("opencode"));
        assert!(registry.contains("hermes"));
        assert!(registry.contains("forge"));
        assert!(registry.contains("claw"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_registry_provider_names() {
        let registry = FactoryRegistry::new();
        let names = registry.provider_names();
        assert!(names.contains(&"claude".to_string()));
        assert!(names.contains(&"minimax".to_string()));
        assert_eq!(names.len(), 9);
    }

    #[tokio::test]
    async fn test_registry_build_unknown_provider() {
        let registry = FactoryRegistry::new();
        let config = ProviderConfig::for_custom("test");
        let result = registry.build_adapter("nonexistent", config).await;
        assert!(result.is_err());
    }
}
