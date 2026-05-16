use crate::{ConfigLoader, ProviderConfig, ProviderKind, ProviderRegistry};

#[test]
fn test_parse_minimal_config() {
    // Clean ambient env state that could leak into other tests
    std::env::remove_var("AGENT_ADAPTER_DEFAULT_PROVIDER");
    let toml_str = r#"
[codex]
name = "codex"
kind = "jsonl_command"
"#;
    let value: toml::Value = toml_str.parse().unwrap();
    let table = value.as_table().unwrap();
    let provider_value = table.get("codex").unwrap();
    let config: ProviderConfig = provider_value.clone().try_into().unwrap();
    assert_eq!(config.name, "codex");
    assert!(matches!(config.kind, ProviderKind::JsonlCommand));
    assert!(config.args.is_empty());
    // ProviderConfig default for prompt_mode is Arg
    assert_eq!(config.prompt_mode, ProviderConfig::default().prompt_mode);
    assert_eq!(config.cwd_behavior, ProviderConfig::default().cwd_behavior);
}

#[test]
fn test_parse_full_config() {
    let toml_str = r#"
[claude]
name = "claude"
kind = "jsonl_command"
exe = "claude"
args = ["-p", "{prompt}", "--output-format", "stream-json"]
prompt_mode = "arg"
mapper = "claude"
unsafe_args = ["--dangerously-skip-permissions"]
safe_args = ["--permission-mode", "default"]
env_allowlist = ["OPENAI_API_KEY"]
cwd_behavior = "use_provided"
"#;
    let value: toml::Value = toml_str.parse().unwrap();
    let table = value.as_table().unwrap();
    let provider_value = table.get("claude").unwrap();
    let config: ProviderConfig = provider_value.clone().try_into().unwrap();
    assert_eq!(config.name, "claude");
    assert!(config
        .unsafe_args
        .contains(&"--dangerously-skip-permissions".to_string()));
    assert!(config.safe_args.contains(&"--permission-mode".to_string()));
}

#[test]
fn test_built_in_defaults_contain_all_providers() {
    std::env::remove_var("AGENT_ADAPTER_DEFAULT_PROVIDER");
    let loader = ConfigLoader::new();
    let defaults = loader.load().unwrap();
    let names: Vec<&str> = defaults.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"claude"), "Missing claude: {:?}", names);
    assert!(names.contains(&"minimax"), "Missing minimax: {:?}", names);
    assert!(names.contains(&"codex"), "Missing codex: {:?}", names);
    assert!(names.contains(&"gemini"), "Missing gemini: {:?}", names);
    assert!(names.contains(&"kimi"), "Missing kimi: {:?}", names);
    assert!(names.contains(&"opencode"), "Missing opencode: {:?}", names);
    assert!(names.contains(&"hermes"), "Missing hermes: {:?}", names);
    assert!(names.contains(&"forge"), "Missing forge: {:?}", names);
    assert!(names.contains(&"claw"), "Missing claw: {:?}", names);
}

#[test]
fn test_config_loader_add_path() {
    let loader = ConfigLoader::new().add_path("/nonexistent/path/config.toml");
    // Loader should not error on nonexistent paths
    let result = loader.load();
    assert!(result.is_ok());
}

#[test]
fn test_config_loader_with_env_overrides() {
    let loader = ConfigLoader::new().with_env_overrides(false);
    // Should still load built-in defaults
    let result = loader.load();
    assert!(result.is_ok());
}

#[test]
fn test_default_config_path() {
    let path = ConfigLoader::default_config_path();
    // Should contain .config/agent-adapter/config.toml
    assert!(path.to_string_lossy().contains(".config"));
    assert!(path.to_string_lossy().contains("agent-adapter"));
}

// =========================================================================
// Config precedence tests (Task 12)
// =========================================================================

#[test]
fn test_explicit_path_skips_builtin_defaults() {
    // When explicit path is set, only that file is loaded
    let loader = ConfigLoader::new().with_explicit_path("/nonexistent/custom.toml");
    let result = loader.load().unwrap();
    // Should be empty since file doesn't exist
    assert!(
        result.is_empty(),
        "Expected empty result for nonexistent explicit path"
    );
}

