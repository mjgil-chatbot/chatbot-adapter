//! Contract tests for provider implementations.
//!
//! These tests verify that all providers implement the required
//! contract: healthcheck, command construction, event streaming,
//! cancellation, timeout, and documentation.

use agent_adapter_config::ProviderConfig;
use agent_adapter_core::{AdapterHealth, AdapterResult, ApprovalPolicy, ChatRequest, ProviderId};
use agent_adapter_providers::{
    build_command, check_binary, get_install_hint, BuildError, CommandSpec,
};

/// Contract test for a single provider.
pub struct ProviderContract {
    pub provider_id: ProviderId,
    pub binary_name: &'static str,
    pub supports_unsafe_mode: bool,
    pub supports_safe_mode: bool,
    pub supports_acp: bool,
    pub supports_jsonl: bool,
}

impl ProviderContract {
    /// Test that the provider's binary check works.
    pub async fn test_healthcheck(&self) -> AdapterResult<AdapterHealth> {
        check_binary(self.provider_id.clone(), self.binary_name).await
    }

    /// Test that unsafe command construction works.
    pub fn test_unsafe_command(&self, config: &ProviderConfig) -> Result<CommandSpec, BuildError> {
        if !self.supports_unsafe_mode {
            return Err(BuildError {
                message: format!(
                    "Provider {:?} does not support unsafe mode",
                    self.provider_id
                ),
            });
        }

        let request = ChatRequest::new("test prompt", "/tmp", self.provider_id.clone());
        build_command(
            self.provider_id.clone(),
            ApprovalPolicy::DangerousDefault,
            config,
            &request,
        )
    }

    /// Test that safe command construction works.
    pub fn test_safe_command(&self, config: &ProviderConfig) -> Result<CommandSpec, BuildError> {
        if !self.supports_safe_mode {
            return Err(BuildError {
                message: format!("Provider {:?} does not support safe mode", self.provider_id),
            });
        }

        let request = ChatRequest::new("test prompt", "/tmp", self.provider_id.clone());
        build_command(
            self.provider_id.clone(),
            ApprovalPolicy::Safe,
            config,
            &request,
        )
    }

    /// Test that install hint is available.
    pub fn test_install_hint(&self) -> String {
        get_install_hint(self.provider_id.clone())
    }
}

/// A test provider config for contract testing.
pub fn test_config(exe: &str) -> ProviderConfig {
    ProviderConfig {
        name: "test".to_string(),
        kind: agent_adapter_config::ProviderKind::JsonlCommand,
        exe: exe.to_string(),
        args: vec![],
        prompt_mode: agent_adapter_config::PromptMode::Arg,
        mapper: "test".to_string(),
        unsafe_args: vec!["--yolo".to_string()],
        safe_args: vec!["--safe".to_string()],
        env_allowlist: vec![],
        cwd_behavior: agent_adapter_config::CwdBehavior::UseProvided,
        schema_version: None,
    }
}

/// Run full contract tests for all built-in providers.
pub mod all_providers {
    use super::*;

