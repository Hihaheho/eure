use url::Url;

use crate::query::TextFile;

/// System/environment errors from query execution.
///
/// These are errors related to the query infrastructure, not user source code.
/// User errors (parse errors, validation errors, etc.) are represented by their
/// concrete types and converted to `ErrorReports` at the presentation layer.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EureQueryError {
    #[error("Content not found: {0}")]
    ContentNotFound(TextFile),
    #[error("Rate limit exceeded for URL {0}: possible infinite invalidation loop detected")]
    RateLimitExceeded(Url),
    #[error("Offline mode: no cached version available for {0}")]
    OfflineNoCache(Url),
    #[error(
        "Remote host not allowed: {host} (URL: {url}). If you trust this host, add it to security.allowed-hosts in Eure.eure"
    )]
    HostNotAllowed { url: Url, host: String },
    #[error("Invalid URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },
}

/// Generic error wrapper with associated file information.
///
/// This wraps any error type with the `TextFile` where the error originated.
/// This allows error reporting to use the correct file location even when
/// the error propagates through nested queries.
///
/// Used for:
/// - `FileError<ConversionError>` - schema conversion errors
/// - `FileError<ConfigError>` - config parsing errors
/// - `FileError<EureParseError>` - parse errors (if needed)
#[derive(Debug, thiserror::Error)]
#[error("{kind}")]
pub struct FileError<T: std::error::Error> {
    /// The file where the error occurred.
    pub file: TextFile,
    /// The underlying error.
    pub kind: T,
}

impl<T: std::error::Error + Clone> Clone for FileError<T> {
    fn clone(&self) -> Self {
        Self {
            file: self.file.clone(),
            kind: self.kind.clone(),
        }
    }
}