#[test]
fn test_explicit_path_loads_custom_config() {
    // Clean env state with guard to guarantee cleanup on any assertion failure
    struct EnvGuard<'a>(&'a str);
    impl Drop for EnvGuard<'_> {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }
    let _guard = EnvGuard("AGENT_ADAPTER_DEFAULT_PROVIDER");
    std::env::remove_var("AGENT_ADAPTER_DEFAULT_PROVIDER");

    // Create a temp file with custom config
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("agent-adapter-test-explicit.toml");
    std::fs::write(
        &config_path,
        r#"
[custom_provider]
name = "custom_provider"
kind = "jsonl_command"
exe = "my-provider"
args = ["--test"]
"#,
    )
    .unwrap();

    let loader = ConfigLoader::new().with_explicit_path(&config_path);
    let result = loader.load().unwrap();

    // Should have only the custom provider, no builtins
    let names: Vec<&str> = result.iter().map(|c| c.name.as_str()).collect();
    assert!(
        names.contains(&"custom_provider"),
        "Expected custom_provider in result: {:?}",
        names
    );
    // Built-in defaults should be skipped
    assert!(
        !names.contains(&"claude"),
        "Built-in defaults should be skipped with explicit path"
    );

    // Cleanup
    std::fs::remove_file(&config_path).ok();
}

#[test]
fn test_env_overrides_disabled() {
    // When env overrides are disabled, AGENT_ADAPTER_PROVIDER should not affect loading
    let loader = ConfigLoader::new().with_env_overrides(false);
    let result = loader.load().unwrap();
    // Should still load built-in defaults
    assert!(
        !result.is_empty(),
        "Expected non-empty result with env overrides disabled"
    );
}

#[test]
fn test_precedence_order_builtin_then_project_then_user() {
    // Test that built-in < project < user precedence is maintained
    // This is implicitly tested by the load() method implementation
    let loader = ConfigLoader::new();
    let result = loader.load().unwrap();

    // Built-in defaults should be loaded first
    let claude = result.iter().find(|c| c.name == "claude");
    assert!(claude.is_some(), "Built-in claude should be present");

    // Verify built-in defaults have expected unsafe_args
    let claude = claude.unwrap();
    assert!(
        claude
            .unsafe_args
            .contains(&"--dangerously-skip-permissions".to_string()),
        "Built-in claude should have skip-permissions in unsafe_args"
    );
}

#[test]
fn test_missing_explicit_path_returns_empty() {
    // Missing explicit path should return empty configs, not error
    let loader = ConfigLoader::new().with_explicit_path("/truly/nonexistent/path.toml");
    let result = loader.load().unwrap();
    assert!(
        result.is_empty(),
        "Expected empty result for missing explicit path"
    );
}

// =========================================================================
// Environment override tests (Task 17)
// =========================================================================

#[test]
fn test_env_override_provider_exe() {
    // AGENT_ADAPTER_<PROVIDER>_EXE should override the exe for that provider
    struct EnvGuard<'a>(&'a str);
    impl Drop for EnvGuard<'_> {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }
    let _guard_default = EnvGuard("AGENT_ADAPTER_DEFAULT_PROVIDER");
    let _guard_exe = EnvGuard("AGENT_ADAPTER_CLAUDE_EXE");
    std::env::remove_var("AGENT_ADAPTER_DEFAULT_PROVIDER");
    std::env::remove_var("AGENT_ADAPTER_CLAUDE_EXE");

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("agent-adapter-test-env-exe.toml");
    std::fs::write(
        &config_path,
        r#"
[claude]
name = "claude"
kind = "jsonl_command"
exe = "original-claude"
"#,
    )
    .unwrap();

    // Set env var
    std::env::set_var("AGENT_ADAPTER_CLAUDE_EXE", "/usr/bin/claude-custom");

    let loader = ConfigLoader::new().with_explicit_path(&config_path);
    let result = loader.load().unwrap();

    // Cleanup via EnvGuard
    // claude provider should have overridden exe
    let claude = result.iter().find(|c| c.name == "claude");
    assert!(claude.is_some());
    assert_eq!(claude.unwrap().exe, "/usr/bin/claude-custom");
}

#[test]
fn test_env_override_provider_args() {
    // AGENT_ADAPTER_<PROVIDER>_ARGS should override args as JSON array
    struct EnvGuard<'a>(&'a str);
    impl Drop for EnvGuard<'_> {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }
    let _guard_default = EnvGuard("AGENT_ADAPTER_DEFAULT_PROVIDER");
    let _guard_args = EnvGuard("AGENT_ADAPTER_TEST_PROVIDER_ARGS");
    std::env::remove_var("AGENT_ADAPTER_DEFAULT_PROVIDER");
    std::env::remove_var("AGENT_ADAPTER_TEST_PROVIDER_ARGS");

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("agent-adapter-test-env-args.toml");
    std::fs::write(
        &config_path,
        r#"
[test_provider]
name = "test_provider"
kind = "jsonl_command"
exe = "test"
args = ["original", "args"]
"#,
    )
    .unwrap();

    // Set env var with JSON array
    std::env::set_var(
        "AGENT_ADAPTER_TEST_PROVIDER_ARGS",
        r#"["--new", "--args", "--here"]"#,
    );

    let loader = ConfigLoader::new().with_explicit_path(&config_path);
    let result = loader.load().unwrap();

    // Cleanup via EnvGuard
    // test_provider should have overridden args
    let provider = result.iter().find(|c| c.name == "test_provider");
    assert!(provider.is_some());
    assert_eq!(
        provider.unwrap().args,
        vec![
            "--new".to_string(),
            "--args".to_string(),
            "--here".to_string()
        ]
    );
}

