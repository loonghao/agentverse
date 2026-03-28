//! Skill package domain model.
//!
//! A `SkillPackage` represents a downloadable archive (zip/tar.gz) associated
//! with a specific `ArtifactVersion` of `kind = skill`. Each package records
//! the backend from which it can be fetched and the canonical download URL
//! that was recorded at publish time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The storage backend that hosts the skill package archive.
///
/// External backends (Clawhub, GitHub, GitHubRepo, Url) reference packages
/// hosted outside this registry.  The `Internal` variant means the archive is
/// stored in the registry's own object store (S3/COS/MinIO/local-disk/etc.)
/// and the `download_url` field holds the canonical URL returned by that store.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// OpenClaw / Clawhub marketplace (https://hub.openclaw.io)
    Clawhub,
    /// GitHub release asset — explicitly renamed so the wire format is "github"
    /// rather than the serde snake_case default "git_hub".
    #[serde(rename = "github")]
    GitHub,
    /// GitHub repository subdirectory skill (anthropics/skills pattern).
    /// The `metadata.github_repo` field holds owner/repo/ref/skill_path.
    #[serde(rename = "github_repo")]
    GitHubRepo,
    /// Generic HTTP/HTTPS URL (custom hosting)
    Url,
    /// Package hosted in the registry's own object store.
    ///
    /// The `download_url` on the `SkillPackage` is the pre-signed or public URL
    /// returned by the configured `ObjectStore` backend at upload time.
    /// At install time this is treated identically to `Url` — the existing
    /// `UrlBackend` handles the HTTP download transparently.
    Internal,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Clawhub => write!(f, "clawhub"),
            SourceType::GitHub => write!(f, "github"),
            SourceType::GitHubRepo => write!(f, "github_repo"),
            SourceType::Url => write!(f, "url"),
            SourceType::Internal => write!(f, "internal"),
        }
    }
}

impl std::str::FromStr for SourceType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "clawhub" => Ok(SourceType::Clawhub),
            "github" => Ok(SourceType::GitHub),
            "github_repo" => Ok(SourceType::GitHubRepo),
            "url" => Ok(SourceType::Url),
            "internal" => Ok(SourceType::Internal),
            other => Err(format!("unknown source_type: {other}")),
        }
    }
}

/// A downloadable skill package linked to one `ArtifactVersion`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPackage {
    pub id: Uuid,
    /// The specific artifact_version this package belongs to.
    pub artifact_version_id: Uuid,
    /// Which backend hosts the archive.
    pub source_type: SourceType,
    /// Canonical download URL resolved at publish time.
    pub download_url: String,
    /// SHA-256 hex checksum of the archive (optional).
    pub checksum: Option<String>,
    /// Compressed file size in bytes (optional, informational).
    pub file_size: Option<i64>,
    /// Arbitrary extra metadata (platform hints, agent compat, etc.).
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Represents an installed skill on a particular agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstall {
    pub id: Uuid,
    pub skill_package_id: Uuid,
    /// The agent runtime this skill is installed for.
    pub agent_kind: AgentKind,
    /// Absolute filesystem path where the skill was extracted.
    pub install_path: String,
    pub installed_at: DateTime<Utc>,
}

