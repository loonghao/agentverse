use std::sync::Arc;

use agentverse_auth::JwtManager;
use agentverse_core::repository::{
    ArtifactRepository, SkillInstallRepository, SkillPackageRepository, SocialRepository,
    UserRepository, VersionRepository,
};
use agentverse_events::EventSink;
use agentverse_search::{FullTextSearch, SemanticSearch};
use agentverse_storage::ObjectStore;

/// Shared application state injected into every Axum handler.
#[derive(Clone)]
pub struct AppState {
    pub artifacts: Arc<dyn ArtifactRepository>,
    pub versions: Arc<dyn VersionRepository>,
    pub social: Arc<dyn SocialRepository>,
    pub users: Arc<dyn UserRepository>,
    pub events: Arc<dyn EventSink>,
    pub fulltext: Arc<FullTextSearch>,
    pub semantic: Arc<SemanticSearch>,
    pub jwt: Arc<JwtManager>,
    pub config: Arc<AppConfig>,
    /// Skill package metadata (source, download URL, checksum).
    pub skill_packages: Arc<dyn SkillPackageRepository>,
    /// Records of where skills are installed per agent runtime.
    pub skill_installs: Arc<dyn SkillInstallRepository>,
    /// Pluggable object store for internally-hosted skill package archives.
    ///
    /// `None` when no `[object_store]` section is present in the server
    /// configuration.  Upload endpoints return 501 in that case.
    pub object_store: Option<Arc<dyn ObjectStore>>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub jwt_secret: String,
    pub anonymous_read: bool,
    pub auto_infer_bump: bool,
    pub access_token_expiry_secs: i64,
}
