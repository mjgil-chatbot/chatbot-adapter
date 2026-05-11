# Testing Guide

## Test Layers

- Unit tests live alongside crate code.
- CLI integration tests live under `crates/agent-adapter-cli/tests/`.
- `agent-adapter-testkit` provides fake providers, fixtures, and validation
  helpers for the workspace.

## Default Local Test Surface

```bash
cargo test --workspace --release -- --test-threads=1
```

## Acceptance Validation

The CLI exposes the repository-level acceptance gate:

```bash
cargo run -p agent-adapter-cli --release -- acceptance
```

That command runs the live-workspace validators for:

- documentation link and consistency checks
- design traceability checks against actual symbols and tests

## Targeted Runs

Useful focused commands:

```bash
cargo test --release -p agent-adapter-cli
cargo test --release -p agent-adapter-testkit
```

## Real Provider Tests

Real provider tests stay disabled by default. Enable them only when you want to
exercise a specific installed provider binary:

```bash
RUN_REAL_CLAUDE=1 cargo test --release -p agent-adapter-testkit
```

Available gates are documented in [testing-real-providers.md](testing-real-providers.md).
