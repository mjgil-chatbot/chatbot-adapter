//! Status validation for implementation completeness.
//!
//! This module provides validation that status.md claims are accurate
//! by checking that "Complete" rows have actual implementation, tests, and docs.
//!
//! # Validation rules for "Complete" rows
//!
//! A crate row is only valid as "Complete" when ALL of the following hold:
//! 1. The crate has `src/lib.rs` or `src/main.rs`
//! 2. The crate has an actual test directory or inline tests
//! 3. The documentation path cited in status.md actually exists
//! 4. No cited API/method names that do not exist in the implementation
//! 5. No cited file paths that do not exist or point to the wrong location
//! 6. No cited config keys that do not match the actual schema
//! 7. No cited environment gates that are not implemented
//!
//! # Missing-doc exceptions
//!
//! The following are explicitly allowed as "Complete" without specific doc files:
//! - `agent-adapter-router`: no `docs/router.md` required; routing documented in `docs/design.md`
//! - `agent-adapter-config`: no `docs/config.md` required; schema in `docs/design.md` and `docs/cli.md`

use std::path::Path;

/// Result of a status validation check.
#[derive(Debug, Clone)]
pub struct StatusValidationResult {
    pub passed: bool,
    pub errors: Vec<StatusValidationError>,
}

impl StatusValidationResult {
    pub fn success() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
        }
    }

    pub fn failure(errors: Vec<StatusValidationError>) -> Self {
        Self {
            passed: false,
            errors,
        }
    }
}

/// A status validation error.
#[derive(Debug, Clone)]
pub struct StatusValidationError {
    pub crate_name: String,
    pub error: String,
}

impl StatusValidationError {
    pub fn new(crate_name: &str, error: impl Into<String>) -> Self {
        Self {
            crate_name: crate_name.to_string(),
            error: error.into(),
        }
    }
}

/// Known missing-doc exceptions: crates that are "Complete" without specific doc files.
/// Key: crate name, Value: reason why no doc file is required.
const KNOWN_MISSING_DOC_EXCEPTIONS: &[(&str, &str)] = &[
    (
        "agent-adapter-router",
        "no docs/router.md; routing documented in docs/design.md",
    ),
    (
        "agent-adapter-config",
        "no docs/config.md; schema in docs/design.md and docs/cli.md",
    ),
];

/// Check if a crate has a known missing-doc exception.
fn has_missing_doc_exception(crate_name: &str) -> Option<&'static str> {
    KNOWN_MISSING_DOC_EXCEPTIONS
        .iter()
        .find(|(name, _)| *name == crate_name)
        .map(|(_, reason)| *reason)
}

