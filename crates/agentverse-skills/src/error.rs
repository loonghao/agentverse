//! Error types for the agentverse-skills crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkillError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip extraction error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("unsupported archive format: {0}")]
    UnsupportedFormat(String),

    #[error("backend error: {0}")]
    Backend(String),

    #[error("deploy error: {0}")]
    Deploy(String),

    #[error("hook error: {0}")]
    Hook(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

