use thiserror::Error;

/// Errors produced by object store operations.
#[derive(Debug, Error)]
pub enum ObjectStoreError {
    /// The requested object was not found in the store.
    #[error("object not found: {0}")]
    NotFound(String),

    /// An I/O error occurred (local disk backend).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An HTTP-level error from a remote backend.
    #[error("HTTP error: {0}")]
    Http(String),

    /// S3/COS/MinIO SDK error.
    #[error("S3 error: {0}")]
    S3(String),

    /// Backend configuration is invalid or incomplete.
    #[error("configuration error: {0}")]
    Config(String),

    /// The upload was rejected by the remote endpoint.
    #[error("upload rejected ({status}): {body}")]
    UploadRejected { status: u16, body: String },

    /// Generic internal error.
    #[error("internal object store error: {0}")]
    Internal(String),
}
