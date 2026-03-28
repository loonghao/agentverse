use async_trait::async_trait;
use uuid::Uuid;

// Re-export so callers can import the macro from here
pub use async_trait::async_trait as repo_async_trait;

use crate::{
    artifact::{Artifact, ArtifactKind, ArtifactVersion},
    error::CoreError,
    skill::{AgentKind, SkillInstall, SkillPackage},
    social::{AgentInteraction, Comment, Like, Rating},
    user::User,
};

/// Query filters for listing artifacts.
#[derive(Debug, Clone, Default)]
pub struct ArtifactFilter {
    pub kind: Option<ArtifactKind>,
    pub namespace: Option<String>,
    pub tag: Option<String>,
    pub status: Option<crate::artifact::ArtifactStatus>,
    pub author_id: Option<Uuid>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Port (interface) for artifact persistence — implemented by the storage crate.
#[async_trait]
pub trait ArtifactRepository: Send + Sync {
    async fn create(&self, artifact: Artifact) -> Result<Artifact, CoreError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Artifact>, CoreError>;
    async fn find_by_namespace_name(
        &self,
        kind: &ArtifactKind,
        namespace: &str,
        name: &str,
    ) -> Result<Option<Artifact>, CoreError>;
    async fn list(&self, filter: ArtifactFilter) -> Result<Vec<Artifact>, CoreError>;
    async fn update(&self, artifact: Artifact) -> Result<Artifact, CoreError>;
    async fn increment_downloads(&self, id: Uuid) -> Result<(), CoreError>;
}

/// Port for artifact version persistence.
#[async_trait]
pub trait VersionRepository: Send + Sync {
    async fn publish(&self, version: ArtifactVersion) -> Result<ArtifactVersion, CoreError>;
    async fn find_latest(&self, artifact_id: Uuid) -> Result<Option<ArtifactVersion>, CoreError>;
    async fn find_by_semver(
        &self,
        artifact_id: Uuid,
        version: &str,
    ) -> Result<Option<ArtifactVersion>, CoreError>;
    async fn list_for_artifact(&self, artifact_id: Uuid)
        -> Result<Vec<ArtifactVersion>, CoreError>;
}

/// Aggregate statistics for a single artifact.
#[derive(Debug, Default, serde::Serialize)]
pub struct ArtifactStats {
    pub likes_count: i64,
    pub comments_count: i64,
    pub ratings_count: i64,
    pub avg_rating: Option<f64>,
    pub interactions_count: i64,
}

/// Port for social features.
#[async_trait]
pub trait SocialRepository: Send + Sync {
    async fn add_comment(&self, comment: Comment) -> Result<Comment, CoreError>;
    async fn list_comments(&self, artifact_id: Uuid) -> Result<Vec<Comment>, CoreError>;
    async fn update_comment(
        &self,
        comment_id: Uuid,
        artifact_id: Uuid,
        author_id: Uuid,
        content: String,
    ) -> Result<Comment, CoreError>;
    async fn delete_comment(
        &self,
        comment_id: Uuid,
        artifact_id: Uuid,
        author_id: Uuid,
    ) -> Result<(), CoreError>;
    async fn add_like(&self, like: Like) -> Result<Like, CoreError>;
    async fn remove_like(&self, artifact_id: Uuid, user_id: Uuid) -> Result<(), CoreError>;
    async fn list_likes(&self, artifact_id: Uuid) -> Result<Vec<Like>, CoreError>;
    async fn add_rating(&self, rating: Rating) -> Result<Rating, CoreError>;
    async fn list_ratings(&self, artifact_id: Uuid) -> Result<Vec<Rating>, CoreError>;
    async fn record_interaction(
        &self,
        interaction: AgentInteraction,
    ) -> Result<AgentInteraction, CoreError>;
    async fn list_interactions(
        &self,
        artifact_id: Uuid,
    ) -> Result<Vec<AgentInteraction>, CoreError>;

    /// Aggregate social statistics for an artifact.
    async fn get_stats(&self, artifact_id: Uuid) -> Result<ArtifactStats, CoreError>;
}

/// Port for user persistence.
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: User) -> Result<User, CoreError>;
    async fn update(&self, user: User) -> Result<User, CoreError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, CoreError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, CoreError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, CoreError>;
}

/// Port for skill package persistence.
///
/// A `SkillPackage` records where a specific artifact version can be downloaded
/// from (Clawhub, GitHub, or custom URL) and stores the canonical download URL
/// that was captured at publish time via the publishing hook.
#[async_trait]
pub trait SkillPackageRepository: Send + Sync {
    /// Record a new skill package (called by the publishing hook).
    async fn register(&self, pkg: SkillPackage) -> Result<SkillPackage, CoreError>;
    /// Fetch a single package by its UUID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SkillPackage>, CoreError>;
    /// Find a package for a specific artifact version and source type.
    async fn find_by_version_and_source(
        &self,
        version_id: Uuid,
        source_type: &crate::skill::SourceType,
    ) -> Result<Option<SkillPackage>, CoreError>;
    /// List all packages for a given artifact version.
    async fn list_for_version(&self, version_id: Uuid) -> Result<Vec<SkillPackage>, CoreError>;
    /// List all packages for any version of an artifact.
    async fn list_for_artifact(&self, artifact_id: Uuid) -> Result<Vec<SkillPackage>, CoreError>;
    /// Remove a package record (does not affect installs already deployed).
    async fn delete(&self, id: Uuid) -> Result<(), CoreError>;
}

/// Port for tracking skill installations on agent runtimes.
#[async_trait]
pub trait SkillInstallRepository: Send + Sync {
    /// Record that a skill was deployed to an agent runtime.
    async fn record(&self, install: SkillInstall) -> Result<SkillInstall, CoreError>;
    /// Find existing install for a package + agent combination.
    async fn find_by_package_and_agent(
        &self,
        package_id: Uuid,
        agent: &AgentKind,
    ) -> Result<Option<SkillInstall>, CoreError>;
    /// List all installs for a skill package.
    async fn list_for_package(&self, package_id: Uuid) -> Result<Vec<SkillInstall>, CoreError>;
    /// List all installs for a specific agent kind across all skills.
    async fn list_for_agent(&self, agent: &AgentKind) -> Result<Vec<SkillInstall>, CoreError>;
}
