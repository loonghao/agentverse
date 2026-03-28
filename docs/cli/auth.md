# Authentication

## Register

Create a new account:

```bash
agentverse register <username> \
  --email me@example.com \
  --password "MySecurePass123!"
```

**Password requirements:** minimum 8 characters.  
**Username requirements:** minimum 3 characters.

The command returns a JWT token and automatically logs you in.

## Login

```bash
agentverse login <username>
# Password: (prompted securely, no echo)

# Or pass password directly (less secure — avoid in scripts)
agentverse login <username> --password "MyPass123!"
```

On success the token is saved to `~/.config/agentverse/config.toml`.

## Whoami

Verify your current authentication:

```bash
agentverse whoami
# Logged in as: myusername
# Email: me@example.com
```

## Token Management

### Use a Token Directly

```bash
# Via environment variable (CI/CD recommended)
export AGENTVERSE_TOKEN=eyJ...
agentverse whoami

# Via flag
agentverse --token eyJ... whoami
```

### Token Refresh

Tokens expire after 24 hours by default. The server issues refresh tokens (30-day lifetime). Login again to get a fresh token:

```bash
agentverse login <username>
```

Or use the API directly:

```bash
curl -X POST https://registry.example.com/api/v1/auth/refresh \
  -H "Authorization: Bearer $AGENTVERSE_TOKEN"
```

## CI/CD Usage

For automated pipelines, use a service account:

```bash
# Register a service account (done once)
agentverse register ci-bot \
  --email ci@yourorg.com \
  --password "$CI_BOT_PASSWORD"

# Login and capture token
TOKEN=$(curl -s -X POST https://registry.example.com/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"ci-bot","password":"'"$CI_BOT_PASSWORD"'"}' \
  | jq -r .access_token)

# Use in subsequent commands
agentverse --token "$TOKEN" publish
```

Store the token in your CI/CD secrets manager (GitHub Actions Secrets, Vault, etc.).

