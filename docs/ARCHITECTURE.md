# Architecture

## Workspace Structure

The workspace contains the agent-adapter crates:

```
crates/
  agent-adapter-core/      - Core types, events, errors, request/response
  agent-adapter-cli/       - CLI implementation
  agent-adapter-config/    - TOML configuration loader
  agent-adapter-transports/ - JSONL, one-shot, ACP, PTY transports
  agent-adapter-providers/  - Command builders for 8 providers
  agent-adapter-router/     - Provider selection and routing
  agent-adapter-testkit/   - Fake adapters and contract tests
```

## Crate Responsibilities

### `agent-adapter-core`

Core types and traits:
- `ChatRequest` - Request model with prompt, provider, session, env
- `apply_prompt_policy()` - Shared prompt-policy injection applied before provider execution
- `AgentEvent` - Canonical event stream (RunStarted, TextDelta, ToolCall*, etc.)
- `AdapterError` - Structured error types
- `AgentAdapter` - Trait for provider adapters
- `EventMapper` - Trait for mapping provider output to events

### `agent-adapter-cli`

Binary entrypoint with commands:
- `run` - Execute a provider with ChatRequest
- `healthcheck` - Check provider availability
- `providers` - List available providers
- `doctor` - Diagnostic command

The `run` path owns terminal CLI error propagation as well. When a transport
emits `RunFailed`, the CLI now preserves the terminal failure message and
returns the matching process exit code instead of collapsing the result into a
generic `Run command failed` wrapper.

### `agent-adapter-transports`

Process-based transports for provider communication:
- `JsonlCommandTransport` - JSONL streaming output
- `OneShotCommandTransport` - Collect output and exit
- `AcpStdioTransport` - ACP JSON-RPC over stdio
- `PtyFallbackTransport` - PTY for TUI-based providers

The JSONL transport keeps its generic stream orchestration in
`src/jsonl_command.rs` and isolates Codex runtime envelope/item mapping in
`src/jsonl_command/codex_runtime.rs` so each source file stays below the
workspace line-count ceiling.

`JsonlCommandTransport` also owns the canonical tool-event normalization
contract for non-exempt providers. Claude, MiniMax, Codex, Gemini, Kimi, and
OpenCode are expected to emit `AgentEvent::ToolCallStarted` /
`ToolCallFinished` from their representative tool output shapes, and the
transport test suite now enforces that provider-wide contract. Forge, Hermes,
and Claw remain explicit passthrough exemptions because they do not currently
run through that normalized JSONL tool-event path.

### `agent-adapter-providers`

Per-provider command builders:
- Codex, Claude, Gemini, Kimi, OpenCode, Hermes, Forge, Claw

Each provider maps `ApprovalPolicy` to provider-specific flags and applies the
shared prompt-policy layer before building the effective command request. That
policy currently enforces code-search behavior across providers by preferring
the `semble` shell command for code spot discovery and the provider-facing
`fff` MCP tools instead of `rg` for exact text matches when those capabilities
are actually available in the provider runtime. The policy now spells out that
`fff` should not be assumed to exist as a shell command unless a runtime
explicitly exposes it that way. When either capability is missing, the injected
policy tells the model to fall back once to the best available provider-native
search or file-inspection tools rather than spinning on repeated availability
probes. On Claude-compatible providers the policy is injected into the
effective system prompt; on other providers it falls back to prompt-prefix
injection.

To keep the provider layer maintainable under the workspace source-file limit,
provider-specific command helpers and plain-text mapper logic live in
submodules (`src/command/custom.rs`, `src/mappers/plain_text.rs`) while the
top-level files retain the shared dispatch and exported types.

### `agent-adapter-router`

Provider selection based on:
- Explicit provider name
- Capability requirements
- Workspace policy

### `agent-adapter-testkit`

**Test-only crate** - Not a production dependency.

Testing utilities (dev/test only):
- `FakeAdapter` - Mock adapter for testing
- `ProviderContract` - Contract tests for providers
- `docs_lint` - Documentation consistency checks

**Important**: `agent-adapter-cli` must NOT depend on `agent-adapter-testkit` in production. The testkit is only linked as a dev dependency.

Large validation helpers in the testkit are split between production modules
and colocated child modules/tests (`src/design_traceability/doc_checks.rs`,
`src/*/tests.rs`) so the validation code stays auditable without relaxing CI
or lint policy.

## Design Principles

1. **Protocol-first, transport-second, provider-last**
2. **Dangerous default mode** - `--safe` opts into safe behavior
3. **Structured protocols preferred** - ACP over PTY when available
4. **Environment isolation** - Allowlist env vars, no inherited secrets
5. **Process cleanup** - Kill process groups on cancel/timeout

## Key Traits

```rust
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> AdapterCapabilities;
    async fn healthcheck(&self) -> AdapterResult<AdapterHealth>;
    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream>;
    async fn cancel(&self, run_id: Uuid) -> AdapterResult<()>;
}
```
