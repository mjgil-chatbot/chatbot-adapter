use super::*;
use agent_adapter_core::{ChatRequest, ProviderId};
use std::time::Duration;
use tokio_stream::StreamExt;

fn test_request() -> ChatRequest {
    ChatRequest::new("test prompt", ".", ProviderId::Claude)
}

fn map_line(
    provider: ProviderId,
    line: &str,
    final_text: &mut String,
) -> Vec<AdapterResult<AgentEvent>> {
    JsonlCommandTransport::map_line(&provider, line, final_text)
}

fn assert_canonical_tool_started(provider: ProviderId, line: &str) {
    let mut final_text = String::new();
    let events = map_line(provider.clone(), line, &mut final_text);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Ok(AgentEvent::ToolCallStarted { .. }))),
        "{provider} should emit AgentEvent::ToolCallStarted for line: {line}"
    );
}

fn assert_canonical_tool_finished(provider: ProviderId, line: &str) {
    let mut final_text = String::new();
    let events = map_line(provider.clone(), line, &mut final_text);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Ok(AgentEvent::ToolCallFinished { .. }))),
        "{provider} should emit AgentEvent::ToolCallFinished for line: {line}"
    );
}

#[test]
fn test_provider_supports_context_dirs_matches_cli_contracts() {
    assert!(!provider_supports_context_dirs(&ProviderId::Claude));
    assert!(!provider_supports_context_dirs(&ProviderId::MiniMax));
    assert!(!provider_supports_context_dirs(&ProviderId::Codex));
    assert!(!provider_supports_context_dirs(&ProviderId::Kimi));
    assert!(provider_supports_context_dirs(&ProviderId::Gemini));
}

#[test]
fn test_map_line_text_delta() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "text_delta", "delta": {"text": "hello"}}"#,
        &mut final_text,
    );
    assert!(matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "hello"));
    assert_eq!(final_text, "hello");
}

#[test]
fn test_map_line_tool_call() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "tool_call", "id": "tool1", "name": "read_file", "input": {}}"#,
        &mut final_text,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "tool1" && name == "read_file")
    );
}

#[test]
fn test_map_line_malformed_json() {
    let mut final_text = String::new();
    let events = map_line(ProviderId::Claude, "not json at all", &mut final_text);
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_map_line_unknown_event_type() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "custom_event", "data": "value"}"#,
        &mut final_text,
    );
    assert!(matches!(
        events[0],
        Ok(AgentEvent::RawProviderEvent { ref provider, .. }) if provider == "claude"
    ));
}

#[test]
fn test_map_line_accumulates_final_text() {
    let mut final_text = String::new();
    map_line(
        ProviderId::Claude,
        r#"{"type": "text_delta", "delta": {"text": "Hello "}}"#,
        &mut final_text,
    );
    map_line(
        ProviderId::Claude,
        r#"{"type": "text_delta", "delta": {"text": "World"}}"#,
        &mut final_text,
    );
    assert_eq!(final_text, "Hello World");
}

#[test]
fn test_map_line_assistant_message_extracts_text() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hello world"}]}}"#,
        &mut final_text,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "hello world")
    );
    assert_eq!(final_text, "hello world");
}

#[test]
fn test_map_line_top_level_assistant_content_extracts_text() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Kimi,
        r#"{"role":"assistant","content":[{"type":"think","think":"internal"},{"type":"text","text":"hello from kimi"}]}"#,
        &mut final_text,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "hello from kimi")
    );
    assert_eq!(final_text, "hello from kimi");
}

#[test]
fn test_map_line_codex_completed_agent_message_extracts_text() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"item.completed","item":{"id":"item_0","text":"Hello from codex","type":"agent_message"}}"#,
        &mut final_text,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "Hello from codex")
    );
    assert_eq!(final_text, "Hello from codex");
}

#[test]
fn test_map_line_codex_completed_agent_message_replaces_previous_intermediate_text() {
    let mut final_text = "intermediate update".to_string();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"item.completed","item":{"id":"item_1","text":"Final codex answer","type":"agent_message"}}"#,
        &mut final_text,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "Final codex answer")
    );
    assert_eq!(final_text, "Final codex answer");
}

