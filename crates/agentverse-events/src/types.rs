use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All domain events produced by the system.
/// Each variant maps to one write operation and is stored append-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    // ── Artifact lifecycle ────────────────────────────────────────────────
    ArtifactCreated {
        artifact_id: Uuid,
        kind: String,
        namespace: String,
        name: String,
        author_id: Uuid,
    },
    ArtifactUpdated {
        artifact_id: Uuid,
        updated_by: Uuid,
    },
    ArtifactDeprecated {
        artifact_id: Uuid,
        deprecated_by: Uuid,
    },
    ArtifactRevoked {
        artifact_id: Uuid,
        revoked_by: Uuid,
        reason: String,
    },

    // ── Version lifecycle ─────────────────────────────────────────────────
    VersionPublished {
        artifact_id: Uuid,
        version_id: Uuid,
        version: String,
        bump_reason: String,
        published_by: Uuid,
    },

    // ── Social ────────────────────────────────────────────────────────────
    CommentAdded {
        artifact_id: Uuid,
        comment_id: Uuid,
        author_id: Uuid,
        kind: String,
    },
    LikeAdded {
        artifact_id: Uuid,
        user_id: Uuid,
    },
    LikeRemoved {
        artifact_id: Uuid,
        user_id: Uuid,
    },
    RatingAdded {
        artifact_id: Uuid,
        user_id: Uuid,
        score: i16,
    },
    ArtifactForked {
        source_artifact_id: Uuid,
        new_artifact_id: Uuid,
        forked_by: Uuid,
    },

    // ── Agent interactions ────────────────────────────────────────────────
    AgentLearned {
        agent_id: Uuid,
        artifact_id: Uuid,
        confidence_score: Option<f64>,
    },
    AgentBenchmarked {
        agent_id: Uuid,
        artifact_id: Uuid,
        version_id: Option<Uuid>,
    },

    // ── User ──────────────────────────────────────────────────────────────
    UserRegistered {
        user_id: Uuid,
        kind: String,
    },
    UserUpdated {
        user_id: Uuid,
    },

    // ── Comment lifecycle ─────────────────────────────────────────────────
    CommentUpdated {
        comment_id: Uuid,
        artifact_id: Uuid,
        updated_by: Uuid,
    },
    CommentDeleted {
        comment_id: Uuid,
        artifact_id: Uuid,
        deleted_by: Uuid,
    },
}

impl DomainEvent {
    pub fn aggregate_type(&self) -> &'static str {
        match self {
            Self::ArtifactCreated { .. }
            | Self::ArtifactUpdated { .. }
            | Self::ArtifactDeprecated { .. }
            | Self::ArtifactRevoked { .. }
            | Self::VersionPublished { .. }
            | Self::CommentAdded { .. }
            | Self::CommentUpdated { .. }
            | Self::CommentDeleted { .. }
            | Self::LikeAdded { .. }
            | Self::LikeRemoved { .. }
            | Self::RatingAdded { .. }
            | Self::ArtifactForked { .. }
            | Self::AgentLearned { .. }
            | Self::AgentBenchmarked { .. } => "artifact",
            Self::UserRegistered { .. } | Self::UserUpdated { .. } => "user",
        }
    }

    pub fn aggregate_id(&self) -> Uuid {
        match self {
            Self::ArtifactCreated { artifact_id, .. }
            | Self::ArtifactUpdated { artifact_id, .. }
            | Self::ArtifactDeprecated { artifact_id, .. }
            | Self::ArtifactRevoked { artifact_id, .. }
            | Self::VersionPublished { artifact_id, .. }
            | Self::CommentAdded { artifact_id, .. }
            | Self::CommentUpdated { artifact_id, .. }
            | Self::CommentDeleted { artifact_id, .. }
            | Self::LikeAdded { artifact_id, .. }
            | Self::LikeRemoved { artifact_id, .. }
            | Self::RatingAdded { artifact_id, .. }
            | Self::ArtifactForked {
                new_artifact_id: artifact_id,
                ..
            }
            | Self::AgentLearned { artifact_id, .. }
            | Self::AgentBenchmarked { artifact_id, .. } => *artifact_id,
            Self::UserRegistered { user_id, .. } | Self::UserUpdated { user_id, .. } => *user_id,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::ArtifactCreated { .. } => "artifact_created",
            Self::ArtifactUpdated { .. } => "artifact_updated",
            Self::ArtifactDeprecated { .. } => "artifact_deprecated",
            Self::ArtifactRevoked { .. } => "artifact_revoked",
            Self::VersionPublished { .. } => "version_published",
            Self::CommentAdded { .. } => "comment_added",
            Self::CommentUpdated { .. } => "comment_updated",
            Self::CommentDeleted { .. } => "comment_deleted",
            Self::LikeAdded { .. } => "like_added",
            Self::LikeRemoved { .. } => "like_removed",
            Self::RatingAdded { .. } => "rating_added",
            Self::ArtifactForked { .. } => "artifact_forked",
            Self::AgentLearned { .. } => "agent_learned",
            Self::AgentBenchmarked { .. } => "agent_benchmarked",
            Self::UserRegistered { .. } => "user_registered",
            Self::UserUpdated { .. } => "user_updated",
        }
    }
}

