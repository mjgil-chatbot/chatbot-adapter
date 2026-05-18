use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_validate_design_traceability_missing_core() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create minimal crate structure with all required files and symbols
    // Note: root is the repo root, so paths go directly under it
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(root.join("crates/agent-adapter-core/src/lib.rs"), "// core").unwrap();
    std::fs::write(
            root.join("crates/agent-adapter-core/src/request.rs"),
            "pub struct ChatRequest { pub approval_policy: ApprovalPolicy }\npub enum ApprovalPolicy { DangerousDefault, Safe }",
        )
        .unwrap();
    std::fs::write(
            root.join("crates/agent-adapter-core/src/events.rs"),
            "pub enum AgentEvent { RunStarted { run_id: Uuid }, RunCompleted { final_text: Option<String> }, RunFailed { code: Option<i32>, message: String } }",
        )
        .unwrap();
    std::fs::write(
            root.join("crates/agent-adapter-core/src/adapter.rs"),
            "pub trait AgentAdapter { async fn run(&self, request: ChatRequest) -> Result<EventStream>; }",
        )
        .unwrap();
    std::fs::write(
            root.join("crates/agent-adapter-core/src/capabilities.rs"),
            "pub struct AdapterCapabilities { pub sessions: bool, pub tools: bool }\npub enum AdapterHealth { Ready { version: Option<String> }, MissingBinary { install_hint: String } }",
        )
        .unwrap();
    std::fs::write(
            root.join("crates/agent-adapter-core/src/errors.rs"),
            "pub enum AdapterError { ProviderNotFound { provider: ProviderId }, fn exit_code(&self) -> i32 }",
        )
        .unwrap();

    // Create transports - path must match what validation expects
    std::fs::create_dir_all(root.join("crates/agent-adapter-transports/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-transports/src/lib.rs"),
        "// transports",
    )
    .unwrap();
    // Create the acp_stdio.rs that validation checks for with required symbols
    std::fs::write(
            root.join("crates/agent-adapter-transports/src/acp_stdio.rs"),
            "pub struct AcpStdioTransport;\nimpl AcpStdioTransport {\n    pub async fn run_stream(&self, _: ChatRequest) -> Result<TransportStream, AdapterError> { todo!() }\n}",
        )
        .unwrap();

    // Write a design.md that claims everything
    let design_md = root.join("design.md");
    std::fs::write(
            &design_md,
            "# Design\n\nChatRequest and AgentEvent are implemented.\nAgentAdapter provides run().\n AcpStdio transport is implemented.\n Router handles routing.",
        )
        .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        result.passed,
        "Should pass when all files exist: {:?}",
        result.errors
    );
}

#[test]
fn test_validate_design_traceability_missing_provider() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create minimal structure - no providers
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(root.join("crates/agent-adapter-core/src/lib.rs"), "// core").unwrap();

    std::fs::create_dir_all(root.join("crates/agent-adapter-transports/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-transports/src/lib.rs"),
        "// transports",
    )
    .unwrap();

    // Write design.md claiming providers
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nCodex provider is implemented.\nClaude provider is implemented.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when provider files are missing"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.error.contains("codex_adapter.rs")));
}

#[test]
fn test_validate_claim() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create the expected file with required symbol
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-core/src/request.rs"),
        "pub struct ChatRequest { pub approval_policy: ApprovalPolicy }",
    )
    .unwrap();

    let result = validate_claim("ChatRequest is implemented", root);
    assert!(result.passed);
}

#[test]
fn test_validate_claim_missing() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Don't create any files
    let result = validate_claim("SomeClaim is implemented", root);
    // Should pass because the claim doesn't match any known keywords
    assert!(result.passed);
}

#[test]
fn test_check_security_claims() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create core with approval policy
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-core/src/request.rs"),
        "pub struct ChatRequest { pub approval_policy: ApprovalPolicy }",
    )
    .unwrap();

    // Content must include DangerousDefault (camelCase) since the check looks for it
    let content = "DangerousDefault is the default. --safe flag enables safe mode. ApprovalPolicy controls mode.";
    let errors = check_security_claims(content, root);
    assert!(
        errors.is_empty(),
        "Should not have errors when policy is in content: {:?}",
        errors
    );
}

