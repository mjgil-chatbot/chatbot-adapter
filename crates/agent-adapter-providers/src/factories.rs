//! Factory implementations for built-in providers.

use std::path::PathBuf;

use agent_adapter_config::{CwdBehavior, PromptMode, ProviderConfig, ProviderKind};
use agent_adapter_core::{
    AdapterCapabilities, AdapterHealth, AdapterResult, AgentAdapter, ProviderId,
};
use async_trait::async_trait;

use crate::traits::{PolicySupport, ProviderFactory, TransportKind};
use crate::{
    ClaudeAdapter, ClawAdapter, CodexAdapter, ForgeAdapter, GeminiAdapter, HermesAdapter,
    KimiAdapter, OpenCodeAdapter,
};

/// Factory for Claude provider.
pub struct ClaudeFactory;

impl ClaudeFactory {
    pub fn new() -> Self {
        Self
    }
}

/// Factory for MiniMax via the Claude-compatible transport.
pub struct MiniMaxFactory;

impl MiniMaxFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MiniMaxFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderFactory for MiniMaxFactory {
    fn id(&self) -> &str {
        "minimax"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/minimax.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "minimax".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claude".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            mapper: "claude".to_string(),
            unsafe_args: vec!["--dangerously-skip-permissions".to_string()],
            safe_args: vec!["--permission-mode".to_string(), "default".to_string()],
            env_allowlist: vec![],
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            sessions: true,
            tools: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::JsonlCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        PolicySupport::both()
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::MiniMax, "claude").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(ClaudeAdapter::new_for_provider(
            config,
            ProviderId::MiniMax,
        )))
    }
}

impl Default for ClaudeFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderFactory for ClaudeFactory {
    fn id(&self) -> &str {
        "claude"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/claude.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "claude".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claude".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            mapper: "claude".to_string(),
            unsafe_args: vec!["--dangerously-skip-permissions".to_string()],
            safe_args: vec!["--permission-mode".to_string(), "default".to_string()],
            env_allowlist: vec![],
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            sessions: true,
            tools: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::JsonlCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        PolicySupport::both()
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Claude, "claude").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(ClaudeAdapter::new(config)))
    }
}

/// Factory for Codex provider.
pub struct CodexFactory;

impl CodexFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodexFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderFactory for CodexFactory {
    fn id(&self) -> &str {
        "codex"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/codex.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "codex".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "codex".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Stdin,
            mapper: "codex".to_string(),
            unsafe_args: vec!["--dangerously-bypass-approvals-and-sandbox".to_string()],
            safe_args: vec!["--sandbox".to_string(), "read-only".to_string()],
            env_allowlist: vec![],
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            sessions: true,
            tools: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::JsonlCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        PolicySupport::both()
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Codex, "codex").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(CodexAdapter::new(config)))
    }
}

/// Factory for Gemini provider.
pub struct GeminiFactory;

impl GeminiFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeminiFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderFactory for GeminiFactory {
    fn id(&self) -> &str {
        "gemini"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/gemini.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "gemini".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "gemini".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            mapper: "gemini".to_string(),
            unsafe_args: vec!["--approval-mode".to_string(), "yolo".to_string()],
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
            jsonl: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::JsonlCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        PolicySupport::both()
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Gemini, "gemini").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(GeminiAdapter::new(config)))
    }
}

/// Factory for Kimi provider.
pub struct KimiFactory;

impl KimiFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KimiFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderFactory for KimiFactory {
    fn id(&self) -> &str {
        "kimi"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/kimi.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "kimi".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "kimi".to_string(),
            args: vec!["--print".to_string()],
            prompt_mode: PromptMode::Arg,
            mapper: "kimi".to_string(),
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
            jsonl: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::JsonlCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        // Kimi dangerous only supports --print mode, safe requires ACP
        PolicySupport {
            supports_safe: false, // safe mode not fully supported for --print
            supports_dangerous: true,
            requires_explicit_unsafe_args: false,
        }
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Kimi, "kimi").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(KimiAdapter::new(config)))
    }
}

/// Factory for OpenCode provider.
pub struct OpenCodeFactory;

impl OpenCodeFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenCodeFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderFactory for OpenCodeFactory {
    fn id(&self) -> &str {
        "opencode"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/opencode.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "opencode".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "opencode".to_string(),
            args: vec![
                "run".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            prompt_mode: PromptMode::Arg,
            mapper: "opencode".to_string(),
            unsafe_args: vec![],
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
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::JsonlCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        PolicySupport::both()
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::OpenCode, "opencode").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(OpenCodeAdapter::new(config)))
    }
}

/// Factory for Hermes provider.
pub struct HermesFactory;

impl HermesFactory {
    pub fn new() -> Self {
        Self
    }
}

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
    }
}

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
    }
}

impl Default for ClawFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderFactory for ClawFactory {
    fn id(&self) -> &str {
        "claw"
    }

    fn docs_path(&self) -> PathBuf {
        PathBuf::from("docs/providers/claw.md")
    }

    fn default_config(&self) -> ProviderConfig {
        ProviderConfig {
            name: "claw".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "claw".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            mapper: "claw".to_string(),
            unsafe_args: vec![],
            safe_args: vec![],
            env_allowlist: vec![],
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            tools: true,
            ..Default::default()
        }
    }

    fn supported_transports(&self) -> Vec<TransportKind> {
        vec![TransportKind::JsonlCommand]
    }

    fn policy_support(&self) -> PolicySupport {
        // Claw requires explicit args for both policies
        PolicySupport {
            supports_safe: true,
            supports_dangerous: true,
            requires_explicit_unsafe_args: true,
        }
    }

    async fn detect(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Claw, "claw").await
    }

    async fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>> {
        Ok(Box::new(ClawAdapter::new(config)))
    }
}

#[cfg(test)]
mod tests;
