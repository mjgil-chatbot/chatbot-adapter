//! Hermes provider adapter.

use async_trait::async_trait;

use agent_adapter_core::{
    apply_prompt_policy, AdapterCapabilities, AdapterHealth, AdapterResult, AgentAdapter,
    ChatRequest, EventStream, ProviderId,
};
use agent_adapter_transports::OneShotCommandTransport;

use crate::config::ProviderConfig;
use crate::{build_command, BuildError, CommandSpec};

pub struct HermesAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}

impl HermesAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            acp: true,
            oneshot: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }

    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, BuildError> {
        build_command(
            ProviderId::Hermes,
            request.approval_policy,
            &self.config,
            request,
        )
    }

    async fn run_transport(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        let request = apply_prompt_policy(&request);
        let spec = self
            .build_spec(&request)
            .map_err(|e| agent_adapter_core::AdapterError::InvalidConfig { message: e.message })?;
        let transport = OneShotCommandTransport::new(spec.exe, spec.args);
        transport.run_stream(request).await
    }
}

#[async_trait]
impl AgentAdapter for HermesAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::Hermes
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }

    async fn healthcheck(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Hermes, &self.config.exe).await
    }

    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        self.run_transport(request).await
    }

    async fn cancel(&self, _run_id: uuid::Uuid) -> AdapterResult<()> {
        Ok(())
    }
}
