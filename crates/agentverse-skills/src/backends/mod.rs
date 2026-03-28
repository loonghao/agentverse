//! Multi-backend skill package providers.
//!
//! Each backend knows how to:
//!  1. Build a canonical download URL given namespace/name/version.
//!  2. Download the archive bytes and write them to a local file.
//!
//! Supported backends:
//!  - [`ClawhubBackend`]      — OpenClaw / Clawhub marketplace
//!  - [`GitHubBackend`]       — GitHub release assets
//!  - [`GitHubRepoBackend`]   — GitHub repo subdirectory (anthropics/skills pattern)
//!  - [`UrlBackend`]          — Generic HTTP/HTTPS URL

pub mod clawhub;
pub mod github;
pub mod github_repo;
pub mod url;

pub use clawhub::ClawhubBackend;
pub use github::GitHubBackend;
pub use github_repo::{parse_github_tree_url, GitHubRepoBackend, GitHubRepoInfo};
pub use url::UrlBackend;

use std::path::Path;

use agentverse_core::skill::SourceType;
use async_trait::async_trait;

use crate::error::SkillError;

/// A storage backend that can resolve and download skill package archives.
#[async_trait]
pub trait PackageBackend: Send + Sync {
    /// The source type this backend represents.
    fn source_type(&self) -> SourceType;

    /// Build the canonical download URL for a skill package.
    ///
    /// Returns `None` when this backend cannot serve the requested combination.
    fn build_download_url(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
    ) -> Option<String>;

    /// Download the archive to `dest` and return the number of bytes written.
    ///
    /// `url` is the canonical URL (may come from `build_download_url` or be
    /// stored in the database from a prior publish).
    async fn download(&self, url: &str, dest: &Path) -> Result<u64, SkillError>;
}

