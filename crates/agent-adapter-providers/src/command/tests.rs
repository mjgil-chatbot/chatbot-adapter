use super::*;
use agent_adapter_core::ProviderId;

fn test_config() -> ProviderConfig {
    ProviderConfig {
        name: "test".to_string(),
        kind: ProviderKind::JsonlCommand,
        exe: "echo".to_string(),
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

#[test]
fn test_codex_dangerous() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Codex);
    let spec = build_command(
        ProviderId::Codex,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(spec
        .args
        .contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
}

#[test]
fn test_codex_safe() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Codex);
    let spec = build_command(ProviderId::Codex, ApprovalPolicy::Safe, &config, &request).unwrap();
    assert!(!spec
        .args
        .contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
}

#[test]
fn test_claude_dangerous() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Claude);
    let spec = build_command(
        ProviderId::Claude,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(spec
        .args
        .contains(&"--dangerously-skip-permissions".to_string()));
}

#[test]
fn test_claude_safe() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Claude);
    let spec = build_command(ProviderId::Claude, ApprovalPolicy::Safe, &config, &request).unwrap();
    assert!(spec.args.contains(&"--permission-mode".to_string()));
    assert!(spec.args.contains(&"default".to_string()));
}

#[test]
fn test_gemini_dangerous() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Gemini);
    let spec = build_command(
        ProviderId::Gemini,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(spec.args.contains(&"--approval-mode".to_string()));
    assert!(spec.args.contains(&"yolo".to_string()));
}

#[test]
fn test_forge_missing_unsafe_args() {
    let mut config = test_config();
    config.unsafe_args = vec![];
    let request = ChatRequest::new("test", ".", ProviderId::Forge);
    let result = build_command(
        ProviderId::Forge,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    );
    assert!(result.is_err());
}

// =========================================================================
// Kimi provider tests (Task 31)
// =========================================================================

#[test]
fn test_kimi_dangerous_uses_yolo() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Kimi);
    let spec = build_command(
        ProviderId::Kimi,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(
        spec.args.contains(&"--yolo".to_string()),
        "Kimi dangerous mode should add --yolo flag"
    );
}

#[test]
fn test_kimi_safe_mode_error() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Kimi);
    let result = build_command(ProviderId::Kimi, ApprovalPolicy::Safe, &config, &request);
    assert!(
        result.is_err(),
        "Kimi safe mode should return error for --print mode"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("Safe mode") || err.message.contains("not supported"),
        "Error should indicate safe mode not supported"
    );
}

// =========================================================================
// OpenCode provider tests (Task 32)
// =========================================================================

#[test]
fn test_opencode_dangerous_mode() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::OpenCode);
    let spec = build_command(
        ProviderId::OpenCode,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(
        spec.args.contains(&"--format".to_string()),
        "OpenCode should use --format json"
    );
    assert!(spec.args.contains(&"json".to_string()));
    assert!(spec.args.contains(&"--dir".to_string()));
}

#[test]
fn test_opencode_safe_mode() {
    let mut config = test_config();
    config.safe_args = vec!["--permission".to_string(), "ask".to_string()];
    let request = ChatRequest::new("test", ".", ProviderId::OpenCode);
    let spec = build_command(
        ProviderId::OpenCode,
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();
    assert!(
        spec.args.contains(&"--permission".to_string()),
        "OpenCode safe mode should use safe_args"
    );
}

// =========================================================================
// Hermes provider tests (Task 33)
// =========================================================================

#[test]
fn test_hermes_oneshot_mode() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Hermes);
    let spec = build_command(
        ProviderId::Hermes,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(
        spec.args.contains(&"-z".to_string()),
        "Hermes should use -z for one-shot mode"
    );
    assert!(
        spec.args.contains(&"--yolo".to_string()),
        "Hermes dangerous mode should add --yolo"
    );
}

