//! GitHub backend — downloads skill packages from GitHub release assets.
//!
//! URL pattern (GitHub releases API asset):
//!   `https://github.com/{owner}/{repo}/releases/download/{tag}/{asset}`
//!
//! The `build_download_url` method returns a conventional URL using the
//! namespace as the GitHub owner and the name as the repo. Callers may
//! override with a fully explicit URL stored in the database.

use std::path::Path;

use agentverse_core::skill::SourceType;
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::error::SkillError;

use super::PackageBackend;

/// Downloads skill packages from GitHub release assets.
pub struct GitHubBackend {
    client: reqwest::Client,
}

impl GitHubBackend {
    pub fn new(token: Option<&str>) -> Self {
        let mut builder = reqwest::Client::builder()
            .user_agent(concat!("agentverse-skills/", env!("CARGO_PKG_VERSION")));

        if let Some(tok) = token {
            let mut headers = reqwest::header::HeaderMap::new();
            let auth = reqwest::header::HeaderValue::from_str(&format!("Bearer {tok}"))
                .expect("invalid token header value");
            headers.insert(reqwest::header::AUTHORIZATION, auth);
            builder = builder.default_headers(headers);
        }

        Self {
            client: builder.build().expect("failed to build HTTP client"),
        }
    }
}

impl Default for GitHubBackend {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl PackageBackend for GitHubBackend {
    fn source_type(&self) -> SourceType {
        SourceType::GitHub
    }

    /// Builds: `https://github.com/{namespace}/{name}/releases/download/v{version}/{name}-v{version}.zip`
    fn build_download_url(&self, namespace: &str, name: &str, version: &str) -> Option<String> {
        Some(format!(
            "https://github.com/{namespace}/{name}/releases/download/v{version}/{name}-v{version}.zip"
        ))
    }

    async fn download(&self, url: &str, dest: &Path) -> Result<u64, SkillError> {
        tracing::debug!(url, "github: downloading skill package");

        // Follow GitHub redirects (assets return 302 to CDN)
        let resp = self
            .client
            .get(url)
            .header("Accept", "application/octet-stream")
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(SkillError::Backend(format!(
                "GitHub returned HTTP {status} for {url}"
            )));
        }

        let mut file = tokio::fs::File::create(dest).await?;
        let bytes = resp.bytes().await?;
        file.write_all(&bytes).await?;
        Ok(bytes.len() as u64)
    }
}

