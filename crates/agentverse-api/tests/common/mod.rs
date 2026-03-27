//! Shared test helpers: in-memory repository implementations and app builder.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agentverse_api::{
    routes::build_router,
    state::{AppConfig, AppState},
};
use agentverse_auth::JwtManager;
use agentverse_core::{
    artifact::{Artifact, ArtifactKind, ArtifactVersion},
    error::CoreError,
    repository::{
        ArtifactFilter, ArtifactRepository, ArtifactStats, SocialRepository, UserRepository,
        VersionRepository,
    },
    social::{AgentInteraction, Comment, Like, Rating},
    user::User,
};
use agentverse_events::NoopEventSink;
use agentverse_search::{FullTextSearch, SemanticSearch};
use agentverse_storage::DatabasePool;
use async_trait::async_trait;
use axum::Router;
use sea_orm::MockDatabase;
use uuid::Uuid;

// ── In-memory repositories ────────────────────────────────────────────────────

pub struct InMemoryUserRepo {
    pub users: Mutex<HashMap<Uuid, User>>,
}

impl InMemoryUserRepo {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_user(user: User) -> Self {
        let mut map = HashMap::new();
        map.insert(user.id, user);
        Self {
            users: Mutex::new(map),
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepo {
    async fn create(&self, user: User) -> Result<User, CoreError> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(user)
    }

    async fn update(&self, user: User) -> Result<User, CoreError> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(user)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, CoreError> {
        Ok(self.users.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, CoreError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.username == username)
            .cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, CoreError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.email.as_deref() == Some(email))
            .cloned())
    }
}

pub struct InMemoryArtifactRepo {
    pub artifacts: Mutex<HashMap<Uuid, Artifact>>,
}

impl InMemoryArtifactRepo {
    pub fn new() -> Self {
        Self {
            artifacts: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ArtifactRepository for InMemoryArtifactRepo {
    async fn create(&self, artifact: Artifact) -> Result<Artifact, CoreError> {
        self.artifacts
            .lock()
            .unwrap()
            .insert(artifact.id, artifact.clone());
        Ok(artifact)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Artifact>, CoreError> {
        Ok(self.artifacts.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_namespace_name(
        &self,
        kind: &ArtifactKind,
        namespace: &str,
        name: &str,
    ) -> Result<Option<Artifact>, CoreError> {
        Ok(self
            .artifacts
            .lock()
            .unwrap()
            .values()
            .find(|a| &a.kind == kind && a.namespace == namespace && a.name == name)
            .cloned())
    }

    async fn list(&self, _filter: ArtifactFilter) -> Result<Vec<Artifact>, CoreError> {
        Ok(self.artifacts.lock().unwrap().values().cloned().collect())
    }

    async fn update(&self, artifact: Artifact) -> Result<Artifact, CoreError> {
        self.artifacts
            .lock()
            .unwrap()
            .insert(artifact.id, artifact.clone());
        Ok(artifact)
    }

    async fn increment_downloads(&self, _id: Uuid) -> Result<(), CoreError> {
        Ok(())
    }
}

/// No-op version repo — returns None/empty for all queries.
pub struct NoopVersionRepo;

#[async_trait]
impl VersionRepository for NoopVersionRepo {
    async fn publish(&self, v: ArtifactVersion) -> Result<ArtifactVersion, CoreError> {
        Ok(v)
    }
    async fn find_latest(&self, _id: Uuid) -> Result<Option<ArtifactVersion>, CoreError> {
        Ok(None)
    }
    async fn find_by_semver(
        &self,
        _id: Uuid,
        _ver: &str,
    ) -> Result<Option<ArtifactVersion>, CoreError> {
        Ok(None)
    }
    async fn list_for_artifact(&self, _id: Uuid) -> Result<Vec<ArtifactVersion>, CoreError> {
        Ok(vec![])
    }
}

/// No-op social repo — returns empty results.
pub struct NoopSocialRepo;

#[async_trait]
impl SocialRepository for NoopSocialRepo {
    async fn add_comment(&self, c: Comment) -> Result<Comment, CoreError> {
        Ok(c)
    }
    async fn list_comments(&self, _id: Uuid) -> Result<Vec<Comment>, CoreError> {
        Ok(vec![])
    }
    async fn update_comment(
        &self,
        _comment_id: Uuid,
        _artifact_id: Uuid,
        _author_id: Uuid,
        _content: String,
    ) -> Result<Comment, CoreError> {
        Err(CoreError::NotFound("comment".into()))
    }
    async fn delete_comment(
        &self,
        _comment_id: Uuid,
        _artifact_id: Uuid,
        _author_id: Uuid,
    ) -> Result<(), CoreError> {
        Ok(())
    }
    async fn add_like(&self, l: Like) -> Result<Like, CoreError> {
        Ok(l)
    }
    async fn remove_like(&self, _artifact_id: Uuid, _user_id: Uuid) -> Result<(), CoreError> {
        Ok(())
    }
    async fn list_likes(&self, _id: Uuid) -> Result<Vec<Like>, CoreError> {
        Ok(vec![])
    }
    async fn add_rating(&self, r: Rating) -> Result<Rating, CoreError> {
        Ok(r)
    }
    async fn list_ratings(&self, _id: Uuid) -> Result<Vec<Rating>, CoreError> {
        Ok(vec![])
    }
    async fn record_interaction(&self, i: AgentInteraction) -> Result<AgentInteraction, CoreError> {
        Ok(i)
    }
    async fn list_interactions(&self, _id: Uuid) -> Result<Vec<AgentInteraction>, CoreError> {
        Ok(vec![])
    }
    async fn get_stats(&self, _id: Uuid) -> Result<ArtifactStats, CoreError> {
        Ok(ArtifactStats::default())
    }
}

// ── Test app builder ──────────────────────────────────────────────────────────

pub const TEST_JWT_SECRET: &str = "test-secret-32-chars-minimum!!!";

fn make_mock_state(users: Arc<dyn UserRepository>) -> AppState {
    let config = AppConfig {
        jwt_secret: TEST_JWT_SECRET.into(),
        anonymous_read: true,
        auto_infer_bump: true,
        access_token_expiry_secs: 3600,
    };
    AppState {
        artifacts: Arc::new(InMemoryArtifactRepo::new()),
        versions: Arc::new(NoopVersionRepo),
        social: Arc::new(NoopSocialRepo),
        users,
        events: Arc::new(NoopEventSink),
        fulltext: Arc::new(FullTextSearch::new(DatabasePool::from_connection(
            MockDatabase::new(sea_orm::DatabaseBackend::Postgres).into_connection(),
        ))),
        semantic: Arc::new(SemanticSearch::new(DatabasePool::from_connection(
            MockDatabase::new(sea_orm::DatabaseBackend::Postgres).into_connection(),
        ))),
        jwt: Arc::new(JwtManager::new(TEST_JWT_SECRET, 3600)),
        config: Arc::new(config),
    }
}

/// Build a fully wired Axum `Router` backed by in-memory stubs.
pub fn build_test_app() -> Router {
    let state = make_mock_state(Arc::new(InMemoryUserRepo::new()));
    build_router(state.clone()).with_state(state)
}

/// Build a test app that pre-seeds a single user into the user repo.
pub fn build_test_app_with_user(user: User) -> Router {
    let state = make_mock_state(Arc::new(InMemoryUserRepo::with_user(user)));
    build_router(state.clone()).with_state(state)
}
