---
name: docker-compose-manager
description: "Manage multi-container Docker environments using docker compose. Start, stop, inspect, and health-check services defined in a compose file — ideal for local dev, CI service orchestration, and integration test setup."
version: "0.1.0"
tags: [docker, containers, devops, ci]
license: MIT
metadata:
  openclaw:
    homepage: https://docs.docker.com/compose
    emoji: "🐳"
    requires:
      bins:
        - docker
    install:
      - kind: shell
        linux: "curl -fsSL https://get.docker.com | sh && docker compose version"
        macos: "brew install --cask docker"
        windows: "winget install Docker.DockerDesktop"
---

# Docker Compose Manager

Orchestrate multi-container environments with [Docker Compose](https://docs.docker.com/compose).

## When to use

- Spinning up local development stacks (postgres + redis + app)
- Setting up integration test environments in CI
- Health-checking running services before running E2E tests
- Tearing down and recreating ephemeral environments

## Inputs

```json
{
  "compose_file": "docker-compose.yml",
  "action": "up",
  "services": ["postgres", "redis"],
  "wait_healthy": true,
  "timeout_seconds": 60
}
```

| Field              | Required | Description                                                |
|--------------------|----------|------------------------------------------------------------|
| `compose_file`     | ✗        | Path to compose file (default: `docker-compose.yml`)       |
| `action`           | ✓        | `up` / `down` / `ps` / `logs` / `restart` / `health`      |
| `services`         | ✗        | Subset of services to target (default: all)                |
| `wait_healthy`     | ✗        | Block until health checks pass (default: `true`)           |
| `timeout_seconds`  | ✗        | Max wait for health checks (default: `60`)                 |

## Example commands

```bash
# Start all services, rebuild images, and wait for health
docker compose up -d --build --wait

# Check running service status
docker compose ps --format json

# Stream logs from the app service
docker compose logs -f app

# Tear everything down and remove volumes
docker compose down --volumes --remove-orphans
```

## Output (action=health)

```json
{
  "services": [
    { "name": "postgres", "state": "running", "health": "healthy", "ports": ["5432:5432"] },
    { "name": "redis",    "state": "running", "health": "healthy", "ports": ["6379:6379"] }
  ],
  "all_healthy": true,
  "elapsed_ms": 1240
}
```

