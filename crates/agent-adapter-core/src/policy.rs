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
