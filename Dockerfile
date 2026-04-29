# Docker image for building agent-adapter
FROM rust:1.94.1-slim

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    mold \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY rust-toolchain.toml rustfmt.toml .cargo/ ./

RUN rustup override set 1.94.1 && \
    rustup component add rustfmt clippy

ENV CARGO_TERM_COLOR=always
ENV RUSTFLAGS="-D warnings"

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates crates/

RUN cargo build --release --dry-run 2>/dev/null || true

CMD ["cargo", "build", "--release"]
