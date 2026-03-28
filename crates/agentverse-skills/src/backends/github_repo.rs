//! GitHub Repository backend — skills stored as directories inside a GitHub repo.
//!
//! This is the pattern used by `anthropics/skills` and `vercel-labs/agent-skills`:
//! each skill is a subdirectory containing a `SKILL.md` file and optional assets.
//!
//! ## URL patterns
//!
//! Input (GitHub tree view):
//!   `https://github.com/{owner}/{repo}/tree/{ref}/{skill_path}`
//!
//! Archive download (no auth required for public repos):
//!   `https://github.com/{owner}/{repo}/archive/{ref}.zip`
//!
//! Raw SKILL.md fetch:
//!   `https://raw.githubusercontent.com/{owner}/{repo}/{ref}/{skill_path}/SKILL.md`
//!
//! ## Metadata stored in `SkillPackage.metadata`
//!
//! ```json
//! { "github_repo": { "owner": "anthropics", "repo": "skills",
//!                    "ref": "main", "skill_path": "skills/algorithmic-art" } }
//! ```

use std::path::Path;

use agentverse_core::skill::SourceType;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::error::SkillError;

use super::PackageBackend;

// ── Parsed GitHub tree URL ────────────────────────────────────────────────────

/// Components extracted from a `github.com/.../tree/...` URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRepoInfo {
    pub owner: String,
    pub repo: String,
    /// Branch name, tag, or full commit SHA.
    pub git_ref: String,
    /// Path within the repo to the skill directory (e.g. `skills/algorithmic-art`).
    pub skill_path: String,
}

impl GitHubRepoInfo {
    /// URL to download the whole-repo archive zip.
    pub fn archive_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/archive/{}.zip",
            self.owner, self.repo, self.git_ref
        )
    }

    /// Raw URL for a specific file inside the skill directory.
    pub fn raw_url(&self, file: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}/{}",
            self.owner, self.repo, self.git_ref, self.skill_path, file
        )
    }

    /// Convert to the `metadata.github_repo` JSON value for `SkillPackage`.
    pub fn to_metadata_json(&self) -> serde_json::Value {
        serde_json::json!({
            "github_repo": {
                "owner": self.owner,
                "repo":  self.repo,
                "ref":   self.git_ref,
                "skill_path": self.skill_path
            }
        })
    }
}

/// Parse a GitHub tree URL into its components.
///
/// Accepts both forms:
/// - `https://github.com/anthropics/skills/tree/main/skills/algorithmic-art`
/// - `https://github.com/vercel-labs/agent-skills/tree/main/skills/deploy-to-vercel`
pub fn parse_github_tree_url(url: &str) -> Option<GitHubRepoInfo> {
    let url = url.trim_end_matches('/');
    let tail = url.strip_prefix("https://github.com/")?;

    // tail: "{owner}/{repo}/tree/{ref}/{...skill_path...}"
    // splitn(5) gives at most 5 parts; the last one absorbs the rest.
    let parts: Vec<&str> = tail.splitn(5, '/').collect();
    if parts.len() < 5 || parts[2] != "tree" {
        return None;
    }

    Some(GitHubRepoInfo {
        owner: parts[0].to_string(),
        repo: parts[1].to_string(),
        git_ref: parts[3].to_string(),
        skill_path: parts[4].to_string(),
    })
}

// ── Backend implementation ────────────────────────────────────────────────────

/// Downloads skill packages from GitHub repository subdirectories.
pub struct GitHubRepoBackend {
    client: reqwest::Client,
}

impl GitHubRepoBackend {
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

    /// Fetch the raw SKILL.md text for a repo-based skill.
    pub async fn fetch_skill_md(&self, info: &GitHubRepoInfo) -> Result<String, SkillError> {
        let url = info.raw_url("SKILL.md");
        tracing::debug!(%url, "fetching SKILL.md");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(SkillError::Backend(format!(
                "GitHub returned HTTP {status} fetching SKILL.md from {url}"
            )));
        }
        Ok(resp.text().await?)
    }
}

impl Default for GitHubRepoBackend {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl PackageBackend for GitHubRepoBackend {
    fn source_type(&self) -> SourceType {
        SourceType::GitHubRepo
    }

    /// Cannot derive a URL purely from namespace/name/version — returns `None`.
    /// The caller stores the archive URL explicitly via `GitHubRepoInfo::archive_url()`.
    fn build_download_url(&self, _namespace: &str, _name: &str, _version: &str) -> Option<String> {
        None
    }

    async fn download(&self, url: &str, dest: &Path) -> Result<u64, SkillError> {
        tracing::debug!(url, "github_repo: downloading repo archive");

        let resp = self
            .client
            .get(url)
            .header("Accept", "application/zip")
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

