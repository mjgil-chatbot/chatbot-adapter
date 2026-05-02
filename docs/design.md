# Design Document

This document is the **acceptance contract** for agent-adapter. Every claim in this document is validated by `validate_design_traceability` against actual implementation, tests, and documentation. The acceptance command runs this validation automatically:

```bash
cargo run -p agent-adapter-cli --release -- acceptance
```

Claims without verifiable evidence will cause acceptance to fail. See the Evidence Requirements section for details.

## Recommended shape: protocol-first, transport-second, provider-last

Build this as a Rust **adapter layer over agent transports**, not as eight unrelated provider integrations.

```text
chatbot / UI / API
        │
        ▼
AgentRouter
        │
        ▼
AgentAdapter trait
        │
        ├── ACP / JSON-RPC stdio adapter
        ├── JSONL / stream-json CLI adapter
        ├── one-shot text CLI adapter
        └── PTY/TUI fallback adapter
                 │
                 ▼
hermes / forge / opencode / kimi / codex / gemini / claude / claw
```

The key design decision: **prefer structured protocols whenever possible**. ACP is especially relevant because it standardizes communication between coding agents and clients, supports local subprocesses over JSON-RPC stdio, and is meant to reduce bespoke agent/editor integrations. ([Agent Client Protocol][1]) Gemini CLI explicitly supports `gemini --acp`; OpenCode exposes `opencode acp`; Kimi exposes `kimi acp`; Hermes exposes `hermes acp`. ([Gemini CLI][2])

## Provider support strategy

I would support these with this priority order:

| Provider                  |            Preferred adapter |                                               Fallback adapter | Notes                                                                                                                                                                                                  |
| ------------------------- | ---------------------------: | -------------------------------------------------------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Gemini CLI**            |          ACP: `gemini --acp` |             JSONL: `gemini -p ... --output-format stream-json` | Good structured support. ([Gemini CLI][2])                                                                                                                                                             |
| **OpenCode**              |          ACP: `opencode acp` |                  JSON events: `opencode run --format json ...` | Also has `opencode serve` if you want a warm daemon. ([OpenCode][3])                                                                                                                                   |
| **Kimi CLI**              |              ACP: `kimi acp` | JSONL: `kimi --print --output-format stream-json --prompt ...` | Kimi also has prompt, quiet, print, yolo/afk, session flags. ([Moonshot AI][4])                                                                                                                        |
| **Codex**                 |                    JSONL CLI |                                                       Text CLI | `codex exec --json --cd <cwd> -` is the clean automation surface; prompt can be read from stdin. ([OpenAI Developers][5])                                                                              |
| **Claude Code CLI**       |                    JSONL CLI |                                                       Text CLI | Assuming “claude cli” means Claude Code’s `claude`; it supports `-p`, `--output-format json/stream-json`, `--input-format text/stream-json`, resume/session flags, and permission modes. ([Claude][6]) |
| **Hermes Agent**          |         ACP or one-shot text |                                             TUI only if needed | Hermes supports `hermes acp`, `hermes chat -q`, and `hermes -z` for clean one-shot final-text output. ([Hermes Agent][7])                                                                              |
| **ForgeCode**             |                One-shot text |                                                        PTY/TUI | Forge documents `forge -p` / `--prompt` one-shot mode, plus an interactive TUI and Zsh plugin mode. ([GitHub][8])                                                                                      |
| **Claw Code / claw-code** | Configurable command adapter |                                                        PTY/TUI | Treat this as unstable/fragmented. Some public Claw Code repos expose `claw prompt`, but the ecosystem appears experimental and fork-heavy, so do not hardcode too much. ([GitHub][9])                 |

## Core Rust API

Use a single canonical event stream. Every CLI, whether JSON-RPC, JSONL, or plain text, should become the same `AgentEvent` stream.

