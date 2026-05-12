//! Lint to ensure ApprovalMode remains legacy-only and providers don't use it.
//!
//! This module provides a lint test to ensure provider modules do not:
//! - Match on `ApprovalMode`
//! - Read `request.approval`
//!
//! The architecture intent is that `ApprovalPolicy` is the authoritative type
//! and `ApprovalMode` is kept only for backwards compatibility.

use std::path::Path;
use std::path::PathBuf;

/// Result of the lint check.
#[derive(Debug, Clone)]
pub struct LegacyPolicyLintResult {
    pub passed: bool,
    pub errors: Vec<LegacyPolicyLintError>,
}

impl LegacyPolicyLintResult {
    pub fn success() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
        }
    }

    pub fn failure(errors: Vec<LegacyPolicyLintError>) -> Self {
        Self {
            passed: false,
            errors,
        }
    }
}

/// A lint error for legacy policy usage.
#[derive(Debug, Clone)]
pub struct LegacyPolicyLintError {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub error: String,
}

/// Lint provider modules to ensure they don't use ApprovalMode or request.approval.
pub fn lint_providers_no_legacy_policy(providers_dir: &Path) -> LegacyPolicyLintResult {
    let mut errors = Vec::new();

    if !providers_dir.exists() {
        return LegacyPolicyLintResult::failure(vec![LegacyPolicyLintError {
            file: providers_dir.to_path_buf(),
            line: None,
            error: format!("Providers directory does not exist: {:?}", providers_dir),
        }]);
    }

    // Find all Rust source files in providers
    let rust_files = find_rust_files(providers_dir);

    for rust_file in rust_files {
        let content = match std::fs::read_to_string(&rust_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check for ApprovalMode usage
        for (line_num, line) in content.lines().enumerate() {
            // Skip comments
            let trimmed = line.trim();
            if trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with("///")
                || trimmed.starts_with("//!")
            {
                continue;
            }

            // Check for ApprovalMode usage (not in comments)
            if line.contains("ApprovalMode") && !line.contains("//") {
                // Make sure it's not just in a comment
                let before_comment = line.split("//").next().unwrap_or("");
                if before_comment.contains("ApprovalMode") {
                    errors.push(LegacyPolicyLintError {
                        file: rust_file.clone(),
                        line: Some(line_num + 1),
                        error: format!(
                            "Provider module uses ApprovalMode - use ApprovalPolicy instead. Line: {}",
                            line.trim()
                        ),
                    });
                }
            }

            // Check for request.approval usage (not in comments) - but NOT approval_policy
            if line.contains("request.approval") && !line.contains("//") {
                let before_comment = line.split("//").next().unwrap_or("");
                // Only flag if it's request.approval (without _policy)
                if before_comment.contains("request.approval")
                    && !before_comment.contains("request.approval_policy")
                {
                    errors.push(LegacyPolicyLintError {
                        file: rust_file.clone(),
                        line: Some(line_num + 1),
                        error: format!(
                            "Provider module reads request.approval - use request.approval_policy instead. Line: {}",
                            line.trim()
                        ),
                    });
                }
            }
        }
    }

    if errors.is_empty() {
        LegacyPolicyLintResult::success()
    } else {
        LegacyPolicyLintResult::failure(errors)
    }
}

/// Find all Rust source files in a directory recursively.
fn find_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_rust_files(&path));
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path);
                }
            }
        }
    }

    files
}

