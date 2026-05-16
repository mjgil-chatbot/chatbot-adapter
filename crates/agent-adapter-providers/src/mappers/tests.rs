use super::*;

#[test]
fn test_claude_mapper_text_delta() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "hello"}}"#);
    assert!(matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "hello"));
}

#[test]
fn test_claude_mapper_tool_call() {
    let mut mapper = ClaudeMapper::new();
    let events =
        mapper.map_line(r#"{"type": "tool_call", "id": "tool1", "name": "read", "input": {}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "tool1" && name == "read")
    );
}

#[test]
fn test_plain_text_mapper() {
    let mut mapper = PlainTextMapper::new();
    let events = mapper.map_line("hello world");
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "hello world")
    );
}

// =========================================================================
// Task 37: Provider-specific mappers - golden fixture tests
// =========================================================================

#[test]
fn test_claude_mapper_finish() {
    // Test that final text is accumulated and emitted on finish
    let mut mapper = ClaudeMapper::new();
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Hello "}}"#);
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "World"}}"#);
    let events = mapper.finish();
    assert!(
        matches!(events[0], Ok(AgentEvent::RunCompleted { final_text: Some(ref text), .. }) if text == "Hello World")
    );
}

#[test]
fn test_claude_mapper_raw_event() {
    // Test that unknown event types become RawProviderEvent
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(r#"{"type": "custom_event", "data": "value"}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "claude")
    );
}

#[test]
fn test_jsonl_mapper_text_delta() {
    let mut mapper = JsonlMapper::new("test_provider");
    let events = mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Hello"}}"#);
    assert!(matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "Hello"));
}

#[test]
fn test_jsonl_mapper_tool_call() {
    let mut mapper = JsonlMapper::new("test_provider");
    let events = mapper.map_line(
        r#"{"type": "tool_call", "id": "tool1", "name": "read", "input": {"file": "test.txt"}}"#,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "tool1" && name == "read")
    );
}

#[test]
fn test_jsonl_mapper_raw_event() {
    // Test that unknown event types become RawProviderEvent with correct provider
    let mut mapper = JsonlMapper::new("gemini");
    let events = mapper.map_line(r#"{"type": "unknown_type", "data": 123}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "gemini")
    );
}

#[test]
fn test_jsonl_mapper_finish() {
    // Test that final text is accumulated for generic JSONL mapper
    let mut mapper = JsonlMapper::new("codex");
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Codex "}}"#);
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Response"}}"#);
    let events = mapper.finish();
    assert!(
        matches!(events[0], Ok(AgentEvent::RunCompleted { final_text: Some(ref text), .. }) if text == "Codex Response")
    );
}

#[test]
fn test_plain_text_mapper_finish() {
    // Test that plain text final output is correctly emitted
    let mut mapper = PlainTextMapper::new();
    mapper.map_line("Line 1\n");
    mapper.map_line("Line 2");
    let events = mapper.finish();
    assert!(
        matches!(events[0], Ok(AgentEvent::RunCompleted { final_text: Some(ref text), .. }) if text.contains("Line 1") && text.contains("Line 2"))
    );
}

#[test]
fn test_mapper_stderr_becomes_log() {
    // Test that stderr lines are logged
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_stderr("Warning: something happened");
    assert!(
        matches!(events[0], Ok(AgentEvent::Log { level: LogLevel::Warn, ref message }) if message.contains("Warning"))
    );
}

#[test]
fn test_mapper_malformed_json_becomes_log() {
    // Test that malformed JSON is logged at debug level
    let mut mapper = JsonlMapper::new("test");
    let events = mapper.map_line("not valid json {");
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_mapper_empty_finish() {
    // Test that empty mappers produce empty finish
    let mapper = PlainTextMapper::new();
    let events = mapper.finish();
    assert!(events.is_empty());
}

#[test]
fn test_claude_mapper_tool_call_delta() {
    // Test tool call delta mapping
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "tool_call_delta", "id": "tool1", "delta": {"text": "partial input"}}"#,
    );
    assert!(matches!(events[0], Ok(AgentEvent::ToolCallDelta { ref id, .. }) if id == "tool1"));
}

#[test]
fn test_claude_mapper_tool_call_finished() {
    // Test tool call finished mapping
    let mut mapper = ClaudeMapper::new();
    let events =
        mapper.map_line(r#"{"type": "tool_call_finished", "id": "tool1", "output": "result"}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallFinished { ref id, output: Some(ref o), .. }) if id == "tool1" && o == "result")
    );
}

#[test]
fn test_claude_mapper_usage() {
    // Test usage event mapping
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "usage", "input_tokens": 100, "output_tokens": 50, "cost_usd": 0.01}"#,
    );
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Usage {
            input_tokens: Some(100),
            output_tokens: Some(50),
            cost_usd: Some(0.01)
        })
    ));
}

