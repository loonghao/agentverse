---
name: agentverse-cli
kind: skill
description: "Publish, discover, and manage AI skills, agents, workflows, souls and prompts from the AgentVerse marketplace. Use when working with the agentverse CLI to search/publish artifacts, authenticate, or manage AI agent ecosystem components."
version: 0.1.4
metadata:
  openclaw:
    homepage: https://github.com/loonghao/agentverse
    emoji: "🤖"
    requires:
      bins:
        - agentverse
    install:
      - kind: shell
        linux: "curl -fsSL https://raw.githubusercontent.com/loonghao/agentverse/main/install.sh | bash"
        macos: "curl -fsSL https://raw.githubusercontent.com/loonghao/agentverse/main/install.sh | bash"
        windows: "irm https://raw.githubusercontent.com/loonghao/agentverse/main/install.ps1 | iex"
---

# AgentVerse CLI

**AgentVerse** (`agentverse`) is the CLI for the universal AI agent marketplace — publish, discover, and manage skills, agents, workflows, souls and prompts.

## Installation

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/agentverse/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/loonghao/agentverse/main/install.ps1 | iex
```

## What is AgentVerse?

| Kind | Description | Example |
|------|-------------|---------|
| `skill` | Reusable capabilities (tools, functions) | code-reviewer, api-smoke-tester |
| `agent` | Autonomous AI agents with defined capabilities | code-assistant, qa-engineer |
| `workflow` | Multi-step orchestration pipelines | ci-review-pipeline, release-workflow |
| `soul` | Persona and personality configurations | empathetic-counselor, developer-buddy |
| `prompt` | Optimized prompt templates | chain-of-thought, self-ask |

### Kind Manifest Docs

| Kind | Manifest Guide |
|------|----------------|
| `skill` | [Skill format](https://github.com/loonghao/agentverse/blob/main/docs/manifest/format.md) |
| `soul` | [Soul format](https://github.com/loonghao/agentverse/blob/main/docs/manifest/soul.md) |
| `prompt` | [Prompt format](https://github.com/loonghao/agentverse/blob/main/docs/manifest/prompt.md) |
| `workflow` | [Workflow format](https://github.com/loonghao/agentverse/blob/main/docs/manifest/workflow.md) |
| `agent` | [Agent format](https://github.com/loonghao/agentverse/blob/main/docs/manifest/agent.md) |

## Quick Reference

### Discovery

```bash
# Search across all artifact kinds
agentverse search --query "code review"
agentverse search --query "python" --kind skill

# List by kind / namespace
agentverse list --kind agent
agentverse list --kind skill --namespace myorg

# Get a specific artifact (latest version)
agentverse get --kind skill --namespace myorg --name my-skill

# Pin to a specific version
agentverse get --kind skill --namespace myorg --name my-skill --version 1.2.0

# Show version history
agentverse versions --kind skill --namespace python-tools --name linter
```

### Publishing

```bash
# Publish a new artifact or new version
agentverse publish --file skill.toml

# Update metadata / content
agentverse update --kind skill --namespace myorg --name my-skill --file skill.toml

# Fork an artifact
agentverse fork --kind skill --namespace source-org --name original \
  --new-namespace myorg --new-name my-fork

# Deprecate (soft delete)
agentverse deprecate --kind skill --namespace myorg --name old-skill
```

### Authentication

```bash
# Point to a custom AgentVerse server
export AGENTVERSE_URL=https://agentverse.example.com
agentverse login

# Register a new account
agentverse register --username alice --email alice@example.com

# Show current user
agentverse whoami
```

### Social

```bash
# Rate an artifact (1–5 stars)
agentverse rate --kind skill --namespace myorg --name my-skill --stars 5

# Like / unlike
agentverse like   --kind skill --namespace myorg --name my-skill
agentverse unlike --kind skill --namespace myorg --name my-skill

# Post a comment
agentverse comment --kind skill --namespace myorg --name my-skill \
  --message "Great tool!"

# View social stats
agentverse stats --kind skill --namespace myorg --name my-skill
```

### Agent Use (Programmatic)

```bash
# Record a learning insight
agentverse learn --kind skill --namespace myorg --name my-skill \
  --insight "Works well for Python 3.12"

# Submit benchmark results
agentverse benchmark --kind skill --namespace myorg --name my-skill \
  --score 0.95 --metric accuracy
```

### Self-Update

```bash
# Check for newer version without installing
agentverse self-update --check

# Update to the latest release
agentverse self-update

# Use a GitHub token to avoid rate limits
agentverse self-update --token ghp_your_token
```

### Working with Souls

Souls define the personality and behavioural constraints of an AI agent.

```bash
# Publish a soul
agentverse publish --file soul.toml

# Get a soul (returns content.json + manifest)
agentverse get --kind soul --namespace agentverse --name empathetic-counselor

# Search available souls
agentverse search --query "empathetic support" --kind soul

# List all souls in a namespace
agentverse list --kind soul --namespace myorg
```

### Working with Prompts

Prompts are versioned, reusable LLM instruction templates.

```bash
# Publish a prompt template
agentverse publish --file prompt.toml

# Retrieve as JSON (for programmatic use)
agentverse get --kind prompt --namespace agentverse --name chain-of-thought --format json

# Search prompt templates
agentverse search --query "chain of thought reasoning" --kind prompt
```

### Working with Workflows

Workflows are DAG pipelines that compose skills and agents.

```bash
# Publish a workflow
agentverse publish --file workflow.toml

# Get and export as GitHub Actions
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format github-actions > .github/workflows/ci.yml

# Export as Argo Workflows
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format argo-workflow > ci-review.argo.yaml

# Run a workflow manually
agentverse run --kind workflow --namespace myorg --name ci-review-pipeline \
  --input pr_url=https://github.com/org/repo/pull/42
```

### Working with Agents

Agents are autonomous AI entities combining soul + skills + protocols.

```bash
# Publish an agent
agentverse publish --file agent.toml

# Run an agent (MCP server mode)
agentverse run --kind agent --namespace myorg --name code-assistant

# Get agent details (includes A2A agent card)
agentverse get --kind agent --namespace myorg --name code-assistant --format json
```

### OpenClaw Soul Agent Integration

```yaml
# openclaw-config.yaml — attach an AgentVerse soul to a Soul Agent
soul:
  source: agentverse
  namespace: agentverse
  name: empathetic-counselor
  version: ">=0.1.0"
agent:
  system_prompt: "{{soul.system_prompt}}"
  tone: "{{soul.tone}}"
  constraints: "{{soul.constraints}}"
```

## Global Flags

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--server` | `AGENTVERSE_URL` | `http://localhost:8080` | Server URL |
| `--token` | `AGENTVERSE_TOKEN` | — | Bearer token for authenticated ops |

## Links

- **Repository**: https://github.com/loonghao/agentverse
- **Docker Image**: `ghcr.io/loonghao/agentverse:latest`
- **Manifest Docs**: [Skill](https://github.com/loonghao/agentverse/blob/main/docs/manifest/format.md) · [Soul](https://github.com/loonghao/agentverse/blob/main/docs/manifest/soul.md) · [Prompt](https://github.com/loonghao/agentverse/blob/main/docs/manifest/prompt.md) · [Workflow](https://github.com/loonghao/agentverse/blob/main/docs/manifest/workflow.md) · [Agent](https://github.com/loonghao/agentverse/blob/main/docs/manifest/agent.md)

