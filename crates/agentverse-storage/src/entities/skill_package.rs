//! SeaORM entity for the `skill_packages` table.

use sea_orm::entity::prelude::*;

/// Maps to the `skill_packages` table (see migration 005_skill_packages.sql).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "skill_packages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub artifact_version_id: Uuid,
    /// "clawhub" | "github" | "url"
    pub source_type: String,
    pub download_url: String,
    pub checksum: Option<String>,
    pub file_size: Option<i64>,
    pub metadata: Json,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artifact_version::Entity",
        from = "Column::ArtifactVersionId",
        to = "super::artifact_version::Column::Id"
    )]
    ArtifactVersion,
}

impl Related<super::artifact_version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtifactVersion.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

