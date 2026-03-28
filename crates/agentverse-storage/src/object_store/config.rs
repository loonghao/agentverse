use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ── Download-auth helpers ─────────────────────────────────────────────────────

/// How the **download URL** returned to the CLI is authenticated.
///
/// When a package is hosted internally, the CLI downloads it directly from the
/// URL the server returns.  This enum controls what credentials (if any) are
/// embedded in or attached to that URL.
///
/// | Variant        | Where token goes                    | URL looks like                             |
/// |----------------|-------------------------------------|--------------------------------------------|
/// | `None`         | nowhere — bucket is public          | `https://cdn.example.com/ns/name/1.0.zip`  |
/// | `QueryParam`   | `?{name}={token}` query string      | `https://cdn.example.com/…?token=abc123`   |
/// | `BearerHeader` | returned alongside URL in API resp  | URL plain, CLI adds `Authorization: Bearer`|
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DownloadAuth {
    /// No authentication — objects are publicly readable.
    #[default]
    None,
    /// Embed a static token in the URL query string.
    ///
    /// The server appends `?{param}={token}` when building download URLs.
    /// The token is stored in the DB as part of the URL, so it is self-contained
    /// and the CLI needs no extra configuration.
    QueryParam {
        /// Query parameter name, e.g. `"token"` or `"api_key"`.
        param: String,
        token: String,
    },
    /// Return the download URL and a separate bearer token in the API response.
    ///
    /// The CLI must add `Authorization: Bearer {token}` to every download
    /// request.  Use this when embedding the token in the URL is undesirable
    /// (e.g., it would appear in server logs).
    BearerHeader { token: String },
}

/// Top-level object store configuration — lives under `[object_store]` in
/// `config/default.toml` (or the equivalent environment-override file).
///
/// # Example TOML
/// ```toml
/// [object_store]
/// backend = "local"
///
/// [object_store.local]
/// base_dir = "/tmp/agentverse-packages"
/// serve_url = "http://localhost:8080/files"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectStoreConfig {
    /// Which storage backend to activate.
    #[serde(flatten)]
    pub backend: ObjectStoreBackend,
}

/// Discriminated union of supported backends.
///
/// The TOML representation uses a `backend` key for the tag:
/// ```toml
/// [object_store]
/// backend = "s3"   # or "local" | "github" | "custom"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "lowercase")]
pub enum ObjectStoreBackend {
    /// AWS S3 / Tencent COS / MinIO / Cloudflare R2 (all S3-protocol compatible).
    S3(S3Config),
    /// Local filesystem — intended for development and E2E tests only.
    Local(LocalConfig),
    /// GitHub Releases — uploads package archives as release assets.
    GitHub(GitHubConfig),
    /// Fully custom HTTP endpoint for organisations with proprietary storage.
    Custom(CustomConfig),
}

// ── S3-compatible ─────────────────────────────────────────────────────────────

/// Configuration for any S3-protocol-compatible store.
///
/// | Service      | `endpoint`                                         | `force_path_style` |
/// |--------------|----------------------------------------------------|--------------------|
/// | AWS S3       | _(empty)_                                          | `false`            |
/// | MinIO        | `http://localhost:9000`                            | `true`             |
/// | Tencent COS  | `https://cos.{region}.myqcloud.com`                | `false`            |
/// | Cloudflare R2| `https://{account_id}.r2.cloudflarestorage.com`   | `true`             |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// Custom endpoint URL.  Leave empty for AWS S3.
    pub endpoint: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    /// AWS region string, e.g. `"us-east-1"` or `"ap-guangzhou"` for COS.
    pub region: String,
    /// Override the base URL embedded in generated download links
    /// (useful when objects are served via a CDN in front of the bucket).
    pub public_url_base: Option<String>,
    /// Use `/{bucket}/{key}` path-style URLs instead of virtual-hosted style.
    /// Required for MinIO and some COS configurations.
    #[serde(default)]
    pub force_path_style: bool,
    /// How long (seconds) a pre-signed download URL is valid.
    /// `0` or `None` → generate a plain public URL (bucket must allow public read).
    /// Set to e.g. `3600` for private buckets.
    #[serde(default)]
    pub presigned_expiry_secs: u64,
}

// ── Local disk ────────────────────────────────────────────────────────────────

/// Local-filesystem backend — no cloud credentials required.
///
/// Files are written to `base_dir` and download URLs use `serve_url` as a
/// prefix.  In a local dev/E2E setup the server should expose a static-file
/// route at the same path prefix (e.g. `GET /files/*`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    /// Absolute directory where uploaded packages are stored.
    pub base_dir: PathBuf,
    /// HTTP URL prefix that maps to `base_dir` (no trailing slash).
    /// Example: `"http://localhost:8080/files"`
    pub serve_url: String,
}

// ── GitHub Releases ───────────────────────────────────────────────────────────

/// Upload packages as assets on a GitHub repository's releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub owner: String,
    pub repo: String,
    /// GitHub personal-access or fine-grained token with `contents: write`.
    /// Falls back to the `GITHUB_TOKEN` environment variable when `None`.
    pub token: Option<String>,
}

// ── Custom HTTP ───────────────────────────────────────────────────────────────

/// Delegate uploads to an organisation-owned HTTP endpoint.
///
/// ## Upload
/// `PUT {upload_url}/{key}`  (raw bytes, `Content-Type: application/zip`)
///
/// ## Download
/// Controlled by `download_auth` — see [`DownloadAuth`] for details.
/// The simplest option is `None` (public CDN).  For private storage, use
/// `QueryParam` to embed a static token in every generated URL, or
/// `BearerHeader` to return the token out-of-band and let the CLI attach it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomConfig {
    /// Base URL for upload `PUT` requests (no trailing slash).
    /// Example: `"https://upload.internal.example.com"`
    pub upload_url: String,
    /// Base URL for public/download access (no trailing slash).
    /// Example: `"https://cdn.example.com"` or `"https://storage.example.com"`
    pub download_url_base: String,
    /// Full value of the `Authorization` header sent on **upload** requests.
    /// Example: `"Bearer <service-token>"` or `"Token <api-key>"`.
    /// Leave `None` if the upload endpoint does not require authentication.
    pub upload_auth_header: Option<String>,
    /// How download URLs presented to the CLI carry their credentials.
    /// Defaults to `None` (no auth / public CDN).
    #[serde(default)]
    pub download_auth: DownloadAuth,
}
