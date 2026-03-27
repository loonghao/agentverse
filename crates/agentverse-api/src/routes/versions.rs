use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use agentverse_core::artifact::{ArtifactStatus, ArtifactVersion};
use agentverse_core::versioning::{VersionBump, VersionEngine};
use agentverse_events::types::DomainEvent;

use crate::{
    error::{ApiError, ApiResult},
    extractors::AuthUser,
    routes::artifacts::{parse_kind, sha256_hex},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct PublishVersionRequest {
    pub content: serde_json::Value,
    pub changelog: Option<String>,
    /// "patch" | "minor" | "major" — if omitted, defaults to "patch"
    pub bump: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeprecateVersionRequest {
    /// Specific semver to deprecate, e.g. "1.2.3". If omitted, deprecates the latest.
    pub version: Option<String>,
    pub reason: Option<String>,
}

/// GET /api/v1/:kind/:namespace/:name/versions
pub async fn list_versions(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!("list versions for {kind_str}/{namespace}/{name}");
    let kind = parse_kind(&kind_str)?;
    let artifact = state
        .artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    let versions = state.versions.list_for_artifact(artifact.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact_id": artifact.id,
            "versions": versions,
            "total": versions.len(),
        })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/publish
pub async fn publish_version(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<PublishVersionRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::info!("publishing new version for {kind_str}/{namespace}/{name}");
    let kind = parse_kind(&kind_str)?;

    let artifact = state
        .artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    // Only the author may publish new versions
    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden(
            "only the author can publish new versions".into(),
        ));
    }

    if !artifact.is_modifiable() {
        return Err(ApiError::BadRequest(format!(
            "artifact is {:?}",
            artifact.status
        )));
    }

    // Resolve bump type
    let current_ver = state
        .versions
        .find_latest(artifact.id)
        .await?
        .map(|v| v.version)
        .unwrap_or_else(|| "0.0.0".into());

    let bump = match req.bump.as_deref() {
        Some("major") => VersionBump::Major,
        Some("minor") => VersionBump::Minor,
        Some("patch") => VersionBump::Patch,
        _ => VersionBump::Patch,
    };

    let next_ver = VersionEngine::bump(&current_ver, bump.clone())
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let content_bytes = serde_json::to_vec(&req.content).unwrap_or_default();
    let version = ArtifactVersion {
        id: Uuid::new_v4(),
        artifact_id: artifact.id,
        major: next_ver
            .split('.')
            .nth(0)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        minor: next_ver
            .split('.')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        patch: next_ver
            .split('.')
            .nth(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        pre_release: None,
        version: next_ver.clone(),
        content: req.content,
        checksum: sha256_hex(&content_bytes),
        signature: None,
        changelog: req.changelog,
        bump_reason: format!("{:?}", bump).to_lowercase(),
        published_by: claims.sub,
        published_at: Utc::now(),
    };
    let version = state.versions.publish(version).await?;

    state
        .events
        .append(DomainEvent::VersionPublished {
            artifact_id: artifact.id,
            version_id: version.id,
            version: next_ver,
            bump_reason: version.bump_reason.clone(),
            published_by: claims.sub,
        })
        .await
        .ok();

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "version": version })),
    ))
}

/// POST /api/v1/:kind/:namespace/:name/deprecate — deprecate a specific version
///
/// If `version` is provided in the body, that specific semver is marked as the last
/// deprecated entry by publishing a "deprecated" pre-release patch on top of it.
/// Otherwise the artifact itself is deprecated (delegates to artifact deprecation logic).
pub async fn deprecate_version(
    Path((kind_str, namespace, name)): Path<(String, String, String)>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<Option<DeprecateVersionRequest>>,
) -> ApiResult<impl IntoResponse> {
    let req = req.unwrap_or(DeprecateVersionRequest {
        version: None,
        reason: None,
    });
    tracing::info!(
        "deprecating {kind_str}/{namespace}/{name} version={:?}",
        req.version
    );

    let kind = parse_kind(&kind_str)?;
    let mut artifact = state
        .artifacts
        .find_by_namespace_name(&kind, &namespace, &name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{kind_str}/{namespace}/{name}")))?;

    // Only the author may deprecate
    if artifact.author_id != claims.sub {
        return Err(ApiError::Forbidden(
            "only the author can deprecate versions".into(),
        ));
    }

    // When a specific version is named, deprecate only the artifact if ALL versions are meant
    // to be retired. For simplicity (no per-version status field) we:
    //  - If no version specified → deprecate the whole artifact
    //  - If a version specified → validate it exists, then deprecate the artifact and note the version
    if let Some(ref ver) = req.version {
        state
            .versions
            .find_by_semver(artifact.id, ver)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("version {ver}")))?;
    }

    artifact.status = ArtifactStatus::Deprecated;
    artifact.updated_at = Utc::now();
    let artifact = state.artifacts.update(artifact).await?;

    state
        .events
        .append(DomainEvent::ArtifactDeprecated {
            artifact_id: artifact.id,
            deprecated_by: claims.sub,
        })
        .await
        .ok();

    let message = match req.version {
        Some(ref v) => format!("version {v} and all versions deprecated"),
        None => "artifact deprecated".into(),
    };
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "artifact": artifact,
            "message": message,
            "reason": req.reason,
        })),
    ))
}
