# API Reference

## Interactive Documentation

When the server is running, full interactive docs are available at:

| Interface | URL |
|-----------|-----|
| **Swagger UI** | `http://localhost:8080/swagger-ui/` |
| **GraphQL Playground** | `http://localhost:8080/graphql-playground` |
| **MCP** | `POST http://localhost:8080/mcp` |

## REST Endpoints

### Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/api/v1/auth/register` | — | Register new user |
| `POST` | `/api/v1/auth/login` | — | Login, get tokens |
| `POST` | `/api/v1/auth/refresh` | Bearer | Refresh access token |
| `GET` | `/api/v1/auth/me` | Bearer | Current user profile |
| `PUT` | `/api/v1/auth/me` | Bearer | Update profile |

### Skills (same pattern for `agents`, `workflows`, `souls`, `prompts`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/api/v1/skills` | Bearer | Create artifact |
| `GET` | `/api/v1/skills` | — | List skills |
| `GET` | `/api/v1/skills/:ns/:name` | — | Get latest version |
| `GET` | `/api/v1/skills/:ns/:name/:version` | — | Get pinned version |
| `POST` | `/api/v1/skills/:ns/:name/publish` | Bearer | Publish new version |
| `GET` | `/api/v1/skills/:ns/:name/versions` | — | Version history |
| `POST` | `/api/v1/skills/:ns/:name/fork` | Bearer | Fork artifact |
| `POST` | `/api/v1/skills/:ns/:name/upload` | Bearer | Upload zip package |
| `GET` | `/api/v1/skills/:ns/:name/packages` | — | List packages |
| `POST` | `/api/v1/skills/:ns/:name/packages` | Bearer | Register package |

### Social

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/api/v1/skills/:ns/:name/likes` | Bearer | Like |
| `DELETE` | `/api/v1/skills/:ns/:name/likes` | Bearer | Unlike |
| `POST` | `/api/v1/skills/:ns/:name/ratings` | Bearer | Rate (1–5) |
| `GET` | `/api/v1/skills/:ns/:name/ratings` | — | List ratings |
| `POST` | `/api/v1/skills/:ns/:name/comments` | Bearer | Post comment |
| `GET` | `/api/v1/skills/:ns/:name/comments` | — | List comments |
| `PUT` | `/api/v1/skills/:ns/:name/comments/:id` | Bearer | Update comment |
| `DELETE` | `/api/v1/skills/:ns/:name/comments/:id` | Bearer | Delete comment |
| `GET` | `/api/v1/skills/:ns/:name/stats` | — | Social stats |

### Discovery

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/search?q=...&kind=skill&limit=10` | Full-text + semantic search |
| `GET` | `/api/v1/trending?kind=skill` | Trending artifacts |
| `GET` | `/api/v1/users/:username/artifacts` | Artifacts by user |

### Health

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Liveness check |
| `GET` | `/ready` | Readiness check (DB + Redis) |

## MCP Integration

Configure your MCP client (e.g., Claude Desktop) to use AgentVerse:

```json
{
  "mcpServers": {
    "agentverse": {
      "url": "http://localhost:8080/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_TOKEN"
      }
    }
  }
}
```

Available MCP tools: `search_artifacts`, `get_artifact`, `publish_artifact`, `list_artifacts`.

## GraphQL

```graphql
query SearchSkills {
  artifacts(kind: SKILL, query: "python") {
    id
    name
    namespace
    manifest { description tags }
    downloads
    latestVersion { version publishedAt }
  }
}
```

