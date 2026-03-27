use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use agentverse_core::{
    artifact::{Artifact, ArtifactStatus},
    social::{AgentInteraction, Comment, CommentKind, InteractionKind, Like, Rating},
};
use agentverse_events::types::DomainEvent;

#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}

use crate::{
    error::{ApiError, ApiResult},
    extractors::AuthUser,
    routes::artifacts::{parse_kind, sha256_hex},
    state::AppState,
};

// ── Request DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddCommentRequest {
    pub content: String,
    pub kind: CommentKind,
    pub parent_id: Option<Uuid>,
    pub version_id: Option<Uuid>,
    pub benchmark_payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AddRatingRequest {
    /// 1..=5
    pub score: i16,
    pub review_text: Option<String>,
    pub version_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ForkRequest {
    pub source_version: Option<String>,
    pub new_namespace: Option<String>,
    pub new_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LearningRequest {
    pub content: String,
    pub confidence_score: Option<f64>,
    pub payload: Option<serde_json::Value>,
    pub version_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct BenchmarkRequest {
    pub metrics: serde_json::Value,
    pub confidence_score: Option<f64>,
    pub version_id: Option<Uuid>,
}

// ── Helper ────────────────────────────────────────────────────────────────────

async fn resolve_artifact(
    state: &AppState,
    kind_str: &str,
    namespace: &str,
    name: &str,
) -> ApiResult<Artifact> {
    let kind = parse_kind(kind_str)?;
    state
        .artifacts
        .find_by_namespace_name(&kind, namespace, name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/v1/:kind/:namespace/:name/comments
pub async fn list_comments(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let comments = state.social.list_comments(artifact.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "comments": comments,
            "total": comments.len(),
        })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/comments
pub async fn add_comment(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<AddCommentRequest>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let now = Utc::now();
    let comment = Comment {
        id: Uuid::new_v4(),
        artifact_id: artifact.id,
        version_id: req.version_id,
        author_id: claims.sub,
        parent_id: req.parent_id,
        content: req.content,
        kind: req.kind.clone(),
        likes_count: 0,
        benchmark_payload: req.benchmark_payload,
        created_at: now,
        updated_at: now,
    };
    let comment = state.social.add_comment(comment).await?;

    state
        .events
        .append(DomainEvent::CommentAdded {
            artifact_id: artifact.id,
            comment_id: comment.id,
            author_id: claims.sub,
            kind: format!("{:?}", req.kind).to_lowercase(),
        })
        .await
        .ok();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "comment": comment })),
    ))
}

/// GET /api/v1/:kind/:namespace/:name/likes — list users who liked this artifact
pub async fn list_likes(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let likes = state.social.list_likes(artifact.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "likes": likes,
            "total": likes.len(),
        })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/likes
pub async fn add_like(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let like = Like {
        id: Uuid::new_v4(),
        artifact_id: artifact.id,
        version_id: None,
        user_id: claims.sub,
        created_at: Utc::now(),
    };
    let like = state.social.add_like(like).await?;

    state
        .events
        .append(DomainEvent::LikeAdded {
            artifact_id: artifact.id,
            user_id: claims.sub,
        })
        .await
        .ok();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "like": like })),
    ))
}

/// DELETE /api/v1/:kind/:namespace/:name/likes
pub async fn remove_like(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    state.social.remove_like(artifact.id, claims.sub).await?;

    state
        .events
        .append(DomainEvent::LikeRemoved {
            artifact_id: artifact.id,
            user_id: claims.sub,
        })
        .await
        .ok();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "unliked" })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/ratings
pub async fn add_rating(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<AddRatingRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.score < 1 || req.score > 5 {
        return Err(ApiError::BadRequest("score must be between 1 and 5".into()));
    }
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let rating = Rating {
        id: Uuid::new_v4(),
        artifact_id: artifact.id,
        version_id: req.version_id,
        user_id: claims.sub,
        score: req.score,
        review_text: req.review_text,
        created_at: Utc::now(),
    };
    let rating = state.social.add_rating(rating).await?;

    state
        .events
        .append(DomainEvent::RatingAdded {
            artifact_id: artifact.id,
            user_id: claims.sub,
            score: req.score,
        })
        .await
        .ok();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "rating": rating })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/fork — Agent or human forks an artifact
