use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Statement,
};
use uuid::Uuid;

use agentverse_core::{
    artifact::{Artifact, ArtifactKind, ArtifactStatus, Manifest},
    error::{CoreError, StorageError},
    repository::{ArtifactFilter, ArtifactRepository},
};

use crate::entities::artifact::{self, Entity as ArtifactEntity};

pub struct ArtifactRepo {
    pub db: DatabaseConnection,
}

impl ArtifactRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn kind_str(k: &ArtifactKind) -> &'static str {
    match k {
        ArtifactKind::Skill => "skill",
        ArtifactKind::Soul => "soul",
        ArtifactKind::Agent => "agent",
        ArtifactKind::Workflow => "workflow",
        ArtifactKind::Prompt => "prompt",
    }
}

fn model_to_domain(m: artifact::Model) -> Result<Artifact, CoreError> {
    let manifest: Manifest = serde_json::from_value(m.manifest)
        .map_err(|e| CoreError::InvalidManifest(e.to_string()))?;
    Ok(Artifact {
        id: m.id,
        kind: match m.kind.as_str() {
            "skill" => ArtifactKind::Skill,
            "soul" => ArtifactKind::Soul,
            "agent" => ArtifactKind::Agent,
            "workflow" => ArtifactKind::Workflow,
            _ => ArtifactKind::Prompt,
        },
        namespace: m.namespace,
        name: m.name,
        display_name: m.display_name,
        manifest,
        status: match m.status.as_str() {
            "deprecated" => ArtifactStatus::Deprecated,
            "retired" => ArtifactStatus::Retired,
            "revoked" => ArtifactStatus::Revoked,
            _ => ArtifactStatus::Active,
        },
        author_id: m.author_id,
        downloads: m.downloads,
        created_at: m.created_at.with_timezone(&Utc),
        updated_at: m.updated_at.with_timezone(&Utc),
    })
}

