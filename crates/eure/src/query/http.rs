//! HTTP utilities for fetching remote files.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use query_flow::QueryError;
use url::Url;

use crate::query::TextFile;
use crate::query::error::EureQueryError;

use super::TextFileContent;

/// Rate limit: minimum interval between requests to the same URL.
const RATE_LIMIT_INTERVAL: Duration = Duration::from_secs(10);

/// Tracks the last request time for each URL to detect infinite invalidation loops.
static LAST_REQUEST_TIME: LazyLock<Mutex<HashMap<Url, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

/// Check rate limit and update last request time.
/// Returns an error if the same URL was requested within the rate limit interval.
fn check_rate_limit(url: &Url, now: Instant) -> Result<(), QueryError> {
    let mut last_times = LAST_REQUEST_TIME.lock().unwrap();

    if let Some(last_time) = last_times.get(url)
        && now.duration_since(*last_time) < RATE_LIMIT_INTERVAL
    {
        return Err(EureQueryError::RateLimitExceeded(url.clone()).into());
    }

    last_times.insert(url.clone(), now);
    Ok(())
}

/// Fetch content from a URL.
///
/// Returns:
/// - `Ok(Content(text))` on success
/// - `Ok(NotFound)` for HTTP 404
/// - `Err(reqwest::Error)` for network/HTTP errors (to be converted to UserError)
/// - `Err(RateLimitExceeded)` if the same URL is requested too frequently
pub fn fetch_url(url: &Url) -> Result<TextFileContent, QueryError> {
    check_rate_limit(url, Instant::now())?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_rate_limit() {
        LAST_REQUEST_TIME.lock().unwrap().clear();
    }

    #[test]
    fn test_rate_limit_first_request_succeeds() {
        reset_rate_limit();
        let url = Url::parse("https://example.com/schema.eure").unwrap();
        let now = Instant::now();

        assert!(check_rate_limit(&url, now).is_ok());
    }

    #[test]
    fn test_rate_limit_second_request_within_interval_fails() {
        reset_rate_limit();
        let url = Url::parse("https://example.com/schema2.eure").unwrap();
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(5); // within 15s interval

        check_rate_limit(&url, t0).unwrap();
        assert!(check_rate_limit(&url, t1).is_err());
    }

    #[test]
    fn test_rate_limit_request_after_interval_succeeds() {
        reset_rate_limit();
        let url = Url::parse("https://example.com/schema3.eure").unwrap();
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(20); // after 15s interval

        check_rate_limit(&url, t0).unwrap();
        assert!(check_rate_limit(&url, t1).is_ok());
    }

    #[test]
    fn test_rate_limit_different_urls_independent() {
        reset_rate_limit();
        let url1 = Url::parse("https://example.com/a.eure").unwrap();
        let url2 = Url::parse("https://example.com/b.eure").unwrap();
        let now = Instant::now();

        check_rate_limit(&url1, now).unwrap();
        assert!(check_rate_limit(&url2, now).is_ok());
    }
}