```rust
use async_trait::async_trait;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, pin::Pin};
use uuid::Uuid;

pub type AdapterResult<T> = Result<T, AdapterError>;
pub type EventStream =
    Pin<Box<dyn Stream<Item = AdapterResult<AgentEvent>> + Send + 'static>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub cwd: PathBuf,

    pub provider: ProviderId,
    pub model: Option<String>,

    pub session: Option<ExternalSessionRef>,
    pub resume: ResumeMode,

    pub files: Vec<PathBuf>,
    pub images: Vec<PathBuf>,

    pub approval_policy: ApprovalPolicy,
    pub sandbox: SandboxMode,

    pub timeout_ms: Option<u64>,
    pub env: HashMap<String, String>,

    /// Free-form provider-specific overrides.
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    RunStarted {
        run_id: Uuid,
        provider: ProviderId,
        external_session: Option<ExternalSessionRef>,
    },

    TextDelta {
        role: Role,
        text: String,
    },

    ToolCallStarted {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    ToolCallDelta {
        id: String,
        delta: serde_json::Value,
    },

    ToolCallFinished {
        id: String,
        output: Option<String>,
        error: Option<String>,
    },

    FileChange {
        path: PathBuf,
        operation: FileOperation,
    },

    PermissionRequest {
        id: String,
        action: String,
        risk: RiskLevel,
        details: serde_json::Value,
    },

    PlanUpdate {
        items: Vec<PlanItem>,
    },

    Usage {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        cost_usd: Option<f64>,
    },

    Log {
        level: LogLevel,
        message: String,
    },

    RunCompleted {
        final_text: Option<String>,
        external_session: Option<ExternalSessionRef>,
    },

    RunFailed {
        code: Option<i32>,
        message: String,
    },

    RunCancelled,
}

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> ProviderId;

    fn capabilities(&self) -> AdapterCapabilities;

    async fn healthcheck(&self) -> AdapterResult<AdapterHealth>;

    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream>;

    async fn cancel(&self, run_id: Uuid) -> AdapterResult<()>;
}
```

The `AgentAdapter` trait should be implemented by transports, then wrapped by provider-specific command builders.

```rust
pub enum TransportKind {
    AcpJsonRpcStdio,
    JsonlCommand,
    OneShotCommand,
    PtyFallback,
}

pub trait EventMapper: Send {
    fn map_stdout_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>>;
    fn map_stderr_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>>;
    fn finish(&mut self, exit_code: Option<i32>) -> Vec<AdapterResult<AgentEvent>>;
}
```

## Transport implementations

### 1. ACP / JSON-RPC stdio adapter

Use this for Gemini, OpenCode, Kimi, and Hermes when possible.

```text
spawn child:
  gemini --acp
  opencode acp
  kimi acp
  hermes acp

adapter:
  initialize
  authenticate/check auth if required
  newSession or loadSession
  prompt
  map JSON-RPC notifications/responses -> AgentEvent
```

For a chatbot, your service becomes the ACP client. You can expose minimal file-system and terminal capabilities, or deny them and let the underlying CLI use its own tools depending on the provider’s ACP behavior. ACP is best when you want bidirectional session control, cancellation, permission mediation, and rich tool/file events.

### 2. JSONL / stream-json command adapter

Use this for Codex, Claude, Gemini, Kimi, and OpenCode.

Example command specs:

```toml
[providers.codex]
kind = "jsonl_command"
exe = "codex"
args = ["exec", "--json", "--cd", "{cwd}", "-"]
prompt_mode = "stdin"
mapper = "codex"

[providers.claude]
kind = "jsonl_command"
exe = "claude"
args = ["-p", "--output-format", "stream-json", "--input-format", "text", "{prompt}"]
prompt_mode = "arg"
mapper = "claude"

[providers.gemini]
kind = "jsonl_command"
exe = "gemini"
args = ["-p", "{prompt}", "--output-format", "stream-json"]
prompt_mode = "arg"
mapper = "gemini"

[providers.kimi]
kind = "jsonl_command"
exe = "kimi"
args = ["--print", "--output-format", "stream-json", "--prompt", "{prompt}"]
prompt_mode = "arg"
mapper = "kimi"

[providers.opencode]
kind = "jsonl_command"
exe = "opencode"
args = ["run", "--format", "json", "--dir", "{cwd}", "{prompt}"]
prompt_mode = "arg"
mapper = "opencode"
```

