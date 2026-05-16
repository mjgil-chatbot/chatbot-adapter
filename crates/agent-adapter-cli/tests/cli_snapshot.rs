//! CLI integration tests using fake providers.

use std::path::PathBuf;
use std::process::Command;

/// Helper to find the CLI binary.
fn cli_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_agent-adapter-cli"))
}

/// Test that `providers --json` produces valid JSON.
#[test]
fn test_providers_json_output() {
    let output = Command::new(cli_binary())
        .args(["providers", "--json"])
        .output()
        .expect("Failed to execute providers --json");

    assert!(output.status.success(), "providers --json should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should be valid JSON with expected structure
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("providers --json should output valid JSON");

    // Check top-level structure
    assert!(
        parsed.get("providers").is_some(),
        "Should have providers array"
    );
    assert!(parsed.get("total").is_some(), "Should have total count");
    assert!(
        parsed.get("builtin_count").is_some(),
        "Should have builtin_count"
    );

    let providers = parsed.get("providers").unwrap().as_array().unwrap();
    assert!(!providers.is_empty(), "Should have at least one provider");

    // Each provider should have required fields
    for provider in providers {
        assert!(provider.get("name").is_some(), "Provider should have name");
        assert!(
            provider.get("display_name").is_some(),
            "Provider should have display_name"
        );
        assert!(
            provider.get("supported_transports").is_some(),
            "Provider should have supported_transports"
        );
        assert!(
            provider.get("health_status").is_some(),
            "Provider should have health_status"
        );
    }

    assert!(stderr.is_empty(), "stderr should be empty");
}

/// Test that `providers` text output contains expected content.
#[test]
fn test_providers_text_output() {
    let output = Command::new(cli_binary())
        .args(["providers"])
        .output()
        .expect("Failed to execute providers");

    assert!(output.status.success(), "providers should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Text output should contain provider listing
    assert!(
        stdout.contains("Available providers:"),
        "Should have header"
    );
    assert!(stdout.contains("Docs:"), "Should show docs paths");
    assert!(stdout.contains("Status:"), "Should show health status");
    assert!(stderr.is_empty(), "stderr should be empty");
}

/// Test that `healthcheck --provider <name> --json` works.
#[test]
fn test_healthcheck_json_output() {
    let output = Command::new(cli_binary())
        .args(["healthcheck", "--provider", "claude", "--json"])
        .output()
        .expect("Failed to execute healthcheck --json");

    // Might fail if claude is not installed, so we just check structure if success
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("healthcheck --json should output valid JSON");

        assert!(
            parsed.get("provider").is_some(),
            "Should have provider field"
        );
        assert!(parsed.get("status").is_some(), "Should have status field");
        assert!(
            parsed.get("transports").is_some(),
            "Should have transports array"
        );
        assert!(
            parsed.get("safe_mode_supported").is_some(),
            "Should have safe_mode_supported"
        );
        assert!(
            parsed.get("unsafe_mode_supported").is_some(),
            "Should have unsafe_mode_supported"
        );
    }

    // stderr should be clean even if provider not found
    assert!(
        !stderr.contains("Error:"),
        "Should not have error in stderr"
    );
}

/// Test that `doctor --json` produces valid JSON.
#[test]
fn test_doctor_json_output() {
    let output = Command::new(cli_binary())
        .args(["doctor", "--json"])
        .output()
        .expect("Failed to execute doctor --json");

    assert!(output.status.success(), "doctor --json should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json should output valid JSON");

    assert!(
        parsed.get("providers").is_some(),
        "Should have providers array"
    );
    assert!(
        parsed.get("env_allowlist_status").is_some(),
        "Should have env_allowlist_status"
    );

    assert!(stderr.is_empty(), "stderr should be empty");
}

/// Test that `run --help` shows the new flags.
#[test]
fn test_run_help_shows_json_and_resume_flags() {
    let output = Command::new(cli_binary())
        .args(["run", "--help"])
        .output()
        .expect("Failed to execute run --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("--json"), "Help should show --json flag");
    assert!(
        stdout.contains("--resume"),
        "Help should show --resume flag"
    );
    assert!(
        stdout.contains("--session-id"),
        "Help should show --session-id flag"
    );
}

/// Test that providers shows docs_path for each provider.
#[test]
fn test_providers_shows_docs_paths() {
    let output = Command::new(cli_binary())
        .args(["providers"])
        .output()
        .expect("Failed to execute providers");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Text output should include docs paths
    assert!(stdout.contains("Docs:"), "Should show docs paths");
    assert!(
        stdout.contains("docs/providers/"),
        "Docs path should reference providers dir"
    );
}

/// Test that providers shows transport information.
#[test]
fn test_providers_shows_transports() {
    let output = Command::new(cli_binary())
        .args(["providers"])
        .output()
        .expect("Failed to execute providers");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show transports in the output
    assert!(
        stdout.contains("transports:"),
        "Should show transports label"
    );
    assert!(
        stdout.contains("jsonl") || stdout.contains("acp") || stdout.contains("oneshot"),
        "Should show at least one transport type"
    );
}

/// Test that providers shows policy support information.
#[test]
fn test_providers_shows_policy_support() {
    let output = Command::new(cli_binary())
        .args(["providers"])
        .output()
        .expect("Failed to execute providers");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show safe/unsafe mode support
    assert!(stdout.contains("Safe:"), "Should show safe mode support");
    assert!(
        stdout.contains("Unsafe:"),
        "Should show unsafe mode support"
    );
    assert!(
        stdout.contains("Requires unsafe args:"),
        "Should show requires_explicit_unsafe_args"
    );
}

/// Test that healthcheck shows transports in text mode.
#[test]
fn test_healthcheck_shows_transports() {
    let output = Command::new(cli_binary())
        .args(["healthcheck", "--provider", "claude"])
        .output()
        .expect("Failed to execute healthcheck");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Transports:"),
            "Text output should show transports"
        );
    }
}
