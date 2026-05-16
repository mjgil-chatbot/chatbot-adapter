use super::*;
use crate::traits::{ProviderFactory, TransportKind};
use agent_adapter_core::ProviderId;

// =========================================================================
// Task 36: Factory tests - id, docs_path, default_config, capabilities,
// supported_transports, policy_support
// =========================================================================

#[test]
fn test_claude_factory_id() {
    let factory = ClaudeFactory::new();
    assert_eq!(factory.id(), "claude");
}

#[test]
fn test_codex_factory_id() {
    let factory = CodexFactory::new();
    assert_eq!(factory.id(), "codex");
}

#[test]
fn test_gemini_factory_id() {
    let factory = GeminiFactory::new();
    assert_eq!(factory.id(), "gemini");
}

#[test]
fn test_kimi_factory_id() {
    let factory = KimiFactory::new();
    assert_eq!(factory.id(), "kimi");
}

#[test]
fn test_opencode_factory_id() {
    let factory = OpenCodeFactory::new();
    assert_eq!(factory.id(), "opencode");
}

#[test]
fn test_hermes_factory_id() {
    let factory = HermesFactory::new();
    assert_eq!(factory.id(), "hermes");
}

#[test]
fn test_forge_factory_id() {
    let factory = ForgeFactory::new();
    assert_eq!(factory.id(), "forge");
}

#[test]
fn test_claw_factory_id() {
    let factory = ClawFactory::new();
    assert_eq!(factory.id(), "claw");
}

#[test]
fn test_all_factories_have_docs_path() {
    // Every factory should return a valid docs path
    let factories: Vec<Box<dyn ProviderFactory>> = vec![
        Box::new(ClaudeFactory::new()),
        Box::new(MiniMaxFactory::new()),
        Box::new(CodexFactory::new()),
        Box::new(GeminiFactory::new()),
        Box::new(KimiFactory::new()),
        Box::new(OpenCodeFactory::new()),
        Box::new(HermesFactory::new()),
        Box::new(ForgeFactory::new()),
        Box::new(ClawFactory::new()),
    ];
    for factory in factories {
        let docs = factory.docs_path();
        assert!(
            docs.to_string_lossy().starts_with("docs/providers/"),
            "Factory {} should have docs path starting with docs/providers/",
            factory.id()
        );
    }
}

#[test]
fn test_all_factories_have_default_config() {
    // Every factory should return a valid default config
    let factories: Vec<Box<dyn ProviderFactory>> = vec![
        Box::new(ClaudeFactory::new()),
        Box::new(MiniMaxFactory::new()),
        Box::new(CodexFactory::new()),
        Box::new(GeminiFactory::new()),
        Box::new(KimiFactory::new()),
        Box::new(OpenCodeFactory::new()),
        Box::new(HermesFactory::new()),
        Box::new(ForgeFactory::new()),
        Box::new(ClawFactory::new()),
    ];
    for factory in factories {
        let config = factory.default_config();
        assert_eq!(config.name, factory.id());
        assert!(!config.exe.is_empty());
        assert!(!config.mapper.is_empty());
    }
}

#[test]
fn test_all_factories_have_supported_transports() {
    // Every factory should return at least one supported transport
    let factories: Vec<Box<dyn ProviderFactory>> = vec![
        Box::new(ClaudeFactory::new()),
        Box::new(MiniMaxFactory::new()),
        Box::new(CodexFactory::new()),
        Box::new(GeminiFactory::new()),
        Box::new(KimiFactory::new()),
        Box::new(OpenCodeFactory::new()),
        Box::new(HermesFactory::new()),
        Box::new(ForgeFactory::new()),
        Box::new(ClawFactory::new()),
    ];
    for factory in factories {
        let transports = factory.supported_transports();
        assert!(
            !transports.is_empty(),
            "Factory {} should support at least one transport",
            factory.id()
        );
    }
}

#[test]
fn test_all_factories_have_policy_support() {
    // Every factory should return policy support
    let factories: Vec<Box<dyn ProviderFactory>> = vec![
        Box::new(ClaudeFactory::new()),
        Box::new(MiniMaxFactory::new()),
        Box::new(CodexFactory::new()),
        Box::new(GeminiFactory::new()),
        Box::new(KimiFactory::new()),
        Box::new(OpenCodeFactory::new()),
        Box::new(HermesFactory::new()),
        Box::new(ForgeFactory::new()),
        Box::new(ClawFactory::new()),
    ];
    for factory in factories {
        let policy = factory.policy_support();
        // At least one of safe or dangerous should be supported
        assert!(
            policy.supports_safe || policy.supports_dangerous,
            "Factory {} should support at least one policy",
            factory.id()
        );
    }
}