#[test]
fn test_map_line_tool_call_delta() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "tool_call_delta", "id": "tool1", "delta": {"text": "partial"}}"#,
        &mut final_text,
    );
    assert!(matches!(events[0], Ok(AgentEvent::ToolCallDelta { ref id, .. }) if id == "tool1"));
}

#[test]
fn test_map_line_tool_call_finished() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "tool_call_finished", "id": "tool1", "output": "result"}"#,
        &mut final_text,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::ToolCallFinished { ref id, output: Some(ref o), .. }) if id == "tool1" && o == "result")
    );
}

#[test]
fn test_non_exempt_providers_emit_canonical_tool_events() {
    let contract_cases = [
        (
            ProviderId::Claude,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"claude_tool_1","name":"Bash","input":{"command":"pwd"}}]}}"#,
            r#"{"role":"user","content":[{"type":"tool_result","tool_use_id":"claude_tool_1","content":"ok"}]}"#,
        ),
        (
            ProviderId::MiniMax,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"minimax_tool_1","name":"Bash","input":{"command":"pwd"}}]}}"#,
            r#"{"role":"user","content":[{"type":"tool_result","tool_use_id":"minimax_tool_1","content":"ok"}]}"#,
        ),
        (
            ProviderId::Codex,
            r#"{"type":"item.started","item":{"id":"codex_tool_1","type":"mcp_tool_call","call":{"id":"codex_tool_1","name":"mcp__bash__run","arguments":"{\"command\":\"pwd\"}"},"status":"in_progress"}}"#,
            r#"{"type":"item.completed","item":{"id":"codex_tool_1","type":"mcp_tool_call","call":{"id":"codex_tool_1","name":"mcp__bash__run"},"result":{"content":[{"text":"/tmp"}]}}}"#,
        ),
        (
            ProviderId::Gemini,
            r#"{"type":"tool_use","id":"gemini_tool_1","name":"Bash","input":{"command":"pwd"}}"#,
            r#"{"type":"tool_result","tool_use_id":"gemini_tool_1","content":"ok"}"#,
        ),
        (
            ProviderId::Kimi,
            r#"{"role":"assistant","content":[{"type":"tool_call","id":"kimi_tool_1","function":{"name":"read_file","arguments":"{\"path\":\"/tmp/main.rs\"}"}}]}"#,
            r#"{"role":"tool","name":"read_file","content":"file contents"}"#,
        ),
        (
            ProviderId::OpenCode,
            r#"{"type":"tool_started","id":"opencode_tool_1","name":"bash","input":{"command":"pwd"}}"#,
            r#"{"type":"tool_finished","id":"opencode_tool_1","output":"ok"}"#,
        ),
    ];

    for (provider, started_line, finished_line) in contract_cases {
        assert_canonical_tool_started(provider.clone(), started_line);
        assert_canonical_tool_finished(provider, finished_line);
    }
}

#[test]
fn test_map_line_multiple_events_from_one_line() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "text_delta", "delta": {"text": "test"}}"#,
        &mut final_text,
    );
    assert_eq!(events.len(), 1);
}

#[test]
fn test_jsonl_transport_new() {
    let transport = JsonlCommandTransport::new("echo", vec!["hello".to_string()]);
    assert_eq!(transport.exe, "echo");
    assert_eq!(transport.args, vec!["hello"]);
    assert!(transport.env.is_empty());
}

#[test]
fn test_jsonl_transport_with_mapper_does_nothing() {
    let transport = JsonlCommandTransport::new("echo", vec![]);
    let _ = transport;
}

#[test]
fn test_build_command_sets_cwd() {
    let transport = JsonlCommandTransport::new("echo", vec![]);
    let request = test_request();
    let cmd = transport.build_command(&request);
    let _ = cmd;
}

