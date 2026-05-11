# Release Process

## Pre-Release Verification

Run the full local verification surface:

```bash
cargo fmt --all -- --check
cargo clippy --release --workspace --all-features -- -D warnings
cargo test --workspace --release -- --test-threads=1
cargo run -p agent-adapter-cli --release -- acceptance
cargo build --workspace --release
```

## Versioning

The workspace version and license metadata live in `Cargo.toml`. Update the
version there when preparing a tagged release.

## Release Checklist

1. Update `CHANGELOG.md`.
2. Run the verification commands above.
3. Tag the release.
4. Publish source and any desired binary artifacts.

## Installation

Source installation is always available:

```bash
cargo install --path crates/agent-adapter-cli
```

If you publish release artifacts, document the download location in the release
notes for that tag instead of keeping a placeholder URL here.
