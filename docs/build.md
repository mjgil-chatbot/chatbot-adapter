# Build Instructions

## Requirements

- Rust toolchain from `rust-toolchain.toml`
- Cargo

Some Linux environments may also need `clang`, `mold`, `pkg-config`, and
OpenSSL development headers depending on linker and crate resolution.

## Workspace Build

```bash
cargo build --workspace --release
```

For faster feedback while developing:

```bash
cargo build --release --workspace
```

## Verification Build

The release-aligned local verification flow is:

```bash
cargo fmt --all -- --check
cargo clippy --release --workspace --all-features -- -D warnings
cargo test --workspace --release -- --test-threads=1
cargo run -p agent-adapter-cli --release -- acceptance
cargo build --workspace --release
```

## Toolchain

The repository pins its Rust toolchain in `rust-toolchain.toml`. If your local
toolchain is out of date, refresh it with `rustup`. The pinned toolchain declares
both `rustfmt` and `clippy` for the verification commands above.

## Troubleshooting

If linking fails on Linux, install the linker/tooling expected by
`.cargo/config.toml` or override it in your local environment.
