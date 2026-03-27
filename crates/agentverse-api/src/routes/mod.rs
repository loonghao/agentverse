pub mod artifacts;
pub mod auth;
pub mod health;
pub mod social;
pub mod versions;

use crate::state::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};

/// Build the complete API router.
/// Returns `Router<AppState>` — caller must call `.with_state(state)` to finalise.
pub fn build_router(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/ready", get(health::readiness_check))
        .merge(auth_routes())
        .merge(artifact_routes())
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/me", get(auth::me).put(auth::update_me))
        // Public user profiles — accessible without authentication
        .route("/api/v1/users/{id_or_username}", get(auth::get_user))
        // List artifacts published by a user
        .route(
            "/api/v1/users/{id_or_username}/artifacts",
            get(auth::list_user_artifacts),
        )
}

fn artifact_routes() -> Router<AppState> {
    Router::new()
        // Search (must come before /{kind} to avoid conflict)
        .route("/api/v1/search", get(artifacts::search_artifacts))
        .route("/api/v1/search/semantic", post(artifacts::semantic_search))
        // List + Create by kind
        .route(
            "/api/v1/{kind}",
            get(artifacts::list_artifacts).post(artifacts::create_artifact),
        )
        // Version management
        .route(
            "/api/v1/{kind}/{namespace}/{name}/versions",
            get(versions::list_versions),
        )
        .route(
            "/api/v1/{kind}/{namespace}/{name}/publish",
            post(versions::publish_version),
        )
        .route(
            "/api/v1/{kind}/{namespace}/{name}/deprecate",
            post(versions::deprecate_version),
        )
        // Revoke (security incident — harder than deprecate)
        .route(
            "/api/v1/{kind}/{namespace}/{name}/revoke",
            post(artifacts::revoke_artifact),
        )
        // Social — likes
        .route(
            "/api/v1/{kind}/{namespace}/{name}/likes",
            get(social::list_likes)
                .post(social::add_like)
                .delete(social::remove_like),
        )
        // Social — comments (collection)
        .route(
            "/api/v1/{kind}/{namespace}/{name}/comments",
            get(social::list_comments).post(social::add_comment),
        )
        // Social — comments (individual)
        .route(
            "/api/v1/{kind}/{namespace}/{name}/comments/{comment_id}",
            put(social::update_comment).delete(social::delete_comment),
        )
        // Social — ratings
        .route(
            "/api/v1/{kind}/{namespace}/{name}/ratings",
            get(social::list_ratings).post(social::add_rating),
        )
        // Social — agent interactions
        .route(
            "/api/v1/{kind}/{namespace}/{name}/interactions",
            get(social::list_interactions),
        )
        // Aggregate stats
        .route(
            "/api/v1/{kind}/{namespace}/{name}/stats",
            get(social::artifact_stats),
        )
        // Tag management
        .route(
            "/api/v1/{kind}/{namespace}/{name}/tags",
            post(social::add_tag),
        )
        .route(
            "/api/v1/{kind}/{namespace}/{name}/tags/{tag}",
            axum::routing::delete(social::remove_tag),
        )
        // Embedding update (for semantic search)
        .route(
            "/api/v1/{kind}/{namespace}/{name}/embedding",
            post(artifacts::update_embedding),
        )
        // Agent-specific actions
        .route(
            "/api/v1/{kind}/{namespace}/{name}/fork",
            post(social::fork_artifact),
        )
        .route(
            "/api/v1/{kind}/{namespace}/{name}/learn",
            post(social::record_learning),
        )
        .route(
            "/api/v1/{kind}/{namespace}/{name}/benchmark",
            post(social::record_benchmark),
        )
        // Artifact CRUD (must come after sub-path routes)
        .route(
            "/api/v1/{kind}/{namespace}/{name}",
            get(artifacts::get_artifact)
                .put(artifacts::update_artifact)
                .delete(artifacts::deprecate_artifact),
        )
        // Specific version
        .route(
            "/api/v1/{kind}/{namespace}/{name}/{version}",
            get(artifacts::get_artifact_version),
        )
}
