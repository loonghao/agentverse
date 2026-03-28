# Server Configuration

Configuration is loaded from `config/default.toml` and overridden by environment variables.

## Full Configuration Reference

```toml
# config/default.toml

[server]
host       = "0.0.0.0"
port       = 8080
# Maximum request body size in bytes (10 MB)
body_limit = 10_485_760

[database]
url                  = "postgres://agentverse:agentverse_dev@localhost:5432/agentverse"
max_connections      = 20
min_connections      = 2
connect_timeout_secs = 10

[redis]
url              = "redis://localhost:6379"
pool_size        = 10
# Default cache TTL in seconds (5 minutes)
default_ttl_secs = 300

[auth]
jwt_secret                 = "change-me-in-production"
# JWT access token expiry (24 hours)
access_token_expiry_secs   = 86400
# Refresh token expiry (30 days)
refresh_token_expiry_secs  = 2_592_000

[versioning]
# Default bump when not explicitly specified
default_bump = "patch"
# Auto-infer version bump from manifest diff
auto_infer   = true

[registry]
# Max versions to keep per artifact (0 = unlimited)
max_versions           = 0
# Allow anonymous (unauthenticated) read access
anonymous_read         = true
# Require email verification before publishing
require_verified_email = false

# Object store is documented in Storage Backends
[object_store]
backend = "local"

[object_store.local]
base_dir  = "/tmp/agentverse-packages"
serve_url = "http://localhost:8080/files"
```

## Environment Variables

All config keys can be overridden with environment variables using the pattern `SECTION__KEY` (double underscore).

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://user:pass@db:5432/agentverse` |
| `REDIS_URL` | Redis connection string | `redis://localhost:6379` |
| `JWT_SECRET` | **Must change in production!** JWT signing secret | `openssl rand -hex 32` |
| `PORT` | Server listening port | `8080` |
| `RUST_LOG` | Log level | `info`, `debug`, `agentverse=debug` |
| `OBJECT_STORE_BACKEND` | Override storage backend at runtime | `s3`, `local`, `github`, `custom` |
| `AGENTVERSE_ANONYMOUS_READ` | Allow unauthenticated reads | `true` / `false` |

## Security Checklist

::: warning Production Checklist
1. **`JWT_SECRET`** — generate with `openssl rand -hex 32`; never use the default
2. **`RUST_LOG`** — set to `warn` or `info` in production (avoid `debug`)
3. **`anonymous_read`** — set to `false` if your registry is private
4. **TLS** — always run behind a reverse proxy (Nginx/Caddy) with HTTPS
5. **Database** — use a least-privilege PostgreSQL user
:::

### Generate Secrets

```bash
# JWT secret
openssl rand -hex 32

# Or with urandom
head -c 32 /dev/urandom | xxd -p
```

