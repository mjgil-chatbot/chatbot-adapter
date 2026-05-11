//! Providers command implementation.

use agent_adapter_core::AdapterCapabilities;
use anyhow::Context;
use serde::Serialize;
#[derive(Serialize)]
struct ProviderInfo {
    name: String,
    display_name: String,
    docs_path: Option<String>,
    installed: bool,
    version: Option<String>,
    safe_mode_supported: bool,
    unsafe_mode_supported: bool,
    requires_explicit_unsafe_args: bool,
    supported_transports: Vec<String>,
    capabilities: AdapterCapabilities,
    health_status: String,
    health_detail: String,
}
#[derive(Serialize)]
struct ProvidersOutput {
    providers: Vec<ProviderInfo>,
    total: usize,
    builtin_count: usize,
    custom_count: usize,
}
/// Get display name for a provider from registry metadata.
fn provider_metadata_display(name: &str) -> String {
    let metadata = [
        ("claude", "Anthropic Claude Code CLI"),
        ("codex", "OpenAI Codex CLI"),
        ("gemini", "Google Gemini CLI"),
        ("kimi", "Moonshot Kimi CLI"),
        ("opencode", "OpenCode CLI"),
        ("hermes", "Hermes Agent"),
        ("forge", "ForgeCode CLI"),
        ("claw", "Claw Code CLI"),
    ];
    metadata
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, d)| d)
        .unwrap_or(&name)
        .to_string()
}