/// Known agent runtimes that consume skills.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    OpenClaw,
    CodeBuddy,
    WorkerBuddy,
    Claude,
    Augment,
    /// Any custom agent using the standard skill layout.
    Custom(String),
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentKind::OpenClaw => write!(f, "openclaw"),
            AgentKind::CodeBuddy => write!(f, "codebuddy"),
            AgentKind::WorkerBuddy => write!(f, "workerbuddy"),
            AgentKind::Claude => write!(f, "claude"),
            AgentKind::Augment => write!(f, "augment"),
            AgentKind::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl std::str::FromStr for AgentKind {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "openclaw" => AgentKind::OpenClaw,
            "codebuddy" => AgentKind::CodeBuddy,
            "workerbuddy" => AgentKind::WorkerBuddy,
            "claude" => AgentKind::Claude,
            "augment" => AgentKind::Augment,
            other => AgentKind::Custom(other.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SourceType Display ──────────────────────────────────────────────────────

    #[test]
    fn source_type_display_clawhub() {
        assert_eq!(SourceType::Clawhub.to_string(), "clawhub");
    }

    #[test]
    fn source_type_display_github() {
        assert_eq!(SourceType::GitHub.to_string(), "github");
    }

    #[test]
    fn source_type_display_github_repo() {
        assert_eq!(SourceType::GitHubRepo.to_string(), "github_repo");
    }

    #[test]
    fn source_type_display_url() {
        assert_eq!(SourceType::Url.to_string(), "url");
    }

    #[test]
    fn source_type_display_internal() {
        assert_eq!(SourceType::Internal.to_string(), "internal");
    }

    // ── SourceType FromStr ──────────────────────────────────────────────────────

    #[test]
    fn source_type_from_str_all_variants() {
        use std::str::FromStr;
        assert_eq!(
            SourceType::from_str("clawhub").unwrap(),
            SourceType::Clawhub
        );
        assert_eq!(SourceType::from_str("github").unwrap(), SourceType::GitHub);
        assert_eq!(
            SourceType::from_str("github_repo").unwrap(),
            SourceType::GitHubRepo
        );
        assert_eq!(SourceType::from_str("url").unwrap(), SourceType::Url);
        assert_eq!(
            SourceType::from_str("internal").unwrap(),
            SourceType::Internal
        );
    }

    #[test]
    fn source_type_from_str_unknown_returns_error() {
        use std::str::FromStr;
        assert!(SourceType::from_str("unknown_backend").is_err());
    }

    // ── SourceType JSON serialization ──────────────────────────────────────────

    #[test]
    fn source_type_serde_round_trip() {
        let variants = [
            SourceType::Clawhub,
            SourceType::GitHub,
            SourceType::GitHubRepo,
            SourceType::Url,
            SourceType::Internal,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).unwrap();
            let back: SourceType = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, back, "round-trip failed for {variant}");
        }
    }

    #[test]
    fn source_type_github_repo_serializes_as_github_repo_not_git_hub_repo() {
        let json = serde_json::to_string(&SourceType::GitHubRepo).unwrap();
        assert_eq!(
            json, r#""github_repo""#,
            "GitHubRepo must serialize as 'github_repo'"
        );
    }

    // ── AgentKind Display ──────────────────────────────────────────────────────

    #[test]
    fn agent_kind_display_known_variants() {
        assert_eq!(AgentKind::OpenClaw.to_string(), "openclaw");
        assert_eq!(AgentKind::CodeBuddy.to_string(), "codebuddy");
        assert_eq!(AgentKind::WorkerBuddy.to_string(), "workerbuddy");
        assert_eq!(AgentKind::Claude.to_string(), "claude");
        assert_eq!(AgentKind::Augment.to_string(), "augment");
    }

    #[test]
    fn agent_kind_display_custom() {
        assert_eq!(AgentKind::Custom("my-agent".into()).to_string(), "my-agent");
    }

    // ── AgentKind FromStr ──────────────────────────────────────────────────────

    #[test]
    fn agent_kind_from_str_known_variants() {
        use std::str::FromStr;
        assert_eq!(
            AgentKind::from_str("openclaw").unwrap(),
            AgentKind::OpenClaw
        );
        assert_eq!(
            AgentKind::from_str("codebuddy").unwrap(),
            AgentKind::CodeBuddy
        );
        assert_eq!(
            AgentKind::from_str("workerbuddy").unwrap(),
            AgentKind::WorkerBuddy
        );
        assert_eq!(AgentKind::from_str("claude").unwrap(), AgentKind::Claude);
        assert_eq!(AgentKind::from_str("augment").unwrap(), AgentKind::Augment);
    }

    #[test]
    fn agent_kind_from_str_unknown_becomes_custom() {
        use std::str::FromStr;
        let ak = AgentKind::from_str("my-special-agent").unwrap();
        assert_eq!(ak, AgentKind::Custom("my-special-agent".into()));
    }
}
