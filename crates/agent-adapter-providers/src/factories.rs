//! Factory implementations for built-in providers.

use std::path::PathBuf;
use async_trait::async_trait;
/// Factory for Claude provider.
pub struct ClaudeFactory;
impl ClaudeFactory {
    pub fn new() -> Self {
        Self
    }}
/// Factory for MiniMax via the Claude-compatible transport.
pub struct MiniMaxFactory;
impl MiniMaxFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for MiniMaxFactory {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for ClaudeFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Codex provider.
pub struct CodexFactory;
impl CodexFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for CodexFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Gemini provider.
pub struct GeminiFactory;
impl GeminiFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for GeminiFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Kimi provider.
pub struct KimiFactory;
impl KimiFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for KimiFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for OpenCode provider.
pub struct OpenCodeFactory;
impl OpenCodeFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for OpenCodeFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Hermes provider.
pub struct HermesFactory;
impl HermesFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for HermesFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Forge provider.
pub struct ForgeFactory;
impl ForgeFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for ForgeFactory {
    fn default() -> Self {
        Self::new()
    }
}
/// Factory for Claw provider.
pub struct ClawFactory;
impl ClawFactory {
    pub fn new() -> Self {
        Self
    }}
impl Default for ClawFactory {
    fn default() -> Self {
        Self::new()
    }
}
