use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "agent_interactions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub from_agent_id: Uuid,
    pub artifact_id: Uuid,
    pub version_id: Option<Uuid>,
    pub kind: String,
    pub payload: Json,
    pub confidence_score: Option<f64>,
    pub created_at: DateTimeWithTimeZone,
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
        from = "Column::FromAgentId",
        to = "super::user::Column::Id"
    )]
    Agent,
}

impl Related<super::artifact::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artifact.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Agent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
