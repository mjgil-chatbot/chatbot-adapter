//! Codex provider adapter.

use async_trait::async_trait;
use agent_adapter_core::AdapterCapabilities;
use agent_adapter_core::AdapterError;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ProviderId;
use agent_adapter_core::ResumeMode;
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
    }}
impl CodexAdapter {
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
    }}
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
