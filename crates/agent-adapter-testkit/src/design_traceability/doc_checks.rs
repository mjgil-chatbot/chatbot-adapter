use super::*;

pub(super) fn check_stale_root_task_lists(
    _content: &str,
    root_dir: &Path,
) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    let docs_dir = root_dir.join("docs");
    let stale_task_lists = vec![
        "tasks-8.md",
        "tasks-9.md",
        "tasks-8-pass-1.md",
        "tasks-8-pass-2.md",
        "tasks-8-pass-3.md",
        "tasks-8-pass-4.md",
        "tasks-8-pass-5.md",
        "tasks-8-pass-6.md",
        "tasks-8-pass-7.md",
        "tasks-8-pass-8.md",
        "tasks-8-pass-9.md",
        "tasks-8-status.md",
        "tasks-8-pass-1-status.md",
        "tasks-8-pass-2-status.md",
        "tasks-8-pass-3-status.md",
        "tasks-8-pass-4-status.md",
        "tasks-8-pass-5-status.md",
        "tasks-8-pass-6-status.md",
        "tasks-8-pass-7-status.md",
        "tasks-8-pass-8-status.md",
        "tasks-8-pass-9-status.md",
    ];

    for stale in stale_task_lists {
        let path = docs_dir.join(stale);
        if path.exists() {
            errors.push(TraceabilityError {
                claim: format!("Stale root doc {} should be archived", stale),
                error: format!(
                    "{} exists in docs/ root but must be in docs/archive/ (acceptance requires superseded task lists to be archived)",
                    stale
                ),
            });
        }
    }

    errors
}

/// Detect stale canonical references to tasks-9.md in tasks-10.md.
/// The tasks-10.md should not claim tasks-9.md is still the active list.
/// tasks-10.md is the new canonical list; tasks-9.md is properly archived.
pub(super) fn check_tasks10_stale_canon_reference(
    _content: &str,
    root_dir: &Path,
) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    // Only check tasks-10.md itself
    let tasks10_path = root_dir.join("docs/tasks-10.md");
    if !tasks10_path.exists() {
        return errors;
    }

    // Check if tasks-10.md explicitly claims tasks-9.md is the active task list
    // (rather than acknowledging it as the previous list)
    let tasks10_content = match std::fs::read_to_string(&tasks10_path) {
        Ok(c) => c,
        Err(_) => return errors,
    };

    // Look for explicit claims that tasks-9 is still active
    // The pattern "tasks-9.md is the active task list" would be a direct contradiction
    let stale_claims = [
        "tasks-9.md is the active task list",
        "tasks-9.md remains the active task list",
    ];

    for claim in stale_claims {
        if tasks10_content.contains(claim) {
            errors.push(TraceabilityError {
                claim: "tasks-10.md stale canon reference to tasks-9.md".to_string(),
                error: format!(
                    "tasks-10.md claims '{}' but tasks-10.md is now the canonical task list",
                    claim
                ),
            });
        }
    }

    // Check if tasks-10.md still requires DANGEROUS_TEST_MODE (implementation uses RUN_REAL_* only)
    if tasks10_content.contains("DANGEROUS_TEST_MODE") {
        let gates_content = std::fs::read_to_string(
            root_dir.join("crates/agent-adapter-testkit/src/real_provider_gates.rs"),
        )
        .unwrap_or_default();
        if !gates_content.contains("DANGEROUS_TEST_MODE") {
            errors.push(TraceabilityError {
                claim: "tasks-10.md requires unsupported DANGEROUS_TEST_MODE gate".to_string(),
                error: "tasks-10.md requires DANGEROUS_TEST_MODE but implementation only gates on RUN_REAL_* vars".to_string(),
            });
        }
    }

    errors
}

/// Detect stale evidence row paths in design.md.
/// Specifically catches the error where temp_workspace.rs is cited as living
/// in agent-adapter-transports instead of agent-adapter-testkit.
pub(super) fn check_design_evidence_row_paths(
    content: &str,
    root_dir: &Path,
) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    // Check for stale agent-adapter-transports/src/temp_workspace.rs citation
    // The live implementation is in agent-adapter-testkit/src/temp_workspace.rs
    if content.contains("agent-adapter-transports/src/temp_workspace.rs") {
        let testkit_path = root_dir.join("crates/agent-adapter-testkit/src/temp_workspace.rs");
        let transports_path =
            root_dir.join("crates/agent-adapter-transports/src/temp_workspace.rs");
        if testkit_path.exists() && !transports_path.exists() {
            errors.push(TraceabilityError {
                claim: "Stale workspace helper evidence path in design.md".to_string(),
                error: "design.md cites temp_workspace.rs in agent-adapter-transports but implementation is in agent-adapter-testkit".to_string(),
            });
        }
    }

    errors
}

