//! Provider registry for agent-adapter.
//!
//! This module provides a registry of provider configurations
//! and factory functions that can be used to build provider adapters.

use crate::{ConfigLoader, ProviderConfig, ProviderKind};
use std::collections::HashMap;

/// A registry of provider configurations.
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    configs: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    /// Create a new registry from configs.
    pub fn new(configs: Vec<ProviderConfig>) -> Self {
        let mut map = HashMap::new();
        for config in configs {
            map.insert(config.name.clone(), config);
        }
        Self { configs: map }
    }

    /// Load registry from config loader.
    pub fn from_loader(loader: &ConfigLoader) -> Result<Self, crate::ConfigError> {
        let configs = loader.load()?;
        Ok(Self::new(configs))
    }

    /// Get a provider config by name.
    pub fn get(&self, name: &str) -> Option<&ProviderConfig> {
        self.configs.get(name)
    }

    /// Get all provider names.
    pub fn names(&self) -> Vec<&str> {
        self.configs.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a provider exists.
    pub fn contains(&self, name: &str) -> bool {
        self.configs.contains_key(name)
    }

    /// Get the provider kind for a named provider.
    pub fn provider_kind(&self, name: &str) -> Option<ProviderKind> {
        self.configs.get(name).map(|c| c.kind.clone())
    }
}

/// Built-in provider names.
/// These are used when no config is provided.
pub fn built_in_provider_names() -> Vec<&'static str> {
    vec![
        "claude", "minimax", "codex", "gemini", "kimi", "opencode", "hermes", "forge", "claw",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    #[test]
    fn test_registry_from_configs() {
        let configs = vec![ProviderConfig {
            name: "claude".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claude".to_string(),
            args: vec!["-p".to_string(), "{prompt}".to_string()],
            ..Default::default()
        }];
        let registry = ProviderRegistry::new(configs);
        assert!(registry.contains("claude"));
        assert!(!registry.contains("codex"));
    }

    #[test]
    fn test_registry_get() {
        let configs = vec![ProviderConfig {
            name: "test".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "test".to_string(),
            ..Default::default()
        }];
        let registry = ProviderRegistry::new(configs);
        let config = registry.get("test").unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.exe, "test");
    }

    #[test]
    fn test_registry_names() {
        let configs = vec![
            ProviderConfig {
                name: "claude".to_string(),
                kind: ProviderKind::JsonlCommand,
                exe: "claude".to_string(),
                ..Default::default()
            },
            ProviderConfig {
                name: "codex".to_string(),
                kind: ProviderKind::JsonlCommand,
                exe: "codex".to_string(),
                ..Default::default()
            },
        ];
        let registry = ProviderRegistry::new(configs);
        let names = registry.names();
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"codex"));
    }

    #[test]
    fn test_registry_provider_kind() {
        let configs = vec![ProviderConfig {
            name: "test".to_string(),
            kind: ProviderKind::AcpStdio,
            exe: "test".to_string(),
            ..Default::default()
        }];
        let registry = ProviderRegistry::new(configs);
        let kind = registry.provider_kind("test").unwrap();
        assert!(matches!(kind, ProviderKind::AcpStdio));
    }

    #[test]
    fn test_registry_nonexistent() {
        let registry = ProviderRegistry::new(vec![]);
        assert!(registry.get("nonexistent").is_none());
        assert!(registry.provider_kind("nonexistent").is_none());
    }

    #[test]
    fn test_built_in_provider_names() {
        let names = built_in_provider_names();
        assert_eq!(names.len(), 9);
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"minimax"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"gemini"));
        assert!(names.contains(&"kimi"));
        assert!(names.contains(&"opencode"));
        assert!(names.contains(&"hermes"));
        assert!(names.contains(&"forge"));
        assert!(names.contains(&"claw"));
    }
}
