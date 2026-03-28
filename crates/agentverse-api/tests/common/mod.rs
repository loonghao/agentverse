//! Shared test helpers: in-memory repository implementations and app builder.
#![allow(dead_code)] // pub helpers are used across multiple test files

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
        ArtifactFilter, ArtifactRepository, ArtifactStats, SkillInstallRepository,
        SkillPackageRepository, SocialRepository, UserRepository, VersionRepository,
    },
    skill::{AgentKind, SkillInstall, SkillPackage, SourceType},
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

/// In-memory version repo that actually stores published versions.
///
/// Using this instead of NoopVersionRepo allows tests that register skill
/// packages to find the skill's latest version via `find_latest`.
pub struct InMemoryVersionRepo {
    pub versions: Mutex<HashMap<Uuid, ArtifactVersion>>,
}

impl InMemoryVersionRepo {
    pub fn new() -> Self {
        Self {
            versions: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl VersionRepository for InMemoryVersionRepo {
    async fn publish(&self, v: ArtifactVersion) -> Result<ArtifactVersion, CoreError> {
        self.versions.lock().unwrap().insert(v.id, v.clone());
        Ok(v)
    }

    async fn find_latest(&self, artifact_id: Uuid) -> Result<Option<ArtifactVersion>, CoreError> {
        let guard = self.versions.lock().unwrap();
        let latest = guard
            .values()
            .filter(|v| v.artifact_id == artifact_id)
            .max_by_key(|v| (v.major, v.minor, v.patch));
        Ok(latest.cloned())
    }

    async fn find_by_semver(
        &self,
        artifact_id: Uuid,
        ver: &str,
    ) -> Result<Option<ArtifactVersion>, CoreError> {
        let guard = self.versions.lock().unwrap();
        Ok(guard
            .values()
            .find(|v| v.artifact_id == artifact_id && v.version == ver)
            .cloned())
    }

    async fn list_for_artifact(&self, artifact_id: Uuid) -> Result<Vec<ArtifactVersion>, CoreError> {
        Ok(self
            .versions
            .lock()
            .unwrap()
            .values()
            .filter(|v| v.artifact_id == artifact_id)
            .cloned()
            .collect())
    }
}

/// Fully stateful in-memory social repository.
///
/// Replaces `NoopSocialRepo` to allow E2E tests to assert that social actions
/// (likes, comments, ratings, interactions) are actually persisted and queryable.
pub struct InMemorySocialRepo {
    pub comments:     Mutex<HashMap<Uuid, Comment>>,
    pub likes:        Mutex<HashMap<Uuid, Like>>,         // keyed by like.id
    pub ratings:      Mutex<HashMap<Uuid, Rating>>,       // keyed by rating.id
    pub interactions: Mutex<HashMap<Uuid, AgentInteraction>>,
}

impl InMemorySocialRepo {
    pub fn new() -> Self {
        Self {
            comments:     Mutex::new(HashMap::new()),
            likes:        Mutex::new(HashMap::new()),
            ratings:      Mutex::new(HashMap::new()),
            interactions: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SocialRepository for InMemorySocialRepo {
    async fn add_comment(&self, c: Comment) -> Result<Comment, CoreError> {
        self.comments.lock().unwrap().insert(c.id, c.clone());
        Ok(c)
    }

    async fn list_comments(&self, artifact_id: Uuid) -> Result<Vec<Comment>, CoreError> {
        Ok(self
            .comments.lock().unwrap().values()
            .filter(|c| c.artifact_id == artifact_id)
            .cloned().collect())
    }

    async fn update_comment(
        &self,
        comment_id: Uuid,
        artifact_id: Uuid,
        author_id: Uuid,
        content: String,
    ) -> Result<Comment, CoreError> {
        let mut guard = self.comments.lock().unwrap();
        let c = guard.get_mut(&comment_id).ok_or_else(|| CoreError::NotFound("comment".into()))?;
        if c.artifact_id != artifact_id || c.author_id != author_id {
            return Err(CoreError::NotFound("comment".into()));
        }
        c.content = content;
        c.updated_at = chrono::Utc::now();
        Ok(c.clone())
    }

    async fn delete_comment(
        &self,
        comment_id: Uuid,
        _artifact_id: Uuid,
        _author_id: Uuid,
    ) -> Result<(), CoreError> {
        self.comments.lock().unwrap().remove(&comment_id);
        Ok(())
    }

    async fn add_like(&self, l: Like) -> Result<Like, CoreError> {
        self.likes.lock().unwrap().insert(l.id, l.clone());
        Ok(l)
    }

    async fn remove_like(&self, artifact_id: Uuid, user_id: Uuid) -> Result<(), CoreError> {
        self.likes.lock().unwrap()
            .retain(|_, l| !(l.artifact_id == artifact_id && l.user_id == user_id));
        Ok(())
    }

    async fn list_likes(&self, artifact_id: Uuid) -> Result<Vec<Like>, CoreError> {
        Ok(self
            .likes.lock().unwrap().values()
            .filter(|l| l.artifact_id == artifact_id)
            .cloned().collect())
    }

    async fn add_rating(&self, r: Rating) -> Result<Rating, CoreError> {
        self.ratings.lock().unwrap().insert(r.id, r.clone());
        Ok(r)
    }

    async fn list_ratings(&self, artifact_id: Uuid) -> Result<Vec<Rating>, CoreError> {
        Ok(self
            .ratings.lock().unwrap().values()
            .filter(|r| r.artifact_id == artifact_id)
            .cloned().collect())
    }

    async fn record_interaction(&self, i: AgentInteraction) -> Result<AgentInteraction, CoreError> {
        self.interactions.lock().unwrap().insert(i.id, i.clone());
        Ok(i)
    }

    async fn list_interactions(&self, artifact_id: Uuid) -> Result<Vec<AgentInteraction>, CoreError> {
        Ok(self
            .interactions.lock().unwrap().values()
            .filter(|i| i.artifact_id == artifact_id)
            .cloned().collect())
    }

    async fn get_stats(&self, artifact_id: Uuid) -> Result<ArtifactStats, CoreError> {
        let likes  = self.list_likes(artifact_id).await?.len() as i64;
        let comments = self.list_comments(artifact_id).await?.len() as i64;
        let ratings_vec = self.list_ratings(artifact_id).await?;
        let ratings_count = ratings_vec.len() as i64;
        let avg_rating = if ratings_count > 0 {
            Some(ratings_vec.iter().map(|r| r.score as f64).sum::<f64>() / ratings_count as f64)
        } else {
            None
        };
        let interactions = self.list_interactions(artifact_id).await?.len() as i64;
        Ok(ArtifactStats {
            likes_count: likes,
            comments_count: comments,
            ratings_count,
            avg_rating,
            interactions_count: interactions,
        })
    }
}

// ── In-memory SkillPackage / SkillInstall repos ───────────────────────────────

pub struct InMemorySkillPackageRepo {
    pub packages: Mutex<HashMap<Uuid, SkillPackage>>,
}

impl InMemorySkillPackageRepo {
    pub fn new() -> Self {
        Self {
            packages: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SkillPackageRepository for InMemorySkillPackageRepo {
    async fn register(&self, pkg: SkillPackage) -> Result<SkillPackage, CoreError> {
        self.packages.lock().unwrap().insert(pkg.id, pkg.clone());
        Ok(pkg)
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SkillPackage>, CoreError> {
        Ok(self.packages.lock().unwrap().get(&id).cloned())
    }
    async fn find_by_version_and_source(
        &self,
        version_id: Uuid,
        source_type: &SourceType,
    ) -> Result<Option<SkillPackage>, CoreError> {
        Ok(self
            .packages
            .lock()
            .unwrap()
            .values()
            .find(|p| p.artifact_version_id == version_id && &p.source_type == source_type)
            .cloned())
    }
    async fn list_for_version(&self, version_id: Uuid) -> Result<Vec<SkillPackage>, CoreError> {
        Ok(self
            .packages
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.artifact_version_id == version_id)
            .cloned()
            .collect())
    }
    async fn list_for_artifact(&self, artifact_id: Uuid) -> Result<Vec<SkillPackage>, CoreError> {
        // Walk all versions: find packages whose version belongs to the artifact.
        // In tests the artifact_id is stored on SkillPackage via the version chain —
        // since we store the version in InMemoryVersionRepo we don't have a direct
        // link here, so we expose a best-effort scan (sufficient for unit tests).
        let _ = artifact_id; // field not stored on SkillPackage directly
        Ok(self.packages.lock().unwrap().values().cloned().collect())
    }
    async fn delete(&self, id: Uuid) -> Result<(), CoreError> {
        self.packages.lock().unwrap().remove(&id);
        Ok(())
    }
}

pub struct InMemorySkillInstallRepo {
    pub installs: Mutex<HashMap<Uuid, SkillInstall>>,
}

impl InMemorySkillInstallRepo {
    pub fn new() -> Self {
        Self {
            installs: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SkillInstallRepository for InMemorySkillInstallRepo {
    async fn record(&self, install: SkillInstall) -> Result<SkillInstall, CoreError> {
        self.installs
            .lock()
            .unwrap()
            .insert(install.id, install.clone());
        Ok(install)
    }
    async fn find_by_package_and_agent(
        &self,
        package_id: Uuid,
        agent: &AgentKind,
    ) -> Result<Option<SkillInstall>, CoreError> {
        Ok(self
            .installs
            .lock()
            .unwrap()
            .values()
            .find(|i| i.skill_package_id == package_id && &i.agent_kind == agent)
            .cloned())
    }
    async fn list_for_package(&self, package_id: Uuid) -> Result<Vec<SkillInstall>, CoreError> {
        Ok(self
            .installs
            .lock()
            .unwrap()
            .values()
            .filter(|i| i.skill_package_id == package_id)
            .cloned()
            .collect())
    }
    async fn list_for_agent(&self, agent: &AgentKind) -> Result<Vec<SkillInstall>, CoreError> {
        Ok(self
            .installs
            .lock()
            .unwrap()
            .values()
            .filter(|i| &i.agent_kind == agent)
            .cloned()
            .collect())
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
        versions: Arc::new(InMemoryVersionRepo::new()),
        social: Arc::new(InMemorySocialRepo::new()),
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
        skill_packages: Arc::new(InMemorySkillPackageRepo::new()),
        skill_installs: Arc::new(InMemorySkillInstallRepo::new()),
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

/// Build a test app and return both the router and the underlying AppState
/// (so tests can inspect repository contents after exercising the API).
pub fn build_test_app_with_state() -> (Router, AppState) {
    let state = make_mock_state(Arc::new(InMemoryUserRepo::new()));
    let router = build_router(state.clone()).with_state(state.clone());
    (router, state)
}
