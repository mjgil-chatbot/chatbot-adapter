use super::*;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_lint_file_with_todo() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.md");

    // Write a file with TODO
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "# Test").unwrap();
    writeln!(file, "TODO: implement this").unwrap();

    let result = lint_file(&path);
    assert!(!result.passed);
    assert!(result.errors[0].error.contains("TODO"));
}

#[test]
fn test_lint_file_clean() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.md");

    // Write a clean file
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "# Test").unwrap();
    writeln!(file, "This is a clean file.").unwrap();

    let result = lint_file(&path);
    assert!(result.passed);
}

#[test]
fn test_collect_markdown_files() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create some markdown files
    std::fs::write(docs.join("a.md"), "# A").unwrap();
    std::fs::create_dir_all(docs.join("sub")).unwrap();
    std::fs::write(docs.join("sub").join("b.md"), "# B").unwrap();

    let files = collect_markdown_files(docs);
    assert_eq!(files.len(), 2);
}

#[test]
fn test_check_doc_links_broken() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create a doc with a broken link
    std::fs::write(docs.join("test.md"), "[link](nonexistent.md)").unwrap();

    let errors = check_doc_links(docs);
    assert!(!errors.is_empty());
    assert!(errors[0].error.contains("nonexistent.md"));
}

#[test]
fn test_check_doc_links_valid() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create two docs with valid links
    std::fs::write(docs.join("a.md"), "[link](b.md)").unwrap();
    std::fs::write(docs.join("b.md"), "# B").unwrap();

    let errors = check_doc_links(docs);
    assert!(errors.is_empty(), "Found unexpected errors: {:?}", errors);
}

#[test]
fn test_check_provider_links_missing() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create providers dir without all providers
    std::fs::create_dir_all(docs.join("providers")).unwrap();
    std::fs::write(docs.join("providers").join("claude.md"), "# Claude").unwrap();

    let errors = check_provider_links(docs);
    assert!(!errors.is_empty());
    assert!(errors[0].error.contains("codex"));
}

#[test]
fn test_check_provider_links_complete() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create all provider docs
    std::fs::create_dir_all(docs.join("providers")).unwrap();
    for provider in &[
        "codex", "claude", "gemini", "kimi", "opencode", "hermes", "forge", "claw",
    ] {
        std::fs::write(
            docs.join("providers").join(format!("{}.md", provider)),
            format!("# {}", provider),
        )
        .unwrap();
    }

    let errors = check_provider_links(docs);
    assert!(errors.is_empty(), "Found unexpected errors: {:?}", errors);
}

#[test]
fn test_check_task_completeness_canonical_only() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    let errors = check_task_completeness(docs);
    assert!(
        errors.is_empty(),
        "Should not error when no internal task artifacts exist: {:?}",
        errors
    );
}

#[test]
fn test_check_task_completeness_legacy_forbidden() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create tasks-3.md (should not exist in root)
    std::fs::write(
        docs.join("tasks-3.md"),
        "# Tasks\n\n| # | Task | Status |\n|---|------|--------|\n| 1 | Test | Complete |",
    )
    .unwrap();

    let errors = check_task_completeness(docs);
    assert!(
        !errors.is_empty(),
        "Should error when tasks-3.md exists in root"
    );
    // Find the error about tasks-3.md
    let legacy_error = errors.iter().find(|e| e.error.contains("tasks-3.md"));
    assert!(
        legacy_error.is_some(),
        "Should mention tasks-3.md in errors: {:?}",
        errors
    );
}

#[test]
fn test_check_task_completeness_archive_required() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    let errors = check_task_completeness(docs);
    assert!(
        errors.is_empty(),
        "Archive dir is optional when no task artifacts remain"
    );
}

#[test]
fn test_check_task_completeness_with_archive() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    std::fs::create_dir_all(docs.join("archive")).unwrap();

    let errors = check_task_completeness(docs);
    assert!(
        errors.is_empty(),
        "Should pass when no root task artifacts exist: {:?}",
        errors
    );
}

