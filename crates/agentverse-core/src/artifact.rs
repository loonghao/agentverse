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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_artifact(kind: ArtifactKind, status: ArtifactStatus) -> Artifact {
        Artifact {
            id: Uuid::new_v4(),
            kind,
            namespace: "test-ns".into(),
            name: "test-art".into(),
            display_name: None,
            manifest: Manifest::default(),
            status,
            author_id: Uuid::new_v4(),
            downloads: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_version(major: u64, minor: u64, patch: u64, pre: Option<&str>) -> ArtifactVersion {
        ArtifactVersion {
            id: Uuid::new_v4(),
            artifact_id: Uuid::new_v4(),
            version: format!("{major}.{minor}.{patch}"),
            major,
            minor,
            patch,
            pre_release: pre.map(String::from),
            content: serde_json::Value::Null,
            checksum: "deadbeef".into(),
            signature: None,
            changelog: None,
            bump_reason: "patch".into(),
            published_by: Uuid::new_v4(),
            published_at: Utc::now(),
        }
    }

    #[test]
    fn registry_id_format() {
        let a = make_artifact(ArtifactKind::Skill, ArtifactStatus::Active);
        assert_eq!(a.registry_id(), "skill/test-ns/test-art");
    }

    #[test]
    fn active_artifact_is_modifiable() {
        let a = make_artifact(ArtifactKind::Agent, ArtifactStatus::Active);
        assert!(a.is_modifiable());
    }

    #[test]
    fn deprecated_artifact_not_modifiable() {
        let a = make_artifact(ArtifactKind::Prompt, ArtifactStatus::Deprecated);
        assert!(!a.is_modifiable());
    }

    #[test]
    fn revoked_artifact_not_modifiable() {
        let a = make_artifact(ArtifactKind::Workflow, ArtifactStatus::Revoked);
        assert!(!a.is_modifiable());
    }

    #[test]
    fn artifact_kind_display_all_variants() {
        assert_eq!(ArtifactKind::Skill.to_string(), "skill");
        assert_eq!(ArtifactKind::Soul.to_string(), "soul");
        assert_eq!(ArtifactKind::Agent.to_string(), "agent");
        assert_eq!(ArtifactKind::Workflow.to_string(), "workflow");
        assert_eq!(ArtifactKind::Prompt.to_string(), "prompt");
    }

    #[test]
    fn semver_string_without_pre_release() {
        let v = make_version(1, 2, 3, None);
        assert_eq!(v.semver_string(), "1.2.3");
    }

    #[test]
    fn semver_string_with_pre_release() {
        let v = make_version(2, 0, 0, Some("alpha.1"));
        assert_eq!(v.semver_string(), "2.0.0-alpha.1");
    }

    #[test]
    fn artifact_kind_serde_round_trip() {
        let kind = ArtifactKind::Workflow;
        let json = serde_json::to_string(&kind).unwrap();
        let back: ArtifactKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn artifact_status_default_is_active() {
        assert_eq!(ArtifactStatus::default(), ArtifactStatus::Active);
    }
}
