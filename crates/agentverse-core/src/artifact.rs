use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The type of artifact stored in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Skill,
    Soul,
    Agent,
    Workflow,
    Prompt,
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::Skill => write!(f, "skill"),
            ArtifactKind::Soul => write!(f, "soul"),
            ArtifactKind::Agent => write!(f, "agent"),
            ArtifactKind::Workflow => write!(f, "workflow"),
            ArtifactKind::Prompt => write!(f, "prompt"),
        }
    }
}

/// Lifecycle status of an artifact, following AgentHub recommendations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    #[default]
    Active,
    Deprecated,
    Retired,
    Revoked,
}

/// Capability declaration inside the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    /// Supported protocols: mcp, openai-function, rest, etc.
    pub protocols: Vec<String>,
    /// Required permissions the caller must grant.
    pub permissions: Vec<String>,
    pub max_tokens: Option<u32>,
}

/// Structured manifest describing the artifact's contract.
/// Inspired by Android permission model + MCP capability schema.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub description: String,
    pub capabilities: Capabilities,
    /// SBOM-style dependency declarations: "namespace/name" -> semver constraint
    pub dependencies: std::collections::HashMap<String, String>,
    pub tags: Vec<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    /// Arbitrary extra metadata (framework-specific extensions)
    pub extra: serde_json::Value,
}

/// The top-level artifact entity (the "package" in registry terms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub kind: ArtifactKind,
    pub namespace: String,
    pub name: String,
    pub display_name: Option<String>,
    pub manifest: Manifest,
    pub status: ArtifactStatus,
    pub author_id: Uuid,
    pub downloads: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Artifact {
    pub fn registry_id(&self) -> String {
        format!("{}/{}/{}", self.kind, self.namespace, self.name)
    }

    pub fn is_modifiable(&self) -> bool {
        matches!(self.status, ArtifactStatus::Active)
    }
}

/// A single immutable version of an artifact.
/// Once published, a version is append-only (content is never mutated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVersion {
    pub id: Uuid,
    pub artifact_id: Uuid,
    /// Semantic version string, e.g. "1.2.3"
    pub version: String,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
    /// The actual payload (prompt text, agent config, workflow DAG, etc.)
    pub content: serde_json::Value,
    /// Sha256 checksum of the canonical content JSON
    pub checksum: String,
    /// Ed25519 signature by the publisher
    pub signature: Option<String>,
    pub changelog: Option<String>,
    /// Why the version was bumped: patch | minor | major
    pub bump_reason: String,
    pub published_by: Uuid,
    pub published_at: DateTime<Utc>,
}

impl ArtifactVersion {
    pub fn semver_string(&self) -> String {
        match &self.pre_release {
            Some(pre) => format!("{}.{}.{}-{}", self.major, self.minor, self.patch, pre),
            None => format!("{}.{}.{}", self.major, self.minor, self.patch),
        }
    }
}

