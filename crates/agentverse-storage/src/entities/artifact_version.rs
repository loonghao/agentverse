use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "artifact_versions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub artifact_id: Uuid,
    pub version: String,
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
    pub pre_release: Option<String>,
    pub content: Json,
    pub checksum: String,
    pub signature: Option<String>,
    pub changelog: Option<String>,
    pub bump_reason: String,
    pub published_by: Uuid,
    pub published_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artifact::Entity",
        from = "Column::ArtifactId",
        to = "super::artifact::Column::Id"
    )]
    Artifact,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::PublishedBy",
        to = "super::user::Column::Id"
    )]
    Publisher,
}

impl Related<super::artifact::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artifact.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Publisher.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

