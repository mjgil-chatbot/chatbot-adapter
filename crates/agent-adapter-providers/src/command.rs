//! Command builder for provider-specific commands.
//!
//! This module provides the command building logic for each provider,
//! mapping ApprovalPolicy to appropriate CLI flags.

use agent_adapter_config::{ProviderConfig, ProviderKind};
use agent_adapter_core::{ApprovalPolicy, ChatRequest, ProviderId, ResumeMode};
use std::time::Duration;

/// Command spec returned by command builders.
///
/// This struct represents a fully populated command ready for execution
/// by a transport layer.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// The executable path or name.
    pub exe: String,
    /// Command-line arguments.
    pub args: Vec<String>,
    /// Environment variables to pass (allowlist only).
    pub env: std::collections::HashMap<String, String>,
    /// Working directory for the command.
    pub cwd: std::path::PathBuf,
    /// How to handle stdin.
    pub stdin_mode: StdinMode,
    /// Prompt file path (for StdinMode::File).
    pub prompt_file: Option<std::path::PathBuf>,
    /// Temporary files to clean up after execution.
    pub temp_files: Vec<std::path::PathBuf>,
    /// Timeout for the command.
    pub timeout: Option<Duration>,
    /// Process group mode - whether to kill process trees on cancel.
    pub process_group: ProcessGroupMode,
    /// Mapper identifier for event parsing.
    pub mapper_id: String,
    /// Transport kind for execution.
    pub transport_kind: TransportKind,
    /// Extra provider-specific arguments (unsafe mode).
    pub unsafe_args: Vec<String>,
    /// Extra provider-specific arguments (safe mode).
    pub safe_args: Vec<String>,
    /// Cleanup handles for deferred cleanup.
    pub cleanup_handles: Vec<CleanupHandle>,
    /// Provider-specific extras (arbitrary key-value pairs).
    pub extras: std::collections::HashMap<String, serde_json::Value>,
}

/// Cleanup handle for deferred cleanup of temp files/processes.
#[derive(Debug, Clone)]
pub struct CleanupHandle {
    /// Kind of cleanup needed.
    pub kind: CleanupKind,
    /// Path to clean up (if applicable).
    pub path: Option<std::path::PathBuf>,
    /// Process ID to kill (if applicable).
    pub pid: Option<u32>,
}

/// Kind of cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupKind {
    /// Remove a file.
    File,
    /// Remove a directory.
    Directory,
    /// Kill a process group.
    ProcessGroup,
    /// Remove a temporary workspace.
    TempWorkspace,
}

impl Default for CommandSpec {
    fn default() -> Self {
        Self {
            exe: String::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            cwd: std::path::PathBuf::from("."),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: None,
            process_group: ProcessGroupMode::KillTree,
            mapper_id: String::new(),
            transport_kind: TransportKind::JsonlCommand,
            unsafe_args: Vec::new(),
            safe_args: Vec::new(),
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        }
    }
}

/// Process group mode for tree cleanup on cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessGroupMode {
    /// Kill entire process tree on cancel/timeout.
    KillTree,
    /// Only kill the main process.
    MainProcessOnly,
}

/// Transport kind for command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    /// JSONL command transport - streaming JSON output.
    JsonlCommand,
    /// One-shot command - collect all output then exit.
    OneShotCommand,
    /// ACP stdio transport - JSON-RPC over stdin/stdout.
    AcpStdio,
    /// PTY fallback for TUI-based providers.
    PtyFallback,
}

