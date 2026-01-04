//! HTTP utilities for fetching remote files.

use std::sync::LazyLock;
use std::time::Duration;

use query_flow::QueryError;
use url::Url;

use crate::query::TextFile;
use crate::query::error::EureQueryError;

use super::TextFileContent;

/// HTTP client with connection pooling and appropriate timeouts.
///
/// - Connect timeout: 10 seconds
/// - Request timeout: 30 seconds
/// - Pool idle timeout: 30 seconds (short since schema fetches are infrequent)
static HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .pool_idle_timeout(Duration::from_secs(30))
        .user_agent(format!(
            "{}@{}",
            option_env!("CARGO_BIN_NAME").unwrap_or("eure"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("Failed to create HTTP client")
});

/// Fetch content from a URL.
///
/// Returns:
/// - `Ok(Content(text))` on success
/// - `Ok(NotFound)` for HTTP 404
/// - `Err(reqwest::Error)` for network/HTTP errors (to be converted to UserError)
pub fn fetch_url(url: &Url) -> Result<TextFileContent, QueryError> {
    let response = HTTP_CLIENT.get(url.as_str()).send()?;
    let status = response.status();

    if status.is_success() {
        let text = response.text()?;
        Ok(TextFileContent(text))
    } else if status == reqwest::StatusCode::NOT_FOUND {
        Err(EureQueryError::ContentNotFound(TextFile::from_url(url.clone())).into())
    } else {
        // Convert non-404 HTTP errors to reqwest::Error
        response.error_for_status()?;
        unreachable!()
    }
}
