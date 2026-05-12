//! Golden JSONL fixtures for provider testing.
//!
//! These fixtures provide deterministic test data for verifying
//! provider event parsing and handling.

/// Claude JSONL fixture - streaming text and tool calls.
pub static CLAUDE_JSONL: &str = r#"{"type": "text_delta", "delta": {"text": "I'll read that file for you."}}
{"type": "text_delta", "delta": {"text": ""}}
{"type": "tool_call", "id": "tool_1", "name": "Read", "input": {"path": "/tmp/test.txt"}}
{"type": "tool_call_delta", "id": "tool_1", "delta": {"text": ""}}
{"type": "tool_call_finished", "id": "tool_1", "output": "Hello, World!"}
{"type": "text_delta", "delta": {"text": "The file contains: Hello, World!"}}
{"type": "usage", "input_tokens": 150, "output_tokens": 75, "cost_usd": 0.002}
{"type": "message_delta", "delta": {"text": ""}}
{"type": "content_block_stop", "type": "content_block"}
{"type": "message_stop", "type": "message"}"#;

/// Codex JSONL fixture.
pub static CODEX_JSONL: &str = r#"{"type": "text_delta", "delta": {"text": "Reading file..."}}
{"type": "tool_call", "id": "tool_1", "name": "Read", "input": {"path": "/tmp/test.txt"}}
{"type": "tool_call_finished", "id": "tool_1", "output": "File contents"}
{"type": "text_delta", "delta": {"text": "Done!"}}
{"type": "message_delta", "delta": {"text": ""}}
{"type": "message_stop", "type": "message"}"#;

/// Gemini JSONL fixture.
pub static GEMINI_JSONL: &str = r#"{"event": "text", "text": "I'll read that file."}
{"event": "tool_call", "id": "tool_1", "name": "Read", "input": {"path": "/tmp/test.txt"}}
{"event": "tool_result", "id": "tool_1", "output": "Hello"}
{"event": "text", "text": "Done reading."}
{"event": "done"}"#;

/// Kimi JSONL fixture (similar to Claude but different field names).
pub static KIMI_JSONL: &str = r#"{"type": "text_delta", "delta": {"text": "Hello"}}
{"type": "tool_started", "id": "tool_1", "name": "read_file", "input": {"path": "/tmp/test.txt"}}
{"type": "tool_delta", "id": "tool_1", "delta": {"text": ""}}
{"type": "tool_finished", "id": "tool_1", "output": "File content here"}
{"type": "text_delta", "delta": {"text": "Finished reading."}}
{"type": "message_stop"}"#;

/// OpenCode JSONL fixture.
pub static OPENCODE_JSONL: &str = r#"{"type": "assistant_delta", "delta": {"text": "Running command..."}}
{"type": "tool_started", "id": "tool_1", "name": "bash", "input": {"command": "ls -la"}}
{"type": "tool_finished", "id": "tool_1", "output": "total 8\ndrwxr-xr-x  4 user user 4096 May 11 12:00 ."}
{"type": "assistant_delta", "delta": {"text": "Command completed."}}
{"type": "message_stop"}"#;

/// Hermes JSONL fixture.
pub static HERMES_JSONL: &str = r#"{"type": "content", "text": "Starting task..."}
{"type": "tool", "id": "tool_1", "name": "read", "input": {}}
{"type": "result", "id": "tool_1", "output": "result"}
{"type": "content", "text": "Task complete."}
{"type": "done"}"#;

/// Forge JSONL fixture (plain text output).
pub static FORGE_TEXT: &str =
    "This is a plain text response from forge.\nIt doesn't emit JSONL.\nJust regular text.";

/// Claw custom fixture.
pub static CLAW_JSONL: &str = r#"{"type": "text_delta", "delta": {"text": "Processing..."}}
{"type": "tool", "id": "tool_1", "name": "custom_tool", "input": {}}
{"type": "tool_result", "id": "tool_1", "result": "ok"}
{"type": "text_delta", "delta": {"text": "Done."}}
{"type": "finish"}"#;

/// Malformed JSONL that should be handled gracefully.
pub static MALFORMED_JSONL: &str = r#"{"type": "text_delta", "delta": {"text": "Valid JSON"}}
this is not json at all
{"type": "text_delta", "delta": {"text": "More valid JSON"}}
{"type": "text_delta"}
{"type": "text_delta", "delta": {"text": "Final"}}"#;

/// Permission request fixture.
pub static PERMISSION_REQUEST_JSONL: &str = r#"{"type": "text_delta", "delta": {"text": "I need to run a command."}}
{"type": "permission_request", "id": "perm_1", "action": "bash", "risk": "high", "details": {"command": "rm -rf /tmp"}}"#;

/// Fixture metadata for programmatic access.
pub struct FixtureMeta {
    pub name: &'static str,
    pub provider: &'static str,
    pub data: &'static str,
}

pub fn all_fixtures() -> Vec<FixtureMeta> {
    vec![
        FixtureMeta {
            name: "claude",
            provider: "claude",
            data: CLAUDE_JSONL,
        },
        FixtureMeta {
            name: "codex",
            provider: "codex",
            data: CODEX_JSONL,
        },
        FixtureMeta {
            name: "gemini",
            provider: "gemini",
            data: GEMINI_JSONL,
        },
        FixtureMeta {
            name: "kimi",
            provider: "kimi",
            data: KIMI_JSONL,
        },
        FixtureMeta {
            name: "opencode",
            provider: "opencode",
            data: OPENCODE_JSONL,
        },
        FixtureMeta {
            name: "hermes",
            provider: "hermes",
            data: HERMES_JSONL,
        },
        FixtureMeta {
            name: "forge",
            provider: "forge",
            data: FORGE_TEXT,
        },
        FixtureMeta {
            name: "claw",
            provider: "claw",
            data: CLAW_JSONL,
        },
        FixtureMeta {
            name: "malformed",
            provider: "generic",
            data: MALFORMED_JSONL,
        },
        FixtureMeta {
            name: "permission_request",
            provider: "generic",
            data: PERMISSION_REQUEST_JSONL,
        },
    ]
}

pub fn fixture_by_name(name: &str) -> Option<&'static str> {
    all_fixtures()
        .into_iter()
        .find(|f| f.name == name)
        .map(|f| f.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_fixtures_count() {
        let fixtures = all_fixtures();
        assert_eq!(fixtures.len(), 10);
    }

    #[test]
    fn test_fixture_by_name() {
        assert!(fixture_by_name("claude").is_some());
        assert!(fixture_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_malformed_fixture_not_empty() {
        let fixture = fixture_by_name("malformed").unwrap();
        assert!(!fixture.is_empty());
    }

    #[test]
    fn test_each_fixture_is_valid_utf8() {
        for fixture in all_fixtures() {
            assert!(
                std::str::from_utf8(fixture.data.as_bytes()).is_ok(),
                "Fixture {} should be valid UTF-8",
                fixture.name
            );
        }
    }
}