#[test]
fn test_check_task_completeness_tasks_4_forbidden() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create tasks-4.md (should not exist in root)
    std::fs::write(
        docs.join("tasks-4.md"),
        "# Tasks\n\n| # | Task | Status |\n|---|------|--------|\n| 1 | Test | Complete |",
    )
    .unwrap();

    let errors = check_task_completeness(docs);
    assert!(
        !errors.is_empty(),
        "Should error when tasks-4.md exists in root"
    );
    let error = errors.iter().find(|e| e.error.contains("tasks-4.md"));
    assert!(
        error.is_some(),
        "Should mention tasks-4.md in errors: {:?}",
        errors
    );
}

#[test]
fn test_check_task_completeness_tasks_5_6_forbidden() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Create both tasks-5.md and tasks-6.md (should not exist in root)
    std::fs::write(docs.join("tasks-5.md"), "# Tasks 5").unwrap();
    std::fs::write(docs.join("tasks-6.md"), "# Tasks 6").unwrap();

    let errors = check_task_completeness(docs);
    assert!(
        !errors.is_empty(),
        "Should error when tasks-5.md or tasks-6.md exists in root"
    );
    let has_5 = errors.iter().any(|e| e.error.contains("tasks-5.md"));
    let has_6 = errors.iter().any(|e| e.error.contains("tasks-6.md"));
    assert!(
        has_5 && has_6,
        "Should mention both tasks-5.md and tasks-6.md: {:?}",
        errors
    );
}

// =========================================================================
// Integration tests: run validators against the real workspace
// =========================================================================

#[test]
fn test_lint_docs_against_real_workspace() {
    // Run lint_docs against the real workspace docs/ directory
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.join("..").join("..");
    let docs_dir = repo_root.join("docs");

    let result = lint_docs(&docs_dir);
    assert!(
        result.passed,
        "lint_docs failed on real workspace: {:?}",
        result.errors
    );
}

// =========================================================================
// Controlled stale-doc fixture tests
// =========================================================================

#[test]
fn test_lint_docs_fails_on_stale_root_task_list() {
    // Verify the acceptance path fails when a superseded task list (tasks-4.md)
    // exists in the root docs/ directory without being archived.
    // This is the controlled stale-doc fixture that acceptance must reject.
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Set up a workspace with an archived copy plus an accidental root leak.
    std::fs::create_dir_all(docs.join("archive")).unwrap();
    std::fs::write(
        docs.join("archive").join("tasks-4.md"),
        "# Tasks 4 (archived)",
    )
    .unwrap();
    // Deliberately put tasks-4.md in root (violation)
    std::fs::write(docs.join("tasks-4.md"), "# Tasks 4 (should be archived)").unwrap();

    let result = lint_docs(docs);
    assert!(
        !result.passed,
        "lint_docs should FAIL when stale task list exists in root"
    );
    let has_stale_error = result.errors.iter().any(|e| e.error.contains("tasks-4.md"));
    assert!(
        has_stale_error,
        "Error should mention tasks-4.md: {:?}",
        result.errors
    );
}

#[test]
fn test_lint_docs_passes_when_stale_task_lists_archived() {
    // Verify the acceptance path PASSES when superseded task lists
    // are properly archived (not in root).
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    // Set up a compliant workspace with archived historical notes only.
    std::fs::create_dir_all(docs.join("archive")).unwrap();
    std::fs::write(
        docs.join("archive").join("tasks-4.md"),
        "# Tasks 4 (archived)",
    )
    .unwrap();
    std::fs::write(
        docs.join("archive").join("tasks-5.md"),
        "# Tasks 5 (archived)",
    )
    .unwrap();
    std::fs::write(
        docs.join("archive").join("tasks-6.md"),
        "# Tasks 6 (archived)",
    )
    .unwrap();

    // Create minimal providers directory to avoid provider link errors
    std::fs::create_dir_all(docs.join("providers")).unwrap();
    for provider in &[
        "codex", "claude", "gemini", "kimi", "opencode", "hermes", "forge", "claw",
    ] {
        std::fs::write(
            docs.join("providers").join(format!("{}.md", provider)),
            format!("# {}", provider),
        )
        .unwrap();
    }

    let result = lint_docs(docs);
    assert!(
        result.passed,
        "lint_docs should PASS when stale task lists are properly archived: {:?}",
        result.errors
    );
}

