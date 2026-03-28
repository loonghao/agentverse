//! Custom HTTP object store backend.
//!
//! ## Upload
//! `PUT {upload_url}/{key}` — raw bytes + `Content-Type: application/zip`.
//! Attach `upload_auth_header` (e.g. `"Bearer <token>"`) if required.
//!
//! ## Download URL strategies
//!
//! | `DownloadAuth`  | Generated URL                                |
//! |-----------------|----------------------------------------------|
//! | `None`          | `{download_url_base}/{key}`                  |
//! | `QueryParam`    | `{download_url_base}/{key}?{param}={token}`  |
//! | `BearerHeader`  | `{download_url_base}/{key}` + separate token |
//!
//! For `BearerHeader` the server returns a `download_token` field alongside
//! `download_url`; the CLI adds `Authorization: Bearer {token}` per-request.

use async_trait::async_trait;
use bytes::Bytes;
use tracing::debug;

use crate::object_store::{
    config::{CustomConfig, DownloadAuth},
    error::ObjectStoreError,
    ObjectStore,
};

pub struct CustomBackend {
    client: reqwest::Client,
    upload_url: String,
    download_url_base: String,
    upload_auth_header: Option<String>,
    download_auth: DownloadAuth,
}

impl CustomBackend {
    pub fn new(cfg: CustomConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "agentverse-object-store/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("failed to build reqwest client for CustomBackend");
        Self {
            client,
            upload_url: cfg.upload_url.trim_end_matches('/').to_string(),
            download_url_base: cfg.download_url_base.trim_end_matches('/').to_string(),
            upload_auth_header: cfg.upload_auth_header,
            download_auth: cfg.download_auth,
        }
    }
}

#[async_trait]
impl ObjectStore for CustomBackend {
    async fn put(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<String, ObjectStoreError> {
        let url = format!("{}/{}", self.upload_url, key.trim_start_matches('/'));
        let mut req = self
            .client
            .put(&url)
            .header("Content-Type", content_type)
            .body(data);
        if let Some(auth) = &self.upload_auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ObjectStoreError::UploadRejected {
                status: status.as_u16(),
                body,
            });
        }

        let download_url = self.public_url(key);
        debug!(
            key,
            upload_url = url,
            download_url,
            "custom: object uploaded"
        );
        Ok(download_url)
    }

    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError> {
        // Use plain base URL for server-side fetches; attach bearer if configured.
        let url = format!("{}/{}", self.download_url_base, key.trim_start_matches('/'));
        let mut req = self.client.get(&url);
        if let DownloadAuth::BearerHeader { token } = &self.download_auth {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ObjectStoreError::NotFound(key.to_string()));
        }
        if !resp.status().is_success() {
            return Err(ObjectStoreError::Http(format!(
                "GET {url} returned {}",
                resp.status()
            )));
        }
        resp.bytes()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let url = format!("{}/{}", self.upload_url, key.trim_start_matches('/'));
        let mut req = self.client.delete(&url);
        if let Some(auth) = &self.upload_auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ObjectStoreError::NotFound(key.to_string()));
        }
        if !resp.status().is_success() {
            return Err(ObjectStoreError::Http(format!(
                "DELETE {url} returned {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// Build the download URL with optional embedded auth.
    fn public_url(&self, key: &str) -> String {
        let key = key.trim_start_matches('/');
        let base = format!("{}/{}", self.download_url_base, key);
        match &self.download_auth {
            DownloadAuth::None | DownloadAuth::BearerHeader { .. } => base,
            DownloadAuth::QueryParam { param, token } => format!("{base}?{param}={token}"),
        }
    }

    /// Return the bearer token when `BearerHeader` auth is configured so the
    /// API layer can include it in the response as `download_token`.
    fn download_bearer_token(&self) -> Option<&str> {
        if let DownloadAuth::BearerHeader { token } = &self.download_auth {
            Some(token.as_str())
        } else {
            None
        }
    }

    fn backend_name(&self) -> &'static str {
        "custom"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_store::config::{CustomConfig, DownloadAuth};

    fn make_backend(download_auth: DownloadAuth) -> CustomBackend {
        CustomBackend::new(CustomConfig {
            upload_url: "https://upload.example.com".into(),
            download_url_base: "https://cdn.example.com".into(),
            upload_auth_header: None,
            download_auth,
        })
    }

    // ── public_url ─────────────────────────────────────────────────────────────

    #[test]
    fn public_url_no_auth_is_plain_url() {
        let b = make_backend(DownloadAuth::None);
        assert_eq!(
            b.public_url("org/skill/0.1.0.zip"),
            "https://cdn.example.com/org/skill/0.1.0.zip"
        );
    }

    #[test]
    fn public_url_strips_leading_slash_from_key() {
        let b = make_backend(DownloadAuth::None);
        // Leading slash must not produce a double-slash in the URL.
        assert_eq!(
            b.public_url("/org/skill/0.1.0.zip"),
            "https://cdn.example.com/org/skill/0.1.0.zip"
        );
    }

    #[test]
    fn public_url_query_param_appends_token() {
        let b = make_backend(DownloadAuth::QueryParam {
            param: "token".into(),
            token: "secret123".into(),
        });
        assert_eq!(
            b.public_url("org/skill/0.1.0.zip"),
            "https://cdn.example.com/org/skill/0.1.0.zip?token=secret123"
        );
    }

    #[test]
    fn public_url_bearer_header_is_plain_url() {
        // BearerHeader strategy must NOT embed the token in the URL.
        let b = make_backend(DownloadAuth::BearerHeader {
            token: "my-bearer-token".into(),
        });
        assert_eq!(
            b.public_url("org/skill/0.1.0.zip"),
            "https://cdn.example.com/org/skill/0.1.0.zip"
        );
    }

    // ── download_bearer_token ──────────────────────────────────────────────────

    #[test]
    fn download_bearer_token_returns_some_when_configured() {
        let b = make_backend(DownloadAuth::BearerHeader {
            token: "my-bearer-token".into(),
        });
        assert_eq!(b.download_bearer_token(), Some("my-bearer-token"));
    }

    #[test]
    fn download_bearer_token_returns_none_for_no_auth() {
        let b = make_backend(DownloadAuth::None);
        assert_eq!(b.download_bearer_token(), None);
    }

    #[test]
    fn download_bearer_token_returns_none_for_query_param() {
        let b = make_backend(DownloadAuth::QueryParam {
            param: "tok".into(),
            token: "abc".into(),
        });
        assert_eq!(b.download_bearer_token(), None);
    }

    // ── misc ───────────────────────────────────────────────────────────────────

    #[test]
    fn backend_name_is_custom() {
        let b = make_backend(DownloadAuth::None);
        assert_eq!(b.backend_name(), "custom");
    }
}
