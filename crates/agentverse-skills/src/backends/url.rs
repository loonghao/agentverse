//! Generic URL backend — downloads skill packages from any HTTP/HTTPS URL.
//!
//! This backend is used when the skill publisher provides an explicit download
//! URL that doesn't map to a specific marketplace. The URL must be stored in
//! the database because `build_download_url` has no templating information.

use std::path::Path;

use agentverse_core::skill::SourceType;
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::error::SkillError;

use super::PackageBackend;

/// Downloads skill packages from an arbitrary HTTP/HTTPS URL.
pub struct UrlBackend {
    client: reqwest::Client,
}

impl UrlBackend {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("agentverse-skills/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }
}

impl Default for UrlBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for UrlBackend {
    fn source_type(&self) -> SourceType {
        SourceType::Url
    }

    /// The generic backend requires the caller to supply the full URL; it cannot
    /// derive one from namespace/name/version alone.
    fn build_download_url(&self, _namespace: &str, _name: &str, _version: &str) -> Option<String> {
        None
    }

    async fn download(&self, url: &str, dest: &Path) -> Result<u64, SkillError> {
        tracing::debug!(url, "url: downloading skill package");

        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(SkillError::Backend(format!(
                "remote server returned HTTP {status} for {url}"
            )));
        }

        let mut file = tokio::fs::File::create(dest).await?;
        let bytes = resp.bytes().await?;
        file.write_all(&bytes).await?;
        Ok(bytes.len() as u64)
    }
}
