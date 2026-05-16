use crate::{ChatRequest, ProviderId};
use serde_json::{Map, Value};

const CODE_SEARCH_POLICY_MARKER: &str = "[agent-adapter-code-search-policy-v1]";
const CODE_SEARCH_POLICY_PREAMBLE: &str = "[agent-adapter-code-search-policy-v1]
Code search policy:
- Prefer the `semble` shell command first for finding relevant code spots and nearby implementations when it is available in your runtime.
- Prefer the provider-facing `fff` MCP tools instead of `rg` for exact or exhaustive text matches when the `fff` server is available in your runtime.
- Do not assume `fff` is available as a shell command unless the runtime explicitly exposes it that way.
- If `semble` or `fff` are unavailable in your runtime, fall back immediately to the best available provider-native search or file-inspection tools instead of retrying unavailable tool lookups.
- Do not loop on repeated availability probes for `semble` or `fff`; check once, then continue with the available tools.
";

/// Apply the adapter's cross-provider prompt policy to a request.
pub fn apply_prompt_policy(request: &ChatRequest) -> ChatRequest {
    let mut effective = request.clone();
    if prompt_policy_present(&effective) {
        return effective;
    }

    if matches!(effective.provider, ProviderId::Claude | ProviderId::MiniMax) {
        let merged = match effective
            .extra
            .get("system_prompt")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(existing) => format!("{CODE_SEARCH_POLICY_PREAMBLE}\n\n{existing}"),
            None => CODE_SEARCH_POLICY_PREAMBLE.to_string(),
        };
        ensure_extra_object(&mut effective)
            .insert("system_prompt".to_string(), Value::String(merged));
    } else {
        effective.prompt = format!("{CODE_SEARCH_POLICY_PREAMBLE}\n\n{}", effective.prompt);
    }

    effective
}

fn prompt_policy_present(request: &ChatRequest) -> bool {
    request.prompt.contains(CODE_SEARCH_POLICY_MARKER)
        || request
            .extra
            .get("system_prompt")
            .and_then(Value::as_str)
            .is_some_and(|prompt| prompt.contains(CODE_SEARCH_POLICY_MARKER))
}

fn ensure_extra_object(request: &mut ChatRequest) -> &mut Map<String, Value> {
    if !request.extra.is_object() {
        request.extra = Value::Object(Map::new());
    }
    request
        .extra
        .as_object_mut()
        .expect("request.extra should be an object after normalization")
}

#[cfg(test)]
mod tests {
    use crate::{ChatRequest, ProviderId};

    use super::{apply_prompt_policy, CODE_SEARCH_POLICY_MARKER};

    #[test]
    fn apply_prompt_policy_injects_search_guidance() {
        let request = ChatRequest::new("find the handler", ".", ProviderId::Codex);

        let effective = apply_prompt_policy(&request);

        assert!(effective.prompt.contains(CODE_SEARCH_POLICY_MARKER));
        assert!(effective
            .prompt
            .contains("Prefer the `semble` shell command first"));
        assert!(effective
            .prompt
            .contains("Prefer the provider-facing `fff` MCP tools instead of `rg`"));
        assert!(effective
            .prompt
            .contains("Do not assume `fff` is available as a shell command"));
        assert!(effective
            .prompt
            .contains("fall back immediately to the best available provider-native search"));
        assert!(effective
            .prompt
            .contains("Do not loop on repeated availability probes"));
        assert!(effective.prompt.ends_with("find the handler"));
    }

    #[test]
    fn apply_prompt_policy_is_idempotent() {
        let request = ChatRequest::new("find the handler", ".", ProviderId::Codex);

        let once = apply_prompt_policy(&request);
        let twice = apply_prompt_policy(&once);

        assert_eq!(once.prompt, twice.prompt);
        assert_eq!(twice.prompt.matches(CODE_SEARCH_POLICY_MARKER).count(), 1);
    }

    #[test]
    fn apply_prompt_policy_uses_system_prompt_for_claude_compatible_providers() {
        let request = ChatRequest::new("find the handler", ".", ProviderId::MiniMax);

        let effective = apply_prompt_policy(&request);

        assert_eq!(effective.prompt, "find the handler");
        let system_prompt = effective
            .extra
            .get("system_prompt")
            .and_then(serde_json::Value::as_str)
            .expect("system prompt should be injected");
        assert!(system_prompt.contains(CODE_SEARCH_POLICY_MARKER));
        assert!(system_prompt.contains("Prefer the `semble` shell command first"));
        assert!(
            system_prompt.contains("Prefer the provider-facing `fff` MCP tools instead of `rg`")
        );
        assert!(system_prompt.contains("Do not assume `fff` is available as a shell command"));
        assert!(system_prompt.contains("Do not loop on repeated availability probes"));
    }

    #[test]
    fn apply_prompt_policy_preserves_existing_system_prompt_for_claude_compatible_providers() {
        let mut request = ChatRequest::new("find the handler", ".", ProviderId::Claude);
        request.extra = serde_json::json!({
            "system_prompt": "existing rules"
        });

        let effective = apply_prompt_policy(&request);

        let system_prompt = effective
            .extra
            .get("system_prompt")
            .and_then(serde_json::Value::as_str)
            .expect("system prompt should be present");
        assert!(system_prompt.starts_with(CODE_SEARCH_POLICY_MARKER));
        assert!(system_prompt.ends_with("existing rules"));
    }
}