/// Convert ProviderKind to TransportKind.
pub fn provider_kind_to_transport(kind: &ProviderKind) -> TransportKind {
    match kind {
        ProviderKind::JsonlCommand => TransportKind::JsonlCommand,
        ProviderKind::OneShotCommand => TransportKind::OneShotCommand,
        ProviderKind::AcpStdio => TransportKind::AcpStdio,
        ProviderKind::PtyFallback => TransportKind::PtyFallback,
        ProviderKind::Custom => TransportKind::JsonlCommand,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdinMode {
    None,
    Prompt,
    File,
}

/// Build a command for a specific provider and policy.
pub fn build_command(
    provider: ProviderId,
    policy: ApprovalPolicy,
    config: &ProviderConfig,
    request: &ChatRequest,
) -> Result<CommandSpec, BuildError> {
    match provider {
        ProviderId::Codex => codex::build(policy, config, request),
        ProviderId::Claude => claude::build(policy, config, request),
        ProviderId::MiniMax => claude::build(policy, config, request),
        ProviderId::Gemini => gemini::build(policy, config, request),
        ProviderId::Kimi => kimi::build(policy, config, request),
        ProviderId::OpenCode => opencode::build(policy, config, request),
        ProviderId::Hermes => hermes::build(policy, config, request),
        ProviderId::Forge => forge::build(policy, config, request),
        ProviderId::Claw => claw::build(policy, config, request),
        ProviderId::Custom(ref name) => custom::build(name, policy, config, request),
    }
}

#[derive(Debug)]
pub struct BuildError {
    pub message: String,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BuildError {}

mod codex {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec!["exec".to_string(), "--json".to_string()];

        // Add working directory
        args.push("--cd".to_string());
        args.push(request.cwd.to_string_lossy().to_string());

        // Add policy-specific flags
        match policy {
            ApprovalPolicy::DangerousDefault => {
                // Add yolo/bypass flag
                args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
            }
            ApprovalPolicy::Safe => {
                // Use safe defaults - ask for approval
                args.push("--sandbox".to_string());
                args.push("read-only".to_string());
                args.push("--ask-for-approval".to_string());
                args.push("on-request".to_string());
            }
        }

        // Add stdin marker for prompt
        args.push("-".to_string());

        let mut env = std::collections::HashMap::new();
        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        // Determine policy-specific args
        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::Prompt,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod claude {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec!["-p".to_string()];

        // Add prompt
        args.push(request.prompt.clone());

        // Claude requires verbose mode when stream-json is paired with print mode.
        args.push("--verbose".to_string());

        // Add output format
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());

        // Add input format
        args.push("--input-format".to_string());
        args.push("text".to_string());

        // Add policy-specific flags
        match policy {
            ApprovalPolicy::DangerousDefault => {
                args.push("--dangerously-skip-permissions".to_string());
            }
            ApprovalPolicy::Safe => {
                args.push("--permission-mode".to_string());
                args.push("default".to_string());
            }
        }

        // Add session/resume flags
        match &request.resume {
            ResumeMode::None => {}
            ResumeMode::ProviderDefault => {
                // Use provider's default resume behavior (no flag needed)
            }
            ResumeMode::External(ext) => {
                // Resume a specific external session
                args.push("--resume".to_string());
                args.push(ext.id.clone());
            }
            ResumeMode::LastInWorkspace => {
                // Resume last session in workspace
                args.push("--resume".to_string());
            }
        }

        // Add model if specified
        if let Some(ref model) = request.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if let Some(system_prompt) = request
            .extra
            .get("system_prompt")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            args.push("--append-system-prompt".to_string());
            args.push(system_prompt.to_string());
        }

        let mut env = std::collections::HashMap::new();
        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod gemini {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec!["-p".to_string()];

        // Add prompt
        args.push(request.prompt.clone());

        // Add output format
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());

        // Add policy-specific flags
        match policy {
            ApprovalPolicy::DangerousDefault => {
                args.push("--approval-mode".to_string());
                args.push("yolo".to_string());
            }
            ApprovalPolicy::Safe => {
                // No yolo flags - use normal approval
            }
        }

        let mut env = std::collections::HashMap::new();
        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod kimi {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec!["--print".to_string()];

        // Add prompt
        args.push("--prompt".to_string());
        args.push(request.prompt.clone());

        // Add output format
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());

        // Add policy-specific flags
        match policy {
            ApprovalPolicy::DangerousDefault => {
                // Note: --print implies --afk
                args.push("--yolo".to_string());
            }
            ApprovalPolicy::Safe => {
                // For safe mode, we should use ACP or avoid --print
                // since --print implies AFK behavior
                return Err(BuildError {
                    message: "Safe mode not fully supported for Kimi --print mode (implies AFK)"
                        .to_string(),
                });
            }
        }

        let mut env = std::collections::HashMap::new();
        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod opencode {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec!["run".to_string()];

        // Add format
        args.push("--format".to_string());
        args.push("json".to_string());

        // Add directory
        args.push("--dir".to_string());
        args.push(request.cwd.to_string_lossy().to_string());

        // Add policy-specific flags
        match policy {
            ApprovalPolicy::DangerousDefault => {
                // Use allow/default config (dangerous behavior is default for OpenCode)
            }
            ApprovalPolicy::Safe => {
                // Use safe args - ask for permission
                args.extend(config.safe_args.clone());
            }
        }

        // Add prompt
        args.push(request.prompt.clone());

        let mut env = std::collections::HashMap::new();

        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod hermes {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec![];

        // For one-shot mode with final text
        args.push("-z".to_string());

        // Add prompt
        args.push(request.prompt.clone());

        // Add policy-specific flags
        match policy {
            ApprovalPolicy::DangerousDefault => {
                args.push("--yolo".to_string());
            }
            ApprovalPolicy::Safe => {
                // No yolo flag
            }
        }

        // Add session/resume flags
        match &request.resume {
            ResumeMode::None => {}
            ResumeMode::ProviderDefault => {
                args.push("--continue".to_string());
            }
            ResumeMode::External(ext) => {
                args.push("--resume".to_string());
                args.push(ext.id.clone());
            }
            ResumeMode::LastInWorkspace => {
                args.push("--continue".to_string());
            }
        }

        let mut env = std::collections::HashMap::new();
        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod forge {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec![];

        match policy {
            ApprovalPolicy::DangerousDefault => {
                // Use unsafe_args from config
                if config.unsafe_args.is_empty() {
                    return Err(BuildError {
                        message: "Forge requires unsafe_args configured for dangerous mode"
                            .to_string(),
                    });
                }
                args.extend(config.unsafe_args.clone());
            }
            ApprovalPolicy::Safe => {
                // Use safe args or default -p
                if !config.safe_args.is_empty() {
                    args.extend(config.safe_args.clone());
                } else {
                    args.push("-p".to_string());
                    args.push(request.prompt.clone());
                }
            }
        }

        let mut env = std::collections::HashMap::new();
        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod claw {
    use super::*;

    pub fn build(
        policy: ApprovalPolicy,
        config: &ProviderConfig,
        request: &ChatRequest,
    ) -> Result<CommandSpec, BuildError> {
        let exe = config.exe.clone();
        let mut args = vec![];

        match policy {
            ApprovalPolicy::DangerousDefault => {
                // Use unsafe_args from config
                if config.unsafe_args.is_empty() {
                    return Err(BuildError {
                        message: "Claw requires unsafe_args configured for dangerous mode"
                            .to_string(),
                    });
                }
                args.extend(config.unsafe_args.clone());
            }
            ApprovalPolicy::Safe => {
                // Use safe args
                if config.safe_args.is_empty() {
                    return Err(BuildError {
                        message: "Claw requires safe_args configured for safe mode".to_string(),
                    });
                }
                args.extend(config.safe_args.clone());
            }
        }

        let mut env = std::collections::HashMap::new();
        for key in &config.env_allowlist {
            if let Some(val) = request.env.get(key) {
                env.insert(key.clone(), val.clone());
            }
        }

        let (unsafe_args, safe_args) = match policy {
            ApprovalPolicy::DangerousDefault => (config.unsafe_args.clone(), Vec::new()),
            ApprovalPolicy::Safe => (Vec::new(), config.safe_args.clone()),
        };

        Ok(CommandSpec {
            exe,
            args,
            env,
            cwd: request.cwd.clone(),
            stdin_mode: StdinMode::None,
            prompt_file: None,
            temp_files: Vec::new(),
            timeout: request.timeout_ms.map(Duration::from_millis),
            process_group: ProcessGroupMode::KillTree,
            mapper_id: config.mapper.clone(),
            transport_kind: provider_kind_to_transport(&config.kind),
            unsafe_args,
            safe_args,
            cleanup_handles: Vec::new(),
            extras: std::collections::HashMap::new(),
        })
    }
}

mod custom;

#[cfg(test)]
mod tests;
