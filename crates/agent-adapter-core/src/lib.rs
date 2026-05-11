//! Core types for the agent adapter.
//!
//! This crate defines the canonical event model, request types, and adapter traits
//! used across all transports and providers.

mod adapter;
mod audit;
mod capabilities;
mod errors;
mod events;
mod policy;
mod request;

pub mod types;

pub use adapter::{AgentAdapter, EventMapper, EventStream};
pub use audit::AuditConfig;
pub use capabilities::{AdapterCapabilities, AdapterHealth};
pub use errors::{AdapterError, AdapterResult, UnsafeModeUnavailable};
pub use events::{AgentEvent, RunCompleted, RunFailed, DANGEROUS_DEFAULT_ACTIVE, SAFE_MODE_ACTIVE};
pub use policy::apply_prompt_policy;
pub use request::{ApprovalPolicy, ChatRequest};

// Re-export types from types module
pub use types::{
    ExternalSessionRef, FileOperation, LogLevel, PlanItem, ProviderId, ResumeMode, RiskLevel, Role,
    SandboxMode, WorkspaceRef,
};
