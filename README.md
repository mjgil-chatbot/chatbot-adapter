# agent-adapter

`agent-adapter` is a Rust workspace for running multiple AI coding assistant CLIs
behind one consistent interface. It normalizes provider-specific command lines,
transport formats, and event streams so callers can route requests without
special-casing each provider.

Supported providers currently include Codex, Claude, Gemini, Kimi, OpenCode,
Hermes, Forge, and Claw.

## Architecture

The workspace keeps responsibilities split by crate:

- `agent-adapter-core` for shared request, event, capability, and error types
- `agent-adapter-config` for provider configuration loading and defaults
- `agent-adapter-providers` for provider-specific command builders and mappers
- `agent-adapter-transports` for JSONL, one-shot, ACP, and PTY execution paths
- `agent-adapter-router` for provider selection and capability checks
- `agent-adapter-cli` for the user-facing CLI
- `agent-adapter-testkit` for fake providers and validator utilities

More detail is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
[docs/design.md](docs/design.md).

## Build

Install the pinned Rust toolchain, then run:

```bash
cargo build --workspace --release
```

Build and environment details are documented in [docs/build.md](docs/build.md).

## Test

The supported local verification surface is:

```bash
cargo fmt --all -- --check
cargo clippy --release --workspace --all-features -- -D warnings
cargo test --workspace --release -- --test-threads=1
cargo run -p agent-adapter-cli --release -- acceptance
cargo build --workspace --release
```

Test strategy and CI expectations are in [docs/testing.md](docs/testing.md) and
[docs/ci.md](docs/ci.md).

## Usage

```bash
agent-adapter-cli run --provider claude --prompt "Fix the failing test"
agent-adapter-cli run --safe --provider codex --prompt "Review this change"
agent-adapter-cli providers --json
agent-adapter-cli doctor
```

The CLI surface is documented in [docs/cli.md](docs/cli.md).

## Configuration

Provider configuration, built-in defaults, and schema details are documented in
[docs/cli.md](docs/cli.md) and [docs/design.md](docs/design.md).

## Release Notes

Public release process notes live in [docs/release.md](docs/release.md).

## License

This project is released under the 0BSD license. See [LICENSE](LICENSE).

Third-party components retain their own terms.
