//! Local-filesystem object store backend.
//!
//! Stores uploaded packages as plain files under `base_dir/{key}`.
//! Download URLs use `serve_url/{key}` so they can be resolved over HTTP
//! when the server exposes a `/files/*` static-file route.
//!
//! **This backend is intended for local development and E2E testing only.**
//! It requires no cloud credentials and zero extra dependencies.

use std::path::PathBuf;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tracing::debug;

use crate::object_store::{config::LocalConfig, error::ObjectStoreError, ObjectStore};

/// Object store backed by the local filesystem.
pub struct LocalDiskBackend {
    base_dir: PathBuf,
    /// HTTP URL prefix (no trailing slash) used to build download URLs.
    serve_url: String,
}

impl LocalDiskBackend {
    /// Create a new backend, ensuring `base_dir` exists.
    pub fn new(cfg: LocalConfig) -> Result<Self, ObjectStoreError> {
        std::fs::create_dir_all(&cfg.base_dir)?;
        Ok(Self {
            base_dir: cfg.base_dir,
            serve_url: cfg.serve_url.trim_end_matches('/').to_string(),
        })
    }

    fn file_path(&self, key: &str) -> PathBuf {
        // Sanitise key: strip leading slashes so we stay inside base_dir.
        let key = key.trim_start_matches('/');
        self.base_dir.join(key)
    }
}

#[async_trait]
impl ObjectStore for LocalDiskBackend {
    async fn put(
        &self,
        key: &str,
        data: Bytes,
        _content_type: &str,
    ) -> Result<String, ObjectStoreError> {
        let path = self.file_path(key);
        // Ensure parent directories exist (keys may contain slashes).
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, &data).await?;
        let url = self.public_url(key);
        debug!(key, path = %path.display(), url, "local: file written");
        Ok(url)
    }

    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError> {
        let path = self.file_path(key);
        let mut file = tokio::fs::File::open(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ObjectStoreError::NotFound(key.to_string())
            } else {
                ObjectStoreError::Io(e)
            }
        })?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;
        Ok(Bytes::from(buf))
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let path = self.file_path(key);
        tokio::fs::remove_file(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ObjectStoreError::NotFound(key.to_string())
            } else {
                ObjectStoreError::Io(e)
            }
        })
    }

    fn public_url(&self, key: &str) -> String {
        let key = key.trim_start_matches('/');
        format!("{}/{}", self.serve_url, key)
    }

    fn backend_name(&self) -> &'static str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_backend() -> LocalDiskBackend {
        let dir = tempdir().unwrap();
        LocalDiskBackend::new(LocalConfig {
            base_dir: dir.keep(),
            serve_url: "http://localhost:8080/files".into(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn round_trip_put_get_delete() {
        let backend = make_backend();
        let data = Bytes::from_static(b"hello skill");

        let url = backend
            .put("test/hello.zip", data.clone(), "application/zip")
            .await
            .unwrap();
        assert!(url.contains("test/hello.zip"), "url should embed key");

        let got = backend.get("test/hello.zip").await.unwrap();
        assert_eq!(got, data);

        backend.delete("test/hello.zip").await.unwrap();
        assert!(backend.get("test/hello.zip").await.is_err());
    }

    #[tokio::test]
    async fn public_url_format() {
        let backend = make_backend();
        assert_eq!(
            backend.public_url("ns/skill/1.0.0.zip"),
            "http://localhost:8080/files/ns/skill/1.0.0.zip"
        );
    }

    #[tokio::test]
    async fn put_creates_nested_directories() {
        let backend = make_backend();
        let data = Bytes::from_static(b"nested payload");
        // Key with two levels of subdirectory — should succeed without pre-creating dirs.
        backend
            .put("org/my-skill/0.2.0.zip", data.clone(), "application/zip")
            .await
            .unwrap();
        let got = backend.get("org/my-skill/0.2.0.zip").await.unwrap();
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn get_missing_key_returns_not_found() {
        let backend = make_backend();
        let err = backend.get("does-not-exist.zip").await.unwrap_err();
        assert!(
            matches!(err, ObjectStoreError::NotFound(_)),
            "expected NotFound, got {err:?}"
        );
    }

    #[tokio::test]
    async fn delete_missing_key_returns_not_found() {
        let backend = make_backend();
        let err = backend.delete("ghost.zip").await.unwrap_err();
        assert!(
            matches!(err, ObjectStoreError::NotFound(_)),
            "expected NotFound, got {err:?}"
        );
    }

    #[tokio::test]
    async fn leading_slash_in_key_is_stripped() {
        let backend = make_backend();
        let data = Bytes::from_static(b"slash test");
        // put with leading slash …
        backend
            .put("/slashed/key.zip", data.clone(), "application/zip")
            .await
            .unwrap();
        // … must be readable without the slash …
        let got = backend.get("slashed/key.zip").await.unwrap();
        assert_eq!(got, data);
        // … and the public_url must not double-slash either.
        let url = backend.public_url("/slashed/key.zip");
        assert_eq!(url, "http://localhost:8080/files/slashed/key.zip");
    }

    #[test]
    fn backend_name_is_local() {
        let backend = make_backend();
        assert_eq!(backend.backend_name(), "local");
    }
}
