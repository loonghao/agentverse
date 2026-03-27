# AgentVerse CLI Skill for ClawHub

This skill provides the **AgentVerse CLI** (`agentverse`) for AI agents, enabling them to interact with the AgentVerse marketplace directly from the command line.

## What is AgentVerse?

AgentVerse is the universal marketplace for AI agent ecosystem components:

| Kind | Description |
|------|-------------|
| `skill` | Reusable capabilities (tools, functions) |
| `agent` | Autonomous AI agents with defined capabilities |
| `workflow` | Multi-step orchestration pipelines |
| `soul` | Persona and personality configurations |
| `prompt` | Optimized prompt templates |

## Commands Available

```bash
# Discovery
agentverse search --query "code review" --kind skill
agentverse list --kind agent --namespace myorg
agentverse get --kind workflow --namespace ops --name deploy-pipeline
agentverse versions --kind skill --namespace python-tools --name linter

# Publishing
agentverse publish --file manifest.toml
agentverse update --kind skill --namespace myorg --name my-skill --file manifest.toml

# Authentication
agentverse login --server https://agentverse.example.com
agentverse whoami
```

## Installation via ClawHub

This skill is automatically installed when you use the AgentVerse CLI through ClawHub.

## Links

- **Repository**: https://github.com/loonghao/agentverse
- **Docker Image**: `ghcr.io/loonghao/agentverse:latest`
- **API Docs**: https://github.com/loonghao/agentverse#api-documentation

