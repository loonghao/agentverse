use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agentverse_core::{
    repository::ArtifactFilter,
    user::{User, UserKind},
};
use agentverse_events::types::DomainEvent;

use axum::extract::Query;

use crate::{
    error::{ApiError, ApiResult},
    extractors::AuthUser,
    state::AppState,
};

#[derive(Debug, serde::Deserialize)]
pub struct UserArtifactsQuery {
    pub kind: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
    /// "human" | "agent"
    pub kind: Option<String>,
    /// Ed25519 public key hex (for agents)
    pub public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMeRequest {
    pub email: Option<String>,
    pub password: Option<String>,
    pub public_key: Option<String>,
    pub capabilities: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// POST /api/v1/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    use agentverse_auth::PasswordManager;

    // --- Input validation ---
    if req.username.len() < 3 || req.username.len() > 32 {
        return Err(ApiError::BadRequest(
            "username must be 3–32 characters".into(),
        ));
    }
    if !req
        .username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ApiError::BadRequest(
            "username may only contain letters, digits, underscores, and hyphens".into(),
        ));
    }
    if req.kind.as_deref() != Some("agent") && req.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    // Check username uniqueness
    if state.users.find_by_username(&req.username).await?.is_some() {
        return Err(ApiError::Conflict(format!(
            "username '{}' already taken",
            req.username
        )));
    }
    // Check email uniqueness if provided
    if let Some(ref email) = req.email {
        if state.users.find_by_email(email).await?.is_some() {
            return Err(ApiError::Conflict(format!(
                "email '{}' already registered",
                email
            )));
        }
    }

    let now = Utc::now();
    let user_id = Uuid::new_v4();
    let kind = match req.kind.as_deref() {
        Some("agent") => UserKind::Agent,
        _ => UserKind::Human,
    };

    // Hash password (agents may omit password, using key-based auth instead)
    let password_hash = if req.password.is_empty() {
        None
    } else {
        Some(
            PasswordManager::hash(&req.password)
                .map_err(|e| ApiError::BadRequest(e.to_string()))?,
        )
    };

    let user = User {
        id: user_id,
        username: req.username.clone(),
        email: req.email.clone(),
        kind,
        capabilities: None,
        public_key: req.public_key.clone(),
        password_hash,
        created_at: now,
    };

    let user = state.users.create(user).await?;

    state
        .events
        .append(DomainEvent::UserRegistered {
            user_id: user.id,
            kind: format!("{:?}", user.kind).to_lowercase(),
        })
        .await
        .ok();

    // Issue token immediately
    let kind_str = format!("{:?}", user.kind).to_lowercase();
    let token = state
        .jwt
        .generate(user.id, &user.username, &kind_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "user": user,
            "access_token": token,
            "token_type": "Bearer",
            "expires_in": state.config.access_token_expiry_secs,
        })),
    ))
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    use agentverse_auth::PasswordManager;

    let user = state
        .users
        .find_by_username(&req.username)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    // Verify password — reject if no hash stored (agent or passwordless account)
    match &user.password_hash {
        Some(hash) => {
            PasswordManager::verify(&req.password, hash).map_err(|_| ApiError::Unauthorized)?;
        }
        None => {
            // Agents authenticate via signed tokens, not passwords
            return Err(ApiError::Unauthorized);
        }
    }

    let kind_str = format!("{:?}", user.kind).to_lowercase();
    let token = state
        .jwt
        .generate(user.id, &user.username, &kind_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "access_token": token,
            "token_type": "Bearer",
            "expires_in": state.config.access_token_expiry_secs,
            "user": user,
        })),
    ))
}

/// POST /api/v1/auth/refresh — re-issue a token from a still-valid token
pub async fn refresh(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    // Verify the user still exists
    let user = state
        .users
        .find_by_id(claims.sub)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("user {}", claims.sub)))?;

    let kind_str = format!("{:?}", user.kind).to_lowercase();
    let token = state
        .jwt
        .generate(user.id, &user.username, &kind_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "access_token": token,
            "token_type": "Bearer",
            "expires_in": state.config.access_token_expiry_secs,
        })),
    ))
}

