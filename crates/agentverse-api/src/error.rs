use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use agentverse_core::CoreError;
use thiserror::Error;

/// API-level error that maps to HTTP responses.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::NotFound(msg) => ApiError::NotFound(msg),
            CoreError::AlreadyExists { namespace, name } => {
                ApiError::Conflict(format!("{namespace}/{name} already exists"))
            }
            CoreError::PermissionDenied { action, .. } => ApiError::Forbidden(action),
            CoreError::Validation(msg) | CoreError::InvalidManifest(msg) => {
                ApiError::BadRequest(msg)
            }
            CoreError::InvalidStatus(s) => ApiError::BadRequest(format!("status is {s}")),
            other => ApiError::Internal(anyhow::anyhow!("{other}")),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".into()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            ApiError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".into(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

