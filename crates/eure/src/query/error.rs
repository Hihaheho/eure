use eure_parol::EureParseError;
use url::Url;

use crate::query::TextFile;

#[derive(Debug, Clone, thiserror::Error)]
pub enum EureQueryError {
    #[error("Content not found: {0}")]
    ContentNotFound(TextFile),
    #[error("Parse error: {0}")]
    ParseError(EureParseError),
    #[error("Rate limit exceeded for URL {0}: possible infinite invalidation loop detected")]
    RateLimitExceeded(Url),
    #[error("Offline mode: no cached version available for {0}")]
    OfflineNoCache(Url),
    #[error(
        "Remote host not allowed: {host} (URL: {url}). Add the host to security.allowed-hosts in Eure.eure"
    )]
    HostNotAllowed { url: Url, host: String },
}
