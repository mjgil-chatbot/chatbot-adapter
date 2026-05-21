use super::*;
use tempfile::TempDir;

#[test]
fn test_parse_crate_table() {
    let content = r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/lib.rs | 24 tests | Rustdoc |
| `agent-adapter-config` | Complete | loader.rs | 7 tests | config.md |
| `agent-adapter-providers` | Incomplete | Provider adapters | 12 tests | Provider docs |"#;

    let rows = parse_crate_table(content);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].crate_name, "`agent-adapter-core`");
    assert_eq!(rows[0].status, "Complete");
    assert_eq!(rows[1].crate_name, "`agent-adapter-config`");
    assert_eq!(rows[1].status, "Complete");
    assert_eq!(rows[2].crate_name, "`agent-adapter-providers`");
    assert_eq!(rows[2].status, "Incomplete");
}

#[test]
fn test_extract_task_number() {
    assert_eq!(extract_task_number("| 1 | Complete |"), Some(1));
    assert_eq!(
        extract_task_number("| 20 | Complete | Finish Codex |"),
        Some(20)
    );
    assert_eq!(
        extract_task_number("| 39 | Complete | Final CI |"),
        Some(39)
    );
    assert_eq!(extract_task_number("| not a number | Complete |"), None);
}

#[test]
fn test_extract_task_statuses() {
    let content = r#"## Tasks Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 1 | Complete | ChatRequest.approval_policy preserved |
| 2 | Complete | ApprovalMode deprecated |
| 3 | Incomplete | See tasks-8.md |
| 39 | Complete | Final CI passed |"#;

    let statuses = extract_task_statuses(content);
    assert_eq!(statuses.get(&1), Some(&"Complete"));
    assert_eq!(statuses.get(&2), Some(&"Complete"));
    assert_eq!(statuses.get(&3), Some(&"Incomplete"));
    assert_eq!(statuses.get(&39), Some(&"Complete"));
}

#[test]
fn test_validate_status_missing_crate() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    // Create a status.md that claims a crate exists
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-nonexistent` | Complete | src/lib.rs | 0 tests | none.md |
|"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(!result.passed);
    assert!(result.errors[0].error.contains("does not exist"));
}

#[test]
fn test_validate_status_valid() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    // Create the docs directory
    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create a real crate with lib.rs AND tests directory
    let core_crate = crates_dir.join("agent-adapter-core");
    std::fs::create_dir_all(core_crate.join("src")).unwrap();
    std::fs::write(core_crate.join("src").join("lib.rs"), "// lib").unwrap();
    // Add a tests directory so the crate is considered to have tests
    std::fs::create_dir_all(core_crate.join("tests")).unwrap();
    std::fs::write(
        core_crate.join("tests").join("basic.rs"),
        "#[test] fn t() {}",
    )
    .unwrap();

    // Create docs - use the correct name
    std::fs::write(docs_dir.join("design.md"), "# Design").unwrap();

    // Create status.md
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/lib.rs | 1 test | design.md |
|"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        result.passed,
        "Expected passed but got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_status_rejects_stale_router_methods() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create router crate with a real method but NOT capability_enforcement
    let router_crate = crates_dir.join("agent-adapter-router");
    std::fs::create_dir_all(router_crate.join("src")).unwrap();
    std::fs::write(
        router_crate.join("src").join("router.rs"),
        "pub fn register() {}",
    )
    .unwrap();
    std::fs::create_dir_all(router_crate.join("tests")).unwrap();

    // status.md that claims a nonexistent router method
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-router` | Complete | src/router.rs (register) | 1 test | design.md |

The router provides capability_enforcement() for capability checks.
"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail on stale router method claim"
    );
    let has_stale_router_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("capability_enforcement"));
    assert!(
        has_stale_router_error,
        "Expected error about capability_enforcement"
    );
}

