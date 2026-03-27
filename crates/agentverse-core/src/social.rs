use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Comment types to distinguish human reviews from agent learning reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentKind {
    /// A human or agent code/quality review
    Review,
    /// An agent reporting what it learned from using this artifact
    Learning,
    /// A suggestion for improvement
    Suggestion,
    /// A bug report
    Bug,
    /// A benchmark result submitted by an agent
    Benchmark,
}

/// A comment (review, learning report, suggestion) on an artifact or version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub artifact_id: Uuid,
    pub version_id: Option<Uuid>,
    pub author_id: Uuid,
    /// Optional parent for threaded replies
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub kind: CommentKind,
    pub likes_count: i64,
    /// For benchmark comments: structured performance payload
    pub benchmark_payload: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A like/upvote on an artifact or version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Like {
    pub id: Uuid,
    pub artifact_id: Uuid,
    pub version_id: Option<Uuid>,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Rating (1-5 stars) with optional review text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    pub id: Uuid,
    pub artifact_id: Uuid,
    pub version_id: Option<Uuid>,
    pub user_id: Uuid,
    /// 1..=5
    pub score: i16,
    pub review_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Agent-to-artifact interaction (learn, fork, cite, benchmark).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    Learn,
    Fork,
    Cite,
    Benchmark,
}

/// Records when an agent interacts with an artifact for learning/forking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInteraction {
    pub id: Uuid,
    pub from_agent_id: Uuid,
    pub artifact_id: Uuid,
    pub version_id: Option<Uuid>,
    pub kind: InteractionKind,
    /// Arbitrary structured payload (e.g. benchmark metrics, learned insights)
    pub payload: serde_json::Value,
    /// How confident the agent is in its reported insight (0.0–1.0)
    pub confidence_score: Option<f64>,
    pub created_at: DateTime<Utc>,
}
