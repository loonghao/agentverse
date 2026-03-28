# Deployment

## Docker Compose (Development)

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse
docker compose up -d
docker compose ps
```

## Docker Compose (Production)

Create `docker-compose.prod.yml`:

```yaml
services:
  server:
    image: ghcr.io/loonghao/agentverse:latest
    restart: unless-stopped
    environment:
      DATABASE_URL: ${DATABASE_URL}
      REDIS_URL: ${REDIS_URL}
      JWT_SECRET: ${JWT_SECRET}
      RUST_LOG: "agentverse=info,tower_http=warn"
    ports:
      - "127.0.0.1:8080:8080"
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
```

```bash
cp .env.example .env
# Edit .env with your values
docker compose -f docker-compose.prod.yml up -d
```

## Kubernetes

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
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 30
          resources:
            requests:
              memory: "128Mi"
              cpu: "100m"
            limits:
              memory: "512Mi"
              cpu: "500m"
```

## Bare Metal / Systemd

```bash
# Download binary
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-server-x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C /usr/local/bin

# Create env file
mkdir -p /etc/agentverse
cat > /etc/agentverse/env << 'EOF'
DATABASE_URL=postgres://agentverse:SECRET@localhost:5432/agentverse
JWT_SECRET=GENERATED_SECRET
REDIS_URL=redis://localhost:6379
PORT=8080
RUST_LOG=info
EOF
chmod 600 /etc/agentverse/env

# Systemd unit
cat > /etc/systemd/system/agentverse.service << 'EOF'
[Unit]
Description=AgentVerse Server
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=agentverse
ExecStart=/usr/local/bin/agentverse-server
Restart=on-failure
EnvironmentFile=/etc/agentverse/env

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now agentverse
```

## Reverse Proxy

### Nginx

```nginx
server {
    listen 443 ssl;
    server_name agentverse.example.com;
    ssl_certificate /etc/letsencrypt/live/agentverse.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/agentverse.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Caddy

```caddyfile
agentverse.example.com {
    reverse_proxy localhost:8080
}
```

