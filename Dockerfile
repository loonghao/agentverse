# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.88-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY apps/server/Cargo.toml ./apps/server/
COPY apps/cli/Cargo.toml    ./apps/cli/
COPY crates/agentverse-core/Cargo.toml        ./crates/agentverse-core/
COPY crates/agentverse-storage/Cargo.toml     ./crates/agentverse-storage/
COPY crates/agentverse-api/Cargo.toml         ./crates/agentverse-api/
COPY crates/agentverse-auth/Cargo.toml        ./crates/agentverse-auth/
COPY crates/agentverse-versioning/Cargo.toml  ./crates/agentverse-versioning/
COPY crates/agentverse-social/Cargo.toml      ./crates/agentverse-social/
COPY crates/agentverse-search/Cargo.toml      ./crates/agentverse-search/
COPY crates/agentverse-events/Cargo.toml      ./crates/agentverse-events/

# Create stub lib files to build deps only
RUN mkdir -p apps/server/src apps/cli/src \
    crates/agentverse-core/src \
    crates/agentverse-storage/src \
    crates/agentverse-api/src \
    crates/agentverse-auth/src \
    crates/agentverse-versioning/src \
    crates/agentverse-social/src \
    crates/agentverse-search/src \
    crates/agentverse-events/src && \
    echo "fn main(){}" > apps/server/src/main.rs && \
    echo "fn main(){}" > apps/cli/src/main.rs && \
    for d in core storage api auth versioning social search events; do \
        echo "" > crates/agentverse-$d/src/lib.rs; \
    done

RUN cargo build --release --bin agentverse-server

# Copy real source and rebuild (only changed code recompiles)
COPY . .
RUN touch apps/server/src/main.rs && \
    cargo build --release --bin agentverse-server

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