/// Check that providers only use ApprovalPolicy in argv construction.
pub fn check_provider_argv_policy_only(providers_dir: &Path) -> LegacyPolicyLintResult {
    let mut errors = Vec::new();

    // This check verifies that provider command building uses approval_policy
    // rather than any legacy ApprovalMode type

    let rust_files = find_rust_files(providers_dir);

    for rust_file in rust_files {
        let content = match std::fs::read_to_string(&rust_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Look for command building code that might use legacy policy
        // The pattern we want to catch is something like:
        // match request.approval { ... }
        // or ApprovalMode::something

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and doc comments
            if trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with("///")
                || trimmed.starts_with("//!")
            {
                continue;
            }

            // Check for explicit ApprovalMode matching
            if trimmed.contains("ApprovalMode::") && !trimmed.contains("//") {
                errors.push(LegacyPolicyLintError {
                    file: rust_file.clone(),
                    line: Some(line_num + 1),
                    error: format!(
                        "Provider uses ApprovalMode variant - use ApprovalPolicy instead. Line: {}",
                        trimmed
                    ),
                });
            }
        }
    }

    if errors.is_empty() {
        LegacyPolicyLintResult::success()
    } else {
        LegacyPolicyLintResult::failure(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lint_providers_no_legacy_policy_clean() {
        let temp = TempDir::new().unwrap();
        let providers_dir = temp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Write a clean provider file using only ApprovalPolicy
        std::fs::write(
            providers_dir.join("codex_adapter.rs"),
            r#"
use crate::ChatRequest;

pub fn build_command(request: &ChatRequest) -> Vec<String> {
    let policy = request.approval_policy.as_mode_string();
    vec!["codex".to_string(), policy]
}
"#,
        )
        .unwrap();

        let result = lint_providers_no_legacy_policy(&providers_dir);
        assert!(
            result.passed,
            "Should pass when ApprovalPolicy is used: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_lint_providers_no_legacy_policy_finds_usage() {
        let temp = TempDir::new().unwrap();
        let providers_dir = temp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Write a provider file that uses ApprovalMode
        std::fs::write(
            providers_dir.join("bad_provider.rs"),
            r#"
use crate::ApprovalMode;
use crate::ChatRequest;

pub fn build_command(request: &ChatRequest) -> Vec<String> {
    match request.approval {
        ApprovalMode::Ask => vec!["safe".to_string()],
        ApprovalMode::Deny => vec!["deny".to_string()],
    }
}
"#,
        )
        .unwrap();

        let result = lint_providers_no_legacy_policy(&providers_dir);
        assert!(!result.passed, "Should fail when ApprovalMode is used");
        assert!(result.errors[0].error.contains("ApprovalMode"));
    }

    #[test]
    fn test_lint_providers_no_legacy_policy_finds_request_approval() {
        let temp = TempDir::new().unwrap();
        let providers_dir = temp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Write a provider file that reads request.approval
        std::fs::write(
            providers_dir.join("bad_provider.rs"),
            r#"
use crate::ChatRequest;

pub fn build_command(request: &ChatRequest) -> Vec<String> {
    let mode = request.approval.as_mode_string();
    vec!["codex".to_string(), mode]
}
"#,
        )
        .unwrap();

        let result = lint_providers_no_legacy_policy(&providers_dir);
        assert!(!result.passed, "Should fail when request.approval is used");
        assert!(result.errors[0].error.contains("request.approval"));
    }

    #[test]
    fn test_check_provider_argv_policy_only() {
        let temp = TempDir::new().unwrap();
        let providers_dir = temp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Write a clean provider using only approval_policy
        std::fs::write(
            providers_dir.join("codex.rs"),
            r#"
use crate::ChatRequest;

pub fn argv_for(request: &ChatRequest) -> Vec<String> {
    let policy_str = request.approval_policy.as_mode_string();
    vec!["codex".to_string(), policy_str]
}
"#,
        )
        .unwrap();

        let result = check_provider_argv_policy_only(&providers_dir);
        assert!(
            result.passed,
            "Should pass when only approval_policy is used: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_check_provider_argv_policy_only_finds_legacy() {
        let temp = TempDir::new().unwrap();
        let providers_dir = temp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Write a provider using ApprovalMode:: variants
        std::fs::write(
            providers_dir.join("legacy.rs"),
            r#"
use crate::ChatRequest;

pub fn argv_for(request: &ChatRequest) -> Vec<String> {
    match ApprovalMode::default() {
        ApprovalMode::Ask => vec!["safe".to_string()],
        _ => vec!["unsafe".to_string()],
    }
}
"#,
        )
        .unwrap();

        let result = check_provider_argv_policy_only(&providers_dir);
        assert!(
            !result.passed,
            "Should fail when ApprovalMode:: variants are used"
        );
    }

    #[test]
    fn test_find_rust_files() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        // Create some Rust files
        std::fs::write(dir.join("lib.rs"), "// lib").unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub").join("module.rs"), "// module").unwrap();

        let files = find_rust_files(dir);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_lint_allows_comments_with_approvalmode() {
        let temp = TempDir::new().unwrap();
        let providers_dir = temp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Write a provider file with ApprovalMode only in comments
        std::fs::write(
            providers_dir.join("provider.rs"),
            r#"
// This code uses ApprovalPolicy, not ApprovalMode (legacy)
use crate::ChatRequest;

/// ApprovalMode was the old type, now we use ApprovalPolicy
pub fn build_command(request: &ChatRequest) -> Vec<String> {
    // request.approval_policy is the correct field
    vec!["codex".to_string()]
}
"#,
        )
        .unwrap();

        let result = lint_providers_no_legacy_policy(&providers_dir);
        assert!(
            result.passed,
            "Should pass when ApprovalMode is only in comments: {:?}",
            result.errors
        );
    }
}
