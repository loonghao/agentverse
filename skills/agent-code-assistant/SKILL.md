---
name: agent-code-assistant
kind: agent
description: >
  An autonomous code-assistant agent that can review code, answer technical questions,
  generate tests, and suggest refactors — powered by AgentVerse skills and an
  empathetic developer soul. Exposes MCP tools and is A2A (Agent-to-Agent) compatible.
tags: [agent, code-assistant, mcp, a2a, developer, autonomous]
version: "0.1.0"
author: agentverse
license: MIT
metadata:
  openclaw:
    homepage: https://github.com/loonghao/agentverse
    emoji: "🤖"
    requires:
      bins:
        - agentverse
---

# Agent: Code Assistant

An **agent** artifact is a fully-described autonomous AI agent — including its soul,
capabilities (skills it can invoke), protocols it speaks, and security permissions.

## What is an Agent?

| Dimension     | Description                                                  |
|---------------|--------------------------------------------------------------|
| `soul`        | Personality and tone (links to a `soul` artifact)            |
| `skills`      | Tools / capabilities the agent can invoke                    |
| `protocols`   | Communication protocols: MCP, A2A, OpenAI-function, etc.     |
| `permissions` | What the agent is allowed to do (fs, network, secrets)       |
| `triggers`    | How the agent is activated (message, event, schedule)        |
| `memory`      | Context window strategy and long-term memory config          |

## Agent Manifest (`agent.toml`)

```toml
[package]
kind        = "agent"
namespace   = "agentverse"
name        = "code-assistant"
description = "Autonomous code-assistant: review, test generation, refactor suggestions"

# ── Soul (personality) ────────────────────────────────────────────────────────
[agent.soul]
namespace = "agentverse"
name      = "empathetic-counselor"
version   = ">=0.1.0"

# ── Skills (tools the agent can call) ─────────────────────────────────────────
[[agent.skills]]
namespace = "agentverse-ci"
name      = "code-reviewer"
version   = ">=0.1.0"
alias     = "review_code"

[[agent.skills]]
namespace = "agentverse-ci"
name      = "release-notes-writer"
version   = ">=0.1.0"
alias     = "write_release_notes"

[[agent.skills]]
namespace   = "agentverse-ci"
name        = "api-smoke-tester"
version     = ">=0.1.0"
alias       = "smoke_test"
optional    = true

# ── Protocols ─────────────────────────────────────────────────────────────────
[agent.protocols]
mcp     = { enabled = true, version = "2024-11-05" }
a2a     = { enabled = true, version = "0.2.5" }          # Google A2A standard
openai  = { enabled = true, functions = true }

# ── Permissions ───────────────────────────────────────────────────────────────
[agent.permissions]
network = ["read"]
fs      = ["read"]
secrets = []

# ── Memory ────────────────────────────────────────────────────────────────────
[agent.memory]
context_window = 128000
long_term      = { enabled = true, backend = "pgvector" }
summarize_at   = 100000     # tokens before auto-summarisation kicks in

# ── Model hints ───────────────────────────────────────────────────────────────
[agent.model]
preferred    = ["claude-3-5-sonnet", "gpt-4o", "gemini-1.5-pro"]
temperature  = 0.2
max_tokens   = 4096

[metadata]
tags     = ["agent", "code-assistant", "mcp", "a2a", "developer"]
homepage = "https://github.com/loonghao/agentverse"
license  = "MIT"
```

## AgentVerse Content (`content.json`)

```json
{
  "schema_version": "1.0",
  "kind": "agent",
  "soul": { "namespace": "agentverse", "name": "empathetic-counselor", "version": ">=0.1.0" },
  "skills": [
    { "namespace": "agentverse-ci", "name": "code-reviewer",       "version": ">=0.1.0", "alias": "review_code" },
    { "namespace": "agentverse-ci", "name": "release-notes-writer","version": ">=0.1.0", "alias": "write_release_notes" }
  ],
  "protocols": {
    "mcp":    { "enabled": true, "version": "2024-11-05" },
    "a2a":    { "enabled": true, "version": "0.2.5" },
    "openai": { "enabled": true, "functions": true }
  },
  "permissions": { "network": ["read"], "fs": ["read"], "secrets": [] },
  "memory": { "context_window": 128000, "long_term": { "enabled": true } },
  "model": { "preferred": ["claude-3-5-sonnet", "gpt-4o"], "temperature": 0.2 }
}
```

## Deploy and Run

```bash
# Publish the agent to AgentVerse
agentverse publish --file agent.toml

# Launch via MCP
agentverse get --kind agent --namespace agentverse --name code-assistant
```

## MCP Integration

```json
{
  "mcpServers": {
    "code-assistant": {
      "command": "agentverse",
      "args": ["run", "--kind", "agent", "--namespace", "agentverse", "--name", "code-assistant"],
      "env": { "AGENTVERSE_TOKEN": "${AGENTVERSE_TOKEN}" }
    }
  }
}
```

## A2A (Agent-to-Agent) Protocol

This agent publishes an **A2A Agent Card** at `/.well-known/agent.json`:

```json
{
  "name": "code-assistant",
  "description": "Autonomous code-assistant agent",
  "version": "0.1.0",
  "capabilities": { "streaming": true, "pushNotifications": false },
  "skills": [
    { "id": "review_code",         "name": "Review Code",         "description": "Review a diff or PR" },
    { "id": "write_release_notes", "name": "Write Release Notes", "description": "Draft release notes from commits" }
  ]
}
```

## Standards Compatibility

| Standard           | Support | Notes                                    |
|--------------------|---------|------------------------------------------|
| MCP 2024-11-05     | ✅       | Full tool-call protocol                  |
| Google A2A 0.2.5   | ✅       | Agent card + task handoff                |
| OpenAI Functions   | ✅       | Function-call JSON schema                |
| LangChain Agent    | ✅       | Can wrap skills as LangChain tools       |
| CrewAI             | ✅       | Agent can be registered as CrewAI member |

## Notes

- A soul is **optional** but strongly recommended for consistent user experience.
- Skills listed under `agent.skills` are exposed as MCP tools automatically.
- A2A agent cards enable peer-to-peer task delegation between agents.
- Use `permissions` to enforce a least-privilege security model.

