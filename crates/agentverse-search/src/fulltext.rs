//! Full-text search using PostgreSQL's built-in `tsvector` + GIN index.
//! No external search service required.

use agentverse_core::error::{CoreError, StorageError};
use agentverse_storage::DatabasePool;
use sea_orm::{FromQueryResult, Statement};
use uuid::Uuid;

use crate::result::SearchResult;

pub struct FullTextSearch {
    db: DatabasePool,
}

impl FullTextSearch {
    pub fn new(db: DatabasePool) -> Self {
        Self { db }
    }

    /// Keyword search across name + description using PostgreSQL full-text search.
    /// Optionally filter by `kind` (skill | soul | agent | workflow | prompt).
    pub async fn search(
        &self,
        query: &str,
        kind: Option<&str>,
        tag: Option<&str>,
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
            score: f64,
        }

        // Build a parameterised query:
        // Use plainto_tsquery which is safe for user input (no operator injection).
        let sql = r#"
            SELECT
                a.id            AS artifact_id,
                a.kind,
                a.namespace,
                a.name,
                a.description,
                a.downloads,
                ts_rank(
                    to_tsvector('english', a.name || ' ' || a.description),
                    plainto_tsquery('english', $1)
                )::float8        AS score
            FROM artifacts a
            WHERE
                a.status = 'active'
                AND to_tsvector('english', a.name || ' ' || a.description)
                    @@ plainto_tsquery('english', $1)
                AND ($2::text IS NULL OR a.kind = $2)
                AND ($3::text IS NULL OR EXISTS (
                    SELECT 1 FROM artifact_tags t
                    WHERE t.artifact_id = a.id AND t.tag = $3
                ))
            ORDER BY score DESC, a.downloads DESC
            LIMIT $4
        "#;

        let kind_param: Option<String> = kind.map(String::from);
        let tag_param: Option<String> = tag.map(String::from);

        sea_orm::ConnectionTrait::query_all(
            &self.db,
            Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [
                    query.into(),
                    kind_param.into(),
                    tag_param.into(),
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
                    score: r.score,
                    downloads: r.downloads,
                })
                .collect()
        })
    }
}
