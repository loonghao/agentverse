use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use agentverse_core::{
    artifact::{Artifact, ArtifactKind, ArtifactStatus, ArtifactVersion, Manifest},
    repository::ArtifactFilter,
};
use agentverse_events::types::DomainEvent;
use crate::{error::{ApiError, ApiResult}, extractors::AuthUser, state::AppState};

// ── Request/Response DTOs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub tag: Option<String>,
    pub namespace: Option<String>,
    pub status: Option<String>,
    pub author_id: Option<Uuid>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub kind: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    /// Pre-computed embedding vector (e.g. from OpenAI text-embedding-3-small or Minilm).
    /// Must match the dimension of the stored embeddings (default: 384).
    pub embedding: Option<Vec<f32>>,
    pub kind: Option<ArtifactKind>,
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateArtifactRequest {
    pub namespace: String,
    pub name: String,
    pub display_name: Option<String>,
    pub manifest: Manifest,
    /// Initial content published as version 0.1.0
    pub content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArtifactRequest {
    pub display_name: Option<String>,
    pub manifest: Option<Manifest>,
    /// Explicit bump override; if None, auto-infer from manifest diff
    pub bump: Option<String>,
    pub content: Option<serde_json::Value>,
    pub changelog: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/v1/:kind — list artifacts
pub async fn list_artifacts(
    Path(kind_str): Path<String>,
    Query(params): Query<ListQuery>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let kind = parse_kind(&kind_str)?;

    // Parse optional status string into the enum
    let status = match params.status.as_deref() {
        Some("active") => Some(ArtifactStatus::Active),
        Some("deprecated") => Some(ArtifactStatus::Deprecated),
        Some("retired") => Some(ArtifactStatus::Retired),
        Some("revoked") => Some(ArtifactStatus::Revoked),
        Some(other) => return Err(ApiError::BadRequest(format!("unknown status: {other}"))),
        None => None,
    };

    let filter = ArtifactFilter {
        kind: Some(kind),
        namespace: params.namespace,
        tag: params.tag,
        author_id: params.author_id,
        status,
        limit: params.limit,
        offset: params.offset,
    };
    let items = state.artifacts.list(filter).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "items": items, "total": items.len() }))))
}

/// POST /api/v1/:kind — create artifact + publish version 0.1.0
pub async fn create_artifact(
    Path(kind_str): Path<String>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateArtifactRequest>,
) -> ApiResult<impl IntoResponse> {
    let kind = parse_kind(&kind_str)?;

    // Guard duplicate
    if state.artifacts.find_by_namespace_name(&kind, &req.namespace, &req.name).await?.is_some() {
        return Err(ApiError::Conflict(format!("{}/{} already exists", req.namespace, req.name)));
    }

    let author_id = claims.sub;
    let artifact_id = Uuid::new_v4();
    let now = Utc::now();

    let artifact = Artifact {
        id: artifact_id,
        kind,
        namespace: req.namespace.clone(),
        name: req.name.clone(),
        display_name: req.display_name.clone(),
        manifest: req.manifest.clone(),
        status: ArtifactStatus::Active,
        author_id,
        downloads: 0,
        created_at: now,
        updated_at: now,
    };

    let artifact = state.artifacts.create(artifact).await?;

    // Compute SHA-256 checksum of the content
    let content_bytes = serde_json::to_vec(&req.content).unwrap_or_default();
    let checksum = sha256_hex(&content_bytes);

    let version = ArtifactVersion {
        id: Uuid::new_v4(),
        artifact_id: artifact.id,
        version: "0.1.0".into(),
        major: 0, minor: 1, patch: 0,
        pre_release: None,
        content: req.content,
        checksum,
        signature: None,
        changelog: Some("Initial release".into()),
        bump_reason: "minor".into(),
        published_by: author_id,
        published_at: now,
    };
    let version = state.versions.publish(version).await?;

    // Emit domain event
    state.events.append(DomainEvent::ArtifactCreated {
        artifact_id: artifact.id,
        kind: kind_str.clone(),
        namespace: req.namespace.clone(),
        name: req.name.clone(),
        author_id,
    }).await.ok();

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "artifact": artifact, "version": version }))))
}

