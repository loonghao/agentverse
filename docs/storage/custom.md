# Custom HTTP Storage

The `custom` backend delegates uploads and downloads to an organisation-owned HTTP endpoint. Use this when your organisation has a proprietary storage service, internal CDN, or any HTTP-accessible artifact store.

## How It Works

- **Upload:** `PUT {upload_url}/{key}` — raw bytes, `Content-Type: application/zip`
- **Download:** the server constructs a URL from `download_url_base/{key}`, optionally with auth credentials embedded

## Configuration

```toml
[object_store]
backend = "custom"

[object_store.custom]
# Base URL for PUT upload requests (no trailing slash)
upload_url        = "https://upload.internal.example.com"

# Base URL for download links returned to the CLI
download_url_base = "https://cdn.example.com"

# Full value of the Authorization header sent on upload requests
# Omit if the upload endpoint does not require authentication
upload_auth_header = "Bearer <service-token>"

# Download auth strategy: "none" | "query_param" | "bearer_header"
# See Download Authentication section below
[object_store.custom.download_auth]
type = "none"
```

## Download Authentication

The `download_auth` section controls how the CLI authenticates download requests.

### None (Public CDN)

No credentials attached — the bucket/storage must allow public reads:

```toml
[object_store.custom.download_auth]
type = "none"
```

Download URL example: `https://cdn.example.com/myorg/my-skill/1.0.0.zip`

### Query Parameter

A static token is embedded in the download URL query string:

```toml
[object_store.custom.download_auth]
type  = "query_param"
param = "token"
token = "YOUR_DOWNLOAD_TOKEN"
```

Download URL example: `https://storage.example.com/myorg/my-skill/1.0.0.zip?token=YOUR_DOWNLOAD_TOKEN`

The token is stored in the database as part of the URL, so it is self-contained.

### Bearer Header

The token is returned alongside the URL in the API response. The CLI adds `Authorization: Bearer {token}` to every download request:

```toml
[object_store.custom.download_auth]
type  = "bearer_header"
token = "YOUR_DOWNLOAD_TOKEN"
```

Use this when embedding the token in the URL is undesirable (e.g., it would appear in server logs).

## Full Example

```toml
[object_store]
backend = "custom"

[object_store.custom]
upload_url         = "https://artifacts.internal.corp.com/upload"
download_url_base  = "https://artifacts.cdn.corp.com"
upload_auth_header = "Bearer eyJhbGciOiJSUzI1NiJ9..."

[object_store.custom.download_auth]
type  = "query_param"
param = "access_token"
token = "dl-token-xxxxxxxxxxxxxxxx"
```

## Implementing the Upload Endpoint

The server will call:

```http
PUT https://upload.internal.example.com/<key>
Content-Type: application/zip
Authorization: Bearer <upload_auth_header>

<binary zip data>
```

Where `<key>` is typically `<namespace>/<name>/<version>.zip`.

Your endpoint must:
1. Accept the raw bytes
2. Store under the given key
3. Return `200 OK` or `201 Created`

