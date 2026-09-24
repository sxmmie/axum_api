# syntax=docker/dockerfile:1.7

# ---- Stage 1: chef base (shared tooling) ----
FROM rust:1.82-slim-bookworm AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

# ---- Stage 2: plan dependencies ----
# This produces a "recipe" describing exactly which deps are needed,
# so the expensive dependency-compile step only reruns when Cargo.toml/lock change.
FROM check AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# ---- Stage 3: build ----
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cache dependency compilation. This layer is only invalidated when
# Cargo.toml/Cargo.lock change, NOT on every source code change.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

# Now copy actual source and build the real binary.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin axum_api && \
    cp target/release/axum_api /app/axum_api

# ---- Stage 4: runtime ----
# Debian slim rather than distroless/scratch so we retain a shell + apt
# for occasional debugging (exec into container). Swap to gcr.io/distroless/cc
# once the app is stable and you want a smaller/harder attack surface.
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 appuser \
    && useradd --uid 1000 --gid appuser --shell /bin/bash --create-home appuser

WORKDIR /app

COPY --from=builder /app/axum_api /app/axum_api
COPY --from=builder /app/migrations /app/migrations

RUN chown -R appuser:appuser /app
USER appuser

EXPOSE 3000

# Requires a GET /health route returning 200 (add this to routes/mod.rs if
# you don't have one yet — trivial handler that returns StatusCode::OK).
HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 CMD curl http://localhost:3000/health || exit 1

ENTRYPOINT [ "/app/axum_api" ]
