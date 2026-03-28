# Introduction

## What is AgentVerse?

**AgentVerse** is an open-source, self-hostable registry and marketplace for everything AI agents need. Think of it as **npm for AI** — but designed from the ground up to handle not just code, but the full spectrum of agent ecosystem artifacts.

| Kind | Description | Example |
|------|-------------|---------|
| 🔧 **Skill** | Reusable capabilities and tools | A web-scraping tool, a code-review function |
| 🤖 **Agent** | Autonomous AI agents with defined personas | A customer-support agent, a QA engineer agent |
| 🔄 **Workflow** | Multi-step orchestration pipelines | A CI/CD pipeline, a data-processing DAG |
| 👤 **Soul** | Personality and persona configurations | An empathetic counselor personality |
| 💬 **Prompt** | Optimized prompt templates | Chain-of-thought prompts, system prompts |

**Built for the future** — the extensible artifact model means new kinds can be registered without breaking existing clients.

## Key Features

### 🔒 Authentication & Security
- JWT-based authentication with refresh tokens
- Ed25519 signed artifact checksums
- Optional email verification before publishing
- Fine-grained permission control (owner-only writes)

### 🔍 Discovery
- Full-text search across all artifact metadata
- Semantic vector search powered by **pgvector** embeddings
- Filter by kind, namespace, tag, or author
- Trending artifacts by downloads and social activity

### 📦 Versioning
- Strict **SemVer** enforcement
- Automatic version bump inference from content diff
- Pinned version fetching (`@1.2.0`)
- Complete version history with changelogs

### 👥 Social Layer
- Comments with threaded replies
- Likes and unlike
- 1–5 star ratings with review text
- Per-artifact social statistics

### 🤖 MCP Native
- Model Context Protocol endpoint at `/mcp`
- AI agents can search, get, and publish artifacts directly
- No custom tooling required — use any MCP-compatible client

### ☁️ Flexible Storage
Any S3-compatible service, GitHub Releases, or a custom HTTP endpoint for storing artifact packages. See [Storage Backends](/storage/) for details.

## Architecture

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
│  │ PostgreSQL  │  │    Redis    │  │  Object     │         │
│  │ + pgvector  │  │   Cache     │  │  Store      │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

## Next Steps

- [Quick Start](/guide/quick-start) — Get up and running in 5 minutes
- [CLI Reference](/cli/) — Full CLI command reference
- [Storage Backends](/storage/) — Configure where packages are stored