Important: never build these through a shell string. Build `Command` with argv pieces so prompts and file paths cannot inject flags or shell commands.

### 3. One-shot text command adapter

Use this for ForgeCode, Hermes final-only mode, Claw fallback, or any unknown CLI.

```toml
[providers.forge]
kind = "oneshot_command"
exe = "forge"
args = ["-p", "{prompt}"]
prompt_mode = "arg"
mapper = "plain_text"

[providers.hermes_text]
kind = "oneshot_command"
exe = "hermes"
args = ["-z", "{prompt}"]
prompt_mode = "arg"
mapper = "plain_text"

[providers.claw]
kind = "oneshot_command"
exe = "claw"
args = ["prompt", "{prompt}"]
prompt_mode = "arg"
mapper = "plain_text"
```

This adapter emits:

```text
RunStarted
TextDelta or final full text
RunCompleted
```

It will not reliably expose intermediate tool calls unless the CLI prints them in a parseable way.

### 4. PTY/TUI fallback adapter

Keep this as a compatibility escape hatch only.

Use `portable-pty` or similar. It should:

1. Spawn the CLI in a pseudo-terminal.
2. Wait for a prompt pattern.
3. Send user input.
4. Capture screen output.
5. Detect completion using inactivity timeout, explicit markers, or user cancellation.

This is brittle because terminal UIs change, ANSI rendering is noisy, and there is often no reliable “turn complete” signal.

## Provider modules

Put provider-specific logic in small modules that only do command construction and event mapping.

```text
src/
  core/
    adapter.rs
    events.rs
    request.rs
    errors.rs
    capabilities.rs

  transport/
    acp.rs
    jsonl_command.rs
    oneshot_command.rs
    pty.rs

  providers/
    codex.rs
    claude.rs
    gemini.rs
    opencode.rs
    kimi.rs
    hermes.rs
    forge.rs
    claw.rs
    custom.rs

  registry.rs
  router.rs
```

Each provider should expose:

```rust
pub trait ProviderFactory {
    fn detect(&self) -> AdapterResult<AdapterHealth>;
    fn build(&self, config: ProviderConfig) -> AdapterResult<Box<dyn AgentAdapter>>;
}
```

## Routing design

Add an `AgentRouter` that selects a provider based on explicit user choice, project policy, or capability requirements.

```rust
pub struct AgentRouter {
    adapters: HashMap<ProviderId, Box<dyn AgentAdapter>>,
}

impl AgentRouter {
    pub async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        let adapter = self
            .adapters
            .get(&request.provider)
            .ok_or_else(|| AdapterError::ProviderNotFound(request.provider.clone()))?;

        let caps = adapter.capabilities();

        if request.images.len() > 0 && !caps.images {
            return Err(AdapterError::UnsupportedCapability {
                provider: request.provider.clone(),
                capability: "images",
            });
        }

        adapter.run(request).await
    }
}
```

Capability checks matter because the CLIs differ. For example, Codex and Claude have strong non-interactive structured modes, Gemini and Kimi expose ACP plus stream JSON, Forge’s documented automation surface is simpler one-shot text, and Claw Code should be kept configurable until a stable machine-readable interface is clear.

## Event mapping rules

Do not try to force every provider into a perfect schema. Normalize what you can, preserve raw events for the rest.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawProviderEvent {
    pub provider: ProviderId,
    pub raw: serde_json::Value,
}
```

For JSONL providers:

```rust
pub struct GenericJsonlMapper {
    provider: ProviderId,
    final_text: String,
}