// =========================================================================
// Stale tasks-8 pass-artifact and ledger fixture tests
// =========================================================================

#[test]
fn test_lint_docs_fails_on_stale_tasks8_pass_root() {
    // Verify lint fails when tasks-8-pass-*.md remain in root (not archived).
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    std::fs::create_dir_all(docs.join("archive")).unwrap();
    // Deliberately put tasks-8-pass-3.md in root (violation)
    std::fs::write(docs.join("tasks-8-pass-3.md"), "# Tasks 8 Pass 3").unwrap();

    let result = lint_docs(docs);
    assert!(
        !result.passed,
        "lint_docs should FAIL when stale tasks-8-pass-*.md exists in root"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("tasks-8-pass-3.md")),
        "Error should mention tasks-8-pass-3.md: {:?}",
        result.errors
    );
}

#[test]
fn test_lint_docs_fails_on_stale_tasks8_status_root() {
    // Verify lint fails when tasks-8-pass-*-status.md remain in root (not archived).
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    std::fs::create_dir_all(docs.join("archive")).unwrap();
    // Deliberately put tasks-8-pass-5-status.md in root (violation)
    std::fs::write(docs.join("tasks-8-pass-5-status.md"), "# Status").unwrap();

    let result = lint_docs(docs);
    assert!(
        !result.passed,
        "lint_docs should FAIL when stale tasks-8-pass-*-status.md exists in root"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("tasks-8-pass-5-status.md")),
        "Error should mention tasks-8-pass-5-status.md: {:?}",
        result.errors
    );
}

#[test]
fn test_lint_docs_fails_on_stale_tasks8_status_ledger_root() {
    // Verify lint fails when tasks-8-status.md (the summary ledger) is in root.
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    std::fs::create_dir_all(docs.join("archive")).unwrap();
    // Deliberately put tasks-8-status.md in root (violation)
    std::fs::write(docs.join("tasks-8-status.md"), "# Tasks 8 Status").unwrap();

    let result = lint_docs(docs);
    assert!(
        !result.passed,
        "lint_docs should FAIL when stale tasks-8-status.md exists in root"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("tasks-8-status.md")),
        "Error should mention tasks-8-status.md: {:?}",
        result.errors
    );
}

#[test]
fn test_lint_docs_passes_when_all_tasks8_artifacts_archived() {
    // Verify lint passes when all tasks-8 pass artifacts and ledgers are archived.
    let temp = TempDir::new().unwrap();
    let docs = temp.path();

    std::fs::create_dir_all(docs.join("archive")).unwrap();

    // Archive all stale tasks-8 pass artifacts
    for i in 1..=9 {
        std::fs::write(
            docs.join("archive").join(format!("tasks-8-pass-{}.md", i)),
            format!("# Tasks 8 Pass {} (archived)", i),
        )
        .unwrap();
        std::fs::write(
            docs.join("archive")
                .join(format!("tasks-8-pass-{}-status.md", i)),
            format!("# Tasks 8 Pass {} Status (archived)", i),
        )
        .unwrap();
    }
    std::fs::write(
        docs.join("archive").join("tasks-8-status.md"),
        "# Tasks 8 Status (archived)",
    )
    .unwrap();

    // Create minimal providers directory to avoid provider link errors
    std::fs::create_dir_all(docs.join("providers")).unwrap();
    for provider in &[
        "codex", "claude", "gemini", "kimi", "opencode", "hermes", "forge", "claw",
    ] {
        std::fs::write(
            docs.join("providers").join(format!("{}.md", provider)),
            format!("# {}", provider),
        )
        .unwrap();
    }

    let result = lint_docs(docs);
    assert!(
        result.passed,
        "lint_docs should PASS when all tasks-8 pass artifacts are archived: {:?}",
        result.errors
    );
}