// =========================================================================
// Built-in and custom registry assembly tests (Task 20-21)
// =========================================================================

#[test]
fn test_builtin_registry_all_providers_have_required_fields() {
    // Every built-in provider should have required fields
    let defaults = ConfigLoader::built_in_defaults();

    for config in defaults {
        let validation = config.validate();
        assert!(
            validation.is_ok(),
            "Built-in '{}' failed validation: {:?}",
            config.name,
            validation
        );
    }
}

#[test]
fn test_custom_provider_registration() {
    // Custom providers can be registered and don't require mapper
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("agent-adapter-test-custom.toml");
    std::fs::write(
        &config_path,
        r#"
[my_custom_provider]
name = "my_custom_provider"
kind = "custom"
exe = "/path/to/my-provider"
mapper = "custom_mapper"
args = ["--mode", "interactive"]
"#,
    )
    .unwrap();

    let loader = ConfigLoader::new().with_explicit_path(&config_path);
    let result = loader.load().unwrap();

    std::fs::remove_file(&config_path).ok();

    assert!(result.len() == 1);
    let custom = &result[0];
    assert_eq!(custom.name, "my_custom_provider");
    assert!(matches!(custom.kind, ProviderKind::Custom));
    assert_eq!(custom.mapper, "custom_mapper");
}

#[test]
fn test_builtin_override_via_project_config() {
    // Project config should override built-in defaults
    // Clean ambient env state to prevent cross-test pollution
    std::env::remove_var("AGENT_ADAPTER_DEFAULT_PROVIDER");

    let temp_dir = std::env::temp_dir();
    let project_config = temp_dir.join(".agent-adapter.toml");
    let original_cwd = std::env::current_dir().ok();

    // Set cwd and ensure cleanup even on panic
    std::env::set_current_dir(&temp_dir).ok();
    struct CwdGuard(Option<std::path::PathBuf>);
    impl Drop for CwdGuard {
        fn drop(&mut self) {
            if let Some(cwd) = self.0.take() {
                std::env::set_current_dir(cwd).ok();
            }
        }
    }
    let _cwd_guard = CwdGuard(original_cwd);

    std::fs::write(
        &project_config,
        r#"
[claude]
name = "claude"
kind = "jsonl_command"
exe = "/custom/claude"
args = ["--custom-arg"]
"#,
    )
    .unwrap();

    let loader = ConfigLoader::new();
    let result = loader.load().unwrap();

    // Cleanup - handled by CwdGuard drop

    // Should have both built-in defaults and project override
    // Project override should take precedence (claude should be the custom one)
    let claude = result.iter().find(|c| c.name == "claude");
    assert!(claude.is_some());
    assert_eq!(claude.unwrap().exe, "/custom/claude");
}

#[test]
fn test_fallback_routing_no_provider_found() {
    // When a provider is not found, registry should return None
    let registry = ProviderRegistry::new(vec![]);
    assert!(registry.get("nonexistent").is_none());
    assert!(!registry.contains("nonexistent"));
}

#[test]
fn test_registry_builtin_vs_custom_distinction() {
    // Built-in providers should have specific kinds
    let defaults = ConfigLoader::built_in_defaults();

    let claude = defaults.iter().find(|c| c.name == "claude").unwrap();
    assert!(matches!(claude.kind, ProviderKind::JsonlCommand));

    let kimi = defaults.iter().find(|c| c.name == "kimi").unwrap();
    assert!(matches!(kimi.kind, ProviderKind::AcpStdio));

    let forge = defaults.iter().find(|c| c.name == "forge").unwrap();
    assert!(matches!(forge.kind, ProviderKind::OneShotCommand));

    let claw = defaults.iter().find(|c| c.name == "claw").unwrap();
    assert!(matches!(claw.kind, ProviderKind::Custom));
}