impl EventMapper for GenericJsonlMapper {
    fn map_stdout_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            return vec![Ok(AgentEvent::Log {
                level: LogLevel::Info,
                message: line.to_string(),
            })];
        };

        let event_type = value
            .get("type")
            .or_else(|| value.get("event"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "text_delta" | "message_delta" | "assistant_delta" => {
                let text = value
                    .pointer("/delta/text")
                    .or_else(|| value.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }

            "tool_call" | "tool_started" => {
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    input: value.get("input").cloned().unwrap_or(serde_json::Value::Null),
                })]
            }

            _ => {
                vec![Ok(AgentEvent::Log {
                    level: LogLevel::Debug,
                    message: value.to_string(),
                })]
            }
        }
    }

    fn map_stderr_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        vec![Ok(AgentEvent::Log {
            level: LogLevel::Warn,
            message: line.to_string(),
        })]
    }

    fn finish(&mut self, exit_code: Option<i32>) -> Vec<AdapterResult<AgentEvent>> {
        if exit_code == Some(0) {
            vec![Ok(AgentEvent::RunCompleted {
                final_text: Some(self.final_text.clone()),
                external_session: None,
            })]
        } else {
            vec![Ok(AgentEvent::RunFailed {
                code: exit_code,
                message: "agent process exited unsuccessfully".to_string(),
            })]
        }
    }
}
```

Then add provider-specific mappers only where needed:

```text
CodexMapper
ClaudeStreamJsonMapper
GeminiStreamJsonMapper
KimiStreamJsonMapper
OpenCodeJsonMapper
PlainTextMapper
```

## Session model

Keep **your own session ID** and the **provider’s external session ID** separate.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSessionRef {
    pub provider: ProviderId,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResumeMode {
    None,
    ProviderDefault,
    External(ExternalSessionRef),
    LastInWorkspace,
}
```

For providers that support resume directly, map this to their flags:

```text
codex exec resume <SESSION_ID>
claude --resume <SESSION_ID> / --session-id ...
kimi --resume <ID> / --continue
opencode run --session <ID> / --continue
hermes --resume <session> / --continue
```

For providers that do not expose a clean resume mechanism, maintain the transcript in your chatbot layer and send a summarized context prefix. Do not blindly resend the full chat forever; add summarization or compaction.

## Security defaults

These CLIs can edit files and run shell commands. Treat the adapter as an execution boundary.

**Default policy: dangerous-default**. The adapter operates in dangerous mode by default, allowing dangerous operations (file modifications, shell commands) without explicit approval. Use `--safe` to opt into safe mode.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalPolicy {
    DangerousDefault,  // Default - allows dangerous operations
    Safe,              // Opt-in via --safe flag
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    IsolatedWorktree,
    Container,
    DangerFullAccess,
}
```

Rules enforced:

1. **Dangerous default mode is active when `--safe` is not specified.** Providers receive appropriate bypass/approval flags by default.
2. **`--safe` flag opts into safe mode** where dangerous operations require explicit approval.
3. Run each request in either a clean git worktree or a container.
4. Use per-provider environment allowlists, not inherited full process env.
5. Redact secrets from logs and `AgentEvent::Log`.
6. Kill the whole process group on cancellation or timeout.
7. Store raw provider events only if audit logging is explicitly enabled.

## Health checks

Each adapter should report:

```rust
pub enum AdapterHealth {
    Ready {
        version: Option<String>,
    },
    MissingBinary {
        install_hint: String,
    },
    NeedsAuth {
        login_hint: String,
    },
    Misconfigured {
        message: String,
    },
}
```

Example checks:

```text
codex      -> which codex; codex --version; maybe codex login status
claude     -> which claude; claude --version
gemini     -> which gemini; gemini --version
opencode   -> which opencode; opencode --version; opencode auth list
kimi       -> which kimi; kimi --version
hermes     -> which hermes; hermes version/status
forge      -> which forge; forge --help
claw       -> which claw; claw doctor if available
```

Do not require auth at compile time; surface `NeedsAuth` to the chatbot so the user can run the appropriate login command.

## Minimal Cargo setup

```toml
[dependencies]
async-trait = "0.1"
futures-core = "0.3"
futures-util = "0.3"
tokio = { version = "1", features = ["process", "io-util", "macros", "rt-multi-thread", "time", "sync"] }
tokio-stream = "0.1"
tokio-util = { version = "0.7", features = ["codec"] }

