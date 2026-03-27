use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use uuid::Uuid;

use agentverse_core::{
    error::{CoreError, StorageError},
    repository::{ArtifactStats, SocialRepository},
    social::{AgentInteraction, Comment, CommentKind, InteractionKind, Like, Rating},
};

use crate::entities::{
    agent_interaction::{self, Entity as InteractionEntity},
    comment::{self, Entity as CommentEntity},
    like::{self, Entity as LikeEntity},
    rating::{self, Entity as RatingEntity},
};

pub struct SocialRepo {
    pub db: DatabaseConnection,
}

impl SocialRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn comment_kind_str(k: &CommentKind) -> &'static str {
    match k {
        CommentKind::Review => "review",
        CommentKind::Learning => "learning",
        CommentKind::Suggestion => "suggestion",
        CommentKind::Bug => "bug",
        CommentKind::Benchmark => "benchmark",
    }
}

fn str_to_comment_kind(s: &str) -> CommentKind {
    match s {
        "learning" => CommentKind::Learning,
        "suggestion" => CommentKind::Suggestion,
        "bug" => CommentKind::Bug,
        "benchmark" => CommentKind::Benchmark,
        _ => CommentKind::Review,
    }
}

fn comment_model_to_domain(m: comment::Model) -> Comment {
    Comment {
        id: m.id,
        artifact_id: m.artifact_id,
        version_id: m.version_id,
        author_id: m.author_id,
        parent_id: m.parent_id,
        content: m.content,
        kind: str_to_comment_kind(&m.kind),
        likes_count: m.likes_count,
        benchmark_payload: m.benchmark_payload,
        created_at: m.created_at.with_timezone(&Utc),
        updated_at: m.updated_at.with_timezone(&Utc),
    }
}