/// GET /api/v1/:kind/:namespace/:name — get latest version
pub async fn get_artifact(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let kind = parse_kind(&kind_str)?;
    let artifact = state.artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    state.artifacts.increment_downloads(artifact.id).await.ok();
    let version = state.versions.find_latest(artifact.id).await?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "artifact": artifact, "version": version }))))
}

/// GET /api/v1/:kind/:namespace/:name/:version — get specific version
pub async fn get_artifact_version(
    Path((kind_str, namespace, name, ver)): Path<(String, String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let kind = parse_kind(&kind_str)?;
    let artifact = state.artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;
    let version = state.versions
        .find_by_semver(artifact.id, &ver)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("version {ver}")))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "artifact": artifact, "version": version }))))
}

/// PUT /api/v1/:kind/:namespace/:name — update artifact manifest + auto-bump version
pub async fn update_artifact(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<UpdateArtifactRequest>,
) -> ApiResult<impl IntoResponse> {
    use agentverse_core::versioning::{VersionBump, VersionEngine};

    let kind = parse_kind(&kind_str)?;
    let mut artifact = state.artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    // Only the author may update their artifact
    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can update this artifact".into()));
    }

    if !artifact.is_modifiable() {
        return Err(ApiError::BadRequest(format!("artifact status is {:?}", artifact.status)));
    }

    let old_manifest = serde_json::to_value(&artifact.manifest).unwrap_or_default();
    if let Some(new_manifest) = req.manifest {
        artifact.manifest = new_manifest;
    }
    if let Some(dn) = req.display_name {
        artifact.display_name = Some(dn);
    }
    artifact.updated_at = Utc::now();

    let artifact = state.artifacts.update(artifact.clone()).await?;

    // Compute bump type
    if let Some(content) = req.content {
        let new_manifest_json = serde_json::to_value(&artifact.manifest).unwrap_or_default();
        let bump = match req.bump.as_deref() {
            Some("major") => VersionBump::Major,
            Some("minor") => VersionBump::Minor,
            _ if state.config.auto_infer_bump =>
                VersionEngine::infer_bump(&old_manifest, &new_manifest_json),
            _ => VersionBump::Patch,
        };

        let current = state.versions.find_latest(artifact.id).await?
            .map(|v| v.version)
            .unwrap_or_else(|| "0.0.0".into());
        let next_ver = VersionEngine::bump(&current, bump.clone())
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        let content_bytes = serde_json::to_vec(&content).unwrap_or_default();
        let version = ArtifactVersion {
            id: Uuid::new_v4(),
            artifact_id: artifact.id,
            version: next_ver.clone(),
            major: next_ver.split('.').nth(0).and_then(|s| s.parse().ok()).unwrap_or(0),
            minor: next_ver.split('.').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0),
            patch: next_ver.split('.').nth(2).and_then(|s| s.parse().ok()).unwrap_or(0),
            pre_release: None,
            content,
            checksum: sha256_hex(&content_bytes),
            signature: None,
            changelog: req.changelog,
            bump_reason: format!("{:?}", bump).to_lowercase(),
            published_by: claims.sub,
            published_at: Utc::now(),
        };
        let version = state.versions.publish(version).await?;
        state.events.append(DomainEvent::ArtifactUpdated {
            artifact_id: artifact.id,
            updated_by: claims.sub,
        }).await.ok();
        return Ok((StatusCode::OK, Json(serde_json::json!({ "artifact": artifact, "version": version }))));
    }

    Ok((StatusCode::OK, Json(serde_json::json!({ "artifact": artifact }))))
}

/// DELETE /api/v1/:kind/:namespace/:name — soft-deprecate
pub async fn deprecate_artifact(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    let kind = parse_kind(&kind_str)?;
    let mut artifact = state.artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    // Only the author can deprecate their artifact
    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can deprecate this artifact".into()));
    }

    artifact.status = ArtifactStatus::Deprecated;
    artifact.updated_at = Utc::now();
    let artifact = state.artifacts.update(artifact).await?;
    state.events.append(DomainEvent::ArtifactDeprecated {
        artifact_id: artifact.id,
        deprecated_by: claims.sub,
    }).await.ok();
    Ok((StatusCode::OK, Json(serde_json::json!({ "artifact": artifact }))))
}