#[test]
fn test_claude_mapper_permission_request() {
    // Test permission request event mapping
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(r#"{"type": "permission_request", "id": "perm_1", "action": "bash", "risk": "high", "details": {"command": "rm -rf /tmp"}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::PermissionRequest { ref id, ref action, risk: agent_adapter_core::RiskLevel::High, .. }) if id == "perm_1" && action == "bash")
    );
}

#[test]
fn test_jsonl_mapper_assistant_delta() {
    // Test assistant_delta event type (alternative naming)
    let mut mapper = JsonlMapper::new("kimi");
    let events =
        mapper.map_line(r#"{"type": "assistant_delta", "delta": {"text": "Response text"}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "Response text")
    );
}

// =========================================================================
// Task 28: Provider-specific mapper tests (CodexMapper, HermesMapper, ClawMapper)
// =========================================================================

#[test]
fn test_codex_mapper_text_delta() {
    let mut mapper = CodexMapper::new();
    let events = mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Reading file..."}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "Reading file...")
    );
}

#[test]
fn test_codex_mapper_tool_call() {
    let mut mapper = CodexMapper::new();
    let events = mapper.map_line(r#"{"type": "tool_call", "id": "tool_1", "name": "Read", "input": {"path": "/tmp/test.txt"}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "tool_1" && name == "Read")
    );
}

#[test]
fn test_codex_mapper_tool_call_finished() {
    let mut mapper = CodexMapper::new();
    let events = mapper
        .map_line(r#"{"type": "tool_call_finished", "id": "tool_1", "output": "File contents"}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallFinished { ref id, output: Some(ref o), .. }) if id == "tool_1" && o == "File contents")
    );
}

#[test]
fn test_codex_mapper_finish() {
    let mut mapper = CodexMapper::new();
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Done!"}}"#);
    let events = mapper.finish();
    assert!(
        matches!(events[0], Ok(AgentEvent::RunCompleted { final_text: Some(ref text), .. }) if text == "Done!")
    );
}

#[test]
fn test_hermes_mapper_text() {
    let mut mapper = HermesMapper::new();
    let json_str = r#"{"type": "content", "text": "Starting task..."}"#;
    let events = mapper.map_line(json_str);
    assert!(!events.is_empty(), "Expected some events");
    assert!(
        matches!(&events[0], Ok(AgentEvent::TextDelta { text, .. }) if text == "Starting task...")
    );
}

#[test]
fn test_hermes_mapper_tool() {
    let mut mapper = HermesMapper::new();
    let events =
        mapper.map_line(r#"{"type": "tool", "id": "tool_1", "name": "read", "input": {}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "tool_1" && name == "read")
    );
}

#[test]
fn test_hermes_mapper_result() {
    let mut mapper = HermesMapper::new();
    let events = mapper.map_line(r#"{"type": "result", "id": "tool_1", "output": "result"}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallFinished { ref id, output: Some(ref o), .. }) if id == "tool_1" && o == "result")
    );
}

#[test]
fn test_hermes_mapper_done() {
    let mut mapper = HermesMapper::new();
    let json_str = r#"{"type": "content", "text": "Task complete."}"#;
    let map_events = mapper.map_line(json_str);
    let finish_events = mapper.finish();
    assert!(
        !finish_events.is_empty(),
        "Expected finish to produce events, got empty. map_line produced: {:?}",
        map_events
    );
    assert!(
        matches!(&finish_events[0], Ok(AgentEvent::RunCompleted { final_text: Some(text), .. }) if text == "Task complete.")
    );
}

#[test]
fn test_claw_mapper_text_delta() {
    let mut mapper = ClawMapper::new();
    let events = mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Processing..."}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "Processing...")
    );
}

#[test]
fn test_claw_mapper_tool() {
    let mut mapper = ClawMapper::new();
    let events =
        mapper.map_line(r#"{"type": "tool", "id": "tool_1", "name": "custom_tool", "input": {}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "tool_1" && name == "custom_tool")
    );
}

#[test]
fn test_claw_mapper_tool_result() {
    let mut mapper = ClawMapper::new();
    let events = mapper.map_line(r#"{"type": "tool_result", "id": "tool_1", "result": "ok"}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallFinished { ref id, output: Some(ref o), .. }) if id == "tool_1" && o == "ok")
    );
}

#[test]
fn test_claw_mapper_finish() {
    let mut mapper = ClawMapper::new();
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Done."}}"#);
    let events = mapper.finish();
    assert!(
        matches!(events[0], Ok(AgentEvent::RunCompleted { final_text: Some(ref text), .. }) if text == "Done.")
    );
}