#[test]
fn test_check_transport_claims_with_symbols() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create transports with crates/ prefix to match expected paths
    std::fs::create_dir_all(root.join("crates/agent-adapter-transports/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-transports/src/lib.rs"),
        "// transports",
    )
    .unwrap();
    // Write file with required symbol
    std::fs::write(
            root.join("crates/agent-adapter-transports/src/acp_stdio.rs"),
            "pub struct AcpStdioTransport;\nimpl AcpStdioTransport {\n    pub async fn run_stream(&self, _: ChatRequest) -> Result<TransportStream, AdapterError> { todo!() }\n}",
        )
        .unwrap();

    let content = "AcpStdio is the preferred transport.";
    let errors = check_transport_claims_with_symbols(content, root);
    assert!(
        errors.is_empty(),
        "Should not have errors when ACP with symbols exists: {:?}",
        errors
    );
}

#[test]
fn test_check_router_claims_with_symbols() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create router with proper path and required symbols
    std::fs::create_dir_all(root.join("crates/agent-adapter-router/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-router/src/lib.rs"),
        "// router lib",
    )
    .unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-router/src/router.rs"),
        "pub struct AgentRouter;\nimpl AgentRouter {\n    pub fn capabilities(&self) {}\n}",
    )
    .unwrap();

    let content = "AgentRouter handles capability enforcement.";
    let errors = check_router_claims(content, root);
    assert!(
        errors.is_empty(),
        "Should not have errors when router with symbols exists: {:?}",
        errors
    );
}

// =========================================================================
// Task 8 Pass 1: New failing-fixture tests
// =========================================================================

#[test]
fn test_rejects_stale_prompt_config_key() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create minimal config crate
    std::fs::create_dir_all(root.join("crates/agent-adapter-config/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-config/src/loader.rs"),
        "// loader",
    )
    .unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-config/src/types.rs"),
        "pub struct ProviderConfig { pub prompt_mode: PromptMode }",
    )
    .unwrap();

    // Write design.md using stale "prompt = " key (not prompt_mode)
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nConfig uses prompt = \"stdin\" for provider args.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when design.md uses stale 'prompt' key instead of 'prompt_mode'"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("stale") && e.error.contains("prompt_mode")),
        "Error should mention stale 'prompt' key: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_nonexistent_evidence_file_mappers_rs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create empty transports (no mappers.rs)
    std::fs::create_dir_all(root.join("crates/agent-adapter-transports/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-transports/src/lib.rs"),
        "// transports - no mappers.rs",
    )
    .unwrap();

    // Write design.md referencing the nonexistent mappers.rs
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nTextDelta evidence: agent-adapter-transports/src/mappers.rs",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when design.md references nonexistent mappers.rs"
    );
    assert!(
        result.errors.iter().any(|e| e.error.contains("mappers.rs")),
        "Error should mention missing mappers.rs: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_unsupported_dangerous_test_mode_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create router so the router check passes
    std::fs::create_dir_all(root.join("crates/agent-adapter-router/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-router/src/lib.rs"),
        "// router",
    )
    .unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-router/src/router.rs"),
        "pub struct AgentRouter;",
    )
    .unwrap();

    // Create testkit with real_provider_gates that does NOT mention DANGEROUS_TEST_MODE
    std::fs::create_dir_all(root.join("crates/agent-adapter-testkit/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-testkit/src/real_provider_gates.rs"),
        "// real provider gates - only RUN_REAL_* vars are checked\npub fn from_env() {}",
    )
    .unwrap();

    // Write design.md claiming DANGEROUS_TEST_MODE enforcement
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nReal provider tests gated by DANGEROUS_TEST_MODE and RUN_REAL_* vars.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when design.md claims DANGEROUS_TEST_MODE gate but implementation lacks it"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("DANGEROUS_TEST_MODE")),
        "Error should mention DANGEROUS_TEST_MODE: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_broad_claim_that_only_matches_existing_symbol() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create core with some symbol like "Forge"
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-core/src/lib.rs"),
        "pub struct Forge {}\npub const VERSION: &str = \"1.0\";",
    )
    .unwrap();

    // Write design.md claiming Forge provider WITHOUT creating forge_adapter.rs
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nForge provider is implemented with ForgeAdapter for agent execution.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    // Should fail because ForgeProvider is claimed but forge_adapter.rs doesn't exist
    // even though "Forge" symbol exists in core/lib.rs
    assert!(
            !result.passed,
            "Should fail when design claims a provider that doesn't exist even if a symbol with similar name exists elsewhere"
        );
}