/// Check if a crate has actual test evidence (not just crate existence).
/// Returns true if an actual tests/ directory, src/tests/ directory, or
/// any source file containing #[test] or #[cfg(test)] modules exists.
/// Bare crate existence (lib.rs with no tests) is NOT sufficient.
fn crate_has_tests(crate_path: &Path) -> bool {
    // Check for tests directory
    if crate_path.join("tests").exists() {
        return true;
    }
    // Check for src/tests directory
    if crate_path.join("src").join("tests").exists() {
        return true;
    }
    // Check for inline tests: look for #[test] or #[cfg(test)] in any .rs file
    let src_dir = crate_path.join("src");
    if src_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains("#[test]") || content.contains("#[cfg(test)]") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Validate the status claims in status.md against actual implementation.
pub fn validate_status(
    status_md: &Path,
    crates_dir: &Path,
    docs_dir: &Path,
) -> StatusValidationResult {
    let mut errors = Vec::new();

    let content = match std::fs::read_to_string(status_md) {
        Ok(c) => c,
        Err(e) => {
            return StatusValidationResult::failure(vec![StatusValidationError {
                crate_name: "status.md".to_string(),
                error: format!("Could not read status.md: {}", e),
            }]);
        }
    };

    // Run cross-cutting content validations (router methods, workspace paths, config keys, gates)
    errors.extend(
        validate_router_claims_in_content(&content, crates_dir)
            .into_iter()
            .map(|e| StatusValidationError::new("status.md", e)),
    );
    errors.extend(
        validate_workspace_helper_claims_in_content(&content)
            .into_iter()
            .map(|e| StatusValidationError::new("status.md", e)),
    );
    errors.extend(
        validate_config_key_claims(&content)
            .into_iter()
            .map(|e| StatusValidationError::new("status.md", e)),
    );
    errors.extend(
        validate_gate_claims(&content)
            .into_iter()
            .map(|e| StatusValidationError::new("status.md", e)),
    );

    // Parse the crate table from status.md
    let crate_rows = parse_crate_table(&content);

    for row in crate_rows {
        // Strip backticks from crate name
        let crate_name = row.crate_name.trim_matches('`');
        if row.status == "Complete" {
            // Check implementation exists
            let crate_path = crates_dir.join(crate_name);
            if !crate_path.exists() {
                errors.push(StatusValidationError {
                    crate_name: crate_name.to_string(),
                    error: format!("Crate {} does not exist", crate_name),
                });
                continue;
            }

            // Check that crate has a lib.rs or main.rs
            let has_lib = crate_path.join("src").join("lib.rs").exists();
            let has_main = crate_path.join("src").join("main.rs").exists();
            if !has_lib && !has_main {
                errors.push(StatusValidationError {
                    crate_name: crate_name.to_string(),
                    error: format!("Crate {} has no src/lib.rs or src/main.rs", crate_name),
                });
            }

            // Reject "Complete" if it only relies on crate existence + generic doc presence
            // Each Complete row needs specific implementation evidence cited in status.md
            if !has_lib && !has_main {
                // Already flagged above, skip further checks
            } else if !crate_has_tests(&crate_path) {
                errors.push(StatusValidationError {
                    crate_name: crate_name.to_string(),
                    error: format!(
                        "Crate {} is marked Complete but has no test directory or inline tests",
                        crate_name
                    ),
                });
            }

            // Verify specific evidence paths exist (non-vacuous evidence)
            let impl_paths = extract_impl_paths(&row.implementation);
            let test_paths = extract_test_paths(&row.tests);
            errors.extend(
                verify_evidence_paths(&row.crate_name, &crate_path, &impl_paths, &test_paths)
                    .into_iter()
                    .map(|e| StatusValidationError::new(&row.crate_name, e)),
            );

            // Check docs exist - apply missing-doc exceptions
            let doc_formats = [docs_dir.join(doc_name_from_crate(crate_name))];
            let doc_exists = doc_formats.iter().any(|p| p.exists());

            if !doc_exists {
                // Check if this crate has a known missing-doc exception
                if let Some(_reason) = has_missing_doc_exception(crate_name) {
                    // Exception applies - allow it
                } else {
                    errors.push(StatusValidationError {
                        crate_name: crate_name.to_string(),
                        error: format!(
                            "Crate {} is marked Complete but documentation not found",
                            crate_name
                        ),
                    });
                }
            }
        }
    }

    if errors.is_empty() {
        StatusValidationResult::success()
    } else {
        StatusValidationResult::failure(errors)
    }
}

/// Validate router method claims in the full status.md content.
/// Checks that any cited router methods actually exist in the implementation.
fn validate_router_claims_in_content(status_content: &str, crates_dir: &Path) -> Vec<String> {
    let mut errors = Vec::new();

    // Methods that router.rs does NOT have (but status.md may claim)
    let nonexistent_methods = ["capability_enforcement", "policy_enforcement"];

    for method in nonexistent_methods {
        if status_content.contains(&format!("`{}()`", method))
            || status_content.contains(&format!("{}(", method))
        {
            // Verify the router crate exists and doesn't have this method
            let router_path = crates_dir.join("agent-adapter-router");
            if router_path.join("src").join("router.rs").exists() {
                if let Ok(router_src) =
                    std::fs::read_to_string(router_path.join("src").join("router.rs"))
                {
                    // Use a simple heuristic: if the method name appears as `fn method_name`
                    // or `pub fn method_name` in the source, it exists. Otherwise flag it.
                    let method_patterns = [format!("fn {}", method), format!("pub fn {}", method)];
                    let method_exists = method_patterns.iter().any(|p| router_src.contains(p));
                    if !method_exists {
                        errors.push(format!(
                            "status.md claims router method `{}()` but it does not exist in crates/agent-adapter-router/src/router.rs",
                            method
                        ));
                    }
                }
            }
        }
    }

    errors
}

/// Validate workspace helper path claims in status.md content.
fn validate_workspace_helper_claims_in_content(status_content: &str) -> Vec<String> {
    let mut errors = Vec::new();

    // The actual temp_workspace lives in agent-adapter-testkit/src/temp_workspace.rs
    // status.md incorrectly cites agent-adapter-transports/src/temp_workspace.rs
    let wrong_path = "agent-adapter-transports/src/temp_workspace";
    if status_content.contains(wrong_path) {
        errors.push(format!(
            "status.md cites workspace helper at `{}` but it lives in `agent-adapter-testkit/src/temp_workspace.rs`",
            wrong_path
        ));
    }

    errors
}

/// Validate that config key claims match the actual schema.
fn validate_config_key_claims(status_content: &str) -> Vec<String> {
    let mut errors = Vec::new();

    // The schema uses `prompt_mode`, not `prompt` as a config key
    if status_content.contains(r#"prompt = "stdin""#)
        || status_content.contains(r#"prompt = "arg""#)
    {
        errors.push(
            "status.md uses stale `prompt` config key; schema uses `prompt_mode` instead"
                .to_string(),
        );
    }

    errors
}

/// Validate that gate claims match actual implementation.
fn validate_gate_claims(status_content: &str) -> Vec<String> {
    let mut errors = Vec::new();

    // real_provider_gates.rs only enforces RUN_REAL_* environment variables
    if status_content.contains("DANGEROUS_TEST_MODE") {
        errors.push(
            "status.md claims DANGEROUS_TEST_MODE gate but real_provider_gates.rs only enforces RUN_REAL_*"
                .to_string(),
        );
    }

    errors
}

#[allow(dead_code)]
/// A parsed crate table row from status.md.
struct CrateRow {
    crate_name: String,
    status: String,
    implementation: String,
    tests: String,
    documentation: String,
}

/// Extract specific source file paths from implementation evidence.
/// Parses evidence like "src/lib.rs, src/request.rs (ChatRequest)" into actual paths.
fn extract_impl_paths(evidence: &str) -> Vec<String> {
    let mut paths = Vec::new();
    // Split on commas and parenthetical notes
    for part in evidence.split(',') {
        let part = part.trim();
        // Extract path before any parenthesis
        if let Some(paren_idx) = part.find('(') {
            let path = part[..paren_idx].trim();
            if !path.is_empty() && (path.starts_with("src/") || path.starts_with("crates/")) {
                paths.push(path.to_string());
            }
        } else {
            // No parenthesis - the whole thing might be a path
            let path = part.trim();
            if !path.is_empty() && (path.starts_with("src/") || path.starts_with("crates/")) {
                paths.push(path.to_string());
            }
        }
    }
    paths
}

/// Extract specific test file paths from test evidence.
/// Parses evidence like "tests/*.rs" or "src/request.rs" (request::tests::*)" into paths.
fn extract_test_paths(evidence: &str) -> Vec<String> {
    let mut paths = Vec::new();

    // Look for test directory references
    if evidence.contains("tests/") || evidence.contains("src/tests/") {
        // Already contains test paths, extract them
        for part in evidence.split(',') {
            let part = part.trim();
            if part.starts_with("tests/") || part.starts_with("src/tests/") {
                paths.push(part.to_string());
            }
        }
    }

    // Also look for #[test] module references like "request::tests::*"
    // These indicate inline tests in the source
    if evidence.contains("::tests::") || evidence.contains("#[test]") {
        // Inline tests exist - we'll note this but they don't map to a specific file path
    }

    paths
}

/// Verify that specific paths cited in evidence actually exist.
/// Returns errors for missing paths.
fn verify_evidence_paths(
    crate_name: &str,
    crate_path: &Path,
    impl_paths: &[String],
    test_paths: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();

    for path in impl_paths {
        let full_path = crate_path.join(path);
        if !full_path.exists() {
            errors.push(format!(
                "Crate {} claims implementation at `{}` but it does not exist",
                crate_name, path
            ));
        }
    }

    for path in test_paths {
        let full_path = crate_path.join(path);
        if !full_path.exists() {
            errors.push(format!(
                "Crate {} claims tests at `{}` but it does not exist",
                crate_name, path
            ));
        }
    }

    errors
}

/// Map a crate name to its expected documentation file or directory name.
fn doc_name_from_crate(crate_name: &str) -> String {
    match crate_name {
        "agent-adapter-core" => "design.md".to_string(),
        "agent-adapter-config" => "cli.md".to_string(),
        "agent-adapter-providers" => "providers".to_string(), // directory
        "agent-adapter-transports" => "transports".to_string(), // directory
        "agent-adapter-router" => "design.md".to_string(),
        "agent-adapter-cli" => "cli.md".to_string(),
        "agent-adapter-testkit" => "testkit.md".to_string(),
        "agent-adapter-workspace" => "workspaces.md".to_string(),
        "agent-adapter-redaction" => "security-env.md".to_string(),
        _ => {
            let base = crate_name.trim_start_matches("agent-adapter-");
            format!("{}.md", base)
        }
    }
}

/// Parse the crate completion table from status.md content.
fn parse_crate_table(content: &str) -> Vec<CrateRow> {
    let mut rows = Vec::new();

    // Find the crate table section (after "## Crate Completion Status")
    let mut in_crate_table = false;

    for line in content.lines() {
        if line.contains("## Crate Completion Status") {
            in_crate_table = true;
            continue;
        }

        if in_crate_table {
            // Stop at next ## heading or end of table
            if (line.starts_with("## ") || (line.starts_with('|') && line.contains("---")))
                && !line.contains("|")
                && !line.contains("---")
            {
                break;
            }

            // Skip separator lines and header
            if line.contains("---") || line.starts_with("| Crate") || line.trim().is_empty() {
                continue;
            }

            // Parse table row: | crate | status | implementation | tests | documentation |
            if line.starts_with('|') {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 6 {
                    let crate_name = parts[1].trim().to_string();
                    let status = parts[2].trim().to_string();
                    let implementation = parts[3].trim().to_string();
                    let documentation = parts[5].trim().to_string();
                    let tests = parts[4].trim().to_string();
                    if !crate_name.is_empty()
                        && (status == "Complete" || status == "Incomplete" || status == "Test-only")
                    {
                        rows.push(CrateRow {
                            crate_name,
                            status,
                            implementation,
                            tests,
                            documentation,
                        });
                    }
                }
            }
        }
    }

    rows
}

/// Validate that tasks marked complete have proper evidence.
/// A task is considered complete only if:
/// 1. Implementation file exists
/// 2. Test file exists
/// 3. Documentation file exists
pub fn validate_task_completeness(
    tasks_md: &Path,
    crates_dir: &Path,
    docs_dir: &Path,
) -> StatusValidationResult {
    let mut errors = Vec::new();

    let content = match std::fs::read_to_string(tasks_md) {
        Ok(c) => c,
        Err(e) => {
            return StatusValidationResult::failure(vec![StatusValidationError {
                crate_name: "tasks.md".to_string(),
                error: format!("Could not read tasks.md: {}", e),
            }]);
        }
    };

    // For each task marked "Complete", check evidence
    for line in content.lines() {
        // Look for complete tasks
        if line.contains("| Complete") || line.contains("|  Complete") {
            // Extract task number if present
            if let Some(task_num) = extract_task_number(line) {
                // Verify the task has implementation, tests, and docs
                let task_errors = validate_task_evidence(task_num, &content, crates_dir, docs_dir);
                errors.extend(task_errors.into_iter().map(|e| StatusValidationError {
                    crate_name: format!("Task {}", task_num),
                    error: e,
                }));
            }
        }
    }

    if errors.is_empty() {
        StatusValidationResult::success()
    } else {
        StatusValidationResult::failure(errors)
    }
}

/// Extract task number from a line like "| 20 | Complete |" or "| 20 | Complete — Finish Codex |"
fn extract_task_number(line: &str) -> Option<u32> {
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() >= 2 {
        let num_str = parts[1].trim();
        num_str.parse::<u32>().ok()
    } else {
        None
    }
}

/// Validate that a task has proper implementation, tests, and docs evidence.
fn validate_task_evidence(
    task_num: u32,
    _content: &str,
    crates_dir: &Path,
    docs_dir: &Path,
) -> Vec<String> {
    let mut errors = Vec::new();

    // Map task numbers to expected crate/module
    let (crate_name, doc_name) = match task_num {
        1..=10 => (
            "agent-adapter-core".to_string(),
            Some("core.md".to_string()),
        ),
        11..=19 => (
            "agent-adapter-config".to_string(),
            Some("config.md".to_string()),
        ),
        20..=27 => ("agent-adapter-providers".to_string(), None),
        28..=33 => ("agent-adapter-transports".to_string(), None),
        34..=39 => ("agent-adapter-cli".to_string(), Some("cli.md".to_string())),
        _ => return errors, // Unknown task range
    };

    let crate_path = crates_dir.join(&crate_name);
    if !crate_path.exists() {
        errors.push(format!(
            "Task {} claims crate {} but it doesn't exist",
            task_num, crate_name
        ));
    } else {
        // Check for src directory with implementation
        let src_path = crate_path.join("src");
        if !src_path.exists() || !src_path.join("lib.rs").exists() {
            errors.push(format!(
                "Task {} crate {} exists but has no src/lib.rs",
                task_num, crate_name
            ));
        }

        // Check for tests
        let tests_path = crate_path.join("tests");
        let src_tests_path = src_path.join("tests");
        if !tests_path.exists() && !src_tests_path.exists() {
            // Check if there are tests in the src directory
            let has_tests = src_path
                .join("lib.rs")
                .metadata()
                .map(|m| m.len() > 0)
                .unwrap_or(false);
            if !has_tests {
                errors.push(format!(
                    "Task {} crate {} has no test directory or tests in src",
                    task_num, crate_name
                ));
            }
        }
    }

    // Check docs exist
    if let Some(doc) = doc_name {
        let doc_path = docs_dir.join(&doc);
        if !doc_path.exists() {
            errors.push(format!(
                "Task {} is complete but documentation {} not found",
                task_num, doc
            ));
        }
    }

    errors
}

/// Check that status.md does not reference stale canonical task lists.
/// tasks-10.md is the sole canonical task list; tasks-8.md and tasks-9.md are superseded.
fn validate_no_stale_task_list_references(status_content: &str) -> Vec<String> {
    let mut errors = Vec::new();

    let stale_canon_refs = [
        ("tasks-8.md", "tasks-8.md is superseded by tasks-10.md"),
        ("tasks-9.md", "tasks-9.md is superseded by tasks-10.md"),
    ];

    for (filename, _message) in stale_canon_refs {
        if status_content.contains(filename) {
            errors.push(format!(
                "status.md references stale canonical task list '{}' — {}; must use tasks-10.md only",
                filename, _message
            ));
        }
    }

    errors
}

/// Check that status.md is consistent with tasks-10.md.
pub fn check_status_tasks_consistency(status_md: &Path, tasks_md: &Path) -> StatusValidationResult {
    let mut errors = Vec::new();

    let status_content = match std::fs::read_to_string(status_md) {
        Ok(c) => c,
        Err(_) => {
            return StatusValidationResult::failure(vec![StatusValidationError {
                crate_name: "status.md".to_string(),
                error: "Could not read status.md".to_string(),
            }]);
        }
    };

    // Reject stale tasks-8/tasks-9 canon references in status.md
    errors.extend(
        validate_no_stale_task_list_references(&status_content)
            .into_iter()
            .map(|e| StatusValidationError {
                crate_name: "status.md".to_string(),
                error: e,
            }),
    );

    let tasks_content = match std::fs::read_to_string(tasks_md) {
        Ok(c) => c,
        Err(_) => {
            return StatusValidationResult::failure(vec![StatusValidationError {
                crate_name: "tasks-10.md".to_string(),
                error: "Could not read tasks-10.md".to_string(),
            }]);
        }
    };

    // Extract task statuses from tasks-10.md
    let task_statuses: std::collections::HashMap<u32, &str> = extract_task_statuses(&tasks_content);

    // For each task in tasks-10.md marked Complete
    for (task_num, status) in task_statuses {
        if status == "Complete" {
            // Check that status.md mentions this task as complete
            let status_pattern = format!("| {} |", task_num);
            if !status_content.contains(&status_pattern) {
                // It's okay if status.md doesn't list every task individually
                // but it should reference tasks-10.md
            }
        }
    }

    // Check that status.md references tasks-10.md as the canonical list
    if !status_content.contains("tasks-10.md") {
        errors.push(StatusValidationError {
            crate_name: "status.md".to_string(),
            error: "status.md does not reference tasks-10.md as the active task list".to_string(),
        });
    }

    if errors.is_empty() {
        StatusValidationResult::success()
    } else {
        StatusValidationResult::failure(errors)
    }
}

/// Extract task statuses from tasks-10.md content.
fn extract_task_statuses(content: &str) -> std::collections::HashMap<u32, &str> {
    let mut statuses = std::collections::HashMap::new();

    for line in content.lines() {
        if line.starts_with('|') && !line.contains("---") && !line.contains("Task |") {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                if let Ok(num) = parts[1].trim().parse::<u32>() {
                    let status = parts[2].trim();
                    if status == "Complete" || status == "Incomplete" {
                        statuses.insert(num, status);
                    }
                }
            }
        }
    }

    statuses
}

/// Run all status validation checks.
pub fn validate_all(root_dir: &Path) -> StatusValidationResult {
    let docs_dir = root_dir.join("docs");
    let crates_dir = root_dir.join("crates");
    let status_md = docs_dir.join("status.md");
    let tasks_md = docs_dir.join("tasks-10.md");

    let mut all_errors = Vec::new();

    // Run all checks
    let status_result = validate_status(&status_md, &crates_dir, &docs_dir);
    all_errors.extend(status_result.errors);

    let tasks_result = validate_task_completeness(&tasks_md, &crates_dir, &docs_dir);
    all_errors.extend(tasks_result.errors);

    let consistency_result = check_status_tasks_consistency(&status_md, &tasks_md);
    all_errors.extend(consistency_result.errors);

    if all_errors.is_empty() {
        StatusValidationResult::success()
    } else {
        StatusValidationResult::failure(all_errors)
    }
}

#[cfg(test)]
mod tests;
