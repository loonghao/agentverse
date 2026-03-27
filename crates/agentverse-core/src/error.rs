use thiserror::Error;
use uuid::Uuid;

/// Core domain errors for agentverse.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("artifact not found: {0}")]
    NotFound(String),

    #[error("artifact already exists: {namespace}/{name}")]
    AlreadyExists { namespace: String, name: String },

    #[error("version conflict: {current} cannot be bumped to {requested}")]
    VersionConflict { current: String, requested: String },

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("permission denied: user {user_id} cannot {action} artifact {artifact_id}")]
    PermissionDenied {
        user_id: Uuid,
        action: String,
        artifact_id: Uuid,
    },

    #[error("artifact is {0} and cannot be modified")]
    InvalidStatus(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("internal error: {0}")]
    Internal(String),
}

/// Opaque storage error forwarded from the storage layer.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct StorageError(pub String);
