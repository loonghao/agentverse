# S3-Compatible Storage

The `s3` backend works with any S3-protocol-compatible object store, including AWS S3, Tencent Cloud COS, MinIO, and Cloudflare R2.

## Configuration Reference

```toml
[object_store]
backend = "s3"

[object_store.s3]
# Custom endpoint. Leave empty for AWS S3.
endpoint            = ""
access_key          = "YOUR_ACCESS_KEY"
secret_key          = "YOUR_SECRET_KEY"
bucket              = "agentverse-packages"
region              = "us-east-1"

# Use /{bucket}/{key} path-style URLs (required for MinIO; false for AWS/COS)
force_path_style    = false

# Pre-signed URL expiry in seconds. 0 = public URL (bucket must allow public reads)
presigned_expiry_secs = 0

# Optional CDN URL override for download links
# e.g. "https://cdn.example.com" → links become https://cdn.example.com/<key>
public_url_base     = ""
```

## AWS S3

```toml
[object_store]
backend = "s3"

[object_store.s3]
# No endpoint needed for AWS S3
access_key = "AKIAIOSFODNN7EXAMPLE"
secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
bucket     = "my-agentverse-packages"
region     = "us-east-1"

# Public bucket → plain URLs; private bucket → use presigned_expiry_secs > 0
force_path_style      = false
presigned_expiry_secs = 0
```

**IAM policy** (minimum required):

```json
{
  "Effect": "Allow",
  "Action": ["s3:PutObject", "s3:GetObject", "s3:DeleteObject"],
  "Resource": "arn:aws:s3:::my-agentverse-packages/*"
}
```

## Tencent Cloud COS

COS is S3-protocol compatible. Use `cos.<region>.myqcloud.com` as the endpoint:

```toml
[object_store]
backend = "s3"

[object_store.s3]
endpoint   = "https://cos.ap-guangzhou.myqcloud.com"
access_key = "AKIDxxxxxxxxxxxxxxxx"
secret_key = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
bucket     = "agentverse-packages-1234567890"
region     = "ap-guangzhou"

force_path_style      = false
presigned_expiry_secs = 3600   # 1-hour signed URLs for private bucket
```

**Supported regions:**

| Region | Endpoint |
|--------|----------|
| 广州 (ap-guangzhou) | `https://cos.ap-guangzhou.myqcloud.com` |
| 上海 (ap-shanghai) | `https://cos.ap-shanghai.myqcloud.com` |
| 北京 (ap-beijing) | `https://cos.ap-beijing.myqcloud.com` |
| 成都 (ap-chengdu) | `https://cos.ap-chengdu.myqcloud.com` |
| 新加坡 (ap-singapore) | `https://cos.ap-singapore.myqcloud.com` |

## MinIO (Self-hosted)

```toml
[object_store]
backend = "s3"

[object_store.s3]
endpoint   = "http://localhost:9000"
access_key = "minioadmin"
secret_key = "minioadmin123"
bucket     = "agentverse"
region     = "us-east-1"    # arbitrary for MinIO

# Required for MinIO
force_path_style      = true
presigned_expiry_secs = 0
```

**Docker Compose snippet:**

```yaml
services:
  minio:
    image: minio/minio
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin123
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_data:/data
```

## Cloudflare R2

R2 is S3-compatible with zero egress fees:

```toml
[object_store]
backend = "s3"

[object_store.s3]
endpoint   = "https://<ACCOUNT_ID>.r2.cloudflarestorage.com"
access_key = "your-r2-access-key-id"
secret_key = "your-r2-secret-access-key"
bucket     = "agentverse-packages"
region     = "auto"

force_path_style      = true
presigned_expiry_secs = 3600

# Optional: R2 public bucket or custom domain
public_url_base = "https://packages.example.com"
```

Get credentials from **Cloudflare Dashboard → R2 → Manage R2 API Tokens**.

## Pre-signed vs Public URLs

| `presigned_expiry_secs` | Behavior |
|-------------------------|----------|
| `0` | Plain public URL — bucket **must** allow public reads |
| `> 0` (e.g. `3600`) | Time-limited signed URL — works with private buckets |

For private buckets, set `presigned_expiry_secs = 3600` (or longer).

