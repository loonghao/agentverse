//! GitHub Releases object store backend.
//!
//! Uploads skill package archives as assets on a GitHub repository's releases.
//! Each upload creates (or reuses) a release tagged `internal-packages` and
//! attaches the zip as an asset named `{key}` (slashes replaced with `--`).
//!
//! ## Download URL
//! GitHub release assets are publicly downloadable without auth:
//! `https://github.com/{owner}/{repo}/releases/download/{tag}/{asset_name}`
//!
//! ## Required token permissions
//! The GitHub token needs `contents: write` on the target repository.
//! Set via config `token` field or the `GITHUB_TOKEN` environment variable.

use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, warn};

use crate::object_store::{config::GitHubConfig, error::ObjectStoreError, ObjectStore};

const RELEASE_TAG: &str = "internal-packages";

pub struct GitHubReleaseBackend {
    client: reqwest::Client,
    owner: String,
    repo: String,
}

impl GitHubReleaseBackend {
    pub fn new(cfg: GitHubConfig) -> Self {
        let token = cfg
            .token
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .unwrap_or_default();

        let mut headers = reqwest::header::HeaderMap::new();
        if !token.is_empty() {
            let auth = format!("Bearer {token}");
            headers.insert(
                reqwest::header::AUTHORIZATION,
                auth.parse().expect("invalid GitHub token"),
            );
        }
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());

        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "agentverse-object-store/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers(headers)
            .build()
            .expect("failed to build reqwest client for GitHubReleaseBackend");

        Self {
            client,
            owner: cfg.owner,
            repo: cfg.repo,
        }
    }

    /// Sanitise a storage key into a valid GitHub asset name (no slashes).
    fn asset_name(key: &str) -> String {
        key.trim_start_matches('/').replace('/', "--")
    }

    /// Ensure the `internal-packages` release exists; return its upload URL.
    async fn ensure_release(&self) -> Result<String, ObjectStoreError> {
        let api_base = format!(
            "https://api.github.com/repos/{}/{}/releases",
            self.owner, self.repo
        );

        // Try to find existing release.
        let list: serde_json::Value = self
            .client
            .get(&api_base)
            .send()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?;

        if let Some(releases) = list.as_array() {
            for r in releases {
                if r["tag_name"].as_str() == Some(RELEASE_TAG) {
                    let upload_url = r["upload_url"].as_str().unwrap_or("").to_string();
                    return Ok(upload_url.split('{').next().unwrap_or("").to_string());
                }
            }
        }

        // Create release if it doesn't exist.
        let body = serde_json::json!({
            "tag_name": RELEASE_TAG,
            "name": "Internal Packages",
            "body": "Managed by agentverse-server. Do not delete.",
            "prerelease": true,
        });
        let resp: serde_json::Value = self
            .client
            .post(&api_base)
            .json(&body)
            .send()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?;

        let upload_url = resp["upload_url"].as_str().unwrap_or("").to_string();
        Ok(upload_url.split('{').next().unwrap_or("").to_string())
    }
}

