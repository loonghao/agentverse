//! Pluggable object storage for skill package archives.
//!
//! # Backends
//!
//! | Backend   | Use case                          | Key deps           |
//! |-----------|-----------------------------------|--------------------|
//! | `local`   | Development / E2E tests           | `tokio::fs`        |
//! | `s3`      | AWS S3, Tencent COS, MinIO, R2    | `object_store/aws` |
//! | `github`  | GitHub Releases as package CDN    | `reqwest`          |
//! | `custom`  | Organisation-owned HTTP endpoint  | `reqwest`          |
//!
//! # Publish + download lifecycle
//!
//! ```text
//! CLI zip  ──► POST /upload ──► server validates + repackages
//!                                └─► ObjectStore::put(key, bytes) → download_url
//!                                      stored as SkillPackage { source_type=internal }
//!
//! CLI install ──► GET /packages ──► { download_url, download_token? }
//!                  └─► CLI: reqwest::get(download_url) [+ optional Authorization header]
//! ```
//!
//! # Download auth strategies
//!
//! | Backend             | Strategy                                    |
//! |---------------------|---------------------------------------------|
//! | S3 public           | plain URL, no credentials                   |
//! | S3 pre-signed       | signature embedded in URL query string      |
//! | Custom / none       | plain URL, bucket is public                 |
//! | Custom / query_param| `?{param}={token}` appended to URL          |
//! | Custom / bearer     | URL plain + `download_token` in API resp    |
//! | Local               | `http://server/files/{key}` — server-served |
//! | GitHub              | public GitHub release asset URL             |

pub mod backends;
pub mod config;
pub mod error;

pub use backends::{CustomBackend, GitHubReleaseBackend, LocalDiskBackend, S3Backend};
pub use config::{
    CustomConfig, DownloadAuth, GitHubConfig, LocalConfig, ObjectStoreBackend, ObjectStoreConfig,
    S3Config,
};
pub use error::ObjectStoreError;

use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

// ── Core trait ────────────────────────────────────────────────────────────────

/// Unified interface for all object storage backends.
///
/// Injected into `AppState` at startup via [`build_object_store`].
#[async_trait]
pub trait ObjectStore: Send + Sync {
    /// Upload `data` under `key`; returns the **public download URL**.
    ///
    /// For S3 pre-signed backends this may be a time-limited signed URL.
    /// For Custom+QueryParam backends the token is already embedded in the URL.
    async fn put(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<String, ObjectStoreError>;

    /// Download the object identified by `key`.
    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError>;

    /// Delete the object identified by `key`.
    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError>;

    /// Return a download URL for `key` (no data transfer).
    ///
    /// For pre-signed S3 this regenerates a fresh signed URL on each call.
    fn public_url(&self, key: &str) -> String;

    /// If the download requires a bearer token **not** embedded in the URL,
    /// return it so the API layer can include `download_token` in its response.
    ///
    /// Only non-None for `Custom { download_auth: BearerHeader }`.
    fn download_bearer_token(&self) -> Option<&str> {
        None
    }

    /// Human-readable backend name for logs and metrics.
    fn backend_name(&self) -> &'static str;
}

// ── Factory ───────────────────────────────────────────────────────────────────

/// Construct the correct backend from the active `[object_store]` configuration.
pub fn build_object_store(
    cfg: &ObjectStoreConfig,
) -> Result<Arc<dyn ObjectStore>, ObjectStoreError> {
    let store: Arc<dyn ObjectStore> = match &cfg.backend {
        ObjectStoreBackend::S3(s3) => Arc::new(S3Backend::new(s3.clone())?),
        ObjectStoreBackend::Local(local) => Arc::new(LocalDiskBackend::new(local.clone())?),
        ObjectStoreBackend::GitHub(gh) => Arc::new(GitHubReleaseBackend::new(gh.clone())),
        ObjectStoreBackend::Custom(custom) => Arc::new(CustomBackend::new(custom.clone())),
    };
    tracing::info!(backend = store.backend_name(), "object store initialised");
    Ok(store)
}