#[test]
fn test_claude_factory_transports() {
    let factory = ClaudeFactory::new();
    let transports = factory.supported_transports();
    assert!(transports.contains(&TransportKind::JsonlCommand));
}

#[test]
fn test_gemini_factory_transports() {
    let factory = GeminiFactory::new();
    let transports = factory.supported_transports();
    assert!(transports.contains(&TransportKind::JsonlCommand));
    // Gemini uses JSONL transport; ACP is documented but not implemented in the adapter
}

#[test]
fn test_hermes_factory_transports() {
    let factory = HermesFactory::new();
    let transports = factory.supported_transports();
    assert!(transports.contains(&TransportKind::OneShotCommand));
    // Hermes uses one-shot transport; ACP mode is documented but not implemented
}

#[test]
fn test_claw_factory_transports() {
    let factory = ClawFactory::new();
    let transports = factory.supported_transports();
    assert!(transports.contains(&TransportKind::JsonlCommand));
    // Claw defaults to JSONL; PtyFallback is for TUI-based interactions
}

#[test]
fn test_kimi_factory_policy_support() {
    // Kimi --print mode is dangerous-only
    let factory = KimiFactory::new();
    let policy = factory.policy_support();
    assert!(
        !policy.supports_safe,
        "Kimi --print mode doesn't support safe"
    );
    assert!(
        policy.supports_dangerous,
        "Kimi --print mode supports dangerous"
    );
}

#[test]
fn test_forge_factory_requires_unsafe_args() {
    let factory = ForgeFactory::new();
    let policy = factory.policy_support();
    assert!(policy.requires_explicit_unsafe_args);
}

#[test]
fn test_claw_factory_requires_explicit_args() {
    let factory = ClawFactory::new();
    let policy = factory.policy_support();
    assert!(policy.requires_explicit_unsafe_args);
}

#[tokio::test]
async fn test_claude_factory_build() {
    let factory = ClaudeFactory::new();
    let config = ProviderConfig {
        name: "claude".to_string(),
        kind: agent_adapter_config::ProviderKind::JsonlCommand,
        exe: "claude".to_string(),
        args: vec![],
        prompt_mode: agent_adapter_config::PromptMode::Arg,
        mapper: "claude".to_string(),
        unsafe_args: vec!["--dangerously-skip-permissions".to_string()],
        safe_args: vec!["--permission-mode".to_string(), "default".to_string()],
        env_allowlist: vec![],
        cwd_behavior: agent_adapter_config::CwdBehavior::UseProvided,
        schema_version: None,
    };
    let adapter = factory.build(config).await.unwrap();
    assert_eq!(adapter.id(), ProviderId::Claude);
}

#[tokio::test]
async fn test_codex_factory_build() {
    let factory = CodexFactory::new();
    let config = ProviderConfig {
        name: "codex".to_string(),
        kind: agent_adapter_config::ProviderKind::JsonlCommand,
        exe: "codex".to_string(),
        args: vec![],
        prompt_mode: agent_adapter_config::PromptMode::Stdin,
        mapper: "codex".to_string(),
        unsafe_args: vec!["--dangerously-bypass-approvals-and-sandbox".to_string()],
        safe_args: vec!["--sandbox".to_string(), "read-only".to_string()],
        env_allowlist: vec![],
        cwd_behavior: agent_adapter_config::CwdBehavior::UseProvided,
        schema_version: None,
    };
    let adapter = factory.build(config).await.unwrap();
    assert_eq!(adapter.id(), ProviderId::Codex);
}

#[tokio::test]
async fn test_gemini_factory_build() {
    let factory = GeminiFactory::new();
    let config = ProviderConfig {
        name: "gemini".to_string(),
        kind: agent_adapter_config::ProviderKind::JsonlCommand,
        exe: "gemini".to_string(),
        args: vec![],
        prompt_mode: agent_adapter_config::PromptMode::Arg,
        mapper: "gemini".to_string(),
        unsafe_args: vec!["--approval-mode".to_string(), "yolo".to_string()],
        safe_args: vec![],
        env_allowlist: vec![],
        cwd_behavior: agent_adapter_config::CwdBehavior::UseProvided,
        schema_version: None,
    };
    let adapter = factory.build(config).await.unwrap();
    assert_eq!(adapter.id(), ProviderId::Gemini);
}
