//! Providers command implementation.

use anyhow::Context;
use serde::Serialize;
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