#[test]
fn test_codex_mapper_raw_event() {
    let mut mapper = CodexMapper::new();
    let events = mapper.map_line(r#"{"type": "unknown_event", "data": "value"}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "codex")
    );
}

#[test]
fn test_hermes_mapper_raw_event() {
    let mut mapper = HermesMapper::new();
    let events = mapper.map_line(r#"{"type": "custom", "value": 123}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "hermes")
    );
}

#[test]
fn test_claw_mapper_raw_event() {
    let mut mapper = ClawMapper::new();
    let events = mapper.map_line(r#"{"type": "custom_event", "data": "value"}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "claw")
    );
}

// =========================================================================
// Task 38 & 39: Permission and FileChange fixture tests
// =========================================================================

#[test]
fn test_claude_mapper_permission_request_high_risk() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(r#"{"type": "permission_request", "id": "perm_1", "action": "bash", "risk": "high", "details": {"command": "rm -rf /tmp"}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::PermissionRequest { ref id, ref action, risk: agent_adapter_core::RiskLevel::High, .. }) if id == "perm_1" && action == "bash")
    );
}

#[test]
fn test_claude_mapper_permission_request_low_risk() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(r#"{"type": "permission_request", "id": "perm_2", "action": "read_file", "risk": "low", "details": {"path": "/tmp/test.txt"}}"#);
    assert!(
        matches!(events[0], Ok(AgentEvent::PermissionRequest { ref id, ref action, risk: agent_adapter_core::RiskLevel::Low, .. }) if id == "perm_2" && action == "read_file")
    );
}

#[test]
fn test_claude_mapper_file_change_create() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "file_change", "path": "/workspace/new.txt", "operation": "create"}"#,
    );
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Create }) if path.to_string_lossy() == "/workspace/new.txt")
    );
}

#[test]
fn test_claude_mapper_file_change_update() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "file_change", "path": "/workspace/existing.txt", "operation": "update"}"#,
    );
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Update }) if path.to_string_lossy() == "/workspace/existing.txt")
    );
}

#[test]
fn test_claude_mapper_file_change_delete() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "file_change", "path": "/workspace/old.txt", "operation": "delete"}"#,
    );
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Delete }) if path.to_string_lossy() == "/workspace/old.txt")
    );
}

#[test]
fn test_claude_mapper_file_change_rename() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "file_change", "path": "/workspace/renamed.txt", "operation": "rename"}"#,
    );
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Rename }) if path.to_string_lossy() == "/workspace/renamed.txt")
    );
}

#[test]
fn test_claude_mapper_file_change_copy() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "file_change", "path": "/workspace/copied.txt", "operation": "copy"}"#,
    );
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Copy }) if path.to_string_lossy() == "/workspace/copied.txt")
    );
}

#[test]
fn test_jsonl_mapper_file_change() {
    let mut mapper = JsonlMapper::new("gemini");
    let events = mapper
        .map_line(r#"{"type": "file_change", "path": "/app/main.py", "operation": "update"}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Update }) if path.to_string_lossy() == "/app/main.py")
    );
}

