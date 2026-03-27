use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single search hit returned by any search backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub artifact_id: Uuid,
    pub kind: String,
    pub namespace: String,
    pub name: String,
    pub description: String,
    /// Relevance score (higher = more relevant; range depends on search type)
    pub score: f64,
    pub downloads: i64,
}