#[async_trait]
impl ArtifactRepository for ArtifactRepo {
    async fn create(&self, a: Artifact) -> Result<Artifact, CoreError> {
        let now = chrono::Utc::now().fixed_offset();
        let manifest_json = serde_json::to_value(&a.manifest)
            .map_err(|e| CoreError::InvalidManifest(e.to_string()))?;
        let model = artifact::ActiveModel {
            id: Set(a.id),
            kind: Set(kind_str(&a.kind).to_string()),
            namespace: Set(a.namespace.clone()),
            name: Set(a.name.clone()),
            display_name: Set(a.display_name.clone()),
            description: Set(a.manifest.description.clone()),
            manifest: Set(manifest_json),
            status: Set("active".into()),
            author_id: Set(a.author_id),
            downloads: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model
            .insert(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
            .and_then(model_to_domain)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Artifact>, CoreError> {
        ArtifactEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
            .map(model_to_domain)
            .transpose()
    }

    async fn find_by_namespace_name(
        &self,
        kind: &ArtifactKind,
        namespace: &str,
        name: &str,
    ) -> Result<Option<Artifact>, CoreError> {
        ArtifactEntity::find()
            .filter(artifact::Column::Kind.eq(kind_str(kind)))
            .filter(artifact::Column::Namespace.eq(namespace))
            .filter(artifact::Column::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
            .map(model_to_domain)
            .transpose()
    }

    async fn list(&self, filter: ArtifactFilter) -> Result<Vec<Artifact>, CoreError> {
        // When a tag filter is requested we need a sub-select, so drop down to raw SQL.
        if filter.tag.is_some() {
            return self.list_with_tag(filter).await;
        }

        let mut q = ArtifactEntity::find();
        if let Some(k) = &filter.kind {
            q = q.filter(artifact::Column::Kind.eq(kind_str(k)));
        }
        if let Some(ns) = &filter.namespace {
            q = q.filter(artifact::Column::Namespace.eq(ns.as_str()));
        }
        if let Some(aid) = filter.author_id {
            q = q.filter(artifact::Column::AuthorId.eq(aid));
        }
        if let Some(status) = &filter.status {
            let s = match status {
                agentverse_core::artifact::ArtifactStatus::Active => "active",
                agentverse_core::artifact::ArtifactStatus::Deprecated => "deprecated",
                agentverse_core::artifact::ArtifactStatus::Retired => "retired",
                agentverse_core::artifact::ArtifactStatus::Revoked => "revoked",
            };
            q = q.filter(artifact::Column::Status.eq(s));
        }

        let limit = filter.limit.unwrap_or(20);
        let offset = filter.offset.unwrap_or(0);
        q.order_by_desc(artifact::Column::Downloads)
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
            .into_iter()
            .map(model_to_domain)
            .collect()
    }

    async fn update(&self, a: Artifact) -> Result<Artifact, CoreError> {
        let manifest_json = serde_json::to_value(&a.manifest)
            .map_err(|e| CoreError::InvalidManifest(e.to_string()))?;
        let model = artifact::ActiveModel {
            id: Set(a.id),
            display_name: Set(a.display_name.clone()),
            description: Set(a.manifest.description.clone()),
            manifest: Set(manifest_json),
            status: Set(match a.status {
                ArtifactStatus::Deprecated => "deprecated",
                ArtifactStatus::Retired => "retired",
                ArtifactStatus::Revoked => "revoked",
                ArtifactStatus::Active => "active",
            }.into()),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
            ..Default::default()
        };
        model
            .update(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
            .and_then(model_to_domain)
    }

    async fn increment_downloads(&self, id: Uuid) -> Result<(), CoreError> {
        use sea_orm::{ConnectionTrait, Statement};
        self.db
            .execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "UPDATE artifacts SET downloads = downloads + 1 WHERE id = $1",
                [id.into()],
            ))
            .await
            .map(|_| ())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }
}

impl ArtifactRepo {
    /// List artifacts matching a tag via a parameterised raw SQL query.
    async fn list_with_tag(&self, filter: ArtifactFilter) -> Result<Vec<Artifact>, CoreError> {
        use sea_orm::FromQueryResult;

        #[derive(FromQueryResult)]
        struct Row {
            id: Uuid,
            kind: String,
            namespace: String,
            name: String,
            display_name: Option<String>,
            #[allow(dead_code)]
            description: String,
            manifest: serde_json::Value,
            status: String,
            author_id: Uuid,
            downloads: i64,
            created_at: chrono::DateTime<chrono::FixedOffset>,
            updated_at: chrono::DateTime<chrono::FixedOffset>,
        }

        let tag = filter.tag.as_deref().unwrap_or("");
        let kind_val: Option<String> = filter.kind.as_ref().map(|k| kind_str(k).to_string());
        let ns_val: Option<String> = filter.namespace.clone();
        let author_val: Option<Uuid> = filter.author_id;
        let status_val: Option<String> = filter.status.as_ref().map(|s| match s {
            agentverse_core::artifact::ArtifactStatus::Active => "active".to_string(),
            agentverse_core::artifact::ArtifactStatus::Deprecated => "deprecated".to_string(),
            agentverse_core::artifact::ArtifactStatus::Retired => "retired".to_string(),
            agentverse_core::artifact::ArtifactStatus::Revoked => "revoked".to_string(),
        });
        let limit = filter.limit.unwrap_or(20) as i64;
        let offset = filter.offset.unwrap_or(0) as i64;

        let sql = r#"
            SELECT a.id, a.kind, a.namespace, a.name, a.display_name,
                   a.description, a.manifest, a.status, a.author_id,
                   a.downloads, a.created_at, a.updated_at
            FROM artifacts a
            WHERE EXISTS (
                SELECT 1 FROM artifact_tags t
                WHERE t.artifact_id = a.id AND t.tag = $1
            )
            AND ($2::text IS NULL OR a.kind = $2)
            AND ($3::text IS NULL OR a.namespace = $3)
            AND ($4::uuid IS NULL OR a.author_id = $4)
            AND ($5::text IS NULL OR a.status = $5)
            ORDER BY a.downloads DESC
            LIMIT $6 OFFSET $7
        "#;

        sea_orm::ConnectionTrait::query_all(
            &self.db,
            Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [
                    tag.into(),
                    kind_val.into(),
                    ns_val.into(),
                    author_val.into(),
                    status_val.into(),
                    limit.into(),
                    offset.into(),
                ],
            ),
        )
        .await
        .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
        .iter()
        .filter_map(|r| Row::from_query_result(r, "").ok())
        .map(|r| {
            let manifest: agentverse_core::artifact::Manifest =
                serde_json::from_value(r.manifest)
                    .map_err(|e| CoreError::InvalidManifest(e.to_string()))?;
            Ok(Artifact {
                id: r.id,
                kind: match r.kind.as_str() {
                    "skill" => agentverse_core::artifact::ArtifactKind::Skill,
                    "soul" => agentverse_core::artifact::ArtifactKind::Soul,
                    "agent" => agentverse_core::artifact::ArtifactKind::Agent,
                    "workflow" => agentverse_core::artifact::ArtifactKind::Workflow,
                    _ => agentverse_core::artifact::ArtifactKind::Prompt,
                },
                namespace: r.namespace,
                name: r.name,
                display_name: r.display_name,
                manifest,
                status: match r.status.as_str() {
                    "deprecated" => ArtifactStatus::Deprecated,
                    "retired" => ArtifactStatus::Retired,
                    "revoked" => ArtifactStatus::Revoked,
                    _ => ArtifactStatus::Active,
                },
                author_id: r.author_id,
                downloads: r.downloads,
                created_at: r.created_at.with_timezone(&Utc),
                updated_at: r.updated_at.with_timezone(&Utc),
            })
        })
        .collect()
    }
}

