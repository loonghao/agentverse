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

#[cfg(test)]
mod tests {
    use super::*;
    use config::{CustomConfig, DownloadAuth, LocalConfig, ObjectStoreBackend, ObjectStoreConfig};

    // ── Factory ────────────────────────────────────────────────────────────────

    #[test]
    fn build_github_backend_returns_github_release_backend() {
        let cfg = ObjectStoreConfig {
            backend: ObjectStoreBackend::GitHub(config::GitHubConfig {
                owner: "testowner".into(),
                repo: "testrepo".into(),
                token: None,
            }),
        };
        let store = build_object_store(&cfg).expect("github backend should build");
        assert_eq!(store.backend_name(), "github_release");
    }

    #[test]
    fn build_local_backend_returns_local_backend() {
        let cfg = ObjectStoreConfig {
            backend: ObjectStoreBackend::Local(LocalConfig {
                base_dir: std::env::temp_dir().join("agentverse-factory-test"),
                serve_url: "http://localhost:8080/files".into(),
            }),
        };
        let store = build_object_store(&cfg).expect("local backend should build");
        assert_eq!(store.backend_name(), "local");
    }

    #[test]
    fn build_custom_backend_returns_custom_backend() {
        let cfg = ObjectStoreConfig {
            backend: ObjectStoreBackend::Custom(CustomConfig {
                upload_url: "https://upload.example.com".into(),
                download_url_base: "https://cdn.example.com".into(),
                upload_auth_header: None,
                download_auth: DownloadAuth::None,
            }),
        };
        let store = build_object_store(&cfg).expect("custom backend should build");
        assert_eq!(store.backend_name(), "custom");
    }

    // ── Config deserialisation ─────────────────────────────────────────────────

    #[test]
    fn deserialize_local_backend_config() {
        // The flattened serde shape: { "backend": "local", "base_dir": …, "serve_url": … }
        let json = serde_json::json!({
            "backend": "local",
            "base_dir": "/tmp/agentverse-pkgs",
            "serve_url": "http://localhost:8080/files"
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        assert!(
            matches!(cfg.backend, ObjectStoreBackend::Local(_)),
            "expected Local variant"
        );
        if let ObjectStoreBackend::Local(local) = cfg.backend {
            assert_eq!(local.serve_url, "http://localhost:8080/files");
        }
    }

    #[test]
    fn deserialize_custom_backend_config_with_bearer_auth() {
        let json = serde_json::json!({
            "backend": "custom",
            "upload_url": "https://upload.example.com",
            "download_url_base": "https://cdn.example.com",
            "upload_auth_header": "Bearer svc-token",
            "download_auth": { "type": "bearer_header", "token": "dl-token" }
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        match cfg.backend {
            ObjectStoreBackend::Custom(c) => {
                assert_eq!(c.upload_url, "https://upload.example.com");
                assert_eq!(c.upload_auth_header.as_deref(), Some("Bearer svc-token"));
                assert!(
                    matches!(c.download_auth, DownloadAuth::BearerHeader { .. }),
                    "expected BearerHeader"
                );
            }
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_custom_backend_config_with_query_param_auth() {
        let json = serde_json::json!({
            "backend": "custom",
            "upload_url": "https://upload.example.com",
            "download_url_base": "https://cdn.example.com",
            "download_auth": {
                "type": "query_param",
                "param": "api_key",
                "token": "abc123"
            }
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        match cfg.backend {
            ObjectStoreBackend::Custom(c) => match c.download_auth {
                DownloadAuth::QueryParam { param, token } => {
                    assert_eq!(param, "api_key");
                    assert_eq!(token, "abc123");
                }
                other => panic!("expected QueryParam, got {other:?}"),
            },
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_download_auth_none_is_default() {
        // When `download_auth` is omitted, should default to None.
        let json = serde_json::json!({
            "backend": "custom",
            "upload_url": "https://upload.example.com",
            "download_url_base": "https://cdn.example.com"
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        match cfg.backend {
            ObjectStoreBackend::Custom(c) => {
                assert!(
                    matches!(c.download_auth, DownloadAuth::None),
                    "default download_auth should be None"
                );
            }
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_github_backend_config() {
        let json = serde_json::json!({
            "backend": "github",
            "owner": "myorg",
            "repo": "myrepo",
            "token": "ghp_fake_token"
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        match cfg.backend {
            ObjectStoreBackend::GitHub(gh) => {
                assert_eq!(gh.owner, "myorg");
                assert_eq!(gh.repo, "myrepo");
                assert_eq!(gh.token.as_deref(), Some("ghp_fake_token"));
            }
            other => panic!("expected GitHub, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_github_backend_config_without_token() {
        let json = serde_json::json!({
            "backend": "github",
            "owner": "testowner",
            "repo": "testrepo"
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        match cfg.backend {
            ObjectStoreBackend::GitHub(gh) => {
                assert!(gh.token.is_none(), "token should be None when not provided");
            }
            other => panic!("expected GitHub, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_s3_backend_config_minimal() {
        let json = serde_json::json!({
            "backend": "s3",
            "bucket": "my-bucket",
            "region": "us-east-1",
            "access_key": "AKIAIOSFODNN7EXAMPLE",
            "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        match cfg.backend {
            ObjectStoreBackend::S3(s3) => {
                assert_eq!(s3.bucket, "my-bucket");
                assert_eq!(s3.region, "us-east-1");
                assert!(s3.endpoint.is_none());
                assert!(!s3.force_path_style);
            }
            other => panic!("expected S3, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_s3_backend_config_with_endpoint_and_path_style() {
        let json = serde_json::json!({
            "backend": "s3",
            "bucket": "minio-bucket",
            "region": "us-east-1",
            "access_key": "minioadmin",
            "secret_key": "minioadmin",
            "endpoint": "http://minio.local:9000",
            "force_path_style": true,
            "presigned_expiry_secs": 3600
        });
        let cfg: ObjectStoreConfig = serde_json::from_value(json).expect("must deserialise");
        match cfg.backend {
            ObjectStoreBackend::S3(s3) => {
                assert_eq!(s3.endpoint.as_deref(), Some("http://minio.local:9000"));
                assert!(s3.force_path_style);
                assert_eq!(s3.presigned_expiry_secs, 3600);
            }
            other => panic!("expected S3, got {other:?}"),
        }
    }
}