#[async_trait]
impl SocialRepository for SocialRepo {
    async fn add_comment(&self, c: Comment) -> Result<Comment, CoreError> {
        let now = chrono::Utc::now().fixed_offset();
        let model = comment::ActiveModel {
            id: Set(c.id),
            artifact_id: Set(c.artifact_id),
            version_id: Set(c.version_id),
            author_id: Set(c.author_id),
            parent_id: Set(c.parent_id),
            content: Set(c.content.clone()),
            kind: Set(comment_kind_str(&c.kind).to_string()),
            likes_count: Set(0),
            benchmark_payload: Set(c.benchmark_payload.clone()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model
            .insert(&self.db)
            .await
            .map(comment_model_to_domain)
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_comments(&self, artifact_id: Uuid) -> Result<Vec<Comment>, CoreError> {
        CommentEntity::find()
            .filter(comment::Column::ArtifactId.eq(artifact_id))
            .order_by_asc(comment::Column::CreatedAt)
            .all(&self.db)
            .await
            .map(|v| v.into_iter().map(comment_model_to_domain).collect())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn update_comment(
        &self,
        comment_id: Uuid,
        artifact_id: Uuid,
        author_id: Uuid,
        content: String,
    ) -> Result<Comment, CoreError> {
        let existing = CommentEntity::find_by_id(comment_id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
            .ok_or_else(|| CoreError::NotFound(format!("comment {comment_id}")))?;

        if existing.artifact_id != artifact_id || existing.author_id != author_id {
            return Err(CoreError::PermissionDenied {
                user_id: author_id,
                action: "update_comment".into(),
                artifact_id,
            });
        }

        let model = comment::ActiveModel {
            id: Set(comment_id),
            content: Set(content),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
            ..Default::default()
        };
        model
            .update(&self.db)
            .await
            .map(comment_model_to_domain)
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn delete_comment(
        &self,
        comment_id: Uuid,
        artifact_id: Uuid,
        author_id: Uuid,
    ) -> Result<(), CoreError> {
        let existing = CommentEntity::find_by_id(comment_id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
            .ok_or_else(|| CoreError::NotFound(format!("comment {comment_id}")))?;

        if existing.artifact_id != artifact_id || existing.author_id != author_id {
            return Err(CoreError::PermissionDenied {
                user_id: author_id,
                action: "delete_comment".into(),
                artifact_id,
            });
        }

        CommentEntity::delete_by_id(comment_id)
            .exec(&self.db)
            .await
            .map(|_| ())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_likes(&self, artifact_id: Uuid) -> Result<Vec<Like>, CoreError> {
        LikeEntity::find()
            .filter(like::Column::ArtifactId.eq(artifact_id))
            .order_by_desc(like::Column::CreatedAt)
            .all(&self.db)
            .await
            .map(|v| {
                v.into_iter()
                    .map(|m| Like {
                        id: m.id,
                        artifact_id: m.artifact_id,
                        version_id: m.version_id,
                        user_id: m.user_id,
                        created_at: m.created_at.with_timezone(&Utc),
                    })
                    .collect()
            })
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn add_like(&self, l: Like) -> Result<Like, CoreError> {
        use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

        // Idempotent INSERT: if the user already liked this artifact just return
        // the existing record instead of failing with a unique-constraint error.
        let sql = r#"
            INSERT INTO likes (id, artifact_id, version_id, user_id, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (artifact_id, user_id) DO NOTHING
            RETURNING id, artifact_id, version_id, user_id, created_at
        "#;

        #[derive(FromQueryResult)]
        struct Row {
            id: Uuid,
            artifact_id: Uuid,
            version_id: Option<Uuid>,
            user_id: Uuid,
            created_at: chrono::DateTime<chrono::FixedOffset>,
        }

        let rows = self.db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [l.id.into(), l.artifact_id.into(), l.version_id.into(), l.user_id.into()],
            ))
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?;

        // ON CONFLICT DO NOTHING returns 0 rows — fetch the existing like instead.
        if let Some(row) = rows.first().and_then(|r| Row::from_query_result(r, "").ok()) {
            return Ok(Like {
                id: row.id,
                artifact_id: row.artifact_id,
                version_id: row.version_id,
                user_id: row.user_id,
                created_at: row.created_at.with_timezone(&Utc),
            });
        }

        // Fetch the pre-existing like
        LikeEntity::find()
            .filter(like::Column::ArtifactId.eq(l.artifact_id))
            .filter(like::Column::UserId.eq(l.user_id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
            .map(|m| Like {
                id: m.id,
                artifact_id: m.artifact_id,
                version_id: m.version_id,
                user_id: m.user_id,
                created_at: m.created_at.with_timezone(&Utc),
            })
            .ok_or_else(|| CoreError::Storage(StorageError("like not found after upsert".into())))
    }

    async fn remove_like(&self, artifact_id: Uuid, user_id: Uuid) -> Result<(), CoreError> {
        LikeEntity::delete_many()
            .filter(like::Column::ArtifactId.eq(artifact_id))
            .filter(like::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await
            .map(|_| ())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn add_rating(&self, r: Rating) -> Result<Rating, CoreError> {
        use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

        // UPSERT: one rating per user per artifact — update score & review on conflict
        let sql = r#"
            INSERT INTO ratings (id, artifact_id, version_id, user_id, score, review_text, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (artifact_id, user_id)
            DO UPDATE SET score = EXCLUDED.score, review_text = EXCLUDED.review_text
            RETURNING id, artifact_id, version_id, user_id, score, review_text, created_at
        "#;

        #[derive(FromQueryResult)]
        struct Row {
            id: Uuid,
            artifact_id: Uuid,
            version_id: Option<Uuid>,
            user_id: Uuid,
            score: i16,
            review_text: Option<String>,
            created_at: chrono::DateTime<chrono::FixedOffset>,
        }

        let rows = self.db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [
                    r.id.into(),
                    r.artifact_id.into(),
                    r.version_id.into(),
                    r.user_id.into(),
                    (r.score as i32).into(),
                    r.review_text.clone().into(),
                ],
            ))
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?;

        let row = rows
            .first()
            .and_then(|r| Row::from_query_result(r, "").ok())
            .ok_or_else(|| CoreError::Storage(StorageError("rating upsert returned no row".into())))?;

        Ok(Rating {
            id: row.id,
            artifact_id: row.artifact_id,
            version_id: row.version_id,
            user_id: row.user_id,
            score: row.score,
            review_text: row.review_text,
            created_at: row.created_at.with_timezone(&Utc),
        })
    }

    async fn list_ratings(&self, artifact_id: Uuid) -> Result<Vec<Rating>, CoreError> {
        RatingEntity::find()
            .filter(rating::Column::ArtifactId.eq(artifact_id))
            .order_by_desc(rating::Column::CreatedAt)
            .all(&self.db)
            .await
            .map(|v| {
                v.into_iter()
                    .map(|m| Rating {
                        id: m.id,
                        artifact_id: m.artifact_id,
                        version_id: m.version_id,
                        user_id: m.user_id,
                        score: m.score,
                        review_text: m.review_text,
                        created_at: m.created_at.with_timezone(&Utc),
                    })
                    .collect()
            })
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn record_interaction(
        &self,
        i: AgentInteraction,
    ) -> Result<AgentInteraction, CoreError> {
        let kind_str = match &i.kind {
            InteractionKind::Learn => "learn",
            InteractionKind::Fork => "fork",
            InteractionKind::Cite => "cite",
            InteractionKind::Benchmark => "benchmark",
        };
        let model = agent_interaction::ActiveModel {
            id: Set(i.id),
            from_agent_id: Set(i.from_agent_id),
            artifact_id: Set(i.artifact_id),
            version_id: Set(i.version_id),
            kind: Set(kind_str.to_string()),
            payload: Set(i.payload.clone()),
            confidence_score: Set(i.confidence_score),
            created_at: Set(chrono::Utc::now().fixed_offset()),
        };
        model
            .insert(&self.db)
            .await
            .map(|m| AgentInteraction {
                id: m.id,
                from_agent_id: m.from_agent_id,
                artifact_id: m.artifact_id,
                version_id: m.version_id,
                kind: i.kind,
                payload: m.payload,
                confidence_score: m.confidence_score,
                created_at: m.created_at.with_timezone(&Utc),
            })
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_interactions(
        &self,
        artifact_id: Uuid,
    ) -> Result<Vec<AgentInteraction>, CoreError> {
        InteractionEntity::find()
            .filter(agent_interaction::Column::ArtifactId.eq(artifact_id))
            .order_by_desc(agent_interaction::Column::CreatedAt)
            .all(&self.db)
            .await
            .map(|v| {
                v.into_iter()
                    .map(|m| AgentInteraction {
                        id: m.id,
                        from_agent_id: m.from_agent_id,
                        artifact_id: m.artifact_id,
                        version_id: m.version_id,
                        kind: match m.kind.as_str() {
                            "fork" => InteractionKind::Fork,
                            "cite" => InteractionKind::Cite,
                            "benchmark" => InteractionKind::Benchmark,
                            _ => InteractionKind::Learn,
                        },
                        payload: m.payload,
                        confidence_score: m.confidence_score,
                        created_at: m.created_at.with_timezone(&Utc),
                    })
                    .collect()
            })
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn get_stats(&self, artifact_id: Uuid) -> Result<ArtifactStats, CoreError> {
        use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

        #[derive(FromQueryResult)]
        struct Row {
            likes_count: i64,
            comments_count: i64,
            ratings_count: i64,
            avg_rating: Option<f64>,
            interactions_count: i64,
        }

        let sql = r#"
            SELECT
                (SELECT COUNT(*) FROM likes             WHERE artifact_id = $1)::bigint AS likes_count,
                (SELECT COUNT(*) FROM comments         WHERE artifact_id = $1)::bigint AS comments_count,
                (SELECT COUNT(*) FROM ratings          WHERE artifact_id = $1)::bigint AS ratings_count,
                (SELECT AVG(score::float8) FROM ratings WHERE artifact_id = $1)         AS avg_rating,
                (SELECT COUNT(*) FROM agent_interactions WHERE artifact_id = $1)::bigint AS interactions_count
        "#;

        let rows = self.db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [artifact_id.into()],
            ))
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?;

        rows.first()
            .and_then(|r| Row::from_query_result(r, "").ok())
            .map(|r| ArtifactStats {
                likes_count: r.likes_count,
                comments_count: r.comments_count,
                ratings_count: r.ratings_count,
                avg_rating: r.avg_rating,
                interactions_count: r.interactions_count,
            })
            .ok_or_else(|| CoreError::Storage(StorageError("stats query returned no row".into())))
    }
}

