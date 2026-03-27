use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

use agentverse_core::{
    artifact::ArtifactVersion,
    error::{CoreError, StorageError},
    repository::VersionRepository,
};

use crate::connection::DatabasePool;
use crate::entities::artifact_version::{self, Entity as VersionEntity};

pub struct VersionRepo {
    pub db: DatabasePool,
}

impl VersionRepo {
    pub fn new(db: DatabasePool) -> Self {
        Self { db }
    }
}

fn model_to_domain(m: artifact_version::Model) -> ArtifactVersion {
    ArtifactVersion {
        id: m.id,
        artifact_id: m.artifact_id,
        version: m.version,
        major: m.major as u64,
        minor: m.minor as u64,
        patch: m.patch as u64,
        pre_release: m.pre_release,
        content: m.content,
        checksum: m.checksum,
        signature: m.signature,
        changelog: m.changelog,
        bump_reason: m.bump_reason,
        published_by: m.published_by,
        published_at: m.published_at.with_timezone(&Utc),
    }
}

#[async_trait]
impl VersionRepository for VersionRepo {
    async fn publish(&self, v: ArtifactVersion) -> Result<ArtifactVersion, CoreError> {
        let model = artifact_version::ActiveModel {
            id: Set(v.id),
            artifact_id: Set(v.artifact_id),
            version: Set(v.version.clone()),
            major: Set(v.major as i32),
            minor: Set(v.minor as i32),
            patch: Set(v.patch as i32),
            pre_release: Set(v.pre_release.clone()),
            content: Set(v.content.clone()),
            checksum: Set(v.checksum.clone()),
            signature: Set(v.signature.clone()),
            changelog: Set(v.changelog.clone()),
            bump_reason: Set(v.bump_reason.clone()),
            published_by: Set(v.published_by),
            published_at: Set(chrono::Utc::now().fixed_offset()),
        };
        model
            .insert(&self.db)
            .await
            .map(model_to_domain)
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_latest(&self, artifact_id: Uuid) -> Result<Option<ArtifactVersion>, CoreError> {
        VersionEntity::find()
            .filter(artifact_version::Column::ArtifactId.eq(artifact_id))
            .order_by_desc(artifact_version::Column::Major)
            .order_by_desc(artifact_version::Column::Minor)
            .order_by_desc(artifact_version::Column::Patch)
            .one(&self.db)
            .await
            .map(|opt| opt.map(model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_by_semver(
        &self,
        artifact_id: Uuid,
        version: &str,
    ) -> Result<Option<ArtifactVersion>, CoreError> {
        VersionEntity::find()
            .filter(artifact_version::Column::ArtifactId.eq(artifact_id))
            .filter(artifact_version::Column::Version.eq(version))
            .one(&self.db)
            .await
            .map(|opt| opt.map(model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_for_artifact(
        &self,
        artifact_id: Uuid,
    ) -> Result<Vec<ArtifactVersion>, CoreError> {
        VersionEntity::find()
            .filter(artifact_version::Column::ArtifactId.eq(artifact_id))
            .order_by_desc(artifact_version::Column::PublishedAt)
            .all(&self.db)
            .await
            .map(|v| v.into_iter().map(model_to_domain).collect())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }
}