serde = { version = "1", features = ["derive"] }
serde_json = "1"

thiserror = "2"
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }
which = "8"
camino = { version = "1", features = ["serde1"] }

# Optional:
portable-pty = "0.9"      # PTY fallback
```

## Rollout order

I would implement in this order:

1. **Core event model + one-shot command adapter**
   Immediately supports Forge, Hermes `-z`, and Claw/custom.

2. **JSONL command adapter**
   Add Codex, Claude, Gemini, Kimi, OpenCode structured runs.

3. **ACP stdio adapter**
   Add richer long-lived sessions for Gemini, OpenCode, Kimi, Hermes.

4. **Provider-specific parsers**
   Start with generic JSONL; only specialize where the raw schemas need it.

5. **PTY fallback**
   Add only for providers that cannot run headlessly.

6. **Contract tests**
   Use fake CLI fixtures first, real CLI integration tests behind env flags.

The main point: make providers **data/config-driven** wherever possible. Hardcode only the adapters and robust parsers. That way a new CLI becomes a TOML entry plus, at most, one event mapper.

## Implementation Status

### Crates

The summary below reflects the current workspace surface:

| Crate | Status | Key Evidence |
|-------|--------|---------------|
| `agent-adapter-core` | **Complete** | Canonical request, event, type, capability, audit, and error models live under `src/request.rs`, `src/events.rs`, `src/types.rs`, `src/capabilities.rs`, and `src/errors.rs`. |
| `agent-adapter-config` | **Complete** | Config loading, defaults, and registry assembly live under `src/loader.rs`, `src/types.rs`, and `src/registry.rs`. |
| `agent-adapter-providers` | **Complete** | Built-in provider adapters, registries, command builders, and event mappers cover Codex, Claude, Gemini, Kimi, OpenCode, Hermes, Forge, and Claw. |
| `agent-adapter-transports` | **Complete** | JSONL, one-shot, ACP stdio, and PTY transports cover the supported provider execution modes. |
| `agent-adapter-router` | **Complete** | `src/router.rs` owns adapter registration and provider routing. |
| `agent-adapter-cli` | **Complete** | The CLI exposes run, healthcheck, providers, doctor, and acceptance entrypoints. |
| `agent-adapter-testkit` | **Complete** | Test helpers, fixtures, documentation validation, traceability checks, and temporary workspace helpers live in the testkit crate. |
| `agent-adapter-workspace` | **Module** | Workspace and sandbox behavior is implemented through `WorkspaceRef`, sandbox types, and temp/worktree helpers rather than a standalone crate. |
| `agent-adapter-redaction` | **Module** | Redaction and audit filtering live in `agent-adapter-core` and are enforced at the event layer. |

### Transports

| Transport | Status | Evidence |
|-----------|--------|----------|
| JSONL Command | **Complete** | `agent-adapter-transports/src/jsonl_command.rs` - `JsonlCommandTransport` with `map_stdout_line`, `map_stderr_line`, `finish` |
| One-Shot Command | **Complete** | `agent-adapter-transports/src/oneshot_command.rs` - `OneShotCommandTransport` for Forge, Hermes, Claw |
| ACP Stdio | **Complete** | `agent-adapter-transports/src/acp_stdio.rs` - `AcpStdioTransport` for Gemini, Kimi, Hermes |
| PTY Fallback | **Complete** | `agent-adapter-transports/src/pty_fallback.rs` - `PtyFallbackTransport` as compatibility escape hatch |

### Providers

| Provider | Transport | Unsafe Mode | Safe Mode | Evidence |
|----------|-----------|-------------|-----------|----------|
| Codex | JSONL | `--dangerously-bypass-approvals-and-sandbox` | `--sandbox read-only` | `src/codex_adapter.rs` |
| Claude | JSONL | `--dangerously-skip-permissions` | `--permission-mode default` | `src/claude_adapter.rs` |
| Gemini | JSONL/ACP | `--approval-mode=yolo` | (omit yolo) | `src/gemini_adapter.rs` |
| Kimi | JSONL/ACP | `--yolo` | ACP preferred | `src/kimi_adapter.rs` |
| OpenCode | JSONL | Config-driven | Config-driven | `src/opencode_adapter.rs` |
| Hermes | ACP/One-shot | `--yolo` | (omit yolo) | `src/hermes_adapter.rs` |
| Forge | One-shot | Requires `unsafe_args` | Uses safe args | `src/forge_adapter.rs` |
| Claw | One-shot | Config-driven | Config-driven | `src/claw_adapter.rs` |

### Configuration Schema

Provider configuration is expressed in TOML:

```toml
[providers.codex]
kind = "jsonl_command"
exe = "codex"
args = ["exec", "--json", "--cd", "{cwd}", "-"]
prompt_mode = "stdin"
mapper = "codex"
unsafe_args = ["--dangerously-bypass-approvals-and-sandbox"]
safe_args = ["--sandbox", "read-only", "--ask-for-approval", "on-request"]

