//! CLI binary for agent-adapter.

use clap::Parser;
use std::path::PathBuf;

use agent_adapter_core::{
    AgentEvent, ApprovalPolicy, ChatRequest, EventStream, ExternalSessionRef, ProviderId,
    ResumeMode, Role, DANGEROUS_DEFAULT_ACTIVE, SAFE_MODE_ACTIVE,
};

mod acceptance;
mod doctor;
mod health;
mod providers;
mod run;

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

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let exit_code = match args.command {
        Command::Run(run_args) => run::run_command(*run_args),
        Command::Healthcheck(hc_args) => health::healthcheck_command(hc_args),
        Command::Providers(providers_args) => providers::providers_command(providers_args),
        Command::Doctor(doctor_args) => doctor::doctor_command(doctor_args),
        Command::Acceptance(acceptance_args) => acceptance::acceptance_command(acceptance_args),
    };

    let code = match exit_code {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
            if let Some(cli_err) = e.downcast_ref::<CliExitError>() {
                cli_err.exit_code
            } else if let Some(adapter_err) = e.downcast_ref::<agent_adapter_core::AdapterError>() {
                adapter_err.exit_code()
            } else {
                1
            }
        }
    };
    std::process::exit(code);
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
    }
}

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

/// Print events from a stream and track terminal state.
/// Returns Ok(()) if run completed successfully, Err with exit code otherwise.
pub async fn print_events(stream: EventStream) -> anyhow::Result<()> {
    use futures_util::StreamExt;

    let mut stream = stream;
    let mut assistant_text = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                println!("{}", serde_json::to_string(&event)?);
                if let AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                } = &event
                {
                    assistant_text.push_str(text);
                }
                if event.is_terminal() {
                    if let AgentEvent::RunFailed { code, message } = event {
                        let exit_code = code.unwrap_or(1);
                        let message =
                            terminal_failure_message(&message, &assistant_text, exit_code);
                        return Err(CliExitError::new(exit_code, message).into());
                    }
                    return Ok(());
                }
            }
            Err(e) => {
                // Stream error - propagate with exit code info
                eprintln!("Error: {}", e);
                return Err(e.into());
            }
        }
    }

    Err(CliExitError::new(1, "Stream ended without terminal event").into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_adapter_core::WorkspaceRef;
    use uuid::Uuid;

    #[test]
    fn test_parse_provider() {
        assert_eq!(parse_provider("codex"), ProviderId::Codex);
        assert_eq!(parse_provider("CLAUDE"), ProviderId::Claude);
        assert_eq!(parse_provider("MINIMAX"), ProviderId::MiniMax);
        assert_eq!(
            parse_provider("custom-provider"),
            ProviderId::Custom("custom-provider".to_string())
        );
    }

    #[test]
    fn test_approval_policy_from_safe_flag() {
        let safe_args = RunArgs {
            provider: "claude".to_string(),
            safe: true,
            prompt: "hello".to_string(),
            system_prompt: None,
            cwd: ".".to_string(),
            timeout: None,
            model: None,
            config: None,
            no_env: false,
            sandbox_dirs: Vec::new(),
            json: false,
            resume: false,
            session_id: None,
        };
        let policy = if safe_args.safe {
            ApprovalPolicy::Safe
        } else {
            ApprovalPolicy::DangerousDefault
        };
        assert!(policy.is_safe());

        let unsafe_args = RunArgs {
            safe: false,
            ..safe_args
        };
        let policy = if unsafe_args.safe {
            ApprovalPolicy::Safe
        } else {
            ApprovalPolicy::DangerousDefault
        };
        assert!(policy.is_dangerous());
    }

    #[test]
    fn test_dangerous_default_mode_string() {
        assert_eq!(
            ApprovalPolicy::DangerousDefault.as_mode_string(),
            DANGEROUS_DEFAULT_ACTIVE
        );
        assert_eq!(ApprovalPolicy::Safe.as_mode_string(), SAFE_MODE_ACTIVE);
    }

    #[test]
    fn test_terminal_failure_message_prefers_assistant_text_for_generic_failure() {
        let message = terminal_failure_message(
            "claude exited with code Some(1)",
            "Your organization does not have access to Claude.",
            1,
        );
        assert_eq!(message, "Your organization does not have access to Claude.");
    }

    #[test]
    fn test_terminal_failure_message_preserves_specific_failure_message() {
        let message = terminal_failure_message(
            "provider rejected request: invalid token",
            "ignored assistant text",
            3,
        );
        assert_eq!(message, "provider rejected request: invalid token");
    }

    #[tokio::test]
    async fn test_print_events_returns_cli_exit_error_with_terminal_message() {
        let stream: EventStream = Box::pin(futures_util::stream::iter(vec![
            Ok(AgentEvent::RunStarted {
                run_id: Uuid::new_v4(),
                provider: ProviderId::MiniMax,
                external_session: None,
                approval_policy: DANGEROUS_DEFAULT_ACTIVE.to_string(),
                workspace: WorkspaceRef::Direct {
                    path: PathBuf::from("."),
                },
            }),
            Ok(AgentEvent::TextDelta {
                role: Role::Assistant,
                text: "Your organization does not have access to Claude.".to_string(),
            }),
            Ok(AgentEvent::RunFailed {
                code: Some(1),
                message: "claude exited with code Some(1)".to_string(),
            }),
        ]));

        let err = print_events(stream).await.expect_err("run should fail");
        let cli_err = err
            .downcast_ref::<CliExitError>()
            .expect("cli exit error should be preserved");
        assert_eq!(cli_err.exit_code, 1);
        assert_eq!(
            cli_err.message,
            "Your organization does not have access to Claude."
        );
    }
}
