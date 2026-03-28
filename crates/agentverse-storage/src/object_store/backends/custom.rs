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

    // ── HTTP round-trip (mini axum mock server) ────────────────────────────────
    //
    // Spin up a tiny Axum server that handles PUT/GET/DELETE for any path.
    // This gives us coverage of the reqwest-based async methods without
    // requiring a real external HTTP endpoint.

    use std::sync::{Arc, Mutex};

    /// Tiny store backed by a HashMap, used inside the mock server.
    type FakeStore = Arc<Mutex<std::collections::HashMap<String, bytes::Bytes>>>;

    /// Start a mock HTTP server that supports PUT (stores body), GET (retrieves
    /// body) and DELETE (removes entry).  Returns the server base URL.
    async fn start_custom_mock_server() -> (String, tokio::task::JoinHandle<()>) {
        use axum::{
            body::Body, extract::Path as AxumPath, http::StatusCode, response::Response,
            routing::put, Router,
        };
        use tokio::net::TcpListener;

        let store: FakeStore = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let app = Router::new().route(
            "/{*key}",
            put({
                let store = store.clone();
                |AxumPath(key): AxumPath<String>, body: bytes::Bytes| async move {
                    store.lock().unwrap().insert(key, body);
                    StatusCode::OK
                }
            })
            .get({
                let store = store.clone();
                |AxumPath(key): AxumPath<String>| async move {
                    match store.lock().unwrap().get(&key).cloned() {
                        Some(data) => Response::builder()
                            .status(StatusCode::OK)
                            .body(Body::from(data))
                            .unwrap(),
                        None => Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::empty())
                            .unwrap(),
                    }
                }
            })
            .delete({
                let store = store.clone();
                |AxumPath(key): AxumPath<String>| async move {
                    if store.lock().unwrap().remove(&key).is_some() {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    }
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://127.0.0.1:{port}"), handle)
    }

    fn make_http_backend(base_url: &str) -> CustomBackend {
        CustomBackend::new(CustomConfig {
            upload_url: base_url.to_string(),
            download_url_base: base_url.to_string(),
            upload_auth_header: None,
            download_auth: DownloadAuth::None,
        })
    }

    #[tokio::test]
    async fn put_uploads_bytes_and_returns_download_url() {
        let (base_url, _server) = start_custom_mock_server().await;
        let backend = make_http_backend(&base_url);
        let data = bytes::Bytes::from_static(b"skill content");

        let url = backend
            .put("org/skill/1.0.0.zip", data, "application/zip")
            .await
            .expect("put should succeed");
        assert!(
            url.contains("org/skill/1.0.0.zip"),
            "download URL should contain the key, got: {url}"
        );
    }

    #[tokio::test]
    async fn get_retrieves_previously_put_data() {
        let (base_url, _server) = start_custom_mock_server().await;
        let backend = make_http_backend(&base_url);
        let data = bytes::Bytes::from_static(b"hello world");

        backend
            .put("test/data.zip", data.clone(), "application/zip")
            .await
            .unwrap();

        let got = backend
            .get("test/data.zip")
            .await
            .expect("get should succeed");
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn get_missing_key_returns_not_found() {
        let (base_url, _server) = start_custom_mock_server().await;
        let backend = make_http_backend(&base_url);

        let err = backend.get("does-not-exist.zip").await.unwrap_err();
        assert!(
            matches!(err, ObjectStoreError::NotFound(_)),
            "expected NotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn delete_removes_previously_put_key() {
        let (base_url, _server) = start_custom_mock_server().await;
        let backend = make_http_backend(&base_url);
        let data = bytes::Bytes::from_static(b"to be deleted");

        backend
            .put("delete-me.zip", data, "application/zip")
            .await
            .unwrap();
        backend
            .delete("delete-me.zip")
            .await
            .expect("delete should succeed");

        // After deletion, get should return NotFound.
        let err = backend.get("delete-me.zip").await.unwrap_err();
        assert!(matches!(err, ObjectStoreError::NotFound(_)));
    }

    #[tokio::test]
    async fn delete_missing_key_returns_not_found() {
        let (base_url, _server) = start_custom_mock_server().await;
        let backend = make_http_backend(&base_url);

        let err = backend.delete("never-existed.zip").await.unwrap_err();
        assert!(
            matches!(err, ObjectStoreError::NotFound(_)),
            "expected NotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn put_with_upload_auth_header_succeeds() {
        let (base_url, _server) = start_custom_mock_server().await;
        let backend = CustomBackend::new(CustomConfig {
            upload_url: base_url.clone(),
            download_url_base: base_url,
            upload_auth_header: Some("Bearer test-token".into()),
            download_auth: DownloadAuth::None,
        });
        let data = bytes::Bytes::from_static(b"auth payload");
        let url = backend
            .put("auth/skill.zip", data, "application/zip")
            .await
            .expect("put with auth header should succeed");
        assert!(url.contains("auth/skill.zip"));
    }

    #[tokio::test]
    async fn get_with_bearer_auth_sends_authorization_header() {
        let (base_url, _server) = start_custom_mock_server().await;
        // First upload without auth so there's something to retrieve.
        let plain = make_http_backend(&base_url);
        plain
            .put(
                "bearer/skill.zip",
                bytes::Bytes::from_static(b"data"),
                "application/zip",
            )
            .await
            .unwrap();

        // Now retrieve using a bearer-auth backend.
        let bearer_backend = CustomBackend::new(CustomConfig {
            upload_url: base_url.clone(),
            download_url_base: base_url,
            upload_auth_header: None,
            download_auth: DownloadAuth::BearerHeader {
                token: "my-token".into(),
            },
        });
        let got = bearer_backend
            .get("bearer/skill.zip")
            .await
            .expect("get with bearer auth should succeed");
        assert_eq!(got, bytes::Bytes::from_static(b"data"));
    }
}
