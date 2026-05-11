//! Capability definitions for adapters.

use serde::{Deserialize, Serialize};

/// Capabilities that an adapter may support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    /// Provider supports image inputs.
    pub images: bool,

    /// Provider supports multimodal inputs.
    pub multimodal: bool,

    /// Provider supports structured JSONL output.
    pub jsonl: bool,

    /// Provider supports ACP protocol.
    pub acp: bool,

    /// Provider supports session resumption.
    pub sessions: bool,

    /// Provider supports tool calls.
    pub tools: bool,

    /// Provider supports permission requests.
    pub permissions: bool,

    /// Provider supports usage/cost reporting.
    pub usage: bool,

    /// Provider supports streaming deltas.
    pub streaming: bool,

    /// Provider supports one-shot (non-interactive) mode.
    pub oneshot: bool,

    /// Provider supports PTY mode.
    pub pty: bool,
}

impl Default for AdapterCapabilities {
    fn default() -> Self {
        Self {
            images: false,
            multimodal: false,
            jsonl: false,
            acp: false,
            sessions: false,
            tools: true,
            permissions: true,
            usage: true,
            streaming: true,
            oneshot: true,
            pty: true,
        }
    }
}

impl AdapterCapabilities {
    /// Returns true if the provider supports a given capability.
    pub fn supports(&self, capability: &str) -> bool {
        match capability {
            "images" => self.images,
            "multimodal" => self.multimodal,
            "jsonl" => self.jsonl,
            "acp" => self.acp,
            "sessions" => self.sessions,
            "tools" => self.tools,
            "permissions" => self.permissions,
            "usage" => self.usage,
            "streaming" => self.streaming,
            "oneshot" => self.oneshot,
            "pty" => self.pty,
            _ => false,
        }
    }
}

/// Health status of an adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AdapterHealth {
    /// Adapter is ready to use.
    Ready { version: Option<String> },

    /// Adapter binary is not installed.
    MissingBinary { install_hint: String },

    /// Adapter needs authentication.
    NeedsAuth { login_hint: String },

    /// Adapter is misconfigured.
    Misconfigured { message: String },

    /// Adapter is unsupported (e.g., unsafe mode not available).
    Unsupported { reason: String },
}

impl AdapterHealth {
    /// Returns true if the adapter is ready.
    pub fn is_ready(&self) -> bool {
        matches!(self, AdapterHealth::Ready { .. })
    }

    /// Returns true if the adapter is usable (ready or just needs auth).
    pub fn is_usable(&self) -> bool {
        matches!(
            self,
            AdapterHealth::Ready { .. } | AdapterHealth::NeedsAuth { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_default() {
        let caps = AdapterCapabilities::default();
        assert!(caps.tools);
        assert!(caps.permissions);
        assert!(caps.usage);
        assert!(caps.streaming);
        assert!(caps.oneshot);
        assert!(caps.pty);
    }

    #[test]
    fn test_capabilities_supports() {
        let caps = AdapterCapabilities::default();
        assert!(caps.supports("tools"));
        assert!(!caps.supports("images"));
        assert!(!caps.supports("unknown"));
    }

    #[test]
    fn test_health_is_ready() {
        assert!(AdapterHealth::Ready {
            version: Some("1.0".to_string())
        }
        .is_ready());
        assert!(!AdapterHealth::MissingBinary {
            install_hint: "Install it".to_string()
        }
        .is_ready());
    }

    #[test]
    fn test_health_is_usable() {
        assert!(AdapterHealth::Ready { version: None }.is_usable());
        assert!(AdapterHealth::NeedsAuth {
            login_hint: "Login".to_string()
        }
        .is_usable());
        assert!(!AdapterHealth::MissingBinary {
            install_hint: "Install".to_string()
        }
        .is_usable());
    }
}
