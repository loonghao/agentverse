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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_user(kind: UserKind) -> User {
        User {
            id: Uuid::new_v4(),
            username: "alice".into(),
            email: Some("alice@example.com".into()),
            kind,
            capabilities: None,
            public_key: None,
            password_hash: Some("$argon2id$...".into()),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn human_is_not_agent() {
        assert!(!make_user(UserKind::Human).is_agent());
    }

    #[test]
    fn agent_is_agent() {
        assert!(make_user(UserKind::Agent).is_agent());
    }

    #[test]
    fn system_is_not_agent() {
        assert!(!make_user(UserKind::System).is_agent());
    }

    #[test]
    fn password_hash_not_serialized() {
        let user = make_user(UserKind::Human);
        let json = serde_json::to_value(&user).unwrap();
        assert!(
            json.get("password_hash").is_none(),
            "password_hash must not appear in JSON"
        );
    }

    #[test]
    fn user_kind_serde_round_trip() {
        for kind in [UserKind::Human, UserKind::Agent, UserKind::System] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: UserKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }
}
