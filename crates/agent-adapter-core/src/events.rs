//! Canonical event model for agent adapters.
//!
//! All providers emit events through this unified interface.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use crate::types::{
    ExternalSessionRef, FileOperation, LogLevel, PlanItem, ProviderId, RiskLevel, Role,
    WorkspaceRef,
};
/// Agent events emitted by adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A run has started.
    RunStarted {
        run_id: Uuid,
        provider: ProviderId,
        external_session: Option<ExternalSessionRef>,
        approval_policy: String,
        workspace: WorkspaceRef,
    },

    /// Text delta from the agent.
    TextDelta { role: Role, text: String },

    /// Tool call started.
    ToolCallStarted {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// Tool call delta (streaming).
    ToolCallDelta {
        id: String,
        delta: serde_json::Value,
    },

    /// Tool call finished.
    ToolCallFinished {
        id: String,
        output: Option<String>,
        error: Option<String>,
    },

    /// File was changed.
    FileChange {
        path: PathBuf,
        operation: FileOperation,
    },

    /// Permission requested by the agent.
    PermissionRequest {
        id: String,
        action: String,
        risk: RiskLevel,
        details: serde_json::Value,
    },

    /// Plan update from the agent.
    PlanUpdate { items: Vec<PlanItem> },

    /// Usage statistics.
    Usage {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        cost_usd: Option<f64>,
    },

    /// Log message.
    Log { level: LogLevel, message: String },

    /// Run completed successfully.
    RunCompleted {
        final_text: Option<String>,
        external_session: Option<ExternalSessionRef>,
    },

    /// Run failed.
    RunFailed { code: Option<i32>, message: String },

    /// Run was cancelled.
    RunCancelled,

    /// Raw provider-specific event preserved for passthrough.
    RawProviderEvent {
        provider: String,
        payload: serde_json::Value,
    },
}
/// Final text event emitted when a run completes with full text in one shot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCompleted {
    pub final_text: Option<String>,
    pub external_session: Option<ExternalSessionRef>,
}
/// Run failed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunFailed {
    pub code: Option<i32>,
    pub message: String,
}