[providers.claude]
kind = "jsonl_command"
exe = "claude"
args = ["-p", "{prompt}", "--output-format", "stream-json"]
prompt_mode = "arg"
mapper = "claude"
unsafe_args = ["--dangerously-skip-permissions"]
safe_args = ["--permission-mode", "default"]
```

Evidence: `agent-adapter-config/src/loader.rs` (ConfigLoader with built-in defaults), `agent-adapter-config/src/types.rs` (ProviderConfig struct), `agent-adapter-cli/src/run.rs` (provider discovery from config)

### Event Schema

All providers emit a canonical `AgentEvent` stream:

- `RunStarted` - Run lifecycle begins (evidence: `agent-adapter-transports` emits on process start)
- `TextDelta` - Streamed text content (evidence: `agent-adapter-transports/src/jsonl_command.rs`, `agent-adapter-transports/src/acp_stdio.rs`)
- `ToolCallStarted/Delta/Finished` - Tool execution (evidence: `agent-adapter-providers/src/mappers.rs`)
- `FileChange` - File system modifications (evidence: `agent-adapter-core/src/events.rs` FileChange variant)
- `PermissionRequest` - Approval requests (evidence: `agent-adapter-core/src/events.rs` PermissionRequest variant)
- `Usage` - Token/cost tracking (evidence: `agent-adapter-core/src/events.rs` Usage variant)
- `RunCompleted/RunFailed/RunCancelled` - Terminal states (evidence: `agent-adapter-transports` finish method)

### Known Limitations

1. **Workspace isolation** - Git worktree creation is optional/skippable; temp workspace is complete, worktree workspace exists with limited tests
2. **Real provider tests** - Opt-in and gated by provider-specific environment variables
3. **HTTP daemon transport** - Removed from active design; no HTTP daemon mode
4. **Permission/file mapping** - Partial implementation; `PermissionRequest` and `FileChange` event variants exist in `AgentEvent` but mapping coverage varies by provider
5. **Topic coverage is broad rather than page-per-topic** - Some subjects are documented in shared guides instead of dedicated per-topic documents

[1]: https://agentclientprotocol.com/get-started/introduction "Introduction - Agent Client Protocol"
[2]: https://geminicli.com/docs/cli/acp-mode/ "ACP Mode | Gemini CLI"
[3]: https://opencode.ai/docs/cli/ "CLI | OpenCode"
[4]: https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html "kimi Command | Kimi Code CLI Docs"
[5]: https://developers.openai.com/codex/cli/reference "Command line options – Codex CLI | OpenAI Developers"
[6]: https://code.claude.com/docs/en/cli-reference "CLI reference - Claude Code Docs"
[7]: https://hermes-agent.nousresearch.com/docs/reference/cli-commands "CLI Commands Reference | Hermes Agent"
[8]: https://github.com/tailcallhq/forgecode "GitHub - tailcallhq/forgecode: AI enabled pair programmer for Claude, GPT, O Series, Grok, Deepseek, Gemini and 300+ models · GitHub"
[9]: https://github.com/ultraworkers/claw-code? "Claw Code"
