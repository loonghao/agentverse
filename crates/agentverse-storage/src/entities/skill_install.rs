//! SeaORM entity for the `skill_installs` table.

use sea_orm::entity::prelude::*;

/// Maps to the `skill_installs` table (see migration 005_skill_packages.sql).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "skill_installs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub skill_package_id: Uuid,
    /// "openclaw" | "codebuddy" | "workerbuddy" | "claude" | "augment" | custom
    pub agent_kind: String,
    pub install_path: String,
    pub installed_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::skill_package::Entity",
        from = "Column::SkillPackageId",
        to = "super::skill_package::Column::Id"
    )]
    SkillPackage,
}

impl Related<super::skill_package::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SkillPackage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

