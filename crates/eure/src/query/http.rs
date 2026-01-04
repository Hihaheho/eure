//! HTTP utilities for fetching remote files.

use std::sync::LazyLock;
use std::time::Duration;

use url::Url;

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
pub fn fetch_url(url: &Url) -> Result<TextFileContent, reqwest::Error> {
    let response = HTTP_CLIENT.get(url.as_str()).send()?;
    let status = response.status();

    if status.is_success() {
        let text = response.text()?;
        Ok(TextFileContent::Content(text))
    } else if status == reqwest::StatusCode::NOT_FOUND {
        Ok(TextFileContent::NotFound)
    } else {
        // Convert non-404 HTTP errors to reqwest::Error
        response.error_for_status()?;
        unreachable!()
    }
}
