//! Claude provider adapter.

use async_trait::async_trait;

use agent_adapter_core::{
    apply_prompt_policy, AdapterCapabilities, AdapterError, AdapterHealth, AdapterResult,
    AgentAdapter, ChatRequest, EventStream, ProviderId,
};
use agent_adapter_transports::JsonlCommandTransport;

use crate::config::ProviderConfig;
use crate::{build_command, CommandSpec};

/// Claude provider adapter.
pub struct ClaudeAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
    provider_id: ProviderId,
}

impl ClaudeAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        Self::new_for_provider(config, ProviderId::Claude)
    }

    pub fn new_for_provider(config: ProviderConfig, provider_id: ProviderId) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
            provider_id,
        }
    }

    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, AdapterError> {
        build_command(
            self.provider_id.clone(),
            request.approval_policy,
            &self.config,
            request,
        )
        .map_err(|e| AdapterError::InvalidConfig { message: e.message })
    }

    async fn run_transport(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        let request = apply_prompt_policy(&request);
        let spec = self.build_spec(&request)?;

        let transport = JsonlCommandTransport::new_with_env(spec.exe, spec.args, spec.env);

        transport.run_stream(request).await
    }
}

#[async_trait]
impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> ProviderId {
        self.provider_id.clone()
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }

    async fn healthcheck(&self) -> AdapterResult<AdapterHealth> {
        super::check_binary(self.provider_id.clone(), &self.config.exe).await
    }

    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        self.run_transport(request).await
    }

    async fn cancel(&self, _run_id: uuid::Uuid) -> AdapterResult<()> {
        Ok(())
    }
}

impl From<super::BuildError> for AdapterError {
    fn from(e: super::BuildError) -> Self {
        AdapterError::InvalidConfig { message: e.message }
    }
}
