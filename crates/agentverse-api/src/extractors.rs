//! Axum extractors: authenticated user, optional auth, etc.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

/// Claims stored in the JWT access token (mirrors agentverse_auth::Claims).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub kind: String,
    pub exp: i64,
}

impl From<agentverse_auth::Claims> for Claims {
    fn from(c: agentverse_auth::Claims) -> Self {
        Self { sub: c.sub, username: c.username, kind: c.kind, exp: c.exp }
    }
}

/// Extractor that requires a valid JWT. Returns 401 if missing or invalid.
pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(&parts.headers)
            .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;
        let auth_claims = state.jwt
            .validate(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
        Ok(AuthUser(Claims::from(auth_claims)))
    }
}

/// Extractor that accepts both authenticated and anonymous callers.
pub struct OptionalAuthUser(pub Option<Claims>);

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let claims = extract_bearer(&parts.headers)
            .and_then(|token| state.jwt.validate(token).ok())
            .map(Claims::from);
        Ok(OptionalAuthUser(claims))
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

