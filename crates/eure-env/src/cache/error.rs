//! Cache error types.

use std::io;

/// Errors that can occur during cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// URL is not HTTPS and --allow-http was not specified
    #[error("HTTPS required: {0}")]
    HttpsRequired(String),

    /// File size exceeds limit
    #[error("File size exceeds limit: {size} > {limit}")]
    FileTooLarge { size: u64, limit: u64 },

    /// Too many redirects
    #[error("Too many redirects (max: {max})")]
    TooManyRedirects { max: usize },

    /// Cache entry not found
    #[error("Cache entry not found: {0}")]
    NotFound(String),

    /// Offline mode and cache miss
    #[error("Offline mode: cache miss for {0}")]
    OfflineCacheMiss(String),

    /// Tempfile persist error
    #[error("Failed to persist temp file: {0}")]
    TempfilePersist(#[from] tempfile::PersistError),
}
