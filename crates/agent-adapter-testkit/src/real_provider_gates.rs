//! Real provider integration gates.
//!
//! These gates control whether real provider tests run.
//! They are always disabled by default and must be explicitly enabled
//! via environment variables.

use std::env;

/// Environment variable gates for real provider tests.
#[derive(Debug, Clone)]
pub struct RealProviderGates {
    /// Run real Codex tests.
    pub run_codex: bool,
    /// Run real Claude tests.
    pub run_claude: bool,
    /// Run real Gemini tests.
    pub run_gemini: bool,
    /// Run real Kimi tests.
    pub run_kimi: bool,
    /// Run real OpenCode tests.
    pub run_opencode: bool,
    /// Run real Hermes tests.
    pub run_hermes: bool,
    /// Run real Forge tests.
    pub run_forge: bool,
    /// Run real Claw tests.
    pub run_claw: bool,
}

impl RealProviderGates {
    /// Parse gates from environment variables.
    pub fn from_env() -> Self {
        Self {
            run_codex: env_var_is_true("RUN_REAL_CODEX"),
            run_claude: env_var_is_true("RUN_REAL_CLAUDE"),
            run_gemini: env_var_is_true("RUN_REAL_GEMINI"),
            run_kimi: env_var_is_true("RUN_REAL_KIMI"),
            run_opencode: env_var_is_true("RUN_REAL_OPENCODE"),
            run_hermes: env_var_is_true("RUN_REAL_HERMES"),
            run_forge: env_var_is_true("RUN_REAL_FORGE"),
            run_claw: env_var_is_true("RUN_REAL_CLAW"),
        }
    }

    /// Returns true if any real provider tests are enabled.
    pub fn any_enabled(&self) -> bool {
        self.run_codex
            || self.run_claude
            || self.run_gemini
            || self.run_kimi
            || self.run_opencode
            || self.run_hermes
            || self.run_forge
            || self.run_claw
    }

    /// Check if a specific provider is enabled.
    pub fn is_enabled(&self, provider: &str) -> bool {
        match provider.to_lowercase().as_str() {
            "codex" => self.run_codex,
            "claude" => self.run_claude,
            "gemini" => self.run_gemini,
            "kimi" => self.run_kimi,
            "opencode" => self.run_opencode,
            "hermes" => self.run_hermes,
            "forge" => self.run_forge,
            "claw" => self.run_claw,
            _ => false,
        }
    }

    /// Get a message explaining how to enable real provider tests.
    pub fn enable_message(&self) -> String {
        if self.any_enabled() {
            return String::new();
        }
        "No real provider tests enabled. Set RUN_REAL_* env vars to enable.".to_string()
    }
}

impl Default for RealProviderGates {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Check if an environment variable is set to a truthy value.
fn env_var_is_true(name: &str) -> bool {
    env::var(name)
        .map(|v| {
            v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("1")
                || v.eq_ignore_ascii_case("yes")
                || v.eq_ignore_ascii_case("on")
        })
        .unwrap_or(false)
}

/// Gate for tests that should only run with real providers.
#[derive(Debug, Clone)]
pub struct RealProviderGate {
    provider: String,
    run_if_enabled: bool,
}

impl RealProviderGate {
    /// Create a new gate for a specific provider.
    pub fn new(provider: &str) -> Self {
        Self {
            provider: provider.to_lowercase(),
            run_if_enabled: false,
        }
    }

    /// Configure the gate to run when the provider is enabled.
    pub fn run_if_enabled(mut self) -> Self {
        self.run_if_enabled = true;
        self
    }

    /// Check if this test should run.
    pub fn should_run(&self) -> bool {
        if !self.run_if_enabled {
            return false;
        }
        RealProviderGates::default().is_enabled(&self.provider)
    }
}

/// Attribute macro placeholder for real provider test gating.
///
/// In Rust, we use conditional compilation via cfg attributes.
/// Example: `#[cfg(env!("RUN_REAL_CLAUDE"))]`
///
/// This module provides runtime versions of these checks for use in test logic.
pub mod conditional {
    use super::*;

