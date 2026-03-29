# Agent Manifest

An **agent** artifact is a fully-described autonomous AI agent — including its soul (personality),
capabilities (skills it can invoke), protocols it speaks, permissions, and memory configuration.
Agents are composable, shareable, and directly deployable via MCP, A2A, or OpenAI Functions.

## Minimum Example

```toml
[package]
kind        = "agent"
namespace   = "myorg"
name        = "greeter"
description = "A friendly greeting agent"

[[agent.skills]]
namespace = "agentverse"
name      = "base-responder"
version   = ">=0.1.0"

[agent.protocols]
mcp = { enabled = true, version = "2024-11-05" }

[metadata]
tags    = ["agent", "greeting"]
license = "MIT"
```

## Full Example

```toml
[package]
kind        = "agent"
namespace   = "myorg"
name        = "code-assistant"
description = "Autonomous code-assistant: review, test generation, refactor suggestions"

# ── Soul (personality & tone) ─────────────────────────────────────────────────
[agent.soul]
namespace = "agentverse"
name      = "empathetic-counselor"
version   = ">=0.1.0"

# ── Skills (tools the agent can call) ─────────────────────────────────────────
[[agent.skills]]
namespace = "agentverse-ci"
name      = "code-reviewer"
version   = ">=0.1.0"
alias     = "review_code"           # name exposed via MCP / OpenAI tool-call

[[agent.skills]]
namespace = "agentverse-ci"
name      = "release-notes-writer"
version   = ">=0.1.0"
alias     = "write_release_notes"
optional  = true                    # agent still works if skill is unavailable

[[agent.skills]]
namespace = "agentverse-ci"
name      = "api-smoke-tester"
version   = ">=0.1.0"
alias     = "smoke_test"
optional  = true

# ── Prompts ───────────────────────────────────────────────────────────────────
[[agent.prompts]]
namespace = "agentverse"
name      = "chain-of-thought"
version   = ">=0.1.0"
role      = "reasoning"              # default prompt used for multi-step reasoning

# ── Protocols ─────────────────────────────────────────────────────────────────
[agent.protocols]
mcp    = { enabled = true,  version = "2024-11-05" }
a2a    = { enabled = true,  version = "0.2.5" }     # Google Agent-to-Agent standard
openai = { enabled = true,  functions = true }
langchain = { enabled = false }

# ── Permissions (least-privilege) ─────────────────────────────────────────────
[agent.permissions]
network = ["read"]           # network:read | network:write
fs      = ["read"]           # fs:read | fs:write
secrets = []                 # secret keys the agent may access

# ── Memory ────────────────────────────────────────────────────────────────────
[agent.memory]
context_window  = 128000
summarize_at    = 100000
long_term       = { enabled = true, backend = "pgvector" }
episodic        = { enabled = true, max_episodes = 100 }

# ── Model preferences ─────────────────────────────────────────────────────────
[agent.model]
preferred   = ["claude-3-5-sonnet", "gpt-4o", "gemini-1.5-pro"]
temperature = 0.2
max_tokens  = 4096

[metadata]
tags     = ["agent", "code-assistant", "mcp", "a2a", "developer"]
homepage = "https://github.com/myorg/agents"
license  = "MIT"

[metadata.openclaw]
emoji   = "🤖"
version = "0.1.0"
```

## Field Reference

### `[agent.soul]`

| Field       | Type   | Required | Description                                         |
|-------------|--------|----------|-----------------------------------------------------|
| `namespace` | string | —        | Namespace of the soul artifact                      |
| `name`      | string | —        | Name of the soul artifact                           |
| `version`   | string | —        | SemVer constraint                                   |

### `[[agent.skills]]`

| Field       | Type    | Required | Description                                             |
|-------------|---------|----------|---------------------------------------------------------|
| `namespace` | string  | ✅        | Namespace of the skill                                  |
| `name`      | string  | ✅        | Skill name                                              |
| `version`   | string  | ✅        | SemVer version constraint                              |
| `alias`     | string  | —        | Tool name exposed via MCP / OpenAI Functions            |
| `optional`  | boolean | —        | If true, agent continues if skill is unavailable        |

### `[[agent.prompts]]`

| Field       | Type   | Description                                   |
|-------------|--------|-----------------------------------------------|
| `namespace` | string | Namespace of the prompt artifact              |
| `name`      | string | Prompt artifact name                          |
| `version`   | string | SemVer constraint                             |
| `role`      | string | How the agent uses this prompt (`reasoning`, `system`, `tool-use`)|

### `[agent.protocols]`

| Protocol    | Version field  | Description                                          |
|-------------|----------------|------------------------------------------------------|
| `mcp`       | `2024-11-05`   | Model Context Protocol (Anthropic standard)          |
| `a2a`       | `0.2.5`        | Agent-to-Agent (Google standard)                     |
| `openai`    | —              | OpenAI tool-call / function-call format              |
| `langchain` | —              | LangChain agent interface                            |

### `[agent.permissions]`

| Key       | Values                    | Description                       |
|-----------|---------------------------|-----------------------------------|
| `network` | `read`, `write`           | Network access level              |
| `fs`      | `read`, `write`           | Filesystem access level           |
| `secrets` | list of secret key names  | Secrets the agent may read        |

### `[agent.memory]`

| Field              | Type    | Description                                       |
|--------------------|---------|---------------------------------------------------|
| `context_window`   | integer | Maximum token context                             |
| `summarize_at`     | integer | Token threshold for auto-summarisation            |
| `long_term.enabled`| boolean | Enable vector-DB long-term memory                 |
| `long_term.backend`| string  | `pgvector`, `pinecone`, `weaviate`, `qdrant`      |
| `episodic.enabled` | boolean | Store conversation episodes for recall            |

## MCP Integration

Skills listed in `agent.skills` are **automatically exposed as MCP tools**:

```json
{
  "mcpServers": {
    "code-assistant": {
      "command": "agentverse",
      "args": ["run", "--kind", "agent", "--namespace", "myorg", "--name", "code-assistant"],
      "env": { "AGENTVERSE_TOKEN": "${AGENTVERSE_TOKEN}" }
    }
  }
}
```

## A2A Agent Card

When `a2a.enabled = true`, the agent publishes an Agent Card at `/.well-known/agent.json`:

```json
{
  "name": "code-assistant",
  "description": "Autonomous code-assistant agent",
  "version": "0.1.0",
  "url": "https://agentverse.example.com/agents/myorg/code-assistant",
  "capabilities": { "streaming": true, "pushNotifications": false },
  "skills": [
    { "id": "review_code",         "name": "Review Code",         "description": "Analyse a diff or PR" },
    { "id": "write_release_notes", "name": "Write Release Notes", "description": "Draft release notes" }
  ]
}
```

## Standards Compatibility

| Standard         | Support | Notes                                        |
|------------------|---------|----------------------------------------------|
| MCP 2024-11-05   | ✅       | Full tool-call; skills auto-registered       |
| Google A2A 0.2.5 | ✅       | Agent card + task handoff protocol           |
| OpenAI Functions | ✅       | Function-call JSON schema from skill specs   |
| LangChain Agent  | ✅       | Wrap skills as `Tool` objects                |
| CrewAI           | ✅       | Register as a `CrewAI` agent member          |
| AutoGen          | ✅       | Compatible via OpenAI functions interface    |

## Publishing

```bash
agentverse publish --file agent.toml
# → Published agent myorg/code-assistant@0.1.0

# Run via CLI
agentverse run --kind agent --namespace myorg --name code-assistant
```

