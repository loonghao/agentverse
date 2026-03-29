# BK-Repo (蓝鲸制品库)

[BK-Repo](https://github.com/TencentBlueKing/bk-repo) is the Tencent BlueKing artifact repository system — an open-source, self-hosted artifact registry that supports Maven, npm, Docker, Helm, PyPI, and **Generic** (arbitrary files) repositories.

AgentVerse has **first-class native support** for BK-Repo via the `bkrepo` storage backend — the default backend since v0.1.7.

## How BK-Repo Generic Works

BK-Repo's Generic repository exposes a simple REST API:

| Operation | Method | Path |
|-----------|--------|------|
| Upload | `PUT` | `/generic/{project}/{repo}/{path}` |
| Download | `GET` | `/generic/{project}/{repo}/{path}?download=true` |
| Delete | `DELETE` | `/generic/{project}/{repo}/{path}` |

Authentication uses HTTP Basic Auth (`username:password`). Credentials can be set in config or via environment variables.

## Configuration

```toml
[object_store]
backend = "bkrepo"

[object_store.bkrepo]
# Base URL of your bk-repo server (no trailing slash)
endpoint = "https://bkrepo.example.com"
# bk-repo project name
project  = "my-project"
# Generic repository name within the project
repo     = "agentverse-packages"
# Credentials — prefer env vars BKREPO_USERNAME / BKREPO_PASSWORD in production
username = "admin"
password = "change-me-in-production"
# Overwrite existing files on re-upload (default: true)
overwrite = true
```

### Environment Variable Overrides

In production, set credentials via environment variables instead of the config file:

| Variable | Description |
|----------|-------------|
| `BKREPO_USERNAME` | BK-Repo authentication username |
| `BKREPO_PASSWORD` | BK-Repo authentication password |
| `OBJECT_STORE_BACKEND` | Override backend at runtime (`bkrepo`) |

## Setup Steps

### 1. Deploy BK-Repo

Follow the [BK-Repo deployment guide](https://github.com/TencentBlueKing/bk-repo) or use the BK-Repo service within the BlueKing PaaS environment.

### 2. Create a Project and Generic Repo

Via the BK-Repo Web UI or API:

```bash
# Create project (if not exists)
curl -X POST "https://bkrepo.example.com/repository/api/project/create" \
  -H "Authorization: Basic $(echo -n 'admin:password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-project", "displayName": "AgentVerse Packages", "description": "AI agent artifact storage"}'

# Create generic repository
curl -X POST "https://bkrepo.example.com/repository/api/repo/create" \
  -H "Authorization: Basic $(echo -n 'admin:password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{
    "projectId": "my-project",
    "name": "agentverse-packages",
    "type": "GENERIC",
    "category": "LOCAL",
    "public": false,
    "description": "AgentVerse skill packages"
  }'
```

### 3. Configure AgentVerse

Update `config/default.toml`:

```toml
[object_store]
backend = "bkrepo"

[object_store.bkrepo]
endpoint = "https://bkrepo.example.com"
project  = "my-project"
repo     = "agentverse-packages"
username = "agentverse-service"
password = "SERVICE_PASSWORD"
overwrite = true
```

Or export environment variables (recommended for production/Docker):

```bash
export BKREPO_USERNAME=agentverse-service
export BKREPO_PASSWORD=SERVICE_PASSWORD
```

### 4. Verify Connectivity

```bash
# Upload a test file
curl -T ./test.zip \
  "https://bkrepo.example.com/generic/my-project/agentverse-packages/test/1.0.0.zip" \
  -H "Authorization: Basic $(echo -n 'agentverse-service:SERVICE_PASSWORD' | base64)" \
  -H "X-BKREPO-OVERWRITE: true"

# Download
curl -OL \
  "https://bkrepo.example.com/generic/my-project/agentverse-packages/test/1.0.0.zip?download=true" \
  -H "Authorization: Basic $(echo -n 'agentverse-service:SERVICE_PASSWORD' | base64)"
```

## Docker / Kubernetes

Pass credentials as environment variables — never bake secrets into the image:

```yaml
# docker-compose.yml
environment:
  - BKREPO_USERNAME=agentverse-service
  - BKREPO_PASSWORD=${BKREPO_PASSWORD}
```

```yaml
# Kubernetes Secret
apiVersion: v1
kind: Secret
metadata:
  name: bkrepo-credentials
stringData:
  BKREPO_USERNAME: agentverse-service
  BKREPO_PASSWORD: "your-secure-password"
```

## Full Production Example

```toml
[object_store]
backend = "bkrepo"

[object_store.bkrepo]
endpoint  = "https://bkrepo.corp.example.com"
project   = "ai-platform"
repo      = "agentverse-pkgs"
# Set via BKREPO_USERNAME / BKREPO_PASSWORD env vars in production
username  = ""
password  = ""
overwrite = true
```

## References

- [BK-Repo GitHub](https://github.com/TencentBlueKing/bk-repo)
- [BK-Repo Generic API Docs](https://github.com/TencentBlueKing/bk-repo/blob/master/docs/apidoc/generic.md)
- [BlueKing PaaS Platform](https://bk.tencent.com)