#[async_trait]
impl ObjectStore for GitHubReleaseBackend {
    async fn put(
        &self,
        key: &str,
        data: Bytes,
        _content_type: &str,
    ) -> Result<String, ObjectStoreError> {
        let asset_name = Self::asset_name(key);
        let upload_base = self.ensure_release().await?;
        let upload_url = format!("{upload_base}?name={asset_name}&label={asset_name}");

        let resp = self
            .client
            .post(&upload_url)
            .header("Content-Type", "application/zip")
            .body(data)
            .send()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ObjectStoreError::UploadRejected { status: 0, body });
        }

        let download_url = self.public_url(key);
        debug!(
            key,
            asset_name, download_url, "github_release: asset uploaded"
        );
        Ok(download_url)
    }

    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError> {
        let url = self.public_url(key);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ObjectStoreError::NotFound(key.to_string()));
        }
        resp.bytes()
            .await
            .map_err(|e| ObjectStoreError::Http(e.to_string()))
    }

    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        // Deleting release assets requires finding the asset ID first.
        // For now, log a warning — this is a non-critical operation.
        warn!(
            key,
            "github_release: delete not implemented; asset left in place"
        );
        Ok(())
    }

    fn public_url(&self, key: &str) -> String {
        let asset = Self::asset_name(key);
        format!(
            "https://github.com/{}/{}/releases/download/{RELEASE_TAG}/{asset}",
            self.owner, self.repo
        )
    }

    fn backend_name(&self) -> &'static str {
        "github_release"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_store::config::GitHubConfig;

    fn make_backend(token: Option<&str>) -> GitHubReleaseBackend {
        GitHubReleaseBackend::new(GitHubConfig {
            owner: "testowner".into(),
            repo: "testrepo".into(),
            token: token.map(str::to_string),
        })
    }

    // ── asset_name ─────────────────────────────────────────────────────────────

    #[test]
    fn asset_name_replaces_slashes_with_double_dash() {
        assert_eq!(
            GitHubReleaseBackend::asset_name("org/skill/1.0.0.zip"),
            "org--skill--1.0.0.zip"
        );
    }

    #[test]
    fn asset_name_strips_leading_slash() {
        assert_eq!(
            GitHubReleaseBackend::asset_name("/org/skill/1.0.0.zip"),
            "org--skill--1.0.0.zip"
        );
    }

    #[test]
    fn asset_name_simple_key_unchanged() {
        assert_eq!(GitHubReleaseBackend::asset_name("skill.zip"), "skill.zip");
    }

    #[test]
    fn asset_name_multiple_levels() {
        assert_eq!(
            GitHubReleaseBackend::asset_name("a/b/c/d.zip"),
            "a--b--c--d.zip"
        );
    }

    // ── public_url ─────────────────────────────────────────────────────────────

    #[test]
    fn public_url_constructs_github_release_asset_url() {
        let b = make_backend(None);
        assert_eq!(
            b.public_url("org/skill/1.0.0.zip"),
            format!(
                "https://github.com/testowner/testrepo/releases/download/{RELEASE_TAG}/org--skill--1.0.0.zip"
            )
        );
    }

    #[test]
    fn public_url_strips_leading_slash_from_key() {
        let b = make_backend(None);
        let with_slash = b.public_url("/ns/skill/0.1.0.zip");
        let without_slash = b.public_url("ns/skill/0.1.0.zip");
        assert_eq!(
            with_slash, without_slash,
            "leading slash should not affect URL"
        );
    }

    #[test]
    fn public_url_contains_release_tag() {
        let b = make_backend(None);
        let url = b.public_url("skill.zip");
        assert!(
            url.contains(RELEASE_TAG),
            "URL must contain the release tag constant, got: {url}"
        );
    }

    // ── backend_name ───────────────────────────────────────────────────────────

    #[test]
    fn backend_name_is_github_release() {
        let b = make_backend(None);
        assert_eq!(b.backend_name(), "github_release");
    }

    // ── constructor ────────────────────────────────────────────────────────────

    #[test]
    fn new_with_no_token_does_not_panic() {
        let b = make_backend(None);
        // Confirm basic fields are set correctly.
        assert_eq!(b.owner, "testowner");
        assert_eq!(b.repo, "testrepo");
    }

    #[test]
    fn new_with_token_does_not_panic() {
        // Token path adds an Authorization header — should not panic.
        let b = make_backend(Some("ghp_fake_token_for_testing"));
        assert_eq!(b.owner, "testowner");
    }

    #[test]
    fn new_falls_back_to_env_token_when_config_token_is_none() {
        // If GITHUB_TOKEN is set in the environment the backend should pick it up.
        // We don't assert the token value here — just that construction succeeds.
        let _b = GitHubReleaseBackend::new(GitHubConfig {
            owner: "o".into(),
            repo: "r".into(),
            token: None, // relies on env fallback
        });
    }
}