pub async fn fork_artifact(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ForkRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::info!("fork {kind_str}/{namespace}/{name} -> {}", req.new_name);
    let source = resolve_artifact(&state, &kind_str, &namespace, &name).await?;

    // Resolve source version content
    let source_version = match &req.source_version {
        Some(ver) => state
            .versions
            .find_by_semver(source.id, ver)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("version {ver}")))?,
        None => state
            .versions
            .find_latest(source.id)
            .await?
            .ok_or_else(|| ApiError::NotFound("no published version".into()))?,
    };

    let new_namespace = req.new_namespace.unwrap_or_else(|| claims.username.clone());
    let new_kind = parse_kind(&kind_str)?;

    // Guard: new name must not conflict
    if state
        .artifacts
        .find_by_namespace_name(&new_kind, &new_namespace, &req.new_name)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "{new_namespace}/{} already exists",
            req.new_name
        )));
    }

    let now = Utc::now();
    let new_id = Uuid::new_v4();
    let forked_artifact = Artifact {
        id: new_id,
        kind: new_kind,
        namespace: new_namespace.clone(),
        name: req.new_name.clone(),
        display_name: source.display_name.clone(),
        manifest: source.manifest.clone(),
        status: ArtifactStatus::Active,
        author_id: claims.sub,
        downloads: 0,
        created_at: now,
        updated_at: now,
    };
    let forked_artifact = state.artifacts.create(forked_artifact).await?;

    // Publish forked version as 0.1.0
    let content_bytes = serde_json::to_vec(&source_version.content).unwrap_or_default();
    let forked_version = agentverse_core::artifact::ArtifactVersion {
        id: Uuid::new_v4(),
        artifact_id: forked_artifact.id,
        version: "0.1.0".into(),
        major: 0,
        minor: 1,
        patch: 0,
        pre_release: None,
        content: source_version.content.clone(),
        checksum: sha256_hex(&content_bytes),
        signature: None,
        changelog: Some(format!(
            "Forked from {}/{}/{}",
            kind_str, source.namespace, source.name
        )),
        bump_reason: "minor".into(),
        published_by: claims.sub,
        published_at: now,
    };
    let forked_version = state.versions.publish(forked_version).await?;

    // Record the fork interaction
    let interaction = AgentInteraction {
        id: Uuid::new_v4(),
        from_agent_id: claims.sub,
        artifact_id: source.id,
        version_id: Some(source_version.id),
        kind: InteractionKind::Fork,
        payload: serde_json::json!({ "forked_to": forked_artifact.id }),
        confidence_score: None,
        created_at: now,
    };
    state.social.record_interaction(interaction).await.ok();

    state
        .events
        .append(DomainEvent::ArtifactForked {
            source_artifact_id: source.id,
            new_artifact_id: forked_artifact.id,
            forked_by: claims.sub,
        })
        .await
        .ok();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "artifact": forked_artifact,
            "version": forked_version,
        })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/learn — Agent submits a learning insight
pub async fn record_learning(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<LearningRequest>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let now = Utc::now();

    // Record as a learning comment
    let comment = Comment {
        id: Uuid::new_v4(),
        artifact_id: artifact.id,
        version_id: req.version_id,
        author_id: claims.sub,
        parent_id: None,
        content: req.content.clone(),
        kind: CommentKind::Learning,
        likes_count: 0,
        benchmark_payload: req.payload.clone(),
        created_at: now,
        updated_at: now,
    };
    let comment = state.social.add_comment(comment).await?;

    // Record agent interaction
    let interaction = AgentInteraction {
        id: Uuid::new_v4(),
        from_agent_id: claims.sub,
        artifact_id: artifact.id,
        version_id: req.version_id,
        kind: InteractionKind::Learn,
        payload: req
            .payload
            .unwrap_or(serde_json::json!({ "content": req.content })),
        confidence_score: req.confidence_score,
        created_at: now,
    };
    state.social.record_interaction(interaction).await.ok();

    state
        .events
        .append(DomainEvent::AgentLearned {
            agent_id: claims.sub,
            artifact_id: artifact.id,
            confidence_score: req.confidence_score,
        })
        .await
        .ok();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "comment": comment })),
    ))
}

/// DELETE /api/v1/:kind/:namespace/:name/comments/:comment_id
pub async fn delete_comment(
    Path((kind_str, namespace, name, comment_id)): Path<(String, String, String, Uuid)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    state
        .social
        .delete_comment(comment_id, artifact.id, claims.sub)
        .await?;

    state
        .events
        .append(DomainEvent::CommentDeleted {
            comment_id,
            artifact_id: artifact.id,
            deleted_by: claims.sub,
        })
        .await
        .ok();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "comment deleted" })),
    ))
}