/// Wrapper that adds envelope metadata to a domain event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub sequence: i64,
    pub occurred_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_registered_aggregate_type_is_user() {
        let uid = Uuid::new_v4();
        let ev = DomainEvent::UserRegistered {
            user_id: uid,
            kind: "human".into(),
        };
        assert_eq!(ev.aggregate_type(), "user");
        assert_eq!(ev.aggregate_id(), uid);
        assert_eq!(ev.event_type(), "user_registered");
    }

    #[test]
    fn artifact_created_aggregate_type_is_artifact() {
        let aid = Uuid::new_v4();
        let ev = DomainEvent::ArtifactCreated {
            artifact_id: aid,
            kind: "skill".into(),
            namespace: "ns".into(),
            name: "art".into(),
            author_id: Uuid::new_v4(),
        };
        assert_eq!(ev.aggregate_type(), "artifact");
        assert_eq!(ev.aggregate_id(), aid);
        assert_eq!(ev.event_type(), "artifact_created");
    }

    #[test]
    fn version_published_aggregate_id_is_artifact_id() {
        let aid = Uuid::new_v4();
        let ev = DomainEvent::VersionPublished {
            artifact_id: aid,
            version_id: Uuid::new_v4(),
            version: "1.0.0".into(),
            bump_reason: "minor".into(),
            published_by: Uuid::new_v4(),
        };
        assert_eq!(ev.aggregate_id(), aid);
        assert_eq!(ev.event_type(), "version_published");
    }

    #[test]
    fn artifact_forked_aggregate_id_is_new_artifact() {
        let new_id = Uuid::new_v4();
        let ev = DomainEvent::ArtifactForked {
            source_artifact_id: Uuid::new_v4(),
            new_artifact_id: new_id,
            forked_by: Uuid::new_v4(),
        };
        assert_eq!(ev.aggregate_id(), new_id);
        assert_eq!(ev.event_type(), "artifact_forked");
    }

    #[test]
    fn domain_event_serializes_with_type_tag() {
        let ev = DomainEvent::UserUpdated {
            user_id: Uuid::new_v4(),
        };
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["type"], "user_updated");
    }

    #[test]
    fn all_social_events_are_artifact_aggregate() {
        let aid = Uuid::new_v4();
        let uid = Uuid::new_v4();
        let events = vec![
            DomainEvent::LikeAdded {
                artifact_id: aid,
                user_id: uid,
            },
            DomainEvent::LikeRemoved {
                artifact_id: aid,
                user_id: uid,
            },
            DomainEvent::RatingAdded {
                artifact_id: aid,
                user_id: uid,
                score: 5,
            },
        ];
        for ev in events {
            assert_eq!(ev.aggregate_type(), "artifact");
            assert_eq!(ev.aggregate_id(), aid);
        }
    }
}