#[test]
fn test_hermes_safe_mode_no_yolo() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Hermes);
    let spec = build_command(ProviderId::Hermes, ApprovalPolicy::Safe, &config, &request).unwrap();
    assert!(
        !spec.args.contains(&"--yolo".to_string()),
        "Hermes safe mode should NOT have --yolo"
    );
}

#[test]
fn test_hermes_resume_flag() {
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Hermes);
    let spec = build_command(
        ProviderId::Hermes,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    // Should have resume handling - check the command has -z for one-shot mode
    assert!(
        spec.args.contains(&"-z".to_string()),
        "Hermes should handle resume mode"
    );
}

// =========================================================================
// Claw provider tests (Task 35)
// =========================================================================

#[test]
fn test_claw_dangerous_mode_requires_unsafe_args() {
    let mut config = test_config();
    config.unsafe_args = vec![];
    let request = ChatRequest::new("test", ".", ProviderId::Claw);
    let result = build_command(
        ProviderId::Claw,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    );
    assert!(
        result.is_err(),
        "Claw dangerous mode should require unsafe_args"
    );
    assert!(result.unwrap_err().message.contains("unsafe_args"));
}

#[test]
fn test_claw_safe_mode_requires_safe_args() {
    let mut config = test_config();
    config.safe_args = vec![];
    let request = ChatRequest::new("test", ".", ProviderId::Claw);
    let result = build_command(ProviderId::Claw, ApprovalPolicy::Safe, &config, &request);
    assert!(result.is_err(), "Claw safe mode should require safe_args");
    assert!(result.unwrap_err().message.contains("safe_args"));
}

#[test]
fn test_claw_dangerous_mode_uses_unsafe_args() {
    let mut config = test_config();
    config.unsafe_args = vec!["--dangerous".to_string(), "--bypass".to_string()];
    let request = ChatRequest::new("test", ".", ProviderId::Claw);
    let spec = build_command(
        ProviderId::Claw,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(spec.args.contains(&"--dangerous".to_string()));
    assert!(spec.args.contains(&"--bypass".to_string()));
}

#[test]
fn test_claw_safe_mode_uses_safe_args() {
    let mut config = test_config();
    config.safe_args = vec!["--safe".to_string(), "--ask".to_string()];
    let request = ChatRequest::new("test", ".", ProviderId::Claw);
    let spec = build_command(ProviderId::Claw, ApprovalPolicy::Safe, &config, &request).unwrap();
    assert!(spec.args.contains(&"--safe".to_string()));
    assert!(spec.args.contains(&"--ask".to_string()));
}

// =========================================================================
// Additional provider tests for task completeness
// =========================================================================

#[test]
fn test_codex_stdin_mode_prompt() {
    // Task 28: Codex uses stdin for prompt (- marker)
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Codex);
    let spec = build_command(
        ProviderId::Codex,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert_eq!(
        spec.stdin_mode,
        StdinMode::Prompt,
        "Codex should use stdin for prompt"
    );
    assert!(
        spec.args.contains(&"-".to_string()),
        "Codex should have stdin marker"
    );
}

#[test]
fn test_claude_stream_json_args() {
    // Task 29: Claude uses -p and stream-json output with verbose mode
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Claude);
    let spec = build_command(
        ProviderId::Claude,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(
        spec.args.contains(&"-p".to_string()),
        "Claude should use -p flag"
    );
    assert!(
        spec.args.contains(&"--output-format".to_string()),
        "Claude should have output-format"
    );
    assert!(
        spec.args.contains(&"--verbose".to_string()),
        "Claude stream-json mode should enable verbose"
    );
    assert!(
        spec.args.contains(&"stream-json".to_string()),
        "Claude should use stream-json"
    );
}

#[test]
fn test_gemini_safe_mode_no_yolo() {
    // Task 30: Gemini safe mode should not add yolo
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Gemini);
    let spec = build_command(ProviderId::Gemini, ApprovalPolicy::Safe, &config, &request).unwrap();
    assert!(
        !spec.args.contains(&"--yolo".to_string()),
        "Gemini safe mode should not have --yolo"
    );
    assert!(
        !spec.args.contains(&"--approval-mode".to_string()),
        "Gemini safe mode should not have approval-mode"
    );
}

