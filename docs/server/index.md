# Server Overview

The AgentVerse server is a single statically-linked binary (`agentverse-server`) that exposes:

- **REST API** with full OpenAPI/Swagger documentation
- **GraphQL** endpoint
- **MCP** (Model Context Protocol) endpoint
- **Static file serving** for local object-store packages

## Requirements

| Dependency | Version | Notes |
|-----------|---------|-------|
| PostgreSQL | 17+ | With `pgvector` extension |
| Redis | 7+ | Caching and optional rate-limiting |
| Object Store | Any | S3-compatible, GitHub Releases, or local disk |

## Running the Server

### Docker (Quickest)

```bash
docker run -d \
  -p 8080:8080 \
  -e DATABASE_URL="postgres://user:pass@db:5432/agentverse" \
  -e REDIS_URL="redis://redis:6379" \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  ghcr.io/loonghao/agentverse:latest
```

### Docker Compose (Recommended for Development)

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse
docker compose up -d
```

### Binary

```bash
# Download from GitHub Releases
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-server-x86_64-unknown-linux-gnu.tar.gz | tar -xz -C /usr/local/bin

DATABASE_URL="postgres://..." JWT_SECRET="..." agentverse-server
```

## Health Checks

```bash
curl http://localhost:8080/health
# {"status":"ok","version":"x.y.z"}

curl http://localhost:8080/ready
# {"status":"ready","checks":{"database":"ok","redis":"ok"}}
```

## Sections

- [Configuration](/server/configuration) — Full config file and environment variable reference
- [Deployment](/server/deployment) — Docker Compose, Kubernetes, bare metal, reverse proxy
- [API Reference](/server/api) — REST endpoints, GraphQL, MCP

