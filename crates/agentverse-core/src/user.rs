use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Who or what is publishing / interacting with the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserKind {
    /// A human developer
    Human,
    /// An AI agent acting autonomously
    Agent,
    /// Internal system actor
    System,
}

/// A registry actor — human or AI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub kind: UserKind,
    /// For agent users: their declared capabilities (MCP schema)
    pub capabilities: Option<serde_json::Value>,
    /// Ed25519 public key (hex) used to verify their signed manifests
    pub public_key: Option<String>,
    /// Argon2id password hash — never serialized to API responses.
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_agent(&self) -> bool {
        matches!(self.kind, UserKind::Agent)
    }
}