#[test]
fn test_map_line_empty_string() {
    let mut final_text = String::new();
    let events = map_line(ProviderId::Claude, "", &mut final_text);
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_map_line_usage_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "usage", "input_tokens": 100, "output_tokens": 50}"#,
        &mut final_text,
    );
    assert!(matches!(
        events[0],
        Ok(AgentEvent::RawProviderEvent { ref provider, .. }) if provider == "claude"
    ));
}

#[test]
fn test_extract_external_session_from_result_line() {
    let session = JsonlCommandTransport::extract_external_session(
        r#"{"type":"result","session_id":"sess-123"}"#,
        &ProviderId::MiniMax,
    )
    .expect("session should be extracted");
    assert_eq!(session.id, "sess-123");
    assert_eq!(session.provider, ProviderId::MiniMax);
}

#[test]
fn test_extract_external_session_ignores_missing_id() {
    let session = JsonlCommandTransport::extract_external_session(
        r#"{"type":"result"}"#,
        &ProviderId::Claude,
    );
    assert!(session.is_none());
}

#[test]
fn test_extract_external_session_captures_codex_thread_id_from_log_message() {
    let session = JsonlCommandTransport::extract_external_session(
        r#"{"type":"log","level":"debug","message":"{\"thread_id\":\"codex-thread-123\",\"type\":\"thread.started\"}"}"#,
        &ProviderId::Codex,
    )
    .expect("codex thread id");
    assert_eq!(session.id, "codex-thread-123");
    assert_eq!(session.provider, ProviderId::Codex);
}

#[test]
fn test_extract_external_session_captures_kimi_resume_id_from_log_message() {
    let session = JsonlCommandTransport::extract_external_session(
        r#"{"type":"log","level":"warn","message":"To resume this session: kimi -r kimi-session-123"}"#,
        &ProviderId::Kimi,
    )
    .expect("kimi session id");
    assert_eq!(session.id, "kimi-session-123");
    assert_eq!(session.provider, ProviderId::Kimi);
}

#[test]
fn test_map_line_malformed_json_becomes_log() {
    let mut final_text = String::new();
    let events = map_line(ProviderId::Claude, "not valid json {", &mut final_text);
    assert!(matches!(
        events[0],
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            ..
        })
    ));
}

#[test]
fn test_map_line_permission_request() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type": "permission_request", "id": "perm_1", "action": "bash", "risk": "high", "details": {}}"#,
        &mut final_text,
    );
    assert!(
        matches!(events[0], Ok(AgentEvent::PermissionRequest { ref id, .. }) if id == "perm_1")
    );
}

#[test]
fn test_map_line_multiple_text_deltas_accumulate() {
    let mut final_text = String::new();
    map_line(
        ProviderId::Claude,
        r#"{"type": "text_delta", "delta": {"text": "Hello"}}"#,
        &mut final_text,
    );
    map_line(
        ProviderId::Claude,
        r#"{"type": "text_delta", "delta": {"text": " World"}}"#,
        &mut final_text,
    );
    assert_eq!(final_text, "Hello World");
}

#[test]
fn test_map_line_assistant_tool_use_emits_normalized_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Claude,
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"git status --short"}}]}}"#,
        &mut final_text,
    );
    assert!(final_text.is_empty());
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, .. }) if id == "toolu_1" && name == "Bash"
    ));
}

#[test]
fn test_map_line_user_tool_result_emits_finished_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::MiniMax,
        r#"{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"tool-ok"}]}"#,
        &mut final_text,
    );
    assert!(final_text.is_empty());
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallFinished { id, output: Some(output), error: None }) if id == "toolu_1" && output == "tool-ok"
    ));
}

#[test]
fn test_map_line_gemini_tool_use_emits_normalized_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Gemini,
        r#"{"type":"tool_use","id":"tool_1","name":"Bash","input":{"command":"pwd"}}"#,
        &mut final_text,
    );
    assert!(final_text.is_empty());
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, input }) if id == "tool_1" && name == "Bash" && input["command"] == "pwd"
    ));
}

