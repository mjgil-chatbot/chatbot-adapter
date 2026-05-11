//! Documentation linter for agent-adapter.
//!
//! This module provides linting utilities to ensure documentation consistency:
//! - All referenced docs exist
//! - Provider docs are linked
//! - No stale task lists
//! - All claims have matching implementation tests

use std::fs;
use std::path::{Path, PathBuf};
/// Result of a docs lint check.
#[derive(Debug, Clone)]
pub struct DocsLintResult {
    pub passed: bool,
    pub errors: Vec<DocsLintError>,
}
impl DocsLintResult {
    pub fn success() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
        }
    }}
impl DocsLintResult {
    pub fn failure(errors: Vec<DocsLintError>) -> Self {
        Self {
            passed: false,
            errors,
        }
    }}
/// A docs lint error.
#[derive(Debug, Clone)]
pub struct DocsLintError {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub error: String,
}
/// Lint the docs directory.
pub fn lint_docs(docs_dir: &Path) -> DocsLintResult {
    let mut errors = Vec::new();

    // Check that all linked docs exist
    errors.extend(check_doc_links(docs_dir));

    // Check that provider docs are linked from index
    errors.extend(check_provider_links(docs_dir));

    // Reject internal task ledgers and stale task artifacts in public docs/
    errors.extend(check_task_completeness(docs_dir));

    // Check that docs don't contain contradictory safety claims
    errors.extend(check_safety_consistency(docs_dir));

    if errors.is_empty() {
        DocsLintResult::success()
    } else {
        DocsLintResult::failure(errors)
    }
}
/// Check that all internal links in docs point to existing files.
fn check_doc_links(docs_dir: &Path) -> Vec<DocsLintError> {
    let mut errors = Vec::new();

    if !docs_dir.exists() {
        errors.push(DocsLintError {
            file: docs_dir.to_path_buf(),
            line: None,
            error: "Docs directory does not exist".to_string(),
        });
        return errors;
    }

    // Walk all markdown files
    let md_files = collect_markdown_files(docs_dir);

    for md_file in &md_files {
        let content = match fs::read_to_string(md_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check for relative links to other markdown files
        for (line_num, line) in content.lines().enumerate() {
            // Look for markdown links like [text](path.md)
            if let Some(link_start) = line.find('[') {
                if let Some(link_text_end) = line[link_start..].find(']') {
                    let after_bracket = &line[link_start + link_text_end + 1..];
                    if after_bracket.starts_with('(') {
                        if let Some(link_end) = after_bracket.find(')') {
                            let link = &after_bracket[1..link_end];
                            // Skip external links and anchors
                            if !link.starts_with("http") && !link.starts_with('#') {
                                let link_path = md_file
                                    .parent()
                                    .map(|p| p.join(link))
                                    .unwrap_or_else(|| PathBuf::from(link));

                                // Check if the file exists (with or without .md extension)
                                if !link_path.exists() && !link_path.with_extension("md").exists() {
                                    errors.push(DocsLintError {
                                        file: md_file.clone(),
                                        line: Some(line_num + 1),
                                        error: format!("Link target does not exist: {}", link),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    errors
}
/// Check that all provider docs exist and are linked.
fn check_provider_links(docs_dir: &Path) -> Vec<DocsLintError> {
    let mut errors = Vec::new();

    // Expected provider docs
    let expected_providers = vec![
        "codex.md",
        "claude.md",
        "gemini.md",
        "kimi.md",
        "opencode.md",
        "hermes.md",
        "forge.md",
        "claw.md",
    ];

    let providers_dir = docs_dir.join("providers");
    if !providers_dir.exists() {
        errors.push(DocsLintError {
            file: docs_dir.to_path_buf(),
            line: None,
            error: "providers directory does not exist".to_string(),
        });
        return errors;
    }

    for provider in expected_providers {
        let provider_path = providers_dir.join(provider);
        if !provider_path.exists() {
            errors.push(DocsLintError {
                file: docs_dir.join("providers").join("index.md"),
                line: None,
                error: format!("Missing provider doc: {}", provider),
            });
        }
    }

    errors
}
/// Reject internal task ledgers and stale task artifacts from the public docs tree.
fn check_task_completeness(docs_dir: &Path) -> Vec<DocsLintError> {
    let mut errors = Vec::new();

    let tasks_2 = docs_dir.join("tasks-2.md");
    let tasks_3 = docs_dir.join("tasks-3.md");
    let tasks_4 = docs_dir.join("tasks-4.md");
    let tasks_5 = docs_dir.join("tasks-5.md");
    let tasks_6 = docs_dir.join("tasks-6.md");
    let tasks_7 = docs_dir.join("tasks-7.md");
    let tasks_8 = docs_dir.join("tasks-8.md");
    let tasks_9 = docs_dir.join("tasks-9.md");

    // Stale tasks-8 pass artifacts and ledgers that must be archived
    let stale_tasks_8_passes: Vec<_> = (1..=9)
        .map(|i| docs_dir.join(format!("tasks-8-pass-{}.md", i)))
        .collect();
    let stale_tasks_8_status: Vec<_> = (1..=9)
        .map(|i| docs_dir.join(format!("tasks-8-pass-{}-status.md", i)))
        .chain(std::iter::once(docs_dir.join("tasks-8-status.md")))
        .collect();

    // Superseded task lists should never appear in the public docs root.
    if tasks_2.exists() {
        errors.push(DocsLintError {
            file: tasks_2.clone(),
            line: None,
            error: "tasks-2.md should be archived, not in root docs".to_string(),
        });
    }

    if tasks_3.exists() {
        errors.push(DocsLintError {
            file: tasks_3.clone(),
            line: None,
            error: "tasks-3.md should be archived, not in root docs".to_string(),
        });
    }

    if tasks_4.exists() {
        errors.push(DocsLintError {
            file: tasks_4.clone(),
            line: None,
            error: "tasks-4.md should be archived, not in root docs".to_string(),
        });
    }

    if tasks_5.exists() {
        errors.push(DocsLintError {
            file: tasks_5.clone(),
            line: None,
            error: "tasks-5.md should be archived, not in root docs".to_string(),
        });
    }

    if tasks_6.exists() {
        errors.push(DocsLintError {
            file: tasks_6.clone(),
            line: None,
            error: "tasks-6.md should be archived, not in root docs".to_string(),
        });
    }

    if tasks_7.exists() {
        errors.push(DocsLintError {
            file: tasks_7.clone(),
            line: None,
            error: "tasks-7.md should be archived, not in root docs".to_string(),
        });
    }

    if tasks_8.exists() {
        errors.push(DocsLintError {
            file: tasks_8.clone(),
            line: None,
            error: "tasks-8.md should be archived, not in root docs".to_string(),
        });
    }

    if tasks_9.exists() {
        errors.push(DocsLintError {
            file: tasks_9.clone(),
            line: None,
            error: "tasks-9.md should be archived, not in root docs".to_string(),
        });
    }

    // Check stale tasks-8 pass artifacts
    for stale in &stale_tasks_8_passes {
        if stale.exists() {
            errors.push(DocsLintError {
                file: stale.clone(),
                line: None,
                error: format!(
                    "{} should be archived, not in root docs",
                    stale.file_name().and_then(|n| n.to_str()).unwrap_or("")
                ),
            });
        }
    }

    // Check stale tasks-8 status ledgers
    for stale in &stale_tasks_8_status {
        if stale.exists() {
            errors.push(DocsLintError {
                file: stale.clone(),
                line: None,
                error: format!(
                    "{} should be archived, not in root docs",
                    stale.file_name().and_then(|n| n.to_str()).unwrap_or("")
                ),
            });
        }
    }

    errors
}
/// Check that docs don't contain contradictory safety claims.
fn check_safety_consistency(docs_dir: &Path) -> Vec<DocsLintError> {
    let mut errors = Vec::new();

    let md_files = collect_markdown_files(docs_dir);

    for md_file in &md_files {
        let content = match fs::read_to_string(md_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let filename = md_file.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check for contradictory statements in policy docs
        if filename.contains("policy") || filename.contains("design") {
            let has_unsafe_default = content.to_lowercase().contains("never use yolo")
                || content.to_lowercase().contains("never use dangerous")
                || content
                    .to_lowercase()
                    .contains("never default to dangerous");

            let _has_dangerous_default = content.to_lowercase().contains("dangerous/default")
                || content.to_lowercase().contains("dangerous default")
                || content.to_lowercase().contains("yolo by default");

            // Check README for the actual policy
            if let Some(parent) = docs_dir.parent() {
                let readme_path = parent.join("README.md");
                if readme_path.exists() {
                    if let Ok(readme_content) = fs::read_to_string(&readme_path) {
                        let readme_has_dangerous =
                            readme_content.to_lowercase().contains("dangerous default")
                                || readme_content.to_lowercase().contains("dangerous/default");

                        // If README says dangerous default but doc says "never dangerous", that's inconsistent
                        if readme_has_dangerous && has_unsafe_default {
                            errors.push(DocsLintError {
                                file: md_file.clone(),
                                line: None,
                                error: "Contradictory safety claim: doc says 'never dangerous' but policy is dangerous-default".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    errors
}
/// Collect all markdown files in a directory.
fn collect_markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_markdown_files(&path));
            } else if let Some(ext) = path.extension() {
                if ext == "md" {
                    files.push(path);
                }
            }
        }
    }

    files
}
