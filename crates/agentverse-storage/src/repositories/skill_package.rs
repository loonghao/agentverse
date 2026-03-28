//! SeaORM implementation of `SkillPackageRepository` and `SkillInstallRepository`.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ModelTrait, QueryFilter,
};
use std::str::FromStr;
use uuid::Uuid;

use agentverse_core::{
    error::{CoreError, StorageError},
    repository::{SkillInstallRepository, SkillPackageRepository},
    skill::{AgentKind, SkillInstall, SkillPackage, SourceType},
};

use crate::connection::DatabasePool;
use crate::entities::skill_install::{self, Entity as InstallEntity};
use crate::entities::skill_package::{self, Entity as PackageEntity};

// ── SkillPackageRepo ──────────────────────────────────────────────────────────

pub struct SkillPackageRepo {
    pub db: DatabasePool,
}

impl SkillPackageRepo {
    pub fn new(db: DatabasePool) -> Self {
        Self { db }
    }
}

fn pkg_model_to_domain(m: skill_package::Model) -> SkillPackage {
    SkillPackage {
        id: m.id,
        artifact_version_id: m.artifact_version_id,
        source_type: SourceType::from_str(&m.source_type).unwrap_or(SourceType::Url),
        download_url: m.download_url,
        checksum: m.checksum,
        file_size: m.file_size,
        metadata: m.metadata,
        created_at: m.created_at.with_timezone(&Utc),
    }
}

#[async_trait]
impl SkillPackageRepository for SkillPackageRepo {
    async fn register(&self, pkg: SkillPackage) -> Result<SkillPackage, CoreError> {
        let model = skill_package::ActiveModel {
            id: Set(pkg.id),
            artifact_version_id: Set(pkg.artifact_version_id),
            source_type: Set(pkg.source_type.to_string()),
            download_url: Set(pkg.download_url.clone()),
            checksum: Set(pkg.checksum.clone()),
            file_size: Set(pkg.file_size),
            metadata: Set(pkg.metadata.clone()),
            created_at: Set(chrono::Utc::now().fixed_offset()),
        };
        model
            .insert(&self.db)
            .await
            .map(pkg_model_to_domain)
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_by_version_and_source(
        &self,
        version_id: Uuid,
        source_type: &SourceType,
    ) -> Result<Option<SkillPackage>, CoreError> {
        PackageEntity::find()
            .filter(skill_package::Column::ArtifactVersionId.eq(version_id))
            .filter(skill_package::Column::SourceType.eq(source_type.to_string()))
            .one(&self.db)
            .await
            .map(|o| o.map(pkg_model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_for_version(&self, version_id: Uuid) -> Result<Vec<SkillPackage>, CoreError> {
        PackageEntity::find()
            .filter(skill_package::Column::ArtifactVersionId.eq(version_id))
            .all(&self.db)
            .await
            .map(|v| v.into_iter().map(pkg_model_to_domain).collect())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<SkillPackage>, CoreError> {
        PackageEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map(|o| o.map(pkg_model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_for_artifact(&self, _artifact_id: Uuid) -> Result<Vec<SkillPackage>, CoreError> {
        // Requires a JOIN through artifact_versions; for now return empty in storage layer
        // (the API layer should call list_for_version per version instead).
        Ok(vec![])
    }

    async fn delete(&self, id: Uuid) -> Result<(), CoreError> {
        let model = PackageEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?
            .ok_or_else(|| CoreError::Storage(StorageError(format!("package {id} not found"))))?;
        model
            .delete(&self.db)
            .await
            .map(|_| ())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }
}

// ── SkillInstallRepo ──────────────────────────────────────────────────────────

pub struct SkillInstallRepo {
    pub db: DatabasePool,
}

impl SkillInstallRepo {
    pub fn new(db: DatabasePool) -> Self {
        Self { db }
    }
}

fn install_model_to_domain(m: skill_install::Model) -> SkillInstall {
    SkillInstall {
        id: m.id,
        skill_package_id: m.skill_package_id,
        agent_kind: AgentKind::from_str(&m.agent_kind).unwrap_or(AgentKind::Custom(m.agent_kind)),
        install_path: m.install_path,
        installed_at: m.installed_at.with_timezone(&Utc),
    }
}

#[async_trait]
impl SkillInstallRepository for SkillInstallRepo {
    async fn record(&self, install: SkillInstall) -> Result<SkillInstall, CoreError> {
        let model = skill_install::ActiveModel {
            id: Set(install.id),
            skill_package_id: Set(install.skill_package_id),
            agent_kind: Set(install.agent_kind.to_string()),
            install_path: Set(install.install_path.clone()),
            installed_at: Set(chrono::Utc::now().fixed_offset()),
        };
        model
            .insert(&self.db)
            .await
            .map(install_model_to_domain)
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_by_package_and_agent(
        &self,
        package_id: Uuid,
        agent: &AgentKind,
    ) -> Result<Option<SkillInstall>, CoreError> {
        InstallEntity::find()
            .filter(skill_install::Column::SkillPackageId.eq(package_id))
            .filter(skill_install::Column::AgentKind.eq(agent.to_string()))
            .one(&self.db)
            .await
            .map(|o| o.map(install_model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_for_package(&self, package_id: Uuid) -> Result<Vec<SkillInstall>, CoreError> {
        InstallEntity::find()
            .filter(skill_install::Column::SkillPackageId.eq(package_id))
            .all(&self.db)
            .await
            .map(|v| v.into_iter().map(install_model_to_domain).collect())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn list_for_agent(&self, agent: &AgentKind) -> Result<Vec<SkillInstall>, CoreError> {
        InstallEntity::find()
            .filter(skill_install::Column::AgentKind.eq(agent.to_string()))
            .all(&self.db)
            .await
            .map(|v| v.into_iter().map(install_model_to_domain).collect())
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }
}

