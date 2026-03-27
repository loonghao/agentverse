use std::sync::Arc;

use agentverse_auth::JwtManager;
use agentverse_core::repository::{ArtifactRepository, SocialRepository, UserRepository, VersionRepository};
use agentverse_events::EventStore;
use agentverse_search::{FullTextSearch, SemanticSearch};

/// Shared application state injected into every Axum handler.
#[derive(Clone)]
pub struct AppState {
    pub artifacts: Arc<dyn ArtifactRepository>,
    pub versions: Arc<dyn VersionRepository>,
    pub social: Arc<dyn SocialRepository>,
    pub users: Arc<dyn UserRepository>,
    pub events: Arc<EventStore>,
    pub fulltext: Arc<FullTextSearch>,
    pub semantic: Arc<SemanticSearch>,
    pub jwt: Arc<JwtManager>,
    pub config: Arc<AppConfig>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub jwt_secret: String,
    pub anonymous_read: bool,
    pub auto_infer_bump: bool,
    pub access_token_expiry_secs: i64,
}

