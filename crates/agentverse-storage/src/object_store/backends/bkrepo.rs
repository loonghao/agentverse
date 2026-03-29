//! Tencent BlueKing bk-repo Generic Repository object store backend.
//!
//! Stores skill package archives in a bk-repo Generic repository using the
//! simple file REST API:
//!
//! | Operation | HTTP Method | Path                                              |
//! |-----------|-------------|---------------------------------------------------|
//! | Upload    | `PUT`       | `/generic/{project}/{repo}/{path}`                |
//! | Download  | `GET`       | `/generic/{project}/{repo}/{path}?download=true`  |
//! | Delete    | `DELETE`    | `/generic/{project}/{repo}/{path}`                |
//!
//! ## Authentication
//! Uses HTTP Basic Authentication (`Authorization: Basic <base64(user:pass)>`).
//! Credentials are resolved from config fields, falling back to the
//! `BKREPO_USERNAME` / `BKREPO_PASSWORD` environment variables.
//!
//! ## Download URLs
//! Public download URLs: `{endpoint}/generic/{project}/{repo}/{key}`
//! For anonymous access the repository must be configured as public in bk-repo.

use async_trait::async_trait;
use base64::Engine as _;
use bytes::Bytes;
use tracing::debug;

use crate::object_store::{config::BkRepoConfig, error::ObjectStoreError, ObjectStore};

pub struct BkRepoBackend {
    client: reqwest::Client,
    endpoint: String,
    project: String,
    repo: String,
    overwrite: bool,
}

impl BkRepoBackend {
    pub fn new(cfg: BkRepoConfig) -> Self {
        let username = cfg
            .username
            .or_else(|| std::env::var("BKREPO_USERNAME").ok())
            .unwrap_or_default();
        let password = cfg
            .password
            .or_else(|| std::env::var("BKREPO_PASSWORD").ok())
            .unwrap_or_default();

        let mut headers = reqwest::header::HeaderMap::new();
        if !username.is_empty() {
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            let value = format!("Basic {encoded}");
            headers.insert(
                reqwest::header::AUTHORIZATION,
                value.parse().expect("invalid auth header"),
            );
        }

        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "agentverse-object-store/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers(headers)
            .build()
            .expect("failed to build reqwest client for BkRepoBackend");

        Self {
            client,
            endpoint: cfg.endpoint.trim_end_matches('/').to_string(),
            project: cfg.project,
            repo: cfg.repo,
            overwrite: cfg.overwrite,
        }
    }

    /// Build the full bk-repo Generic API URL for a given key.
    fn api_url(&self, key: &str) -> String {
        let key = key.trim_start_matches('/');
        format!(
            "{}/generic/{}/{}/{}",
            self.endpoint, self.project, self.repo, key
        )
    }
}

