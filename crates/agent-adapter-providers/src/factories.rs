//! Factory implementations for built-in providers.

use std::path::PathBuf;
use agent_adapter_config::{CwdBehavior, PromptMode, ProviderConfig, ProviderKind};
use agent_adapter_core::{
    AdapterCapabilities, AdapterHealth, AdapterResult, AgentAdapter, ProviderId,
};
use async_trait::async_trait;
use crate::traits::{PolicySupport, ProviderFactory, TransportKind};
use crate::ForgeAdapter;
use crate::HermesAdapter;
/// Factory for Claude provider.
pub struct ClaudeFactory;
impl ClaudeFactory {
    pub fn new() -> Self {
        Self
    }}
/// Factory for MiniMax via the Claude-compatible transport.
pub struct MiniMaxFactory;
impl MiniMaxFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for MiniMaxFactory {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for ClaudeFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Codex provider.
pub struct CodexFactory;
impl CodexFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for CodexFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Gemini provider.
pub struct GeminiFactory;
impl GeminiFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for GeminiFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Kimi provider.
pub struct KimiFactory;
impl KimiFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for KimiFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for OpenCode provider.
pub struct OpenCodeFactory;
impl OpenCodeFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for OpenCodeFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Hermes provider.
pub struct HermesFactory;
impl HermesFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for HermesFactory {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl ProviderFactory for HermesFactory {
    fn id(&self) -> &str {
        "hermes"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/hermes.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "hermes".to_string(),
            kind: ProviderKind::OneShotCommand,
            exe: "hermes".to_string(),
            args: vec!["-z".to_string()],
            prompt_mode: PromptMode::Arg,
            mapper: "hermes".to_string(),
            unsafe_args: vec!["--yolo".to_string()],
            safe_args: vec![],
            env_allowlist: vec![],
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            sessions: true,
            tools: true,
            oneshot: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::OneShotCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        PolicySupport::both()
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Hermes, "hermes").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(HermesAdapter::new(config)))
    }
}
/// Factory for Forge provider.
pub struct ForgeFactory;
impl ForgeFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for ForgeFactory {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl ProviderFactory for ForgeFactory {
    fn id(&self) -> &str {
        "forge"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/forge.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "forge".to_string(),
            kind: ProviderKind::OneShotCommand,
            exe: "forge".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            mapper: "forge".to_string(),
            unsafe_args: vec![],
            safe_args: vec!["-p".to_string()],
            env_allowlist: vec![],
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            tools: true,
            oneshot: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::OneShotCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        // Forge dangerous mode requires explicit unsafe_args
        PolicySupport {
            supports_safe: true,
            supports_dangerous: true,
            requires_explicit_unsafe_args: true,
        }
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Forge, "forge").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(ForgeAdapter::new(config)))
    }
}
/// Factory for Claw provider.
pub struct ClawFactory;
impl ClawFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for ClawFactory {
    fn default() -> Self {
        Self::new()
    }
}
