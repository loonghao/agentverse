//! Clawhub backend — OpenClaw marketplace package registry.
//!
//! Default base URL: `https://hub.openclaw.io`
//!
//! Package URL pattern:
//!   `{base_url}/skills/{namespace}/{name}/{version}.zip`

use std::path::Path;

use agentverse_core::skill::SourceType;
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::error::SkillError;

use super::PackageBackend;

/// Downloads skill packages from the OpenClaw / Clawhub marketplace.
pub struct ClawhubBackend {
    client: reqwest::Client,
    /// Base URL of the Clawhub registry (no trailing slash).
    base_url: String,
}

impl ClawhubBackend {
    /// Create a backend pointing at the public Clawhub registry.
    pub fn new() -> Self {
        Self::with_base_url("https://hub.openclaw.io")
    }

    /// Create a backend pointing at a custom Clawhub-compatible registry.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("agentverse-skills/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }
}

impl Default for ClawhubBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for ClawhubBackend {
    fn source_type(&self) -> SourceType {
        SourceType::Clawhub
    }

    fn build_download_url(&self, namespace: &str, name: &str, version: &str) -> Option<String> {
        Some(format!(
            "{}/skills/{namespace}/{name}/{version}.zip",
            self.base_url
        ))
    }

    async fn download(&self, url: &str, dest: &Path) -> Result<u64, SkillError> {
        tracing::debug!(url, "clawhub: downloading skill package");
        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(SkillError::Backend(format!(
                "Clawhub returned HTTP {status} for {url}"
            )));
        }

        let mut file = tokio::fs::File::create(dest).await?;
        let bytes = resp.bytes().await?;
        file.write_all(&bytes).await?;
        Ok(bytes.len() as u64)
    }
}