/// POST /api/v1/:kind/:namespace/:name/revoke — hard-revoke (security incident)
pub async fn revoke_artifact(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<impl IntoResponse> {
    let kind = parse_kind(&kind_str)?;
    let mut artifact = state.artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    // Only the author can revoke
    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can revoke this artifact".into()));
    }

    let reason = body.get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("security incident")
        .to_string();

    artifact.status = ArtifactStatus::Revoked;
    artifact.updated_at = Utc::now();
    let artifact = state.artifacts.update(artifact).await?;

    state.events.append(agentverse_events::types::DomainEvent::ArtifactRevoked {
        artifact_id: artifact.id,
        revoked_by: claims.sub,
        reason: reason.clone(),
    }).await.ok();

    Ok((StatusCode::OK, Json(serde_json::json!({
        "artifact": artifact,
        "reason": reason,
    }))))
}

/// POST /api/v1/:kind/:namespace/:name/embedding — set embedding vector for semantic search
///
/// Only the artifact author may update the embedding.
/// `embedding` must be a flat array of f32 matching the configured vector dimension (e.g. 384 or 1536).
pub async fn update_embedding(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<impl IntoResponse> {
    let kind = parse_kind(&kind_str)?;
    let artifact = state.artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden("only the author can update the embedding".into()));
    }

    let embedding: Vec<f32> = body
        .get("embedding")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::BadRequest("missing `embedding` array field".into()))?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    if embedding.is_empty() {
        return Err(ApiError::BadRequest("embedding vector must not be empty".into()));
    }

    state.semantic
        .update_embedding(artifact.id, &embedding)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "artifact_id": artifact.id,
        "embedding_dim": embedding.len(),
        "message": "embedding updated",
    }))))
}

/// GET /api/v1/search?q=...
pub async fn search_artifacts(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let items = state.fulltext
        .search(
            &params.q,
            params.kind.as_deref(),
            params.tag.as_deref(),
            params.limit.unwrap_or(20),
        )
        .await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "items": items, "total": items.len() }))))
}

/// POST /api/v1/search/semantic
///
/// Performs vector similarity search when an `embedding` vector is provided.
/// Without a vector the endpoint returns guidance on how to obtain one.
pub async fn semantic_search(
    State(state): State<AppState>,
    Json(req): Json<SemanticSearchRequest>,
) -> ApiResult<impl IntoResponse> {
    let limit = req.limit.unwrap_or(10);
    let kind = req.kind.as_ref().map(|k| match k {
        ArtifactKind::Skill => "skill",
        ArtifactKind::Soul => "soul",
        ArtifactKind::Agent => "agent",
        ArtifactKind::Workflow => "workflow",
        ArtifactKind::Prompt => "prompt",
    });

    match req.embedding {
        Some(ref vec) => {
            let items = state.semantic
                .search_by_vector(vec, kind, limit)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
            Ok((StatusCode::OK, Json(serde_json::json!({
                "query": req.query,
                "items": items,
                "total": items.len(),
            }))))
        }
        None => {
            // No embedding provided — return instructional stub
            Ok((StatusCode::OK, Json(serde_json::json!({
                "query": req.query,
                "items": [],
                "total": 0,
                "note": "Provide a pre-computed `embedding` (f32 array) matching the stored \
                         vector dimension. Use an embedding model such as \
                         text-embedding-3-small (OpenAI) or all-MiniLM-L6-v2 (local) to \
                         convert your query string into a vector first.",
            }))))
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn parse_kind(s: &str) -> ApiResult<ArtifactKind> {
    match s {
        "skill" => Ok(ArtifactKind::Skill),
        "soul" => Ok(ArtifactKind::Soul),
        "agent" => Ok(ArtifactKind::Agent),
        "workflow" => Ok(ArtifactKind::Workflow),
        "prompt" => Ok(ArtifactKind::Prompt),
        other => Err(ApiError::BadRequest(format!("unknown kind: {other}"))),
    }
}

/// Compute SHA-256 hex digest of bytes.
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

