# ── Stage 0: Base image with system deps + cargo-chef ────────────────────────
# cargo-chef separates dependency compilation from application code compilation,
# so that Docker layer caching works correctly and build scripts (e.g. the
# utoipa-swagger-ui downloader) run exactly once per dependency change.
FROM rust:1.88-slim AS chef

WORKDIR /app

# pkg-config + libssl-dev: required by TLS crates (rustls, ring, openssl-sys)
# curl: required by utoipa-swagger-ui build script to download Swagger UI assets
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-chef --locked

# ── Stage 1: Compute the dependency recipe ───────────────────────────────────
FROM chef AS planner

COPY . .
# Produce a minimal recipe.json that captures only the dependency graph,
# not application source code, so the cook layer is invalidated only when
# Cargo.toml / Cargo.lock change.
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 2: Cook (pre-build) all dependencies ────────────────────────────────
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

# Build every dependency declared in the workspace.
# This is the layer that's cached: utoipa-swagger-ui downloads Swagger UI
# assets here (via curl), and the result is reused on every subsequent build
# that doesn't touch Cargo.toml / Cargo.lock.
RUN cargo chef cook --release --recipe-path recipe.json

# Copy real application source and compile only the changed code.
# Dependencies are already built in the layer above — only workspace crates
# and the final binary need to be compiled here.
COPY . .
RUN cargo build --release --bin agentverse-server

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# ca-certificates: TLS connections to PostgreSQL/Redis/MinIO
# curl: used by docker-compose healthcheck
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/agentverse-server /app/agentverse-server
COPY --from=builder /app/config /app/config
# Migrations are embedded at compile time via sqlx::migrate!, but we keep
# the directory here as a reference and for potential runtime tooling.
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=5 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/agentverse-server"]

