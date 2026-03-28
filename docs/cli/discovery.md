# Discovery Commands

## search

Full-text and semantic search across all artifacts.

```bash
agentverse search <query> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `<query>` | _(required)_ | Search query string |
| `--kind <KIND>` | all | Filter: `skill` \| `agent` \| `workflow` \| `soul` \| `prompt` |
| `--tag <TAG>` | — | Filter by tag |
| `--limit <N>` | `10` | Max results to return |

### Examples

```bash
# Search all kinds
agentverse search "python code review"

# Filter by kind
agentverse search "deployment" --kind workflow
agentverse search "customer support" --kind agent

# Filter by tag
agentverse search "linter" --tag python

# Increase result count
agentverse search "code" --kind skill --limit 25
```

## get

Retrieve a specific artifact — latest version or a pinned version.

```bash
agentverse get <kind>/<namespace>/<name>[@version]
```

### Examples

```bash
# Get latest version
agentverse get skill/myorg/code-linter

# Get a pinned version
agentverse get skill/myorg/code-linter@1.2.0

# Agent artifact
agentverse get agent/myorg/support-bot@2.0.0

# Workflow artifact
agentverse get workflow/ops/deploy-pipeline
```

**Output includes:** kind, namespace, name, version, description, tags, checksum, download count, changelog.

## list

List artifacts with optional filters.

```bash
agentverse list <kind> [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `<kind>` | `skill` \| `agent` \| `workflow` \| `soul` \| `prompt` |
| `--namespace <NS>` | Filter by namespace/owner |
| `--limit <N>` | Max results (default: 20) |

### Examples

```bash
# All skills
agentverse list skill

# All agents in a namespace
agentverse list agent --namespace myorg

# Workflows with limit
agentverse list workflow --limit 50
```

## versions

Show the complete version history of an artifact.

```bash
agentverse versions <kind>/<namespace>/<name>
```

### Examples

```bash
agentverse versions skill/myorg/code-linter
# v0.3.0  (minor)  2025-06-01  "Added Python 3.12 support"
# v0.2.1  (patch)  2025-05-20  "Fix edge case in blank lines"
# v0.2.0  (minor)  2025-05-01  "New --strict mode"
# v0.1.0  (initial) 2025-04-15  "Initial release"
```

