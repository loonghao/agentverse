# BK-Repo (蓝鲸制品库)

[BK-Repo](https://github.com/TencentBlueKing/bk-repo) is the Tencent BlueKing artifact repository system — an open-source, self-hosted artifact registry that supports Maven, npm, Docker, Helm, PyPI, and **Generic** (arbitrary files) repositories.

Use the `custom` storage backend to integrate AgentVerse with BK-Repo's Generic repository API.

## How BK-Repo Generic Works

BK-Repo's Generic repository exposes a simple REST API:

| Operation | Method | Path |
|-----------|--------|------|
| Upload | `PUT` | `/generic/{project}/{repo}/{path}` |
| Download | `GET` | `/generic/{project}/{repo}/{path}` |
| Delete | `DELETE` | `/generic/{project}/{repo}/{path}` |

Authentication is via HTTP Basic Auth (`username:password`) or a platform access key.

## Configuration

```toml
[object_store]
backend = "custom"

[object_store.custom]
# BK-Repo Generic API base URL for uploads
# Format: https://<bkrepo-host>/generic/<project>/<repo>
upload_url        = "https://bkrepo.example.com/generic/MY_PROJECT/agentverse-packages"

# Public download base URL (same path, different auth strategy)
download_url_base = "https://bkrepo.example.com/generic/MY_PROJECT/agentverse-packages"

# HTTP Basic Auth for upload: "Basic base64(username:password)"
# Generate with: echo -n "user:password" | base64
upload_auth_header = "Basic dXNlcjpwYXNzd29yZA=="

# Download auth: embed credentials in the download URL
[object_store.custom.download_auth]
type  = "query_param"
param = "token"
token = "YOUR_BKREPO_ACCESS_TOKEN"
```

## Setup Steps

### 1. Deploy BK-Repo

Follow the [BK-Repo deployment guide](https://github.com/TencentBlueKing/bk-repo). Or use the BK-Repo service within the BlueKing PaaS environment.

### 2. Create a Project and Generic Repo

Via the BK-Repo Web UI or API:

```bash
# Create project (if not exists)
curl -X POST "https://bkrepo.example.com/repository/api/project/create" \
  -H "Authorization: Basic $(echo -n 'admin:password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{"name": "MY_PROJECT", "displayName": "AgentVerse Packages", "description": "AI agent artifact storage"}'

# Create generic repository
curl -X POST "https://bkrepo.example.com/repository/api/repo/create" \
  -H "Authorization: Basic $(echo -n 'admin:password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{
    "projectId": "MY_PROJECT",
    "name": "agentverse-packages",
    "type": "GENERIC",
    "category": "LOCAL",
    "public": false,
    "description": "AgentVerse skill packages"
  }'
```

### 3. Create an Access Token (Recommended)

In BK-Repo, create a platform token or user token for service-to-service auth:

```bash
curl -X POST "https://bkrepo.example.com/auth/api/user/token/create" \
  -H "Authorization: Basic $(echo -n 'admin:password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{"userId": "agentverse-service", "name": "agentverse-token", "expiredAt": null}'
```

### 4. Test Upload

```bash
# Upload a test file
curl -T ./test.zip \
  "https://bkrepo.example.com/generic/MY_PROJECT/agentverse-packages/myorg/my-skill/1.0.0.zip" \
  -H "Authorization: Basic $(echo -n 'agentverse-service:SERVICE_PASSWORD' | base64)"

# Download
curl -O \
  "https://bkrepo.example.com/generic/MY_PROJECT/agentverse-packages/myorg/my-skill/1.0.0.zip?token=ACCESS_TOKEN"
```

## Public vs Private Repositories

| BK-Repo Repo Type | Recommended `download_auth` |
|-------------------|-----------------------------|
| `public: true` | `type = "none"` |
| `public: false` | `type = "query_param"` with access token |

## Full Production Example

```toml
[object_store]
backend = "custom"

[object_store.custom]
upload_url        = "https://bkrepo.corp.example.com/generic/ai-platform/agentverse-pkgs"
download_url_base = "https://bkrepo.corp.example.com/generic/ai-platform/agentverse-pkgs"
upload_auth_header = "Basic YWdlbnR2ZXJzZS1zdmM6U0VDUkVUX1BBU1NXT1JE"

[object_store.custom.download_auth]
type  = "query_param"
param = "token"
token = "bk-repo-access-token-xxxxxxxxxxxx"
```

## References

- [BK-Repo GitHub](https://github.com/TencentBlueKing/bk-repo)
- [BK-Repo Generic API Docs](https://github.com/TencentBlueKing/bk-repo/blob/master/docs/apidoc/generic.md)
- [BlueKing PaaS Platform](https://bk.tencent.com)

