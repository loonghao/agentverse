# Local Filesystem Storage

The `local` backend stores packages on the server's local disk. The server also serves them over HTTP via a built-in static file route.

::: warning Development Only
The local backend is intended for **development and E2E testing only**. It does not scale horizontally — packages on one server instance are not available to other instances. Use S3, GitHub Releases, or a custom backend in production.
:::

## Configuration

```toml
[object_store]
backend = "local"

[object_store.local]
# Absolute directory where uploaded packages are stored on disk
base_dir  = "/tmp/agentverse-packages"

# HTTP URL prefix the server uses to construct download URLs.
# Must match a static-file route served by the same server.
# No trailing slash.
serve_url = "http://localhost:8080/files"
```

The server automatically serves files from `base_dir` at the path `GET /files/*`.

## Environment Variables

```bash
# Override at runtime (uses default sub-keys)
OBJECT_STORE_BACKEND=local

# Override specific values via config override (if supported by your config layer)
```

## Docker Compose Example

```yaml
services:
  server:
    image: ghcr.io/loonghao/agentverse:latest
    environment:
      OBJECT_STORE_BACKEND: local
    volumes:
      # Persist packages across container restarts
      - agentverse_packages:/tmp/agentverse-packages

volumes:
  agentverse_packages:
```

## File Layout

Packages are stored as:

```
/tmp/agentverse-packages/
└── <namespace>/
    └── <name>/
        └── <version>.zip
```

And served at:

```
http://localhost:8080/files/<namespace>/<name>/<version>.zip
```

## Limitations

- **No CDN** — all traffic goes through the server process
- **No replication** — single point of failure
- **No access control** — files are publicly accessible at the static URL
- **Disk space** — you are responsible for managing disk usage

