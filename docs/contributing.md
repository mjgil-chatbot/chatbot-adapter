# Contributing

## Prerequisites

- Stable Rust with the pinned toolchain available through `rustup`
- Git

## Local Workflow

Build the workspace:

```bash
cargo build --workspace --release
```

Run the supported verification surface before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --release --workspace --all-features -- -D warnings
cargo test --workspace --release -- --test-threads=1
cargo run -p agent-adapter-cli --release -- acceptance
```

## Documentation Expectations

Changes that affect behavior should update the relevant public docs under
`docs/`. The acceptance command validates documentation links and consistency
against the live workspace.

## Real Provider Tests

Real provider tests are disabled by default. Opt in with the provider-specific
environment variables documented in [testing-real-providers.md](testing-real-providers.md).

## Pull Requests

Before opening a PR:

1. Keep changes scoped to the behavior being changed.
2. Update docs when behavior or operator workflows change.
3. Make sure the verification commands above pass locally.

## Project Layout

- `crates/agent-adapter-core` contains shared protocol types
- `crates/agent-adapter-config` contains config loading and defaults
- `crates/agent-adapter-providers` contains provider-specific adapters
- `crates/agent-adapter-transports` contains process transport implementations
- `crates/agent-adapter-router` contains routing and capability enforcement
- `crates/agent-adapter-cli` contains the user-facing CLI
- `crates/agent-adapter-testkit` contains fake providers and validators
