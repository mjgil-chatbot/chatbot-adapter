//! Providers crate for agent-adapter.

pub mod claude_adapter;
pub mod claw_adapter;
pub mod codex_adapter;
pub mod command;
pub mod config;
pub mod forge_adapter;
pub mod gemini_adapter;
pub mod healthcheck;
pub mod hermes_adapter;
pub mod kimi_adapter;
pub mod mappers;
pub use claude_adapter::ClaudeAdapter;
pub use claw_adapter::ClawAdapter;
pub use codex_adapter::CodexAdapter;
pub use command::{build_command, BuildError, CommandSpec};
pub use forge_adapter::ForgeAdapter;
pub use gemini_adapter::GeminiAdapter;
pub use healthcheck::{check_auth_status, check_binary, full_healthcheck, get_install_hint};
pub use hermes_adapter::HermesAdapter;
pub use kimi_adapter::KimiAdapter;
