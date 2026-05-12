use super::*;
impl JsonlCommandTransport {
    fn codex_exec_command_output(payload: &serde_json::Value) -> Option<String> {
        for candidate in [
            payload.get("aggregated_output"),
            payload.get("formatted_output"),
            payload.get("stdout"),
            payload.get("stderr"),
        ] {
            if let Some(output) = candidate.and_then(|value| value.as_str()) {
                if !output.trim().is_empty() {
                    return Some(output.to_string());
                }
            }
        }
        None
    }}
impl JsonlCommandTransport {
    fn codex_exec_command_error(payload: &serde_json::Value) -> Option<String> {
        let exit_code = payload.get("exit_code").and_then(|code| code.as_i64())?;
        (exit_code != 0).then(|| format!("command exited with code {exit_code}"))
    }}
impl JsonlCommandTransport {
    fn codex_mcp_tool_error(payload: &serde_json::Value) -> Option<String> {
        payload
            .pointer("/result/Err")
            .and_then(|error| error.as_str())
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
    }}
impl JsonlCommandTransport {
    fn shell_tool_name(command: &str) -> &'static str {
        match command
            .split_whitespace()
            .next()
            .map(|token| token.trim_matches('\'').trim_matches('"'))
        {
            Some("rg") | Some("grep") => "grep",
            _ => "bash",
        }
    }}