#[test]
fn test_rejects_nonexistent_stale_temp_workspace_path() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create router so router check passes
    std::fs::create_dir_all(root.join("crates/agent-adapter-router/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-router/src/lib.rs"),
        "// router",
    )
    .unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-router/src/router.rs"),
        "pub struct AgentRouter; impl AgentRouter { pub fn capabilities(&self) {} }",
    )
    .unwrap();

    // Create core types.rs for workspace check
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(root.join("crates/agent-adapter-core/src/lib.rs"), "// core").unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-core/src/types.rs"),
        "pub enum WorkspaceRef { Direct, Temp }",
    )
    .unwrap();

    // NOTE: temp_workspace.rs does NOT exist in the temp dir
    // Write design.md referencing the stale temp_workspace path
    let design_md = root.join("design.md");
    std::fs::write(
            &design_md,
            "# Design\n\nTempWorkspace lives at agent-adapter-transports/src/temp_workspace.rs for isolation.",
        )
        .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when design.md references nonexistent temp_workspace.rs"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("temp_workspace.rs")),
        "Error should mention missing temp_workspace.rs: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_stale_root_tasks8_tasks9_task_list() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create minimal crate structure for Implementation Status section
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(root.join("crates/agent-adapter-core/src/lib.rs"), "// core").unwrap();
    std::fs::create_dir_all(root.join("crates/agent-adapter-testkit/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-testkit/src/temp_workspace.rs"),
        "// temp workspace",
    )
    .unwrap();

    // Create docs directory with tasks-10.md canonical list AND stale tasks-8.md/tasks-9.md in root
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(
        root.join("docs/tasks-10.md"),
        "# Tasks 10\n\n| # | Status |\n|---|--------|\n| 1 | Complete |",
    )
    .unwrap();
    // Deliberately create stale tasks-8.md in root (violation)
    std::fs::write(
        root.join("docs/tasks-8.md"),
        "# Tasks 8\n\nsuperseded task list",
    )
    .unwrap();
    // Deliberately create stale tasks-9.md in root (violation)
    std::fs::write(
        root.join("docs/tasks-9.md"),
        "# Tasks 9\n\nsuperseded task list",
    )
    .unwrap();

    let design_md = root.join("docs/design.md");
    std::fs::write(
        &design_md,
        "# Design\n\n## Implementation Status\n\nCrate complete.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when tasks-8.md or tasks-9.md exists in root docs instead of archive"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("tasks-8.md") && e.error.contains("archive")),
        "Error should mention tasks-8.md needs archiving: {:?}",
        result.errors
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("tasks-9.md") && e.error.contains("archive")),
        "Error should mention tasks-9.md needs archiving: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_stale_temp_workspace_evidence_path_in_design_md() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create transports crate WITHOUT temp_workspace.rs
    std::fs::create_dir_all(root.join("crates/agent-adapter-transports/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-transports/src/lib.rs"),
        "// transports - no temp_workspace.rs here",
    )
    .unwrap();

    // Create testkit crate WITH temp_workspace.rs
    std::fs::create_dir_all(root.join("crates/agent-adapter-testkit/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-testkit/src/temp_workspace.rs"),
        "pub struct TempWorkspace;",
    )
    .unwrap();

    // Create docs directory and write design.md citing the WRONG path (stale evidence row)
    std::fs::create_dir_all(root.join("docs")).unwrap();
    let design_md = root.join("docs/design.md");
    std::fs::write(
            &design_md,
            "# Design\n\n## Implementation Status\n\n| `agent-adapter-workspace` | Module | `agent-adapter-transports/src/temp_workspace.rs` (TempWorkspace) |",
        )
        .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when design.md cites temp_workspace.rs in wrong crate"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("agent-adapter-transports")
                && e.error.contains("agent-adapter-testkit")),
        "Error should mention stale transport path vs actual testkit location: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_tasks10_md_referencing_tasks9_as_canonical() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create docs with tasks-10.md that still references tasks-9 as active
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(
            root.join("docs/tasks-10.md"),
            "# Tasks 10\n\ntasks-9.md is the active task list.\n\nReal provider tests gated by DANGEROUS_TEST_MODE.",
        )
        .unwrap();

    // Create real_provider_gates.rs WITHOUT DANGEROUS_TEST_MODE
    std::fs::create_dir_all(root.join("crates/agent-adapter-testkit/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-testkit/src/real_provider_gates.rs"),
        "// only RUN_REAL_* vars are checked\npub fn from_env() -> bool { false }",
    )
    .unwrap();

    let result = validate_design_traceability(&root.join("docs/tasks-10.md"), root);
    assert!(
        !result.passed,
        "Should fail when tasks-10.md cites tasks-9.md as active and requires DANGEROUS_TEST_MODE"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("tasks-9.md is the active task list")),
        "Error should mention stale tasks-9 reference: {:?}",
        result.errors
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.error.contains("DANGEROUS_TEST_MODE")),
        "Error should mention unsupported DANGEROUS_TEST_MODE gate: {:?}",
        result.errors
    );
}

