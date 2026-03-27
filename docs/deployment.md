# AgentVerse Deployment Guide

This guide covers deploying AgentVerse in various environments, from local development to production.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Docker Compose (Development)](#docker-compose-development)
- [Docker Compose (Production)](#docker-compose-production)
- [Environment Variables](#environment-variables)
- [Database Setup](#database-setup)
- [Kubernetes](#kubernetes)
- [Bare Metal / Systemd](#bare-metal--systemd)
- [Reverse Proxy](#reverse-proxy-nginx--caddy)
- [Monitoring & Observability](#monitoring--observability)
- [Security Hardening](#security-hardening)

---

## Prerequisites

| Dependency | Version | Notes |
|-----------|---------|-------|
| PostgreSQL | 17+ | With `pgvector` extension |
| Redis | 7+ | For caching and rate limiting |
| MinIO / S3 | Any | For artifact binary storage |
| Docker | 24+ | Optional, for containerized deployment |

---

## Docker Compose (Development)

The fastest way to get AgentVerse running locally:

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse

# Start all services (PostgreSQL, Redis, MinIO, AgentVerse server)
docker compose up -d

# Verify all services are healthy
docker compose ps

# View server logs
docker compose logs -f server

# Access the API
curl http://localhost:8080/health
```

**Endpoints:**
- API Server: http://localhost:8080
- Swagger UI: http://localhost:8080/swagger-ui/
- MCP Endpoint: http://localhost:8080/mcp
- MinIO Console: http://localhost:9001 (admin/minioadmin123)

---

## Docker Compose (Production)

Create a `docker-compose.prod.yml` file:

```yaml
version: "3.9"

services:
  server:
    image: ghcr.io/loonghao/agentverse:latest
    restart: unless-stopped
    environment:
      DATABASE_URL: ${DATABASE_URL}
      REDIS_URL: ${REDIS_URL}
      S3_ENDPOINT: ${S3_ENDPOINT}
      S3_ACCESS_KEY: ${S3_ACCESS_KEY}
      S3_SECRET_KEY: ${S3_SECRET_KEY}
      S3_BUCKET: ${S3_BUCKET}
      JWT_SECRET: ${JWT_SECRET}
      RUST_LOG: "agentverse=info,tower_http=warn"
      PORT: "8080"
    ports:
      - "127.0.0.1:8080:8080"
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
    deploy:
      resources:
        limits:
          memory: 512M
```

Run production deployment:

```bash
# Create .env file with your secrets
cp .env.example .env
# Edit .env with your values

docker compose -f docker-compose.prod.yml up -d
```

---

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | ✅ | `postgres://agentverse:agentverse_dev@localhost:5432/agentverse` | PostgreSQL connection string |
| `REDIS_URL` | ❌ | `redis://localhost:6379` | Redis connection string |
| `S3_ENDPOINT` | ❌ | `http://localhost:9000` | S3-compatible storage endpoint |
| `S3_ACCESS_KEY` | ❌ | `minioadmin` | S3 access key |
| `S3_SECRET_KEY` | ❌ | `minioadmin123` | S3 secret key |
| `S3_BUCKET` | ❌ | `agentverse` | S3 bucket name |
| `JWT_SECRET` | ✅ | ❌ | **Must change in production!** JWT signing secret |
| `PORT` | ❌ | `8080` | Server listening port |
| `RUST_LOG` | ❌ | `info` | Log level (trace/debug/info/warn/error) |
| `AGENTVERSE_ANONYMOUS_READ` | ❌ | `true` | Allow unauthenticated reads |

### Security Warning

⚠️ **NEVER** use default values in production. Generate strong secrets:

```bash
# Generate a secure JWT_SECRET
openssl rand -hex 32
```

---

## Database Setup

AgentVerse automatically runs database migrations on startup. However, you can run them manually:

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run --database-url "$DATABASE_URL"
```

### Required PostgreSQL Extension

```sql
-- Connect to your database and enable pgvector
CREATE EXTENSION IF NOT EXISTS vector;
```

Use the `pgvector/pgvector:pg17` Docker image which has pgvector pre-installed.

---

## Kubernetes

Example Kubernetes deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agentverse
  namespace: ai-platform
spec:
  replicas: 3
  selector:
    matchLabels:
      app: agentverse
  template:
    metadata:
      labels:
        app: agentverse
    spec:
      containers:
        - name: agentverse
          image: ghcr.io/loonghao/agentverse:latest
          ports:
            - containerPort: 8080
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: agentverse-secrets
                  key: database-url
            - name: JWT_SECRET
              valueFrom:
                secretKeyRef:
                  name: agentverse-secrets
                  key: jwt-secret
          readinessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 5
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 30
          resources:
            requests:
              memory: "128Mi"
              cpu: "100m"
            limits:
              memory: "512Mi"
              cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: agentverse
  namespace: ai-platform
spec:
  selector:
    app: agentverse
  ports:
    - port: 80
      targetPort: 8080
```

---

## Bare Metal / Systemd

Download the latest server binary:

```bash
# Download for Linux x86_64
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-server-x86_64-unknown-linux-gnu.tar.gz | tar -xz -C /usr/local/bin

# Create systemd service
cat > /etc/systemd/system/agentverse.service << 'EOF'
[Unit]
Description=AgentVerse Server
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=agentverse
WorkingDirectory=/opt/agentverse
ExecStart=/usr/local/bin/agentverse-server
Restart=on-failure
RestartSec=5
EnvironmentFile=/etc/agentverse/env

[Install]
WantedBy=multi-user.target
EOF

# Create environment file
mkdir -p /etc/agentverse
cat > /etc/agentverse/env << 'EOF'
DATABASE_URL=postgres://agentverse:YOUR_PASSWORD@localhost:5432/agentverse
JWT_SECRET=YOUR_SECURE_SECRET
REDIS_URL=redis://localhost:6379
PORT=8080
RUST_LOG=info
EOF

chmod 600 /etc/agentverse/env

# Start and enable
systemctl daemon-reload
systemctl enable --now agentverse
```

---

## Reverse Proxy (Nginx / Caddy)

### Nginx

```nginx
server {
    listen 443 ssl;
    server_name agentverse.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/agentverse.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/agentverse.yourdomain.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket support (for future real-time features)
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Caddy

```caddyfile
agentverse.yourdomain.com {
    reverse_proxy localhost:8080
}
```

---

## Monitoring & Observability

AgentVerse emits structured JSON logs and OpenTelemetry traces.

```bash
# JSON structured logging
RUST_LOG=info,agentverse=debug docker compose up

# View logs in JSON format
docker compose logs server | jq '.message'
```

### Health Check

```bash
curl http://localhost:8080/health
# Response: {"status":"ok","version":"0.1.0"}
```

---

## Security Hardening

1. **Change JWT_SECRET**: Use `openssl rand -hex 32` to generate
2. **Use TLS**: Always run behind HTTPS in production
3. **Restrict database access**: Use least-privilege PostgreSQL user
4. **Set `AGENTVERSE_ANONYMOUS_READ=false`**: Require auth for all operations
5. **Use secrets management**: Vault, AWS Secrets Manager, or Kubernetes Secrets
6. **Enable rate limiting**: Configure Redis-backed rate limits
7. **Regular backups**: Backup PostgreSQL database and MinIO buckets