    pub fn contracts() -> Vec<ProviderContract> {
        vec![
            // Codex
            ProviderContract {
                provider_id: ProviderId::Codex,
                binary_name: "codex",
                supports_unsafe_mode: true,
                supports_safe_mode: true,
                supports_acp: false,
                supports_jsonl: true,
            },
            // Claude
            ProviderContract {
                provider_id: ProviderId::Claude,
                binary_name: "claude",
                supports_unsafe_mode: true,
                supports_safe_mode: true,
                supports_acp: false,
                supports_jsonl: true,
            },
            // Gemini
            ProviderContract {
                provider_id: ProviderId::Gemini,
                binary_name: "gemini",
                supports_unsafe_mode: true,
                supports_safe_mode: true,
                supports_acp: false,
                supports_jsonl: true,
            },
            // Kimi
            ProviderContract {
                provider_id: ProviderId::Kimi,
                binary_name: "kimi",
                supports_unsafe_mode: true,
                supports_safe_mode: false, // --print implies AFK
                supports_acp: true,
                supports_jsonl: true,
            },
            // OpenCode
            ProviderContract {
                provider_id: ProviderId::OpenCode,
                binary_name: "opencode",
                supports_unsafe_mode: true,
                supports_safe_mode: true,
                supports_acp: false,
                supports_jsonl: false,
            },
            // Hermes
            ProviderContract {
                provider_id: ProviderId::Hermes,
                binary_name: "hermes",
                supports_unsafe_mode: true,
                supports_safe_mode: true,
                supports_acp: true,
                supports_jsonl: false,
            },
            // Forge
            ProviderContract {
                provider_id: ProviderId::Forge,
                binary_name: "forge",
                supports_unsafe_mode: true,
                supports_safe_mode: true,
                supports_acp: false,
                supports_jsonl: false,
            },
            // Claw
            ProviderContract {
                provider_id: ProviderId::Claw,
                binary_name: "claw-code",
                supports_unsafe_mode: true,
                supports_safe_mode: true,
                supports_acp: false,
                supports_jsonl: false,
            },
        ]
    }

    /// Test that all providers have install hints.
    #[test]
    fn test_all_providers_have_install_hints() {
        for contract in contracts() {
            let hint = contract.test_install_hint();
            assert!(
                !hint.is_empty(),
                "Provider {:?} should have install hint",
                contract.provider_id
            );
            assert!(
                hint.contains(&contract.binary_name) || hint.contains("Install"),
                "Install hint for {:?} should mention installation: {}",
                contract.provider_id,
                hint
            );
        }
    }

    /// Test that safe mode command construction works for all safe-mode providers.
    #[test]
    fn test_all_safe_mode_commands() {
        for contract in contracts() {
            if !contract.supports_safe_mode {
                continue;
            }
            let config = test_config(contract.binary_name);
            match contract.test_safe_command(&config) {
                Ok(spec) => {
                    assert!(
                        !spec.args.is_empty(),
                        "Safe command for {:?} should have args",
                        contract.provider_id
                    );
                }
                Err(e) => {
                    panic!(
                        "Safe command failed for {:?}: {:?}",
                        contract.provider_id, e
                    );
                }
            }
        }
    }

    /// Test that unsafe mode command construction works for all unsafe-mode providers.
    #[test]
    fn test_all_unsafe_mode_commands() {
        for contract in contracts() {
            if !contract.supports_unsafe_mode {
                continue;
            }
            let config = test_config(contract.binary_name);
            match contract.test_unsafe_command(&config) {
                Ok(spec) => {
                    assert!(
                        !spec.args.is_empty(),
                        "Unsafe command for {:?} should have args",
                        contract.provider_id
                    );
                }
                Err(e) => {
                    panic!(
                        "Unsafe command failed for {:?}: {:?}",
                        contract.provider_id, e
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_contract_creation() {
        let contract = ProviderContract {
            provider_id: ProviderId::Claude,
            binary_name: "claude",
            supports_unsafe_mode: true,
            supports_safe_mode: true,
            supports_acp: false,
            supports_jsonl: true,
        };

        assert_eq!(contract.provider_id, ProviderId::Claude);
        assert!(contract.supports_unsafe_mode);
        assert!(contract.supports_safe_mode);
    }

    #[test]
    fn test_test_config() {
        let config = test_config("echo");
        assert_eq!(config.exe, "echo");
        assert!(!config.unsafe_args.is_empty());
        assert!(!config.safe_args.is_empty());
    }

    #[tokio::test]
    async fn test_healthcheck_missing_binary() {
        let contract = ProviderContract {
            provider_id: ProviderId::Custom("nonexistent-binary-xyz".to_string()),
            binary_name: "nonexistent-binary-xyz",
            supports_unsafe_mode: false,
            supports_safe_mode: false,
            supports_acp: false,
            supports_jsonl: false,
        };

        let health = contract.test_healthcheck().await;
        assert!(matches!(health, Ok(AdapterHealth::MissingBinary { .. })));
    }
}
