//! CLI binary for agent-adapter.

use clap::Parser;
use std::path::PathBuf;
use agent_adapter_core::ApprovalPolicy;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ExternalSessionRef;
use agent_adapter_core::ProviderId;
use agent_adapter_core::ResumeMode;
mod acceptance;
mod doctor;
mod health;
mod providers;
#[derive(Parser, Debug)]
#[command(name = "agent-adapter")]
#[command(version = "0.1.0")]
#[command(about = "Agent adapter for AI coding assistants")]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Parser, Debug)]
pub(crate) enum Command {
    Run(Box<RunArgs>),
    Healthcheck(HealthcheckArgs),
    Providers(ProvidersArgs),
    Doctor(DoctorArgs),
    /// Run repo-level acceptance validation against the workspace.
    Acceptance(acceptance::AcceptanceArgs),
}
#[derive(Parser, Debug)]
pub(crate) struct RunArgs {
    #[arg(long, help = "Provider to use (codex, claude, gemini, etc.)")]
    provider: String,

    #[arg(
        long,
        help = "Enable safe mode - requires explicit approval for dangerous operations"
    )]
    safe: bool,

    #[arg(long, help = "Prompt to send to the agent")]
    prompt: String,

    #[arg(long, help = "Provider-specific system prompt")]
    system_prompt: Option<String>,

    #[arg(long, default_value = ".", help = "Working directory")]
    cwd: String,

    #[arg(long, help = "Timeout in milliseconds")]
    timeout: Option<u64>,

    #[arg(long, help = "Model to use (provider-specific)")]
    model: Option<String>,

    #[arg(
        long,
        help = "Explicit config file path (skip env/config dir discovery)"
    )]
    config: Option<String>,

    #[arg(long, help = "Skip environment variable overrides")]
    no_env: bool,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Additional directories to include in sandbox context (comma-separated)"
    )]
    sandbox_dirs: Vec<String>,

    #[arg(long, help = "Output events as JSON lines")]
    json: bool,

    #[arg(long, help = "Resume a previous session")]
    resume: bool,

    #[arg(long, help = "Session ID to resume")]
    session_id: Option<String>,
}
#[derive(Parser, Debug)]
pub(crate) struct HealthcheckArgs {
    #[arg(long, help = "Provider to check")]
    provider: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
}
#[derive(Parser, Debug)]
pub(crate) struct DoctorArgs {
    #[arg(long, help = "Output JSON")]
    json: bool,
}
#[derive(Parser, Debug)]
pub(crate) struct ProvidersArgs {
    #[arg(long, help = "Output as JSON")]
    json: bool,
}
/// Parse provider string into ProviderId
pub fn parse_provider(s: &str) -> ProviderId {
    match s.to_lowercase().as_str() {
        "codex" => ProviderId::Codex,
        "claude" => ProviderId::Claude,
        "minimax" => ProviderId::MiniMax,
        "gemini" => ProviderId::Gemini,
        "kimi" => ProviderId::Kimi,
        "opencode" => ProviderId::OpenCode,
        "hermes" => ProviderId::Hermes,
        "forge" => ProviderId::Forge,
        "claw" => ProviderId::Claw,
        other => ProviderId::Custom(other.to_string()),
    }
}
/// Create a ChatRequest from run arguments
pub(crate) fn create_request(args: &RunArgs) -> anyhow::Result<ChatRequest> {
    let provider = parse_provider(&args.provider);
    let provider_for_request = provider.clone();
    let policy = if args.safe {
        ApprovalPolicy::Safe
    } else {
        ApprovalPolicy::DangerousDefault
    };

    let mut request =
        ChatRequest::new(&args.prompt, PathBuf::from(&args.cwd), provider_for_request);
    request.timeout_ms = args.timeout;
    request.model = args.model.clone();
    request.approval_policy = policy;
    if let Some(system_prompt) = args.system_prompt.as_ref() {
        request.extra = serde_json::json!({
            "system_prompt": system_prompt,
        });
    }

    // Handle sandbox directories (runsc workspace includes)
    for dir in &args.sandbox_dirs {
        request.files.push(PathBuf::from(dir));
    }

    // Handle session resume
    if args.resume {
        if let Some(session_id) = &args.session_id {
            request.resume = ResumeMode::External(ExternalSessionRef {
                id: session_id.clone(),
                provider: provider.clone(),
            });
        } else {
            request.resume = ResumeMode::LastInWorkspace;
        }
    }

    Ok(request)
}
#[derive(Debug)]
struct CliExitError {
    exit_code: i32,
    message: String,
}
impl CliExitError {
    fn new(exit_code: i32, message: impl Into<String>) -> Self {
        Self {
            exit_code,
            message: message.into(),
        }
    }}
impl std::fmt::Display for CliExitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CliExitError {}
fn is_generic_run_failed_message(message: &str) -> bool {
    let trimmed = message.trim();
    trimmed.is_empty()
        || trimmed.starts_with("Provider exited with code ")
        || trimmed.contains(" exited with code ")
        || trimmed.ends_with(" failed")
}
fn terminal_failure_message(
    run_failed_message: &str,
    assistant_text: &str,
    exit_code: i32,
) -> String {
    let assistant_text = assistant_text.trim();
    if !assistant_text.is_empty() && is_generic_run_failed_message(run_failed_message) {
        return assistant_text.to_string();
    }

    let trimmed = run_failed_message.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    format!("Provider exited with code {exit_code}")
}
fn main() {}