#[test]
fn test_map_line_tool_use_accepts_tool_name_alias() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Gemini,
        r#"{"type":"tool_use","id":"tool_semble","tool_name":"mcp__semble__search","input":{"query":"stream output"}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, input }) if id == "tool_semble" && name == "mcp__semble__search" && input["query"] == "stream output"
    ));
}

#[test]
fn test_map_line_kimi_tool_call_emits_normalized_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Kimi,
        r#"{"role":"assistant","content":[{"type":"tool_call","id":"tool_1","function":{"name":"read_file","arguments":"{\"path\":\"/tmp/main.rs\"}"}}]}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, input }) if id == "tool_1" && name == "read_file" && input["path"] == "/tmp/main.rs"
    ));
}

#[test]
fn test_map_line_kimi_tool_result_emits_finished_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Kimi,
        r#"{"role":"tool","name":"read_file","content":"file contents"}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallFinished { id, output: Some(output), .. }) if id == "read_file" && output == "file contents"
    ));
}

#[test]
fn test_map_line_codex_command_execution_started_emits_normalized_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"item.started","item":{"id":"cmd_1","type":"command_execution","command":"rg -n stream src","status":"in_progress"}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, input }) if id == "cmd_1" && name == "grep" && input["command"] == "rg -n stream src"
    ));
}

#[test]
fn test_map_line_codex_command_execution_completed_emits_finished_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"item.completed","item":{"id":"cmd_1","type":"command_execution","command":"rg -n stream src","aggregated_output":"src/main.rs:12:stream","exit_code":0,"status":"completed"}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallFinished { id, output: Some(output), error: None }) if id == "cmd_1" && output == "src/main.rs:12:stream"
    ));
}

#[test]
fn test_map_line_codex_mcp_tool_started_emits_normalized_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"item.started","item":{"id":"tool_1","type":"mcp_tool_call","call":{"id":"tool_1","name":"mcp__bash__run","arguments":"{\"command\":\"pwd\"}"},"status":"in_progress"}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, input }) if id == "tool_1" && name == "mcp__bash__run" && input["command"] == "pwd"
    ));
}

#[test]
fn test_map_line_codex_mcp_tool_completed_emits_finished_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"item.completed","item":{"id":"tool_1","type":"mcp_tool_call","call":{"id":"tool_1","name":"mcp__bash__run"},"result":{"content":[{"text":"/workspace/project"}]}}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallFinished { id, output: Some(output), error: None }) if id == "tool_1" && output == "/workspace/project"
    ));
}

#[test]
fn test_map_line_codex_response_item_exec_command_uses_shell_tool_name() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{\"cmd\":\"grep -RIn tool src\"}","call_id":"call_exec_1"}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, input }) if id == "call_exec_1" && name == "grep" && input["command"] == "grep -RIn tool src"
    ));
}

#[test]
fn test_map_line_codex_response_item_mcp_function_call_uses_namespace_and_name() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"response_item","payload":{"type":"function_call","name":"search_memory","namespace":"mcp__chatbot_memory__","arguments":"{\"workspace_root\":\"/workspace/project\",\"query\":\"imports\"}","call_id":"call_mcp_1"}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallStarted { id, name, input }) if id == "call_mcp_1" && name == "mcp__chatbot_memory__search_memory" && input["workspace_root"] == "/workspace/project"
    ));
}

#[test]
fn test_map_line_codex_event_msg_exec_command_end_emits_finished_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"event_msg","payload":{"type":"exec_command_end","call_id":"call_exec_1","aggregated_output":"src/main.rs:12:tool\n","exit_code":0}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallFinished { id, output: Some(output), error: None }) if id == "call_exec_1" && output == "src/main.rs:12:tool\n"
    ));
}

#[test]
fn test_map_line_codex_event_msg_mcp_tool_call_end_emits_finished_tool_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"event_msg","payload":{"type":"mcp_tool_call_end","call_id":"call_mcp_1","invocation":{"server":"chatbot_memory","tool":"search_memory","arguments":{"workspace_root":"/workspace/project","query":"imports"}},"result":{"Ok":{"content":[{"type":"text","text":"Search results for: imports"}],"isError":false}}}}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::ToolCallFinished { id, output: Some(output), error: None }) if id == "call_mcp_1" && output == "Search results for: imports"
    ));
}