/// PUT /api/v1/:kind/:namespace/:name/comments/:comment_id
pub async fn update_comment(
    Path((kind_str, namespace, name, comment_id)): Path<(String, String, String, Uuid)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<UpdateCommentRequest>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let comment = state
        .social
        .update_comment(comment_id, artifact.id, claims.sub, req.content)
        .await?;

    state
        .events
        .append(DomainEvent::CommentUpdated {
            comment_id,
            artifact_id: artifact.id,
            updated_by: claims.sub,
        })
        .await
        .ok();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "comment": comment })),
    ))
}

/// GET /api/v1/:kind/:namespace/:name/ratings — list all ratings
pub async fn list_ratings(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let ratings = state.social.list_ratings(artifact.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "ratings": ratings,
            "total": ratings.len(),
        })),
    ))
}

/// GET /api/v1/:kind/:namespace/:name/stats — aggregate statistics for an artifact
pub async fn artifact_stats(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let stats = state.social.get_stats(artifact.id).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id":        artifact.id,
            "downloads":          artifact.downloads,
            "likes_count":        stats.likes_count,
            "comments_count":     stats.comments_count,
            "ratings_count":      stats.ratings_count,
            "avg_rating":         stats.avg_rating,
            "interactions_count": stats.interactions_count,
        })),
    ))
}

/// GET /api/v1/:kind/:namespace/:name/interactions — list all agent interactions
pub async fn list_interactions(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let interactions = state.social.list_interactions(artifact.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "interactions": interactions,
            "total": interactions.len(),
        })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/tags — add a tag to an artifact's manifest
///
/// Idempotent: adding an existing tag is a no-op.
pub async fn add_tag(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<impl IntoResponse> {
    let tag = body
        .get("tag")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("missing `tag` field".into()))?
        .to_string();

    if tag.is_empty() || tag.len() > 64 {
        return Err(ApiError::BadRequest("tag must be 1-64 characters".into()));
    }

    let mut artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;

    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden(
            "only the author can manage tags".into(),
        ));
    }

    // Idempotent insert
    if !artifact.manifest.tags.contains(&tag) {
        artifact.manifest.tags.push(tag.clone());
        artifact.updated_at = chrono::Utc::now();
        state.artifacts.update(artifact.clone()).await?;
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "tags": artifact.manifest.tags,
            "added": tag,
        })),
    ))
}

/// DELETE /api/v1/:kind/:namespace/:name/tags/:tag — remove a tag from an artifact's manifest
pub async fn remove_tag(
    Path((kind_str, namespace, name, tag)): Path<(String, String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    let mut artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;

    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden(
            "only the author can manage tags".into(),
        ));
    }

    let before_len = artifact.manifest.tags.len();
    artifact.manifest.tags.retain(|t| t != &tag);

    if artifact.manifest.tags.len() < before_len {
        artifact.updated_at = chrono::Utc::now();
        state.artifacts.update(artifact.clone()).await?;
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "tags": artifact.manifest.tags,
            "removed": tag,
        })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/benchmark — Agent submits benchmark results
pub async fn record_benchmark(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<BenchmarkRequest>,
) -> ApiResult<impl IntoResponse> {
    let artifact = resolve_artifact(&state, &kind_str, &namespace, &name).await?;
    let now = Utc::now();

    // Record as a benchmark comment with structured payload
    let comment = Comment {
        id: Uuid::new_v4(),
        artifact_id: artifact.id,
        version_id: req.version_id,
        author_id: claims.sub,
        parent_id: None,
        content: serde_json::to_string(&req.metrics).unwrap_or_default(),
        kind: CommentKind::Benchmark,
        likes_count: 0,
        benchmark_payload: Some(req.metrics.clone()),
        created_at: now,
        updated_at: now,
    };
    let comment = state.social.add_comment(comment).await?;

    // Record agent interaction
    let interaction = AgentInteraction {
        id: Uuid::new_v4(),
        from_agent_id: claims.sub,
        artifact_id: artifact.id,
        version_id: req.version_id,
        kind: InteractionKind::Benchmark,
        payload: req.metrics.clone(),
        confidence_score: req.confidence_score,
        created_at: now,
    };
    state.social.record_interaction(interaction).await.ok();

    state
        .events
        .append(DomainEvent::AgentBenchmarked {
            agent_id: claims.sub,
            artifact_id: artifact.id,
            version_id: req.version_id,
        })
        .await
        .ok();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "comment": comment })),
    ))
}