/// Check that doc files cited in design.md have substantial content (not stubs).
/// A stale doc is a file that exists but has less than 100 chars of content
/// or contains only placeholder text like "TODO", "FIXME", or is empty.
pub(super) fn check_doc_freshness(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    // Extract doc paths that design.md mentions
    let doc_paths = vec![
        ("docs/core.md", content.contains("docs/core.md")),
        ("docs/events.md", content.contains("docs/events.md")),
        ("docs/errors.md", content.contains("docs/errors.md")),
        ("docs/config.md", content.contains("docs/config.md")),
        ("docs/cli.md", content.contains("docs/cli.md")),
        ("docs/policy.md", content.contains("docs/policy.md")),
        ("docs/design.md", content.contains("docs/design.md")),
        (
            "docs/transports/jsonl.md",
            content.contains("docs/transports/jsonl.md"),
        ),
        (
            "docs/transports/oneshot.md",
            content.contains("docs/transports/oneshot.md"),
        ),
        (
            "docs/transports/acp.md",
            content.contains("docs/transports/acp.md"),
        ),
        (
            "docs/transports/pty.md",
            content.contains("docs/transports/pty.md"),
        ),
        (
            "docs/providers/index.md",
            content.contains("docs/providers/index.md"),
        ),
        (
            "docs/providers/codex.md",
            content.contains("docs/providers/codex.md"),
        ),
        (
            "docs/providers/claude.md",
            content.contains("docs/providers/claude.md"),
        ),
        (
            "docs/providers/gemini.md",
            content.contains("docs/providers/gemini.md"),
        ),
        (
            "docs/providers/kimi.md",
            content.contains("docs/providers/kimi.md"),
        ),
        (
            "docs/providers/opencode.md",
            content.contains("docs/providers/opencode.md"),
        ),
        (
            "docs/providers/hermes.md",
            content.contains("docs/providers/hermes.md"),
        ),
        (
            "docs/providers/forge.md",
            content.contains("docs/providers/forge.md"),
        ),
        (
            "docs/providers/claw.md",
            content.contains("docs/providers/claw.md"),
        ),
        ("docs/workspaces.md", content.contains("docs/workspaces.md")),
        (
            "docs/security-env.md",
            content.contains("docs/security-env.md"),
        ),
        ("docs/audit.md", content.contains("docs/audit.md")),
        ("docs/testing.md", content.contains("docs/testing.md")),
        ("docs/testkit.md", content.contains("docs/testkit.md")),
        (
            "docs/healthchecks.md",
            content.contains("docs/healthchecks.md"),
        ),
        (
            "docs/cancellation.md",
            content.contains("docs/cancellation.md"),
        ),
    ];

    for (doc_path, mentioned) in doc_paths {
        if mentioned {
            let full_path = root_dir.join(doc_path);
            if full_path.exists() {
                if let Ok(doc_content) = std::fs::read_to_string(&full_path) {
                    let trimmed = doc_content.trim();
                    // Check for stale/placeholder content
                    let is_stale = trimmed.len() < 100
                        || trimmed.starts_with("# TODO")
                        || trimmed.starts_with("<!-- TODO")
                        || trimmed == "# Placeholder";
                    if is_stale {
                        errors.push(TraceabilityError {
                            claim: format!("Doc file {} is stale/placeholder", doc_path),
                            error: format!("{} exists but has insufficient content ({} chars) - appears to be a placeholder or stub", doc_path, trimmed.len()),
                        });
                    }
                }
            }
        }
    }

    errors
}

/// Check that cited test names and test files actually exist.
/// This addresses "test exists but unrelated" - when design.md claims
/// a test exists but it doesn't actually test the claimed behavior.
pub(super) fn check_cited_tests_exist(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    // Look for test module paths cited in design.md evidence
    // e.g., "temp_workspace::tests::" or "types::tests::"
    let test_patterns = vec![
        (
            "temp_workspace::tests",
            "crates/agent-adapter-testkit/src/temp_workspace.rs",
        ),
        ("types::tests", "crates/agent-adapter-core/src/types.rs"),
        ("request::tests", "crates/agent-adapter-core/src/request.rs"),
        (
            "registry::tests",
            "crates/agent-adapter-config/src/registry.rs",
        ),
        (
            "mappers::tests",
            "crates/agent-adapter-providers/src/mappers.rs",
        ),
        (
            "jsonl_command::tests",
            "crates/agent-adapter-transports/src/jsonl_command.rs",
        ),
        (
            "oneshot_command::tests",
            "crates/agent-adapter-transports/src/oneshot_command.rs",
        ),
        (
            "pty_fallback::tests",
            "crates/agent-adapter-transports/src/pty_fallback.rs",
        ),
    ];

    for (test_module, source_file) in test_patterns {
        if content.contains(test_module) {
            let full_path = root_dir.join(source_file);
            if full_path.exists() {
                if let Ok(source_content) = std::fs::read_to_string(&full_path) {
                    // Check that the file actually contains #[test] modules
                    if !source_content.contains("#[test]")
                        && !source_content.contains("#[cfg(test)]")
                    {
                        errors.push(TraceabilityError {
                            claim: format!("Test module {} cited in design", test_module),
                            error: format!(
                                "{} is cited but {} contains no #[test] or #[cfg(test)] modules",
                                test_module, source_file
                            ),
                        });
                    }
                }
            }
        }
    }

    errors
}