/// GET /api/v1/auth/me — returns the authenticated caller's profile
pub async fn me(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ApiResult<impl IntoResponse> {
    let user = state
        .users
        .find_by_id(claims.sub)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("user {}", claims.sub)))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "user": user }))))
}

/// GET /api/v1/users/:id_or_username — public profile of any user (by UUID or username)
pub async fn get_user(
    Path(id_or_username): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    // Try UUID first, fall back to username lookup
    let user = if let Ok(id) = id_or_username.parse::<Uuid>() {
        state
            .users
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("user {id_or_username}")))?
    } else {
        state
            .users
            .find_by_username(&id_or_username)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("user {id_or_username}")))?
    };

    // Strip sensitive fields by serializing through the domain type
    // (password_hash is skip_serializing, so it won't appear in the response)
    Ok((StatusCode::OK, Json(serde_json::json!({ "user": user }))))
}

/// PUT /api/v1/auth/me — update profile (email, password, public_key, capabilities)
pub async fn update_me(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<UpdateMeRequest>,
) -> ApiResult<impl IntoResponse> {
    use agentverse_auth::PasswordManager;

    let mut user = state
        .users
        .find_by_id(claims.sub)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("user {}", claims.sub)))?;

    // Check email uniqueness if changed
    if let Some(ref new_email) = req.email {
        if user.email.as_deref() != Some(new_email.as_str())
            && state.users.find_by_email(new_email).await?.is_some()
        {
            return Err(ApiError::Conflict(format!(
                "email '{}' already registered",
                new_email
            )));
        }
        user.email = Some(new_email.clone());
    }

    if let Some(new_password) = req.password {
        if new_password.is_empty() {
            return Err(ApiError::BadRequest("password cannot be empty".into()));
        }
        user.password_hash = Some(
            PasswordManager::hash(&new_password)
                .map_err(|e| ApiError::BadRequest(e.to_string()))?,
        );
    }

    if let Some(pk) = req.public_key {
        user.public_key = Some(pk);
    }

    if let Some(caps) = req.capabilities {
        user.capabilities = Some(caps);
    }

    let user = state.users.update(user).await?;

    state
        .events
        .append(DomainEvent::UserUpdated { user_id: user.id })
        .await
        .ok();

    Ok((StatusCode::OK, Json(serde_json::json!({ "user": user }))))
}

/// GET /api/v1/users/:id_or_username/artifacts — list all artifacts published by a user
pub async fn list_user_artifacts(
    Path(id_or_username): Path<String>,
    Query(params): Query<UserArtifactsQuery>,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    use agentverse_core::artifact::ArtifactKind;

    // Resolve user to get their UUID
    let user = if let Ok(id) = id_or_username.parse::<Uuid>() {
        state
            .users
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("user {id_or_username}")))?
    } else {
        state
            .users
            .find_by_username(&id_or_username)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("user {id_or_username}")))?
    };

    // Parse optional kind filter
    let kind = match params.kind.as_deref() {
        Some("skill") => Some(ArtifactKind::Skill),
        Some("soul") => Some(ArtifactKind::Soul),
        Some("agent") => Some(ArtifactKind::Agent),
        Some("workflow") => Some(ArtifactKind::Workflow),
        Some("prompt") => Some(ArtifactKind::Prompt),
        Some(other) => return Err(ApiError::BadRequest(format!("unknown kind: {other}"))),
        None => None,
    };

    let filter = ArtifactFilter {
        author_id: Some(user.id),
        kind,
        limit: params.limit,
        offset: params.offset,
        ..Default::default()
    };

    let items = state.artifacts.list(filter).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "user_id":  user.id,
            "username": user.username,
            "items":    items,
            "total":    items.len(),
        })),
    ))
}
