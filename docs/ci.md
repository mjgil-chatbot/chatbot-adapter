# CI

The GitHub Actions workflow validates the same core surface expected locally:

```bash
cargo fmt --all -- --check
cargo clippy --release --workspace --all-features -- -D warnings
cargo test --workspace --release -- --test-threads=1
cargo run -p agent-adapter-cli --release -- acceptance
cargo build --workspace --release
```

## Workflow Goals

The pinned toolchain includes both `rustfmt` and `clippy` so the verification
commands also work in a fresh local checkout.

- Catch formatting drift
- Keep warnings denied in clippy
- Run the release-mode workspace tests
- Verify documentation and traceability through `agent-adapter-cli acceptance`
- Confirm the workspace still builds in release mode

## Notes

- Real provider tests remain opt-in and are not part of default CI.
- The acceptance command is the canonical docs/design validation entrypoint.
