//! Semantic similarity search using pgvector.
//!
//! Embeddings are stored in the `embedding` column (vector(384)).
//! Vectors should be generated externally (e.g. by a sidecar or via an AI model call)
//! and stored via `update_embedding()`.  The search here uses cosine distance.

use sea_orm::{DatabaseConnection, FromQueryResult, Statement};
use agentverse_core::error::{CoreError, StorageError};
use uuid::Uuid;

use crate::result::SearchResult;

pub struct SemanticSearch {
    db: DatabaseConnection,
}

impl SemanticSearch {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Store or update the embedding for an artifact.
    /// `embedding` must be a vector of length 384 (minilm) or 1536 (ada-002).
    pub async fn update_embedding(
        &self,
        artifact_id: Uuid,
        embedding: &[f32],
    ) -> Result<(), CoreError> {
        // pgvector accepts arrays as text: '[0.1,0.2,...]'
        let vec_str = format!(
            "[{}]",
            embedding
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        sea_orm::ConnectionTrait::execute(
            &self.db,
            Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "UPDATE artifacts SET embedding = $1::vector WHERE id = $2",
                [vec_str.into(), artifact_id.into()],
            ),
        )
        .await
        .map(|_| ())
        .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    /// Find the top-k artifacts whose embeddings are closest to the query vector.
    pub async fn search_by_vector(
        &self,
        query_vector: &[f32],
        kind: Option<&str>,
        limit: u64,
    ) -> Result<Vec<SearchResult>, CoreError> {
        #[derive(FromQueryResult)]
        struct Row {
            artifact_id: Uuid,
            kind: String,
            namespace: String,
            name: String,
            description: String,
            downloads: i64,
            distance: f64,
        }

        let vec_str = format!(
            "[{}]",
            query_vector
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let sql = r#"
            SELECT
                a.id                                     AS artifact_id,
                a.kind,
                a.namespace,
                a.name,
                a.description,
                a.downloads,
                (a.embedding <=> $1::vector)::float8     AS distance
            FROM artifacts a
            WHERE
                a.status = 'active'
                AND a.embedding IS NOT NULL
                AND ($2::text IS NULL OR a.kind = $2)
            ORDER BY a.embedding <=> $1::vector
            LIMIT $3
        "#;

        sea_orm::ConnectionTrait::query_all(
            &self.db,
            Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [
                    vec_str.into(),
                    kind.map(String::from).into(),
                    (limit as i64).into(),
                ],
            ),
        )
        .await
        .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
        .map(|rows| {
            rows.iter()
                .filter_map(|r| Row::from_query_result(r, "").ok())
                .map(|r| SearchResult {
                    artifact_id: r.artifact_id,
                    kind: r.kind,
                    namespace: r.namespace,
                    name: r.name,
                    description: r.description,
                    // Convert cosine distance to similarity score (0–1)
                    score: 1.0 - r.distance,
                    downloads: r.downloads,
                })
                .collect()
        })
    }
}

