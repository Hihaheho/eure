use eure_parol::EureParseError;
use url::Url;

use crate::query::TextFile;

#[derive(Debug, thiserror::Error)]
pub enum EureQueryError {
    #[error("File not found: {0}")]
    FileNotFound(TextFile),
    #[error("Content not found: {0}")]
    ContentNotFound(TextFile),
    #[error("Parse error: {0}")]
    ParseError(EureParseError),
    #[error("Rate limit exceeded for URL {0}: possible infinite invalidation loop detected")]
    RateLimitExceeded(Url),
}