#[test]
fn test_jsonl_mapper_permission_request() {
    // JsonlMapper doesn't have built-in permission request support
    // but should preserve it as raw event
    let mut mapper = JsonlMapper::new("kimi");
    let events = mapper
        .map_line(r#"{"type": "permission_request", "id": "p1", "action": "run", "risk": "high"}"#);
    // This becomes a RawProviderEvent since JsonlMapper doesn't have permission_request
    assert!(
        matches!(&events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "kimi")
    );
}

#[test]
fn test_codex_mapper_file_change() {
    let mut mapper = CodexMapper::new();
    let events = mapper
        .map_line(r#"{"type": "file_change", "path": "/codex/file.txt", "operation": "create"}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Create }) if path.to_string_lossy() == "/codex/file.txt")
    );
}

#[test]
fn test_hermes_mapper_file_change() {
    let mut mapper = HermesMapper::new();
    let events = mapper
        .map_line(r#"{"type": "file_change", "path": "/hermes/data.json", "operation": "update"}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Update }) if path.to_string_lossy() == "/hermes/data.json")
    );
}

#[test]
fn test_claw_mapper_file_change() {
    let mut mapper = ClawMapper::new();
    let events = mapper
        .map_line(r#"{"type": "file_change", "path": "/claw/output.log", "operation": "create"}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::FileChange { path, operation: agent_adapter_core::FileOperation::Create }) if path.to_string_lossy() == "/claw/output.log")
    );
}

// =========================================================================
// Malformed payload tests
// =========================================================================

#[test]
fn test_claude_mapper_malformed_json() {
    let mut mapper = ClaudeMapper::new();
    // Malformed JSON should become a debug log
    let events = mapper.map_line("not valid json {");
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_jsonl_mapper_malformed_json() {
    let mut mapper = JsonlMapper::new("test");
    let events = mapper.map_line("malformed{{");
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_codex_mapper_malformed_json() {
    let mut mapper = CodexMapper::new();
    let events = mapper.map_line("{broken");
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_hermes_mapper_malformed_json() {
    let mut mapper = HermesMapper::new();
    let events = mapper.map_line("not json at all");
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_claw_mapper_malformed_json() {
    let mut mapper = ClawMapper::new();
    let events = mapper.map_line("definitely not json");
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

// =========================================================================
// Unknown event preservation tests
// =========================================================================

#[test]
fn test_claude_mapper_preserves_unknown_event() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(
        r#"{"type": "custom_event", "provider_data": "value", "nested": {"foo": "bar"}}"#,
    );
    let event = &events[0];
    assert!(
        matches!(event, Ok(AgentEvent::RawProviderEvent { provider, .. }) if provider == "claude")
    );
}

#[test]
fn test_jsonl_mapper_preserves_unknown_event() {
    let mut mapper = JsonlMapper::new("opencode");
    let events = mapper.map_line(r#"{"type": "unknown_type", "custom": "data"}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "opencode")
    );
}

#[test]
fn test_codex_mapper_preserves_unknown_event() {
    let mut mapper = CodexMapper::new();
    let events = mapper.map_line(r#"{"type": "codex_specific", "info": 123}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "codex")
    );
}

#[test]
fn test_hermes_mapper_preserves_unknown_event() {
    let mut mapper = HermesMapper::new();
    let events = mapper.map_line(r#"{"type": "hermes_internal", "data": {"nested": true}}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "hermes")
    );
}

#[test]
fn test_claw_mapper_preserves_unknown_event() {
    let mut mapper = ClawMapper::new();
    let events = mapper.map_line(r#"{"type": "claw_special", "value": 42}"#);
    assert!(
        matches!(&events[0], Ok(AgentEvent::RawProviderEvent { provider: ref p, .. }) if p == "claw")
    );
}

// =========================================================================
// Multiple events in sequence tests
// =========================================================================

#[test]
fn test_claude_mapper_multiple_events() {
    let mut mapper = ClaudeMapper::new();

    // Multiple text deltas
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": "Hello"}}"#);
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": " World"}}"#);

    // Tool call in between
    mapper.map_line(
        r#"{"type": "tool_call", "id": "tool1", "name": "bash", "input": {"command": "ls"}}"#,
    );

    // Permission request
    mapper.map_line(r#"{"type": "permission_request", "id": "perm1", "action": "bash", "risk": "high", "details": {}}"#);

    // File change
    mapper.map_line(
        r#"{"type": "file_change", "path": "/workspace/test.txt", "operation": "create"}"#,
    );

    // Final text delta
    mapper.map_line(r#"{"type": "text_delta", "delta": {"text": " Done!"}}"#);

    let finish_events = mapper.finish();
    // Should have accumulated final text and completed
    assert!(!finish_events.is_empty());
    if let Some(Ok(AgentEvent::RunCompleted {
        final_text: Some(text),
        ..
    })) = finish_events.first()
    {
        assert_eq!(text, "Hello World Done!");
    } else {
        panic!("Expected RunCompleted with final text");
    }
}

// =========================================================================
// Task 38: Unsupported provider permission handling
// =========================================================================

#[test]
fn test_plain_text_mapper_does_not_support_permission_requests() {
    // PlainTextMapper should not map permission requests - they become raw text
    let mut mapper = PlainTextMapper::new();
    let events = mapper.map_line(r#"{"type": "permission_request", "id": "p1", "action": "test"}"#);
    // Plain text mapper just treats everything as text
    assert!(!events.is_empty());
}

#[test]
fn test_plain_text_mapper_does_not_support_file_changes() {
    let mut mapper = PlainTextMapper::new();
    let events =
        mapper.map_line(r#"{"type": "file_change", "path": "/test", "operation": "create"}"#);
    // Plain text mapper treats JSON as literal text
    assert!(!events.is_empty());
}

// =========================================================================
// PlanUpdate event tests (if supported)
// =========================================================================

#[test]
fn test_claude_mapper_plan_update() {
    let mut mapper = ClaudeMapper::new();
    let events = mapper.map_line(r#"{"type": "plan_update", "items": [{"id": "1", "description": "Step 1", "status": "completed"}, {"id": "2", "description": "Step 2", "status": "in_progress"}]}"#);
    // Currently plan_update falls through to RawProviderEvent
    assert!(matches!(events[0], Ok(AgentEvent::RawProviderEvent { .. })));
}