#[async_trait]
impl ObjectStore for BkRepoBackend {
    async fn put(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<String, ObjectStoreError> {
        let url = self.api_url(key);
        let resp = self
            .client
            .put(&url)
            .header("Content-Type", content_type)
            .header("X-BKREPO-OVERWRITE", self.overwrite.to_string())
            .body(data)
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
            "bkrepo: object uploaded"
        );
        Ok(download_url)
    }

    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError> {
        let url = format!("{}?download=true", self.api_url(key));
        let resp = self
            .client
            .get(&url)
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
        let url = self.api_url(key);
        let resp = self
            .client
            .delete(&url)
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

    fn public_url(&self, key: &str) -> String {
        self.api_url(key)
    }

    fn backend_name(&self) -> &'static str {
        "bkrepo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_store::config::BkRepoConfig;

    fn make_backend() -> BkRepoBackend {
        BkRepoBackend::new(BkRepoConfig {
            endpoint: "https://bkrepo.example.com".into(),
            project: "myproject".into(),
            repo: "agentverse-packages".into(),
            username: Some("admin".into()),
            password: Some("secret".into()),
            overwrite: true,
        })
    }

    // ── api_url ────────────────────────────────────────────────────────────────

    #[test]
    fn api_url_constructs_correct_path() {
        let b = make_backend();
        assert_eq!(
            b.api_url("org/skill/1.0.0.zip"),
            "https://bkrepo.example.com/generic/myproject/agentverse-packages/org/skill/1.0.0.zip"
        );
    }

    #[test]
    fn api_url_strips_leading_slash_from_key() {
        let b = make_backend();
        assert_eq!(
            b.api_url("/org/skill/1.0.0.zip"),
            b.api_url("org/skill/1.0.0.zip")
        );
    }

    #[test]
    fn api_url_trims_trailing_slash_from_endpoint() {
        let b = BkRepoBackend::new(BkRepoConfig {
            endpoint: "https://bkrepo.example.com/".into(),
            project: "p".into(),
            repo: "r".into(),
            username: None,
            password: None,
            overwrite: true,
        });
        assert_eq!(b.api_url("k"), "https://bkrepo.example.com/generic/p/r/k");
    }

    // ── public_url / backend_name ──────────────────────────────────────────────

    #[test]
    fn public_url_equals_api_url() {
        let b = make_backend();
        assert_eq!(b.public_url("x/y.zip"), b.api_url("x/y.zip"));
    }

    #[test]
    fn backend_name_is_bkrepo() {
        let b = make_backend();
        assert_eq!(b.backend_name(), "bkrepo");
    }

    // ── HTTP round-trip (axum mock server) ────────────────────────────────────

    use std::sync::{Arc, Mutex};

    type FakeStore = Arc<Mutex<std::collections::HashMap<String, Bytes>>>;

    async fn start_bkrepo_mock_server() -> (String, tokio::task::JoinHandle<()>) {
        use axum::{
            body::Body, extract::Path as AxumPath, http::StatusCode, response::Response,
            routing::put, Router,
        };
        use tokio::net::TcpListener;

        let store: FakeStore = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let app = Router::new().route(
            "/generic/{project}/{repo}/{*key}",
            put({
                let store = store.clone();
                |AxumPath((_p, _r, key)): AxumPath<(String, String, String)>,
                 body: Bytes| async move {
                    store.lock().unwrap().insert(key, body);
                    StatusCode::OK
                }
            })
            .get({
                let store = store.clone();
                |AxumPath((_p, _r, key)): AxumPath<(String, String, String)>| async move {
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
                |AxumPath((_p, _r, key)): AxumPath<(String, String, String)>| async move {
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
        let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://127.0.0.1:{port}"), handle)
    }

    fn make_http_backend(base_url: &str) -> BkRepoBackend {
        BkRepoBackend::new(BkRepoConfig {
            endpoint: base_url.to_string(),
            project: "proj".into(),
            repo: "repo".into(),
            username: None,
            password: None,
            overwrite: true,
        })
    }

    #[tokio::test]
    async fn put_uploads_bytes_and_returns_download_url() {
        let (base_url, _srv) = start_bkrepo_mock_server().await;
        let b = make_http_backend(&base_url);
        let url = b
            .put(
                "org/skill/1.0.0.zip",
                Bytes::from_static(b"skill"),
                "application/zip",
            )
            .await
            .expect("put should succeed");
        assert!(url.contains("org/skill/1.0.0.zip"), "got: {url}");
    }

    #[tokio::test]
    async fn get_retrieves_previously_put_data() {
        let (base_url, _srv) = start_bkrepo_mock_server().await;
        let b = make_http_backend(&base_url);
        let data = Bytes::from_static(b"hello bkrepo");
        b.put("test/data.zip", data.clone(), "application/zip")
            .await
            .unwrap();
        let got = b.get("test/data.zip").await.expect("get should succeed");
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn get_missing_key_returns_not_found() {
        let (base_url, _srv) = start_bkrepo_mock_server().await;
        let b = make_http_backend(&base_url);
        let err = b.get("does-not-exist.zip").await.unwrap_err();
        assert!(matches!(err, ObjectStoreError::NotFound(_)), "got: {err:?}");
    }

    #[tokio::test]
    async fn delete_removes_previously_put_key() {
        let (base_url, _srv) = start_bkrepo_mock_server().await;
        let b = make_http_backend(&base_url);
        b.put("del.zip", Bytes::from_static(b"bye"), "application/zip")
            .await
            .unwrap();
        b.delete("del.zip").await.expect("delete should succeed");
        assert!(matches!(
            b.get("del.zip").await.unwrap_err(),
            ObjectStoreError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn delete_missing_key_returns_not_found() {
        let (base_url, _srv) = start_bkrepo_mock_server().await;
        let b = make_http_backend(&base_url);
        let err = b.delete("never-existed.zip").await.unwrap_err();
        assert!(matches!(err, ObjectStoreError::NotFound(_)), "got: {err:?}");
    }
}
