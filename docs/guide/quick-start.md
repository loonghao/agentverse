# Quick Start

Get AgentVerse running in under 5 minutes.

## Option 1: Docker Compose (Recommended)

The fastest way to spin up a full stack locally (PostgreSQL + Redis + MinIO + Server):

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse
docker compose up -d
```

Verify everything is healthy:

```bash
docker compose ps
curl http://localhost:8080/health
# {"status":"ok","version":"x.y.z"}
```

**Default endpoints:**

| Endpoint | URL |
|----------|-----|
| REST API | `http://localhost:8080` |
| Swagger UI | `http://localhost:8080/swagger-ui/` |
| MCP | `http://localhost:8080/mcp` |
| MinIO Console | `http://localhost:9001` (admin / minioadmin123) |

## Option 2: Install CLI Only

If you just want to interact with an existing server:

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-aarch64-apple-darwin.tar.gz | tar -xz
sudo mv agentverse /usr/local/bin/

# macOS (Intel) / Linux x86_64
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-x86_64-unknown-linux-gnu.tar.gz | tar -xz
sudo mv agentverse /usr/local/bin/

# Windows (PowerShell)
irm https://github.com/loonghao/agentverse/releases/latest/download/agentverse-x86_64-pc-windows-msvc.zip -OutFile agentverse.zip
Expand-Archive agentverse.zip -DestinationPath "$env:LOCALAPPDATA\agentverse"
```

## Your First Skill

### 1. Register & Login

```bash
# Point the CLI at your server (or use the default localhost)
export AGENTVERSE_URL=http://localhost:8080

agentverse register myusername --email me@example.com --password "MyPass123!"
agentverse login myusername --password "MyPass123!"
agentverse whoami
```

### 2. Create a Manifest

```bash
mkdir my-first-skill && cd my-first-skill
cat > agentverse.toml << 'EOF'
[package]
kind        = "skill"
namespace   = "myorg"
name        = "hello-world"
description = "My first AgentVerse skill"

[capabilities]
input_modalities  = ["text"]
output_modalities = ["text"]
protocols         = ["mcp"]
permissions       = []

[metadata]
tags = ["demo", "hello"]
EOF
```

### 3. Publish

```bash
agentverse publish
# ✓ Published skill/myorg/hello-world  v0.1.0
```

### 4. Search & Get

```bash
agentverse search "hello world"
agentverse get skill/myorg/hello-world
agentverse get skill/myorg/hello-world@0.1.0  # pinned version
```

## Next Steps

- [CLI Reference](/cli/) — All commands and flags
- [Manifest Format](/manifest/format) — Full `agentverse.toml` schema
- [Storage Backends](/storage/) — Configure S3, COS, or GitHub Releases
- [Server Configuration](/server/configuration) — Production deployment

