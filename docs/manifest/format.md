# Manifest Format

The `agentverse.toml` manifest file describes your artifact to the registry. It is used by `agentverse publish` to create or update artifacts.

## Full Example

```toml
# agentverse.toml

# ── Package identity ─────────────────────────────────────────────────────────
[package]
kind        = "skill"           # skill | agent | workflow | soul | prompt
namespace   = "myorg"           # your username or org name
name        = "code-linter"     # unique within kind+namespace
description = "A Python code linter skill with AST-based analysis"

# ── Capabilities ─────────────────────────────────────────────────────────────
[capabilities]
input_modalities  = ["text", "json"]
output_modalities = ["text", "json"]
protocols         = ["mcp", "openai-function"]
permissions       = ["network:read", "fs:read"]
max_tokens        = 4096

# ── Dependencies ─────────────────────────────────────────────────────────────
[dependencies]
"python-tools/ast-parser" = ">=1.0.0"
"myorg/base-linter"       = "^2.1.0"

# ── Metadata ─────────────────────────────────────────────────────────────────
[metadata]
tags     = ["python", "linting", "ast", "automation"]
homepage = "https://github.com/myorg/code-linter"
license  = "MIT"
```

## Section Reference

### `[package]`

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `kind` | string | ✅ | Artifact type: `skill`, `agent`, `workflow`, `soul`, or `prompt` |
| `namespace` | string | ✅ | Owner namespace (your username or org) |
| `name` | string | ✅ | Artifact name (lowercase, hyphens, alphanumeric) |
| `description` | string | — | Short description (shown in search results) |

### `[capabilities]`

Stored in the manifest and used for capability-based discovery.

| Key | Type | Description |
|-----|------|-------------|
| `input_modalities` | string[] | Accepted input types: `text`, `json`, `image`, `audio`, `file` |
| `output_modalities` | string[] | Produced output types |
| `protocols` | string[] | Supported protocols: `mcp`, `openai-function`, `a2a`, etc. |
| `permissions` | string[] | Required permissions: `network:read`, `fs:read`, `fs:write`, etc. |
| `max_tokens` | integer | Maximum token context size |

### `[dependencies]`

Declare other AgentVerse artifacts this artifact depends on.

```toml
[dependencies]
"python-tools/ast-parser" = ">=1.0.0, <2.0.0"
"myorg/base-skill"        = "^1.2.0"
```

Version constraint syntax follows SemVer ranges.

### `[metadata]`

| Key | Type | Description |
|-----|------|-------------|
| `tags` | string[] | Searchable tags |
| `homepage` | string | URL to project homepage or GitHub repo |
| `license` | string | SPDX license identifier (e.g. `MIT`, `Apache-2.0`) |

## Artifact Kinds

| Kind | Use Case |
|------|----------|
| `skill` | Reusable tool or capability (code-review, web-scrape, etc.) |
| `agent` | Autonomous AI agent with a defined persona |
| `workflow` | Multi-step orchestration pipeline or DAG |
| `soul` | Personality / persona configuration for an agent |
| `prompt` | Optimized prompt template or chain-of-thought |

## Content File

Alongside `agentverse.toml`, you can provide `content.json` with the actual artifact content (prompt text, agent config, workflow DAG, etc.):

```json
{
  "schema_version": "1.0",
  "system_prompt": "You are a Python code linter...",
  "config": {
    "rules": ["E501", "F401"],
    "max_line_length": 88
  }
}
```

The CLI automatically reads `content.json` from the same directory as the manifest.

## OpenClaw Extension

AgentVerse supports the **OpenClaw** metadata standard for richer skill descriptions. Include it under `[metadata.openclaw]`:

```toml
[metadata]
tags     = ["python", "linting"]
homepage = "https://github.com/myorg/code-linter"
license  = "MIT"

[metadata.openclaw]
name        = "Python Code Linter"
description = "A Python code linter with AST-based analysis"
version     = "1.0.0"
author      = "myorg"

  [[metadata.openclaw.commands]]
  name        = "lint"
  description = "Lint Python source files"

    [[metadata.openclaw.commands.arguments]]
    name        = "files"
    description = "List of Python files to lint"
    required    = true

  [[metadata.openclaw.commands]]
  name        = "check"
  description = "Check a single file and return diagnostics"
```

Skills published with OpenClaw metadata are automatically compatible with the [ClawHub](https://clawhub.dev) skill registry and AI agent tools that use the OpenClaw standard.

## Version Bumping

The `--bump` flag or `default_bump` server config controls versioning:

| Bump | Before | After |
|------|--------|-------|
| `patch` | `1.2.0` | `1.2.1` |
| `minor` | `1.2.0` | `1.3.0` |
| `major` | `1.2.0` | `2.0.0` |

The first publish always starts at `0.1.0`.

## Validation Rules

| Field | Constraint |
|-------|-----------|
| `kind` | Must be one of: `skill`, `agent`, `workflow`, `soul`, `prompt` |
| `namespace` | Lowercase alphanumeric + hyphens; must match your username/org |
| `name` | Lowercase alphanumeric + hyphens; unique within kind + namespace |
| `protocols` | Supported: `mcp`, `openai-function`, `a2a`, `langchain` |
| `permissions` | Supported: `network:read`, `network:write`, `fs:read`, `fs:write` |