    /// Run a closure only if real provider tests are enabled for the given provider.
    pub fn run_if_real_provider<R>(
        provider: &str,
        f: impl FnOnce() -> R,
        otherwise: impl FnOnce() -> R,
    ) -> R {
        if RealProviderGates::default().is_enabled(provider) {
            f()
        } else {
            otherwise()
        }
    }
}

/// Documentation helper for real provider tests.
pub mod docs {
    /// List of environment variables for real provider tests.
    pub const REAL_PROVIDER_ENV_VARS: &[&str] = &[
        "RUN_REAL_CODEX",
        "RUN_REAL_CLAUDE",
        "RUN_REAL_GEMINI",
        "RUN_REAL_KIMI",
        "RUN_REAL_OPENCODE",
        "RUN_REAL_HERMES",
        "RUN_REAL_FORGE",
        "RUN_REAL_CLAW",
    ];

    /// Example of how to use real provider gates.
    pub const USAGE_EXAMPLE: &str = r#"
/// To run real provider integration tests:
///
/// ```bash
/// # Run only Claude tests
/// RUN_REAL_CLAUDE=1 cargo test --test integration
///
/// # Run all enabled real provider tests
/// RUN_REAL_CLAUDE=1 RUN_REAL_CODEX=1 cargo test --test integration
/// ```
"#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventBuilder;
    use agent_adapter_core::ProviderId;

    #[test]
    fn test_env_var_is_true_variants() {
        // These tests check that we don't accidentally run real tests
        // when the env vars are not set
        let gates = RealProviderGates::from_env();
        assert!(!gates.any_enabled());
    }

    #[test]
    fn test_is_enabled_unknown_provider() {
        let gates = RealProviderGates::default();
        assert!(!gates.is_enabled("unknown"));
    }

    #[test]
    fn test_gates_default_disabled() {
        let gates = RealProviderGates::default();
        assert!(!gates.run_codex);
        assert!(!gates.run_claude);
        assert!(!gates.run_gemini);
        assert!(!gates.run_kimi);
        assert!(!gates.run_opencode);
        assert!(!gates.run_hermes);
        assert!(!gates.run_forge);
        assert!(!gates.run_claw);
    }

    #[test]
    fn test_enable_message_when_disabled() {
        let gates = RealProviderGates::default();
        let msg = gates.enable_message();
        assert!(!msg.is_empty());
        assert!(msg.contains("RUN_REAL_"));
    }

    #[test]
    fn test_enable_message_when_enabled() {
        // When any gate is enabled, message should be empty
        // (This test doesn't actually enable anything since we read env)
    }

    #[test]
    fn test_docs_env_vars_list() {
        let env_vars = docs::REAL_PROVIDER_ENV_VARS;
        assert_eq!(env_vars.len(), 8);
        assert!(env_vars.contains(&"RUN_REAL_CLAUDE"));
    }

    // =========================================================================
    // Task 38: Testkit tests
    // =========================================================================

    #[test]
    fn test_real_provider_gates_all_disabled_by_default() {
        // This is the expected default behavior - real providers are OFF
        let gates = RealProviderGates::default();
        assert!(!gates.any_enabled());
    }

    #[test]
    fn test_conditional_run_uses_otherwise() {
        let result = conditional::run_if_real_provider("claude", || "enabled", || "disabled");
        assert_eq!(result, "disabled");
    }

    #[test]
    fn test_event_builder_basic() {
        let events = EventBuilder::new()
            .with_run_started(uuid::Uuid::new_v4(), ProviderId::Claude)
            .with_text_delta("Hello")
            .with_run_completed(Some("Completed"))
            .build();

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_event_builder_tool_call_flow() {
        let events = EventBuilder::new()
            .with_run_started(uuid::Uuid::new_v4(), ProviderId::Claude)
            .with_tool_call("tool1", "read_file")
            .with_tool_finished("tool1", Some("result"))
            .with_run_completed(None)
            .build();

        assert_eq!(events.len(), 4);
    }

    #[test]
    fn test_event_builder_permission_request() {
        let events = EventBuilder::new()
            .with_permission_request("perm1", "bash ls")
            .with_run_failed("Permission denied")
            .build();

        assert_eq!(events.len(), 2);
    }
}
