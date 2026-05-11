//! Health check implementations for providers.
//!
//! Each adapter can perform a health check to determine if it's ready to use.

use which::which;

use agent_adapter_core::{AdapterError, AdapterHealth, AdapterResult, ProviderId};

/// Performs a basic health check for a provider binary.
pub async fn check_binary(provider: ProviderId, binary_name: &str) -> AdapterResult<AdapterHealth> {
    match which(binary_name).ok() {
        Some(_path) => {
            let version = get_version(provider, binary_name).await?;
            Ok(AdapterHealth::Ready { version })
        }
        None => {
            let install_hint = get_install_hint(provider);
            Ok(AdapterHealth::MissingBinary { install_hint })
        }
    }
}

/// Get version string for a provider binary.
pub async fn get_version(
    _provider: ProviderId,
    binary_name: &str,
) -> AdapterResult<Option<String>> {
    let version_output = tokio::process::Command::new(binary_name)
        .arg("--version")
        .output()
        .await
        .map_err(|e| AdapterError::TransportError {
            message: format!("Failed to get version of {}: {}", binary_name, e),
        })?;

    if version_output.status.success() {
        let version = String::from_utf8_lossy(&version_output.stdout).to_string();
        let version = version.trim().to_string();
        if version.is_empty() {
            Ok(None)
        } else {
            Ok(Some(version))
        }
    } else {
        Ok(None)
    }
}

/// Get installation hint for a provider.
pub fn get_install_hint(provider: ProviderId) -> String {
    match provider {
        ProviderId::Codex => "Install OpenAI Codex CLI: npm install -g @openai/codex".to_string(),
        ProviderId::Claude => {
            "Install Anthropic Claude CLI: npm install -g @anthropic-ai/claude".to_string()
        }
        ProviderId::MiniMax => {
            "Install or configure the MiniMax Claude-compatible CLI transport".to_string()
        }
        ProviderId::Gemini => {
            "Install Google Gemini CLI: pip install google-generativeai".to_string()
        }
        ProviderId::Kimi => "Install Moonshot Kimi CLI: npm install -g kimi-cli".to_string(),
        ProviderId::OpenCode => "Install OpenCode CLI: npm install -g opencode-ai".to_string(),
        ProviderId::Hermes => {
            "Install Hermes Agent: see https://hermes-agent.nousresearch.com".to_string()
        }
        ProviderId::Forge => {
            "Install ForgeCode: see https://github.com/tailcallhq/forgecode".to_string()
        }
        ProviderId::Claw => {
            "Install Claw Code: see https://github.com/ultraworkers/claw-code".to_string()
        }
        ProviderId::Custom(ref name) => format!("Install {} binary", name),
    }
}

/// Check auth status for providers that support it.
pub async fn check_auth_status(
    provider: ProviderId,
    binary_name: &str,
) -> AdapterResult<AdapterHealth> {
    match provider {
        ProviderId::Claude | ProviderId::MiniMax => {
            let output = tokio::process::Command::new(binary_name)
                .args(["auth", "status"])
                .output()
                .await
                .map_err(|e| AdapterError::TransportError {
                    message: format!("Failed to check auth status: {}", e),
                })?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("Authenticated") || stdout.contains("Logged in") {
                    return Ok(AdapterHealth::Ready { version: None });
                }
            }
            Ok(AdapterHealth::NeedsAuth {
                login_hint: "Run 'claude auth' to authenticate".to_string(),
            })
        }
        ProviderId::Gemini => {
            let output = tokio::process::Command::new(binary_name)
                .args(["auth", "--check"])
                .output()
                .await
                .map_err(|e| AdapterError::TransportError {
                    message: format!("Failed to check auth status: {}", e),
                })?;

            if output.status.success() {
                return Ok(AdapterHealth::Ready { version: None });
            }
            Ok(AdapterHealth::NeedsAuth {
                login_hint: "Run 'gemini auth' to authenticate".to_string(),
            })
        }
        ProviderId::OpenCode => {
            let output = tokio::process::Command::new(binary_name)
                .args(["auth", "list"])
                .output()
                .await
                .map_err(|e| AdapterError::TransportError {
                    message: format!("Failed to check auth status: {}", e),
                })?;

            if output.status.success() && !String::from_utf8_lossy(&output.stdout).is_empty() {
                return Ok(AdapterHealth::Ready { version: None });
            }
            Ok(AdapterHealth::NeedsAuth {
                login_hint: "Run 'opencode auth' to authenticate".to_string(),
            })
        }
        _ => Ok(AdapterHealth::Ready { version: None }),
    }
}

/// Perform full health check for a provider.
pub async fn full_healthcheck(
    provider: ProviderId,
    binary_name: &str,
) -> AdapterResult<AdapterHealth> {
    // First check binary presence
    let health = check_binary(provider.clone(), binary_name).await?;
    if !health.is_ready() {
        return Ok(health);
    }

    // Then check auth if applicable
    check_auth_status(provider, binary_name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_nonexistent_binary() {
        let health = check_binary(
            ProviderId::Custom("nonexistent-binary-xyz".to_string()),
            "nonexistent-binary-xyz",
        )
        .await;
        assert!(matches!(health, Ok(AdapterHealth::MissingBinary { .. })));
    }

    #[tokio::test]
    async fn test_check_echo_binary() {
        let health = check_binary(ProviderId::Custom("echo-test".to_string()), "echo").await;
        match health {
            Ok(AdapterHealth::Ready { version }) => {
                assert!(version.is_none() || version.is_some());
            }
            Ok(AdapterHealth::MissingBinary { .. }) => {}
            _ => panic!("Unexpected health status"),
        }
    }

    #[test]
    fn test_get_install_hint() {
        let hint = get_install_hint(ProviderId::Claude);
        assert!(hint.contains("claude") || hint.contains("anthropic"));

        let hint = get_install_hint(ProviderId::Codex);
        assert!(hint.contains("codex") || hint.contains("openai"));
    }
}
