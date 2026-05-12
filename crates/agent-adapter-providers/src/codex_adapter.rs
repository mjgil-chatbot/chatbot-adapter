//! Codex provider adapter.

use async_trait::async_trait;

use agent_adapter_core::{
    apply_prompt_policy, AdapterCapabilities, AdapterError, AdapterHealth, AdapterResult,
    AgentAdapter, ChatRequest, EventStream, ProviderId, ResumeMode,
};
use agent_adapter_transports::JsonlCommandTransport;

use crate::command::StdinMode;
use crate::config::ProviderConfig;
use crate::{build_command, CommandSpec};

pub struct CodexAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}

impl CodexAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }

    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, AdapterError> {
        let mut spec = build_command(
            ProviderId::Codex,
            request.approval_policy,
            &self.config,
            request,
        )
        .map_err(|e| AdapterError::InvalidConfig { message: e.message })?;
        rewrite_resume_args(request, &mut spec);
        Ok(spec)
    }

    async fn run_transport(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        let request = apply_prompt_policy(&request);
        let spec = self.build_spec(&request)?;
        let prompt_stdin =
            matches!(spec.stdin_mode, StdinMode::Prompt).then(|| request.prompt.clone());
        let transport = JsonlCommandTransport::new_with_env(spec.exe, spec.args, spec.env)
            .with_prompt_stdin(prompt_stdin);
        transport.run_stream(request).await
    }
}

fn rewrite_resume_args(request: &ChatRequest, spec: &mut CommandSpec) {
    match &request.resume {
        ResumeMode::External(ext) => {
            spec.args = build_resume_args(request, Some(ext.id.as_str()), false);
        }
        ResumeMode::LastInWorkspace => {
            spec.args = build_resume_args(request, None, true);
        }
        ResumeMode::None | ResumeMode::ProviderDefault => {}
    }
}

fn build_resume_args(
    request: &ChatRequest,
    session_id: Option<&str>,
    use_last: bool,
) -> Vec<String> {
    let mut args = vec![
        "--cd".to_string(),
        request.cwd.to_string_lossy().to_string(),
    ];

    match request.approval_policy {
        agent_adapter_core::ApprovalPolicy::DangerousDefault => {
            args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        }
        agent_adapter_core::ApprovalPolicy::Safe => {
            args.push("--sandbox".to_string());
            args.push("read-only".to_string());
            args.push("--ask-for-approval".to_string());
            args.push("on-request".to_string());
        }
    }

    args.push("exec".to_string());
    args.push("resume".to_string());
    args.push("--json".to_string());

    if use_last {
        args.push("--last".to_string());
    } else if let Some(session_id) = session_id {
        args.push(session_id.to_string());
    }

    args.push("-".to_string());
    args
}

#[cfg(test)]
mod tests {
    use agent_adapter_config::{CwdBehavior, PromptMode, ProviderKind};
    use agent_adapter_core::{ApprovalPolicy, ExternalSessionRef};

    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            name: "codex".to_string(),
            kind: ProviderKind::JsonlCommand,
            exe: "codex".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            mapper: "codex".to_string(),
            unsafe_args: vec![],
            safe_args: vec![],
            env_allowlist: vec![],
            cwd_behavior: CwdBehavior::UseProvided,
            schema_version: None,
        }
    }

    #[test]
    fn build_spec_uses_codex_resume_subcommand_for_external_sessions() {
        let adapter = CodexAdapter::new(test_config());
        let mut request = ChatRequest::new("test", "/tmp/work", ProviderId::Codex);
        request.resume = ResumeMode::External(ExternalSessionRef {
            provider: ProviderId::Codex,
            id: "session-123".to_string(),
        });

        let spec = adapter
            .build_spec(&request)
            .expect("resume build should succeed");

        assert_eq!(
            spec.args,
            vec![
                "--cd",
                "/tmp/work",
                "--dangerously-bypass-approvals-and-sandbox",
                "exec",
                "resume",
                "--json",
                "session-123",
                "-",
            ]
        );
    }

    #[test]
    fn build_spec_uses_global_safe_flags_for_last_session_resume() {
        let adapter = CodexAdapter::new(test_config());
        let mut request = ChatRequest::new("test", "/tmp/work", ProviderId::Codex);
        request.approval_policy = ApprovalPolicy::Safe;
        request.resume = ResumeMode::LastInWorkspace;

        let spec = adapter
            .build_spec(&request)
            .expect("last-session resume should succeed");

        assert_eq!(
            spec.args,
            vec![
                "--cd",
                "/tmp/work",
                "--sandbox",
                "read-only",
                "--ask-for-approval",
                "on-request",
                "exec",
                "resume",
                "--json",
                "--last",
                "-",
            ]
        );
    }

    #[test]
    fn run_policy_wraps_codex_prompt_with_search_guidance() {
        let request = ChatRequest::new("locate the parser", "/tmp/work", ProviderId::Codex);

        let effective = apply_prompt_policy(&request);

        assert!(effective
            .prompt
            .contains("Prefer the `semble` shell command first"));
        assert!(effective
            .prompt
            .contains("Prefer the provider-facing `fff` MCP tools instead of `rg`"));
        assert!(effective.prompt.ends_with("locate the parser"));
    }
}

#[async_trait]
impl AgentAdapter for CodexAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::Codex
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }

    async fn healthcheck(&self) -> AdapterResult<AdapterHealth> {
        super::check_binary(ProviderId::Codex, &self.config.exe).await
    }

    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        self.run_transport(request).await
    }

    async fn cancel(&self, _run_id: uuid::Uuid) -> AdapterResult<()> {
        Ok(())
    }
}