#[test]
fn test_validate_status_rejects_wrong_workspace_path() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create workspace crate
    let ws_crate = crates_dir.join("agent-adapter-workspace");
    std::fs::create_dir_all(ws_crate.join("src")).unwrap();
    std::fs::write(ws_crate.join("src").join("lib.rs"), "// lib").unwrap();
    std::fs::create_dir_all(ws_crate.join("tests")).unwrap();
    std::fs::write(ws_crate.join("tests").join("ws.rs"), "#[test] fn t() {}").unwrap();

    // Create the CORRECT docs directory for workspace
    std::fs::create_dir_all(docs_dir.join("transports")).unwrap();
    std::fs::write(docs_dir.join("workspaces.md"), "# Workspaces").unwrap();

    // status.md with WRONG path for workspace helpers
    let status_md = temp.path().join("status.md");
    std::fs::write(
            &status_md,
            r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-workspace` | Module | agent-adapter-transports/src/temp_workspace.rs (TempWorkspace, WorktreeWorkspace) | 17 temp_workspace tests | docs/workspaces.md |
"#,
        )
        .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail on wrong workspace path"
    );
    let has_wrong_path_error = result.errors.iter().any(|e| {
        e.error
            .contains("agent-adapter-transports/src/temp_workspace")
    });
    assert!(
        has_wrong_path_error,
        "Expected error about wrong temp_workspace path"
    );
}

#[test]
fn test_validate_status_rejects_dangerous_test_mode_gate() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create a minimal crate
    let test_crate = crates_dir.join("agent-adapter-core");
    std::fs::create_dir_all(test_crate.join("src")).unwrap();
    std::fs::write(test_crate.join("src").join("lib.rs"), "// lib").unwrap();
    std::fs::create_dir_all(test_crate.join("tests")).unwrap();
    std::fs::write(test_crate.join("tests").join("t.rs"), "#[test] fn t() {}").unwrap();
    std::fs::write(docs_dir.join("design.md"), "# Design").unwrap();

    // status.md that claims DANGEROUS_TEST_MODE gate
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/lib.rs | 1 test | design.md |

Real provider tests are gated behind DANGEROUS_TEST_MODE and RUN_REAL_*.
"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail on DANGEROUS_TEST_MODE claim"
    );
    let has_gate_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("DANGEROUS_TEST_MODE"));
    assert!(has_gate_error, "Expected error about DANGEROUS_TEST_MODE");
}

#[test]
fn test_validate_status_rejects_stale_prompt_config_key() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create a minimal crate
    let test_crate = crates_dir.join("agent-adapter-core");
    std::fs::create_dir_all(test_crate.join("src")).unwrap();
    std::fs::write(test_crate.join("src").join("lib.rs"), "// lib").unwrap();
    std::fs::create_dir_all(test_crate.join("tests")).unwrap();
    std::fs::write(test_crate.join("tests").join("t.rs"), "#[test] fn t() {}").unwrap();
    std::fs::write(docs_dir.join("design.md"), "# Design").unwrap();

    // status.md with stale prompt = "stdin" config key
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/lib.rs | 1 test | design.md |

Example: prompt = "stdin" selects stdin mode.
"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail on stale prompt key"
    );
    let has_config_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("prompt_mode"));
    assert!(
        has_config_error,
        "Expected error about stale prompt config key"
    );
}

#[test]
fn test_validate_status_complete_row_requires_tests() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create a crate with lib.rs but NO tests directory
    let test_crate = crates_dir.join("agent-adapter-core");
    std::fs::create_dir_all(test_crate.join("src")).unwrap();
    std::fs::write(test_crate.join("src").join("lib.rs"), "// lib").unwrap();
    // NO tests directory
    std::fs::write(docs_dir.join("design.md"), "# Design").unwrap();

    // status.md claims Complete but only has crate existence + doc
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/lib.rs | 0 tests | design.md |
"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail on Complete row without tests"
    );
    let has_no_tests_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("no test directory"));
    assert!(has_no_tests_error, "Expected error about missing tests");
}

#[test]
fn test_check_status_tasks_consistency() {
    let temp = TempDir::new().unwrap();

    // Create status.md that references tasks-10.md
    let status_md = temp.path().join("status.md");
    std::fs::write(&status_md, "The active task list is `docs/tasks-10.md`.").unwrap();

    // Create tasks-10.md with some complete tasks
    let tasks_md = temp.path().join("tasks-10.md");
    std::fs::write(
        &tasks_md,
        r#"## Tasks Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 1 | Complete | Done |
| 2 | Incomplete | Pending |
|"#,
    )
    .unwrap();

    let result = check_status_tasks_consistency(&status_md, &tasks_md);
    assert!(
        result.passed,
        "Expected passed but got errors: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_task_completeness_no_implementation() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    // Create tasks-10.md with task 5 (core crate range) marked Complete but no crate exists
    let tasks_md = temp.path().join("tasks-10.md");
    std::fs::write(
        &tasks_md,
        r#"## Tasks Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 5 | Complete | Should fail - no implementation |
|"#,
    )
    .unwrap();

    let result = validate_task_completeness(&tasks_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail for task 5 without implementation"
    );
}

#[test]
fn test_validate_status_rejects_nonexistent_impl_path() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create a real crate with lib.rs
    let test_crate = crates_dir.join("agent-adapter-core");
    std::fs::create_dir_all(test_crate.join("src")).unwrap();
    std::fs::write(test_crate.join("src").join("lib.rs"), "// lib").unwrap();
    // Create tests directory
    std::fs::create_dir_all(test_crate.join("tests")).unwrap();
    std::fs::write(test_crate.join("tests").join("t.rs"), "#[test] fn t() {}").unwrap();
    std::fs::write(docs_dir.join("design.md"), "# Design").unwrap();

    // status.md claims implementation at src/nonexistent.rs (does not exist)
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/nonexistent.rs | tests/t.rs | design.md |
"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail on nonexistent impl path"
    );
    let has_nonexistent_impl_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("src/nonexistent.rs") && e.error.contains("does not exist"));
    assert!(
        has_nonexistent_impl_error,
        "Expected error about nonexistent impl path"
    );
}

#[test]
fn test_validate_status_rejects_nonexistent_test_path() {
    let temp = TempDir::new().unwrap();
    let crates_dir = temp.path().join("crates");
    let docs_dir = temp.path().join("docs");

    std::fs::create_dir_all(&docs_dir).unwrap();

    // Create a real crate with lib.rs and tests directory
    let test_crate = crates_dir.join("agent-adapter-core");
    std::fs::create_dir_all(test_crate.join("src")).unwrap();
    std::fs::write(test_crate.join("src").join("lib.rs"), "// lib").unwrap();
    std::fs::create_dir_all(test_crate.join("tests")).unwrap();
    std::fs::write(test_crate.join("tests").join("t.rs"), "#[test] fn t() {}").unwrap();
    std::fs::write(docs_dir.join("design.md"), "# Design").unwrap();

    // status.md claims tests at tests/nonexistent.rs (does not exist)
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/lib.rs | tests/nonexistent.rs | design.md |
"#,
    )
    .unwrap();

    let result = validate_status(&status_md, &crates_dir, &docs_dir);
    assert!(
        !result.passed,
        "Expected validation to fail on nonexistent test path"
    );
    let has_nonexistent_test_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("tests/nonexistent.rs") && e.error.contains("does not exist"));
    assert!(
        has_nonexistent_test_error,
        "Expected error about nonexistent test path"
    );
}

#[test]
fn test_extract_impl_paths_parses_correctly() {
    // Test that extract_impl_paths correctly parses various evidence formats
    let evidence = "src/lib.rs (re-exports), src/request.rs (ChatRequest, ApprovalPolicy)";
    let paths = extract_impl_paths(evidence);
    assert!(paths.contains(&"src/lib.rs".to_string()));
    assert!(paths.contains(&"src/request.rs".to_string()));
}

#[test]
fn test_extract_impl_paths_ignores_non_src_paths() {
    let evidence = "README.md, src/lib.rs, docs/design.md";
    let paths = extract_impl_paths(evidence);
    // Should only contain src/ paths
    assert!(paths.contains(&"src/lib.rs".to_string()));
    assert!(!paths
        .iter()
        .any(|p| p.contains("README.md") || p.contains("docs/")));
}

// =========================================================================
// Stale task-list reference fixtures
// =========================================================================

#[test]
fn test_rejects_stale_tasks8_canon_reference_in_status() {
    let temp = TempDir::new().unwrap();

    // Create tasks-10.md (canonical)
    let tasks_md = temp.path().join("tasks-10.md");
    std::fs::write(
        &tasks_md,
        r#"## Tasks Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 1 | Complete | Done |
|"#,
    )
    .unwrap();

    // status.md that erroneously references tasks-8.md as canonical
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        "The active task list is `docs/tasks-8.md`.\nSee tasks-10.md for reference.",
    )
    .unwrap();

    let result = check_status_tasks_consistency(&status_md, &tasks_md);
    assert!(
        !result.passed,
        "Should fail when status.md cites tasks-8.md as canonical"
    );
    let has_stale_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("tasks-8.md") && e.error.contains("superseded"));
    assert!(
        has_stale_error,
        "Error should mention tasks-8.md is superseded: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_stale_tasks9_canon_reference_in_status() {
    let temp = TempDir::new().unwrap();

    // Create tasks-10.md (canonical)
    let tasks_md = temp.path().join("tasks-10.md");
    std::fs::write(
        &tasks_md,
        r#"## Tasks Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 1 | Complete | Done |
|"#,
    )
    .unwrap();

    // status.md that erroneously references tasks-9.md as canonical
    let status_md = temp.path().join("status.md");
    std::fs::write(
        &status_md,
        "The active task list is `docs/tasks-9.md`.\ntasks-10.md is for reference only.",
    )
    .unwrap();

    let result = check_status_tasks_consistency(&status_md, &tasks_md);
    assert!(
        !result.passed,
        "Should fail when status.md cites tasks-9.md as canonical"
    );
    let has_stale_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("tasks-9.md") && e.error.contains("superseded"));
    assert!(
        has_stale_error,
        "Error should mention tasks-9.md is superseded: {:?}",
        result.errors
    );
}

#[test]
fn test_passes_when_status_references_only_tasks10() {
    let temp = TempDir::new().unwrap();

    // Create tasks-10.md (canonical)
    let tasks_md = temp.path().join("tasks-10.md");
    std::fs::write(
        &tasks_md,
        r#"## Tasks Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 1 | Complete | Done |
|"#,
    )
    .unwrap();

    // status.md that correctly references only tasks-10.md
    let status_md = temp.path().join("status.md");
    std::fs::write(&status_md, "The active task list is `docs/tasks-10.md`.").unwrap();

    let result = check_status_tasks_consistency(&status_md, &tasks_md);
    assert!(
        result.passed,
        "Should pass when status.md only references tasks-10.md: {:?}",
        result.errors
    );
}

// =========================================================================
// Integration coverage uses a self-contained planning fixture. Public source
// checkouts need not retain completed status/task ledgers.
// =========================================================================

#[test]
fn test_validate_all_against_isolated_planning_fixture() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let docs = root.join("docs");
    let core = root.join("crates/agent-adapter-core");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(core.join("src")).unwrap();
    std::fs::create_dir_all(core.join("tests")).unwrap();
    std::fs::write(core.join("src/lib.rs"), "pub fn example() {}\n").unwrap();
    std::fs::write(
        core.join("tests/basic.rs"),
        "#[test] fn example_is_available() {}\n",
    )
    .unwrap();
    std::fs::write(docs.join("core.md"), "# Core fixture\n").unwrap();
    std::fs::write(docs.join("design.md"), "# Design fixture\n").unwrap();
    std::fs::write(
        docs.join("status.md"),
        r#"## Crate Completion Status

| Crate | Status | Implementation | Tests | Documentation |
|-------|--------|----------------|-------|---------------|
| `agent-adapter-core` | Complete | src/lib.rs | 1 test | design.md |

The active task list is `docs/tasks-10.md`.
"#,
    )
    .unwrap();
    std::fs::write(
        docs.join("tasks-10.md"),
        r#"## Tasks Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 1 | Complete | Fixture implementation, regression, and core documentation |
"#,
    )
    .unwrap();
    let result = validate_all(root);
    assert!(
        result.passed,
        "complete fixture failed: {:?}",
        result.errors
    );

    std::fs::remove_file(docs.join("core.md")).unwrap();
    assert!(
        !validate_all(root).passed,
        "missing evidence must remain an error"
    );
}