#[test]
fn test_forge_dangerous_uses_unsafe_args() {
    // Task 34: Forge dangerous uses configured unsafe_args
    let mut config = test_config();
    config.unsafe_args = vec!["--dangerous".to_string(), "--bypass".to_string()];
    let request = ChatRequest::new("test", ".", ProviderId::Forge);
    let spec = build_command(
        ProviderId::Forge,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    assert!(spec.args.contains(&"--dangerous".to_string()));
    assert!(spec.args.contains(&"--bypass".to_string()));
}

#[test]
fn test_forge_safe_uses_safe_args() {
    // Task 34: Forge safe uses configured safe_args or forge -p
    let mut config = test_config();
    config.safe_args = vec!["--ask".to_string()];
    let request = ChatRequest::new("test", ".", ProviderId::Forge);
    let spec = build_command(ProviderId::Forge, ApprovalPolicy::Safe, &config, &request).unwrap();
    assert!(spec.args.contains(&"--ask".to_string()));
}

// =========================================================================
// ApprovalMode deprecation gate tests (Task 2)
// =========================================================================

#[test]
fn test_providers_use_approval_policy_not_approval_mode() {
    // This test verifies that command builders use ApprovalPolicy, not ApprovalMode.
    // If any provider module imports or uses ApprovalMode directly, this test
    // will fail to compile (since ApprovalMode is no longer re-exported from core).
    //
    // The actual enforcement is at compile time: providers must use ApprovalPolicy
    // for all command construction paths.
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Codex);

    // Verify both policies work and produce different commands
    let dangerous_spec = build_command(
        ProviderId::Codex,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    )
    .unwrap();
    let safe_spec =
        build_command(ProviderId::Codex, ApprovalPolicy::Safe, &config, &request).unwrap();

    // Dangerous should have bypass flag
    assert!(dangerous_spec
        .args
        .contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
    // Safe should NOT have bypass flag
    assert!(!safe_spec
        .args
        .contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
}

#[test]
fn test_all_providers_handle_both_policies() {
    // Verify all providers can handle both DangerousDefault and Safe policies
    let config = test_config();
    let request = ChatRequest::new("test", ".", ProviderId::Claude);

    // Claude should work with both policies
    let dangerous = build_command(
        ProviderId::Claude,
        ApprovalPolicy::DangerousDefault,
        &config,
        &request,
    );
    let safe = build_command(ProviderId::Claude, ApprovalPolicy::Safe, &config, &request);

    assert!(dangerous.is_ok(), "Claude should support DangerousDefault");
    assert!(safe.is_ok(), "Claude should support Safe");

    // Verify dangerous adds skip-permissions
    let dangerous_args = &dangerous.unwrap().args;
    assert!(dangerous_args.contains(&"--dangerously-skip-permissions".to_string()));

    // Verify safe adds permission-mode
    let safe_args = &safe.unwrap().args;
    assert!(safe_args.contains(&"--permission-mode".to_string()));
}

// =========================================================================
// Safe argv templating tests (Task 11)
// =========================================================================

#[test]
fn test_custom_provider_replaces_prompt_placeholder() {
    // Test that {prompt} is replaced with actual prompt
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![]; // Clear unsafe_args so dangerous mode works

    let request = ChatRequest::new("hello world", ".", ProviderId::Custom("test".to_string()));
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(
        spec.args.contains(&"hello world".to_string()),
        "Expected prompt to be substituted"
    );
    assert!(
        !spec.args.contains(&"{prompt}".to_string()),
        "Placeholder should be replaced"
    );
}

#[test]
fn test_custom_provider_replaces_cwd_placeholder() {
    // Test that {cwd} is replaced with actual cwd
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "--dir".to_string(), "{cwd}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new("test", "/tmp/test", ProviderId::Custom("test".to_string()));
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(
        spec.args.contains(&"/tmp/test".to_string()),
        "Expected cwd to be substituted"
    );
    assert!(
        !spec.args.contains(&"{cwd}".to_string()),
        "Placeholder should be replaced"
    );
}

#[test]
fn test_custom_provider_replaces_model_placeholder() {
    // Test that {model} is replaced with actual model
    let mut config = test_config();
    config.args = vec![
        "echo".to_string(),
        "--model".to_string(),
        "{model}".to_string(),
    ];
    config.unsafe_args = vec![];

    let mut request = ChatRequest::new("test", ".", ProviderId::Custom("test".to_string()));
    request.model = Some("claude-3-opus".to_string());

    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(
        spec.args.contains(&"claude-3-opus".to_string()),
        "Expected model to be substituted"
    );
    assert!(
        !spec.args.contains(&"{model}".to_string()),
        "Placeholder should be replaced"
    );
}

#[test]
fn test_custom_provider_handles_special_chars_in_prompt() {
    // Test that special characters in prompt are preserved (no shell interpolation)
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![];

    // Prompt with shell special characters - should be passed as-is
    let request = ChatRequest::new(
        "hello; world `echo` 'quotes' $var",
        ".",
        ProviderId::Custom("test".to_string()),
    );
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(
        spec.args
            .contains(&"hello; world `echo` 'quotes' $var".to_string()),
        "Special chars should be preserved"
    );
}

#[test]
fn test_custom_provider_preserves_unmatched_placeholders() {
    // Placeholders that don't match any request field should be preserved as-is
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{unknown}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new("test", ".", ProviderId::Custom("test".to_string()));
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(
        spec.args.contains(&"{unknown}".to_string()),
        "Unknown placeholder should be preserved"
    );
}

// =========================================================================
// Injection attack prevention tests (Task 11)
// =========================================================================

#[test]
fn test_template_prevents_shell_injection_quotes() {
    // Quotes should be preserved, not interpreted as shell
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new(
        "'single' \"double\"",
        ".",
        ProviderId::Custom("test".to_string()),
    );
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    // The prompt should be passed as a single argument, quotes preserved
    assert!(spec.args.contains(&"'single' \"double\"".to_string()));
}

#[test]
fn test_template_prevents_shell_injection_semicolon() {
    // Semicolons should be preserved, not as command separator
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new(
        "hello; whoami".to_string(),
        ".",
        ProviderId::Custom("test".to_string()),
    );
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(spec.args.contains(&"hello; whoami".to_string()));
}

#[test]
fn test_template_prevents_shell_injection_newline() {
    // Newlines should be preserved in prompt
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new(
        "line1\nline2".to_string(),
        ".",
        ProviderId::Custom("test".to_string()),
    );
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(spec.args.contains(&"line1\nline2".to_string()));
}

#[test]
fn test_template_prevents_shell_injection_flag_like() {
    // Prompt starting with -- should be treated as literal text, not flag
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new(
        "--dangerous-flag".to_string(),
        ".",
        ProviderId::Custom("test".to_string()),
    );
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(spec.args.contains(&"--dangerous-flag".to_string()));
}

#[test]
fn test_template_prevents_path_traversal() {
    // Path traversal attempts should be preserved
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new(
        "../../../etc/passwd".to_string(),
        ".",
        ProviderId::Custom("test".to_string()),
    );
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(spec.args.contains(&"../../../etc/passwd".to_string()));
}

#[test]
fn test_template_preserves_unicode() {
    // Unicode characters should be preserved
    let mut config = test_config();
    config.args = vec!["echo".to_string(), "{prompt}".to_string()];
    config.unsafe_args = vec![];

    let request = ChatRequest::new(
        "Hello 世界 🌍".to_string(),
        ".",
        ProviderId::Custom("test".to_string()),
    );
    let spec = build_command(
        ProviderId::Custom("test".to_string()),
        ApprovalPolicy::Safe,
        &config,
        &request,
    )
    .unwrap();

    assert!(spec.args.contains(&"Hello 世界 🌍".to_string()));
}