#[test]
fn test_map_line_codex_nested_log_message_maps_to_raw_provider_event() {
    let mut final_text = String::new();
    let events = map_line(
        ProviderId::Codex,
        r#"{"type":"log","level":"debug","message":"{\"type\":\"thread.started\",\"thread_id\":\"codex-thread-123\"}"}"#,
        &mut final_text,
    );
    assert!(matches!(
        &events[0],
        Ok(AgentEvent::RawProviderEvent { provider, payload }) if provider == "codex" && payload["type"] == "thread.started"
    ));
}

#[tokio::test]
async fn test_jsonl_exactly_one_run_started() {
    let transport = JsonlCommandTransport::new(
        "echo",
        vec!["{\"type\": \"text_delta\", \"delta\": {\"text\": \"hello\"}}".to_string()],
    );
    let request = test_request();
    let stream = transport.run_stream(request).await.unwrap();

    let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
        .await
        .unwrap();

    let run_started_count = events
        .iter()
        .filter(|e| matches!(e, Ok(AgentEvent::RunStarted { .. })))
        .count();

    assert_eq!(
        run_started_count, 1,
        "Expected exactly one RunStarted event, got {}",
        run_started_count
    );
}

#[tokio::test]
async fn test_jsonl_exactly_one_terminal_event() {
    let transport = JsonlCommandTransport::new(
        "echo",
        vec!["{\"type\": \"text_delta\", \"delta\": {\"text\": \"hello\"}}".to_string()],
    );
    let request = test_request();
    let stream = transport.run_stream(request).await.unwrap();

    let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
        .await
        .unwrap();

    let terminal_count = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Ok(AgentEvent::RunCompleted { .. })
                    | Ok(AgentEvent::RunFailed { .. })
                    | Ok(AgentEvent::RunCancelled { .. })
            )
        })
        .count();

    assert_eq!(
        terminal_count, 1,
        "Expected exactly one terminal event (RunCompleted/RunFailed/RunCancelled), got {}",
        terminal_count
    );
}

#[tokio::test]
async fn test_jsonl_run_started_before_other_events() {
    let transport = JsonlCommandTransport::new(
        "echo",
        vec!["{\"type\": \"text_delta\", \"delta\": {\"text\": \"hello\"}}".to_string()],
    );
    let request = test_request();
    let stream = transport.run_stream(request).await.unwrap();

    let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
        .await
        .unwrap();

    let run_started_idx = events
        .iter()
        .position(|e| matches!(e, Ok(AgentEvent::RunStarted { .. })));

    assert!(
        run_started_idx.is_some(),
        "Expected at least one RunStarted event"
    );
    assert_eq!(
        run_started_idx.unwrap(),
        0,
        "RunStarted should be the first event"
    );
}

#[tokio::test]
async fn test_jsonl_no_events_after_terminal() {
    let transport = JsonlCommandTransport::new(
        "echo",
        vec!["{\"type\": \"text_delta\", \"delta\": {\"text\": \"hello\"}}".to_string()],
    );
    let request = test_request();
    let stream = transport.run_stream(request).await.unwrap();

    let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
        .await
        .unwrap();

    let terminal_idx = events.iter().position(|e| {
        matches!(
            e,
            Ok(AgentEvent::RunCompleted { .. })
                | Ok(AgentEvent::RunFailed { .. })
                | Ok(AgentEvent::RunCancelled { .. })
        )
    });

    if let Some(idx) = terminal_idx {
        let events_after = &events[idx + 1..];
        let non_empty = events_after
            .iter()
            .filter(|e| !matches!(e, Ok(AgentEvent::Log { .. })))
            .count();
        assert_eq!(
            non_empty, 0,
            "No non-log events should appear after terminal event"
        );
    }
}
