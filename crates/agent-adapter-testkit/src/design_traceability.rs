//! Design traceability validation for implementation completeness.
//!
//! This module provides validation that design.md claims are backed by actual
//! code, tests, and documentation. Every provider, transport, policy, event,
//! config, session, workspace, testing, and limitation claim must map to
//! code and tests.
//!
//! Task 7: Strengthened to verify symbols exist within files, not just file existence.

mod doc_checks;
use std::path::Path;
/// Result of a design traceability check.
#[derive(Debug, Clone)]
pub struct TraceabilityResult {
    pub passed: bool,
    pub errors: Vec<TraceabilityError>,
}
impl TraceabilityResult {
    pub fn success() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
        }
    }}
impl TraceabilityResult {
    pub fn failure(errors: Vec<TraceabilityError>) -> Self {
        Self {
            passed: false,
            errors,
        }
    }}
/// A traceability error.
#[derive(Debug, Clone)]
pub struct TraceabilityError {
    pub claim: String,
    pub error: String,
}
/// Symbol-level evidence requirement for a claim.
/// Each entry is (keyword, file_path, required_symbols_in_file).
struct SymbolRequirement(&'static str, &'static str, Vec<&'static str>);
/// Validate that design.md claims have implementation evidence.
pub fn validate_design_traceability(design_md: &Path, root_dir: &Path) -> TraceabilityResult {
    let mut errors = Vec::new();

    let content = match std::fs::read_to_string(design_md) {
        Ok(c) => c,
        Err(e) => {
            return TraceabilityResult::failure(vec![TraceabilityError {
                claim: "design.md".to_string(),
                error: format!("Could not read design.md: {}", e),
            }]);
        }
    };

    // Check core event model claims with symbol-level evidence
    errors.extend(check_core_event_model_with_symbols(&content, root_dir));

    // Check transport implementation claims with symbol evidence
    errors.extend(check_transport_claims_with_symbols(&content, root_dir));

    // Check provider adapter claims with symbol evidence
    errors.extend(check_provider_claims_with_symbols(&content, root_dir));

    // Check router capability enforcement claim
    errors.extend(check_router_claims(&content, root_dir));

    // Check real provider gate claims (always check, not conditional on router)
    errors.extend(check_real_provider_gate_claims(&content, root_dir));

    // Check config schema claim
    errors.extend(check_config_claims(&content, root_dir));

    // Check security defaults claim
    errors.extend(check_security_claims(&content, root_dir));

    // Check health check claim
    errors.extend(check_health_claims(&content, root_dir));

    // Check workspace claims
    errors.extend(check_workspace_ref_claims(&content, root_dir));

    // Check workspace path claims (always checked, separate from WorkspaceRef existence)
    errors.extend(check_workspace_claims(&content, root_dir));

    // Check lifecycle ownership claims
    errors.extend(check_lifecycle_claims(&content, root_dir));

    // Check stale root task list presence (tasks-8*, tasks-9 should be archived)
    errors.extend(doc_checks::check_stale_root_task_lists(&content, root_dir));

    // Check tasks-10.md canon reference staleness
    errors.extend(doc_checks::check_tasks10_stale_canon_reference(
        &content, root_dir,
    ));

    // Check design.md evidence row paths
    errors.extend(doc_checks::check_design_evidence_row_paths(
        &content, root_dir,
    ));

    // Check doc freshness - ensure cited docs have substantial content
    errors.extend(doc_checks::check_doc_freshness(&content, root_dir));

    // Check that cited test names/files actually exist
    errors.extend(doc_checks::check_cited_tests_exist(&content, root_dir));

    if errors.is_empty() {
        TraceabilityResult::success()
    } else {
        TraceabilityResult::failure(errors)
    }
}
/// Check core event model claims with symbol-level evidence.
fn check_core_event_model_with_symbols(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    let requirements = vec![
        SymbolRequirement(
            "ChatRequest",
            "crates/agent-adapter-core/src/request.rs",
            vec!["pub struct ChatRequest", "approval_policy"],
        ),
        SymbolRequirement(
            "AgentEvent",
            "crates/agent-adapter-core/src/events.rs",
            vec![
                "pub enum AgentEvent",
                "RunStarted",
                "RunCompleted",
                "RunFailed",
            ],
        ),
        SymbolRequirement(
            "AgentAdapter",
            "crates/agent-adapter-core/src/adapter.rs",
            vec!["pub trait AgentAdapter", "async fn run"],
        ),
        SymbolRequirement(
            "AdapterCapabilities",
            "crates/agent-adapter-core/src/capabilities.rs",
            vec!["pub struct AdapterCapabilities"],
        ),
        SymbolRequirement(
            "AdapterHealth",
            "crates/agent-adapter-core/src/capabilities.rs",
            vec!["pub enum AdapterHealth", "Ready", "MissingBinary"],
        ),
        SymbolRequirement(
            "AdapterError",
            "crates/agent-adapter-core/src/errors.rs",
            vec!["pub enum AdapterError", "exit_code"],
        ),
        SymbolRequirement(
            "ApprovalPolicy",
            "crates/agent-adapter-core/src/request.rs",
            vec!["pub enum ApprovalPolicy", "DangerousDefault", "Safe"],
        ),
        SymbolRequirement(
            "WorkspaceRef",
            "crates/agent-adapter-core/src/types.rs",
            vec![
                "pub enum WorkspaceRef",
                "Direct",
                "Temp",
                "IsolatedWorktree",
                "Container",
            ],
        ),
    ];

    for req in requirements {
        if content.contains(req.0) {
            let full_path = root_dir.join(req.1);
            if !full_path.exists() {
                errors.push(TraceabilityError {
                    claim: format!("Core type {} mentioned in design", req.0),
                    error: format!("{} does not exist", req.1),
                });
            } else {
                // Check for required symbols within the file
                let file_content = match std::fs::read_to_string(&full_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                for symbol in req.2 {
                    if !file_content.contains(symbol) {
                        errors.push(TraceabilityError {
                            claim: format!("Symbol '{}' for {} not found", symbol, req.0),
                            error: format!(
                                "File {} exists but required symbol '{}' not found",
                                req.1, symbol
                            ),
                        });
                    }
                }
            }
        }
    }

    errors
}
/// Check transport claims with symbol evidence.
fn check_transport_claims_with_symbols(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    let transport_requirements = vec![
        SymbolRequirement(
            "AcpStdio",
            "crates/agent-adapter-transports/src/acp_stdio.rs",
            vec!["pub struct AcpStdioTransport", "pub async fn run_stream"],
        ),
        SymbolRequirement(
            "JsonlCommand",
            "crates/agent-adapter-transports/src/jsonl_command.rs",
            vec![
                "pub struct JsonlCommandTransport",
                "pub async fn run_stream",
            ],
        ),
        SymbolRequirement(
            "OneShot",
            "crates/agent-adapter-transports/src/oneshot_command.rs",
            vec![
                "pub struct OneShotCommandTransport",
                "pub async fn run_stream",
            ],
        ),
        SymbolRequirement(
            "PtyFallback",
            "crates/agent-adapter-transports/src/pty_fallback.rs",
            vec!["pub struct PtyFallbackTransport", "pub async fn run_stream"],
        ),
    ];

    for req in transport_requirements {
        if content.contains(req.0) {
            let full_path = root_dir.join(req.1);
            if !full_path.exists() {
                errors.push(TraceabilityError {
                    claim: format!("Transport {} mentioned in design", req.0),
                    error: format!("{} does not exist", req.1),
                });
            } else {
                let file_content = match std::fs::read_to_string(&full_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                for symbol in req.2 {
                    if !file_content.contains(symbol) {
                        errors.push(TraceabilityError {
                            claim: format!("Symbol '{}' for transport {} not found", symbol, req.0),
                            error: format!(
                                "File {} exists but required symbol '{}' not found",
                                req.1, symbol
                            ),
                        });
                    }
                }
            }
        }
    }

    // Reject nonexistent evidence file paths that design.md mentions
    let nonexistent_evidence = vec![(
        "agent-adapter-transports/src/mappers.rs",
        content.contains("agent-adapter-transports/src/mappers.rs"),
    )];
    for (path, mentioned) in nonexistent_evidence {
        if mentioned {
            let full_path = root_dir.join(path);
            if !full_path.exists() {
                errors.push(TraceabilityError {
                    claim: format!("Evidence file {} referenced", path),
                    error: format!("{} does not exist (renamed or removed)", path),
                });
            }
        }
    }

    errors
}
/// Check provider claims with symbol evidence.
fn check_provider_claims_with_symbols(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    let provider_requirements = vec![
        SymbolRequirement(
            "Codex",
            "crates/agent-adapter-providers/src/codex_adapter.rs",
            vec![
                "pub struct CodexAdapter",
                "impl AgentAdapter for CodexAdapter",
            ],
        ),
        SymbolRequirement(
            "Claude",
            "crates/agent-adapter-providers/src/claude_adapter.rs",
            vec![
                "pub struct ClaudeAdapter",
                "impl AgentAdapter for ClaudeAdapter",
            ],
        ),
        SymbolRequirement(
            "Gemini",
            "crates/agent-adapter-providers/src/gemini_adapter.rs",
            vec![
                "pub struct GeminiAdapter",
                "impl AgentAdapter for GeminiAdapter",
            ],
        ),
        SymbolRequirement(
            "Kimi",
            "crates/agent-adapter-providers/src/kimi_adapter.rs",
            vec![
                "pub struct KimiAdapter",
                "impl AgentAdapter for KimiAdapter",
            ],
        ),
        SymbolRequirement(
            "OpenCode",
            "crates/agent-adapter-providers/src/opencode_adapter.rs",
            vec![
                "pub struct OpenCodeAdapter",
                "impl AgentAdapter for OpenCodeAdapter",
            ],
        ),
        SymbolRequirement(
            "Hermes",
            "crates/agent-adapter-providers/src/hermes_adapter.rs",
            vec![
                "pub struct HermesAdapter",
                "impl AgentAdapter for HermesAdapter",
            ],
        ),
        SymbolRequirement(
            "Forge",
            "crates/agent-adapter-providers/src/forge_adapter.rs",
            vec![
                "pub struct ForgeAdapter",
                "impl AgentAdapter for ForgeAdapter",
            ],
        ),
        SymbolRequirement(
            "Claw",
            "crates/agent-adapter-providers/src/claw_adapter.rs",
            vec![
                "pub struct ClawAdapter",
                "impl AgentAdapter for ClawAdapter",
            ],
        ),
    ];

    for req in provider_requirements {
        // Match bare provider name, not "CodexProvider" etc
        if content.contains(&format!(" {} ", req.0))
            || content.contains(&format!("\n{}", req.0))
            || content.contains(&format!("{}Provider", req.0))
        {
            // Only check if the bare provider name appears (not "CodexProvider")
            let full_path = root_dir.join(req.1);
            if !full_path.exists() {
                errors.push(TraceabilityError {
                    claim: format!("Provider {} mentioned in design", req.0),
                    error: format!("{} does not exist", req.1),
                });
            } else {
                let file_content = match std::fs::read_to_string(&full_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                for symbol in req.2 {
                    if !file_content.contains(symbol) {
                        errors.push(TraceabilityError {
                            claim: format!("Symbol '{}' for provider {} not found", symbol, req.0),
                            error: format!(
                                "File {} exists but required symbol '{}' not found",
                                req.1, symbol
                            ),
                        });
                    }
                }
            }
        }
    }

    errors
}
/// Check workspace claims (WorkspaceRef in types.rs).
fn check_workspace_ref_claims(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    if content.contains("WorkspaceRef") || content.contains("workspace") {
        let path = root_dir.join("crates/agent-adapter-core/src/types.rs");
        if !path.exists() {
            errors.push(TraceabilityError {
                claim: "Workspace functionality".to_string(),
                error: "types.rs not found for WorkspaceRef".to_string(),
            });
        } else {
            let file_content = std::fs::read_to_string(&path).unwrap_or_default();
            if !file_content.contains("WorkspaceRef") {
                errors.push(TraceabilityError {
                    claim: "WorkspaceRef in types.rs".to_string(),
                    error: "WorkspaceRef not found in types.rs".to_string(),
                });
            }
        }
    }

    errors
}
/// Check lifecycle event ownership claims.
fn check_lifecycle_claims(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    if content.contains("RunStarted") || content.contains("lifecycle") {
        let path = root_dir.join("crates/agent-adapter-core/src/events.rs");
        if path.exists() {
            let file_content = std::fs::read_to_string(&path).unwrap_or_default();
            for event in ["RunStarted", "RunCompleted", "RunFailed", "RunCancelled"] {
                if content.contains(event) && !file_content.contains(event) {
                    errors.push(TraceabilityError {
                        claim: format!("Lifecycle event {} claimed", event),
                        error: format!("{} not found in events.rs", event),
                    });
                }
            }
        }
    }

    errors
}
/// Check router claims.
fn check_router_claims(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    if content.contains("AgentRouter") || content.contains("capability enforcement") {
        let router_path = root_dir.join("crates/agent-adapter-router/src/router.rs");
        if !router_path.exists() {
            errors.push(TraceabilityError {
                claim: "Router with capability enforcement".to_string(),
                error: "crates/agent-adapter-router/src/router.rs does not exist".to_string(),
            });
        } else {
            let file_content = std::fs::read_to_string(&router_path).unwrap_or_default();
            if !file_content.contains("impl AgentRouter")
                && !file_content.contains("pub struct AgentRouter")
            {
                errors.push(TraceabilityError {
                    claim: "AgentRouter struct/impl".to_string(),
                    error: "AgentRouter not properly defined in router.rs".to_string(),
                });
            }
            if !file_content.contains("capabilities()") && !file_content.contains("capabilities") {
                errors.push(TraceabilityError {
                    claim: "Router capability enforcement".to_string(),
                    error: "Router capability enforcement not found in router.rs".to_string(),
                });
            }
        }
    }

    errors
}
/// Check workspace path claims (always checked).
/// Note: Only validates that referenced workspace paths exist; does NOT enforce
/// a specific canonical location since workspace helpers may legitimately live
/// in different crates.
fn check_workspace_claims(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    // Check workspace paths mentioned in design.md exist somewhere in the workspace
    // This catches typos and completely nonexistent paths but allows legitimate
    // alternative locations (e.g., temp_workspace.rs in testkit vs transports)
    let workspace_path_mentions = vec!["temp_workspace.rs"];
    for path_mention in workspace_path_mentions {
        if content.contains(path_mention) {
            // Find the full path by searching for a pattern like "src/some_module/temp_workspace.rs"
            // and verify it exists somewhere under root
            if content.contains(path_mention) {
                // Check if ANY path with this filename exists in the workspace
                let mut found = false;
                for entry in walkdir::WalkDir::new(root_dir)
                    .max_depth(4)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_name().to_string_lossy().contains(path_mention) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    errors.push(TraceabilityError {
                        claim: format!("Workspace helper {} referenced in design", path_mention),
                        error: format!("{} not found anywhere in workspace", path_mention),
                    });
                }
            }
        }
    }

    errors
}
/// Check real provider gate claims (always checked, not conditional on router).
fn check_real_provider_gate_claims(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    // Reject unsupported DANGEROUS_TEST_MODE gate enforcement claim
    // The real_provider_gates.rs only enforces RUN_REAL_* vars, not DANGEROUS_TEST_MODE
    if content.contains("DANGEROUS_TEST_MODE") {
        let gates_path = root_dir.join("crates/agent-adapter-testkit/src/real_provider_gates.rs");
        let gates_content = std::fs::read_to_string(&gates_path).unwrap_or_default();
        if !gates_content.contains("DANGEROUS_TEST_MODE") {
            errors.push(TraceabilityError {
                claim: "DANGEROUS_TEST_MODE gate enforcement".to_string(),
                error: "design.md claims DANGEROUS_TEST_MODE gate but real_provider_gates.rs does not enforce it (only RUN_REAL_* vars are checked)".to_string(),
            });
        }
    }

    errors
}
/// Detect stale task lists (tasks-8*.md, tasks-9.md) remaining in root docs instead of archived.
/// Acceptance requires these to be moved to docs/archive/.
/// Check config schema claims.
fn check_config_claims(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    if content.contains("ConfigLoader")
        || content.contains("TOML schema")
        || content.contains("ProviderConfig")
    {
        let config_path = root_dir.join("crates/agent-adapter-config/src/loader.rs");
        let types_path = root_dir.join("crates/agent-adapter-config/src/types.rs");
        if !config_path.exists() && !types_path.exists() {
            errors.push(TraceabilityError {
                claim: "Config schema".to_string(),
                error: "agent-adapter-config loader/types not found".to_string(),
            });
        }
    }

    // Detect stale config key "prompt" (renamed to prompt_mode in ProviderConfig)
    if content.contains("prompt = ") && !content.contains("prompt_mode") {
        errors.push(TraceabilityError {
            claim: "Stale config key 'prompt'".to_string(),
            error: "Config uses stale 'prompt' key; should be 'prompt_mode' (PromptMode enum)"
                .to_string(),
        });
    }

    errors
}
/// Check security claims.
fn check_security_claims(content: &str, _root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    // Check for dangerous-default and safe mode claims
    if content.contains("dangerous-default") || content.contains("DangerousDefault") {
        let has_dangerous =
            content.contains("DangerousDefault") || content.contains("approval_policy");
        if !has_dangerous {
            errors.push(TraceabilityError {
                claim: "Dangerous default security policy".to_string(),
                error: "Design claims dangerous-default but implementation not found".to_string(),
            });
        }
    }

    if (content.contains("--safe") || content.contains("safe mode"))
        && !content.contains("ApprovalPolicy")
        && !content.contains("approval_policy")
    {
        errors.push(TraceabilityError {
            claim: "Safe mode via --safe flag".to_string(),
            error: "Design claims --safe but ApprovalPolicy not found in design.md".to_string(),
        });
    }

    errors
}
/// Check health check claims.
fn check_health_claims(content: &str, root_dir: &Path) -> Vec<TraceabilityError> {
    let mut errors = Vec::new();

    if content.contains("healthcheck") || content.contains("AdapterHealth") {
        let health_path = root_dir.join("crates/agent-adapter-providers/src/healthcheck.rs");
        if !health_path.exists() {
            errors.push(TraceabilityError {
                claim: "Health check implementation".to_string(),
                error: "agent-adapter-providers/src/healthcheck.rs not found".to_string(),
            });
        }
    }

    errors
}
/// Validate a specific design claim has evidence.
pub fn validate_claim(claim: &str, root_dir: &Path) -> TraceabilityResult {
    let claim_lower = claim.to_lowercase();

    // Map claims to their expected locations with required symbols
    let claim_mappings: Vec<(&str, &str, Vec<&str>)> = vec![
        (
            "chatrequest",
            "crates/agent-adapter-core/src/request.rs",
            vec!["pub struct ChatRequest"],
        ),
        (
            "agentevent",
            "crates/agent-adapter-core/src/events.rs",
            vec!["pub enum AgentEvent"],
        ),
        (
            "agentadapter",
            "crates/agent-adapter-core/src/adapter.rs",
            vec!["pub trait AgentAdapter"],
        ),
        (
            "acp",
            "crates/agent-adapter-transports/src/acp_stdio.rs",
            vec!["AcpStdioTransport"],
        ),
        (
            "jsonl",
            "crates/agent-adapter-transports/src/jsonl_command.rs",
            vec!["JsonlCommandTransport"],
        ),
        (
            "one-shot",
            "crates/agent-adapter-transports/src/oneshot_command.rs",
            vec!["OneShotCommandTransport"],
        ),
        (
            "pty",
            "crates/agent-adapter-transports/src/pty_fallback.rs",
            vec!["PtyFallbackTransport"],
        ),
        (
            "codex",
            "crates/agent-adapter-providers/src/codex_adapter.rs",
            vec!["CodexAdapter"],
        ),
        (
            "claude",
            "crates/agent-adapter-providers/src/claude_adapter.rs",
            vec!["ClaudeAdapter"],
        ),
        (
            "gemini",
            "crates/agent-adapter-providers/src/gemini_adapter.rs",
            vec!["GeminiAdapter"],
        ),
        (
            "kimi",
            "crates/agent-adapter-providers/src/kimi_adapter.rs",
            vec!["KimiAdapter"],
        ),
        (
            "opencode",
            "crates/agent-adapter-providers/src/opencode_adapter.rs",
            vec!["OpenCodeAdapter"],
        ),
        (
            "hermes",
            "crates/agent-adapter-providers/src/hermes_adapter.rs",
            vec!["HermesAdapter"],
        ),
        (
            "forge",
            "crates/agent-adapter-providers/src/forge_adapter.rs",
            vec!["ForgeAdapter"],
        ),
        (
            "claw",
            "crates/agent-adapter-providers/src/claw_adapter.rs",
            vec!["ClawAdapter"],
        ),
        (
            "router",
            "crates/agent-adapter-router/src/router.rs",
            vec!["AgentRouter"],
        ),
        (
            "config",
            "crates/agent-adapter-config/src/loader.rs",
            vec!["ConfigLoader"],
        ),
        (
            "dangerousdefault",
            "crates/agent-adapter-core/src/request.rs",
            vec!["ApprovalPolicy"],
        ),
        (
            "approvalpolicy",
            "crates/agent-adapter-core/src/request.rs",
            vec!["ApprovalPolicy"],
        ),
        (
            "healthcheck",
            "crates/agent-adapter-providers/src/healthcheck.rs",
            vec!["check_binary"],
        ),
        (
            "workspace",
            "crates/agent-adapter-core/src/types.rs",
            vec!["WorkspaceRef"],
        ),
    ];

    for (keyword, expected_path, required_symbols) in claim_mappings {
        if claim_lower.contains(keyword) {
            let full_path = root_dir.join(expected_path);
            if !full_path.exists() {
                return TraceabilityResult::failure(vec![TraceabilityError {
                    claim: claim.to_string(),
                    error: format!("Expected file not found: {}", expected_path),
                }]);
            }
            // Check for required symbols
            let content = std::fs::read_to_string(&full_path).unwrap_or_default();
            for symbol in required_symbols {
                if !content.contains(symbol) {
                    return TraceabilityResult::failure(vec![TraceabilityError {
                        claim: format!("Symbol '{}' not found in {}", symbol, expected_path),
                        error: format!("File exists but required symbol '{}' not found", symbol),
                    }]);
                }
            }
        }
    }

    TraceabilityResult::success()
}
