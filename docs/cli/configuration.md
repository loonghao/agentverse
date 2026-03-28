# CLI Configuration

## Config File

The CLI saves credentials and the server URL in a config file at:

| OS | Path |
|----|------|
| Linux / macOS | `~/.config/agentverse/config.toml` |
| Windows | `%APPDATA%\agentverse\config.toml` |

Example `config.toml`:

```toml
server = "https://registry.example.com"
token  = "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9..."
username = "myusername"
```

The config is written automatically when you run `agentverse login`.

## Environment Variables

Environment variables take precedence over the config file:

| Variable | Description |
|----------|-------------|
| `AGENTVERSE_URL` | Server base URL (e.g. `https://registry.example.com`) |
| `AGENTVERSE_TOKEN` | Bearer token — overrides saved token |

## Per-command Flags

Flags take the highest precedence:

```bash
agentverse --server https://other.example.com --token $TEMP_TOKEN whoami
```

## Priority Order

```
CLI flags  >  Environment variables  >  Config file  >  Defaults
```

## Multiple Servers

You can switch between servers using environment variables or flags — no need to re-login:

```bash
# Development server
export AGENTVERSE_URL=http://localhost:8080
agentverse search "..."

# Production server (per-command override)
agentverse --server https://prod.example.com search "..."
```

