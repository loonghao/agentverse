//! S3-protocol-compatible object store backend.
//!
//! Uses the [`object_store`] crate (Apache Arrow project) which supports:
//! - **AWS S3** — leave `endpoint` empty, set `region`.
//! - **Tencent COS** — set `endpoint = "https://cos.{region}.myqcloud.com"`.
//! - **MinIO** — set `endpoint = "http://host:9000"`, `force_path_style = true`.
//! - **Cloudflare R2** — set `endpoint` to the R2 endpoint, `force_path_style = true`.
//!
//! The `public_url_base` field lets you serve objects via a CDN: if set, the
//! generated download URL uses the CDN base instead of the bucket endpoint.

use async_trait::async_trait;
use bytes::Bytes;
// In object_store 0.13 the async methods (get/put/delete) are defined on
// ObjectStore but only accessible via Arc<dyn ObjectStore> through
// ObjectStoreExt, which must be explicitly in scope.
use object_store::{
    aws::AmazonS3Builder, path::Path as OsPath, ObjectStore as ApacheObjectStore, ObjectStoreExt,
};
use std::sync::Arc;
use tracing::debug;

use crate::object_store::{config::S3Config, error::ObjectStoreError, ObjectStore};

pub struct S3Backend {
    store: Arc<dyn ApacheObjectStore>,
    bucket: String,
    /// Base URL for public download links.  May be a CDN URL or the bucket URL.
    public_url_base: String,
}

impl S3Backend {
    pub fn new(cfg: S3Config) -> Result<Self, ObjectStoreError> {
        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(&cfg.bucket)
            .with_region(&cfg.region)
            .with_access_key_id(&cfg.access_key)
            .with_secret_access_key(&cfg.secret_key);

        if let Some(endpoint) = &cfg.endpoint {
            builder = builder.with_endpoint(endpoint);
        }
        if cfg.force_path_style {
            builder = builder.with_virtual_hosted_style_request(false);
        }

        let store = builder
            .build()
            .map_err(|e| ObjectStoreError::Config(e.to_string()))?;

        // Determine the base URL used in public download links.
        let public_url_base = cfg.public_url_base.clone().unwrap_or_else(|| {
            if let Some(ep) = &cfg.endpoint {
                // Path-style: {endpoint}/{bucket}
                if cfg.force_path_style {
                    format!("{}/{}", ep.trim_end_matches('/'), cfg.bucket)
                } else {
                    // Virtual-hosted: endpoint as-is (COS uses host-based routing)
                    ep.trim_end_matches('/').to_string()
                }
            } else {
                // AWS S3 virtual-hosted default
                format!("https://{}.s3.{}.amazonaws.com", cfg.bucket, cfg.region)
            }
        });

        Ok(Self {
            store: Arc::new(store),
            bucket: cfg.bucket,
            public_url_base,
        })
    }
}

#[async_trait]
impl ObjectStore for S3Backend {
    async fn put(
        &self,
        key: &str,
        data: Bytes,
        _content_type: &str,
    ) -> Result<String, ObjectStoreError> {
        let path = OsPath::from(key);
        let payload = object_store::PutPayload::from_bytes(data);
        self.store
            .put(&path, payload)
            .await
            .map_err(|e| ObjectStoreError::S3(e.to_string()))?;
        let url = self.public_url(key);
        debug!(key, bucket = self.bucket, url, "s3: object uploaded");
        Ok(url)
    }

    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError> {
        let path = OsPath::from(key);
        let result = self.store.get(&path).await.map_err(|e| match e {
            object_store::Error::NotFound { .. } => ObjectStoreError::NotFound(key.to_string()),
            other => ObjectStoreError::S3(other.to_string()),
        })?;
        result
            .bytes()
            .await
            .map_err(|e| ObjectStoreError::S3(e.to_string()))
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let path = OsPath::from(key);
        self.store.delete(&path).await.map_err(|e| match e {
            object_store::Error::NotFound { .. } => ObjectStoreError::NotFound(key.to_string()),
            other => ObjectStoreError::S3(other.to_string()),
        })
    }

    fn public_url(&self, key: &str) -> String {
        let key = key.trim_start_matches('/');
        format!("{}/{}", self.public_url_base, key)
    }

    fn backend_name(&self) -> &'static str {
        "s3"
    }
}