// =========================================================================
// Failure fixtures for "doc exists but stale"
// =========================================================================

#[test]
fn test_rejects_stale_placeholder_doc() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create a stale placeholder doc
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/core.md"), "# TODO: Write documentation").unwrap();

    // Create design.md that cites this doc
    let design_md = root.join("design.md");
    std::fs::write(&design_md, "# Design\n\nSee docs/core.md for the core API.").unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when cited doc is a placeholder with < 100 chars"
    );
    let has_stale_error = result
        .errors
        .iter()
        .any(|e| e.claim.contains("stale/placeholder"));
    assert!(
        has_stale_error,
        "Expected stale doc error: {:?}",
        result.errors
    );
}

#[test]
fn test_rejects_stub_doc_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create a stub doc (short placeholder)
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/events.md"), "# Placeholder").unwrap();

    // Create design.md that cites this doc
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nSee docs/events.md for event schema.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(!result.passed, "Should fail when cited doc is a stub");
    let has_stub_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("stub") || e.error.contains("placeholder"));
    assert!(
        has_stub_error,
        "Expected stub doc error: {:?}",
        result.errors
    );
}

// =========================================================================
// Failure fixtures for "test exists but unrelated"
// =========================================================================

#[test]
fn test_rejects_cited_test_module_that_has_no_tests() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create types.rs WITHOUT any #[test] modules
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-core/src/types.rs"),
        "pub struct Foo;\npub enum Bar { A, B }",
    )
    .unwrap();

    // Create design.md that cites types::tests (but there are no tests)
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nSee types::tests::* for unit tests.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    assert!(
        !result.passed,
        "Should fail when design cites test module but file has no tests"
    );
    let has_unrelated_test_error = result
        .errors
        .iter()
        .any(|e| e.error.contains("contains no #[test]"));
    assert!(
        has_unrelated_test_error,
        "Expected 'no tests' error: {:?}",
        result.errors
    );
}

#[test]
fn test_passes_when_cited_test_module_has_tests() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create types.rs WITH #[test] modules
    std::fs::create_dir_all(root.join("crates/agent-adapter-core/src")).unwrap();
    std::fs::write(
        root.join("crates/agent-adapter-core/src/types.rs"),
        "pub struct Foo;\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn test_foo() {}\n}",
    )
    .unwrap();

    // Create design.md that cites types::tests
    let design_md = root.join("design.md");
    std::fs::write(
        &design_md,
        "# Design\n\nSee types::tests::* for unit tests.",
    )
    .unwrap();

    let result = validate_design_traceability(&design_md, root);
    // Should pass - cited test module has tests
    assert!(
        result.passed,
        "Should pass when cited test module has tests: {:?}",
        result.errors
    );
}

// =========================================================================
// Integration tests: run validators against the real workspace
// =========================================================================

#[test]
fn test_validate_design_traceability_against_real_workspace() {
    // Run validate_design_traceability against the real workspace
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.join("..").join("..");
    let design_md = repo_root.join("docs").join("design.md");

    let result = validate_design_traceability(&design_md, &repo_root);
    assert!(
        result.passed,
        "validate_design_traceability failed on real workspace: {:?}",
        result.errors
    );
}
