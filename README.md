<div align="center">

# 🌌 AgentVerse

**The Universal Marketplace for AI Agent Ecosystems**

*Publish, discover, and compose AI skills, agents, workflows, souls, and more — all in one place.*

[![CI](https://github.com/loonghao/agentverse/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/agentverse/actions/workflows/ci.yml)
[![Release](https://github.com/loonghao/agentverse/actions/workflows/release-please.yml/badge.svg)](https://github.com/loonghao/agentverse/actions/workflows/release-please.yml)
[![Docker](https://ghcr-badge.egpl.dev/loonghao/agentverse/latest_tag?color=%2344cc11&ignore=latest&label=docker)](https://github.com/loonghao/agentverse/pkgs/container/agentverse)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

[中文文档](README_zh.md) · [Deployment Guide](docs/deployment.md) · [Usage Guide](docs/usage.md) · [API Docs](https://github.com/loonghao/agentverse#api)

</div>

---

## ✨ What is AgentVerse?

AgentVerse is an open-source, self-hostable registry and marketplace for everything AI agents need. Think of it as **npm for AI** — but designed from the ground up to handle not just code, but the full spectrum of agent ecosystem artifacts:

| Kind | Description | Example |
|------|-------------|---------|
| 🔧 **Skill** | Reusable capabilities and tools | A web-scraping tool, a code-review function |
| 🤖 **Agent** | Autonomous AI agents with defined personas | A customer-support agent, a QA engineer agent |
| 🔄 **Workflow** | Multi-step orchestration pipelines | A CI/CD pipeline, a data-processing DAG |
| 👤 **Soul** | Personality and persona configurations | An empathetic counselor personality |
| 💬 **Prompt** | Optimized prompt templates | Chain-of-thought prompts, system prompts |

**Built for the future** — the extensible artifact model means new kinds can be registered without breaking existing clients.

## 🚀 Quick Start

### Option 1: Docker Compose (Recommended)

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse
docker compose up -d
```

The server is now available at `http://localhost:8080`.

### Option 2: Download CLI Binary

```bash
# macOS / Linux
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-$(uname -m)-apple-darwin.tar.gz | tar -xz
./agentverse --help

# Windows (PowerShell)
irm https://github.com/loonghao/agentverse/releases/latest/download/agentverse-x86_64-pc-windows-msvc.zip -OutFile agentverse.zip
Expand-Archive agentverse.zip
```

### Option 3: Docker Image

```bash
docker pull ghcr.io/loonghao/agentverse:latest
docker run -d \
  -e DATABASE_URL=postgres://... \
  -e JWT_SECRET=your-secret \
  -p 8080:8080 \
  ghcr.io/loonghao/agentverse:latest
```

## 🎯 CLI Usage

```bash
# Search for anything
agentverse search --query "code review" --kind skill

# Publish your skill
agentverse publish --file skill.toml

# Get a specific artifact
agentverse get --kind agent --namespace myorg --name code-reviewer

# Social features
agentverse like --kind skill --namespace python-tools --name linter
agentverse rate --kind workflow --namespace ops --name deploy --stars 5

# Agent machine-to-machine
agentverse learn --kind skill --id <uuid> --insight "Works well for Python 3.12+"
agentverse benchmark --kind agent --id <uuid> --score 0.92
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     AgentVerse Platform                      │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  REST API   │  │  GraphQL    │  │    MCP Protocol     │ │
│  │  (OpenAPI)  │  │  Endpoint   │  │  (AI agent native)  │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         └────────────────┼──────────────────────┘           │
│                   ┌──────┴──────┐                           │
│                   │  Core Logic │                           │
│                   │  + Auth/JWT │                           │
│                   └──────┬──────┘                           │
│         ┌────────────────┼────────────────┐                 │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌──────┴──────┐         │
│  │ PostgreSQL  │  │    Redis    │  │    MinIO    │         │
│  │ + pgvector  │  │   Cache     │  │  Artifacts  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

**Key Features:**
- 🔒 **JWT Authentication** with Ed25519 signed artifacts
- 🔍 **Full-text + Semantic search** (pgvector embeddings)
- 📦 **Semantic versioning** with automatic bump inference
- 👥 **Social layer**: comments, likes, ratings, forks
- 🤖 **MCP-native**: AI agents interact via Model Context Protocol
- 📊 **Event sourcing**: full audit trail and analytics
- 🌐 **OpenAPI + GraphQL**: developer-friendly APIs

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Deployment Guide](docs/deployment.md) | Docker, Kubernetes, bare-metal setup |
| [Usage Guide](docs/usage.md) | CLI commands, API examples, manifest format |
| [API Reference](http://localhost:8080/swagger-ui/) | Interactive OpenAPI docs (when running) |

## 🛠️ Development

```bash
# Clone and setup
git clone https://github.com/loonghao/agentverse.git
cd agentverse

# Start dev dependencies
docker compose up postgres redis minio -d

# Run tests
just test

# Build release
just build-release

# Format and lint
just ci
```

### Requirements

- Rust 1.75+
- PostgreSQL 17 with pgvector extension
- Redis 7+
- MinIO (or any S3-compatible storage)

## 🗺️ Roadmap

- [ ] Web UI marketplace dashboard
- [ ] Homebrew/Scoop/Chocolatey install scripts
- [ ] Federated registry support (cross-instance discovery)
- [ ] Plugin SDK for custom artifact kinds
- [ ] OAuth2/OIDC integration
- [ ] AI-powered semantic search enhancements
- [ ] Rate limiting and quota management

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

