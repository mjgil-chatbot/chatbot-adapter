//! Gemini provider adapter.

use async_trait::async_trait;

use agent_adapter_core::{
    apply_prompt_policy, AdapterCapabilities, AdapterError, AdapterHealth, AdapterResult,
    AgentAdapter, ChatRequest, EventStream, ProviderId,
};
use agent_adapter_transports::JsonlCommandTransport;

use crate::config::ProviderConfig;
use crate::{build_command, CommandSpec};

pub struct GeminiAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}

impl GeminiAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            jsonl: true,
            acp: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }

    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, AdapterError> {
        build_command(
            ProviderId::Gemini,
            request.approval_policy,
            &self.config,
            request,
        )
        .map_err(|e| AdapterError::InvalidConfig { message: e.message })
    }

    async fn run_transport(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        let request = apply_prompt_policy(&request);
        let spec = self.build_spec(&request)?;
        let transport = JsonlCommandTransport::new(spec.exe, spec.args);
        transport.run_stream(request).await
    }
}

#[async_trait]
impl AgentAdapter for GeminiAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::Gemini
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }

    async fn healthcheck(&self) -> AdapterResult<AdapterHealth> {
        super::check_binary(ProviderId::Gemini, &self.config.exe).await
    }

    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        self.run_transport(request).await
    }

    async fn cancel(&self, _run_id: uuid::Uuid) -> AdapterResult<()> {
        Ok(())
    }
}
