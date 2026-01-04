//! Cache metadata types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata for a cached file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMeta {
    /// Original URL
    pub url: String,
    /// When the file was fetched
    pub fetched_at: DateTime<Utc>,
    /// When the file was last used
    pub last_used_at: DateTime<Utc>,
    /// HTTP ETag header (for conditional GET)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// HTTP Last-Modified header (for conditional GET)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    /// SHA256 hash of content
    pub content_sha256: String,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Result of checking cache freshness.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action")]
pub enum CacheAction {
    /// No cache exists, fetch fresh.
    #[serde(rename = "fetch")]
    Fetch,
    /// Cache is fresh, use it directly.
    #[serde(rename = "use_cached")]
    UseCached,
    /// Cache is stale, revalidate with conditional headers.
    #[serde(rename = "revalidate")]
    Revalidate {
        /// Headers to send for conditional GET.
        headers: ConditionalHeaders,
    },
}

/// Conditional GET headers.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConditionalHeaders {
    /// If-None-Match header value (ETag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_none_match: Option<String>,
    /// If-Modified-Since header value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_modified_since: Option<String>,
}

impl CacheMeta {
    /// Create a new CacheMeta with current timestamp.
    pub fn new(
        url: String,
        etag: Option<String>,
        last_modified: Option<String>,
        content_sha256: String,
        size_bytes: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            url,
            fetched_at: now,
            last_used_at: now,
            etag,
            last_modified,
            content_sha256,
            size_bytes,
        }
    }

    /// Update last_used_at to current time.
    pub fn touch(&mut self) {
        self.last_used_at = Utc::now();
    }

    /// Check if this cache entry is fresh based on max_age.
    ///
    /// Returns the appropriate action to take.
    pub fn check_freshness(&self, max_age_secs: u32) -> CacheAction {
        let now = Utc::now();
        let age = now.signed_duration_since(self.last_used_at);
        let max_age = chrono::TimeDelta::seconds(max_age_secs as i64);

        if age < max_age {
            return CacheAction::UseCached;
        }

        // Cache is stale, need revalidation
        if self.etag.is_some() || self.last_modified.is_some() {
            CacheAction::Revalidate {
                headers: ConditionalHeaders {
                    if_none_match: self.etag.clone(),
                    if_modified_since: self.last_modified.clone(),
                },
            }
        } else {
            // No conditional headers available, fetch fresh
            CacheAction::Fetch
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_meta_new() {
        let meta = CacheMeta::new(
            "https://example.com/schema.eure".to_string(),
            Some("\"abc123\"".to_string()),
            Some("Mon, 01 Jan 2024 00:00:00 GMT".to_string()),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            1024,
        );

        assert_eq!(meta.url, "https://example.com/schema.eure");
        assert_eq!(meta.etag, Some("\"abc123\"".to_string()));
        assert_eq!(
            meta.last_modified,
            Some("Mon, 01 Jan 2024 00:00:00 GMT".to_string())
        );
        assert_eq!(meta.size_bytes, 1024);
        // fetched_at and last_used_at should be the same initially
        assert_eq!(meta.fetched_at, meta.last_used_at);
    }

    #[test]
    fn test_cache_meta_serde_roundtrip() {
        let meta = CacheMeta::new(
            "https://example.com/schema.eure".to_string(),
            Some("\"abc123\"".to_string()),
            None,
            "deadbeef".to_string(),
            512,
        );

        let json = serde_json::to_string(&meta).unwrap();
        let parsed: CacheMeta = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.url, meta.url);
        assert_eq!(parsed.etag, meta.etag);
        assert_eq!(parsed.last_modified, meta.last_modified);
        assert_eq!(parsed.content_sha256, meta.content_sha256);
        assert_eq!(parsed.size_bytes, meta.size_bytes);
    }

    #[test]
    fn test_check_freshness_fresh() {
        let meta = CacheMeta::new(
            "https://example.com/schema.eure".to_string(),
            None,
            None,
            "hash".to_string(),
            100,
        );

        // With a large max_age, cache should be fresh
        match meta.check_freshness(3600) {
            CacheAction::UseCached => {}
            other => panic!("Expected UseCached, got {:?}", other),
        }
    }

    #[test]
    fn test_check_freshness_stale_with_etag() {
        let mut meta = CacheMeta::new(
            "https://example.com/schema.eure".to_string(),
            Some("\"abc123\"".to_string()),
            None,
            "hash".to_string(),
            100,
        );

        // Set last_used_at to the past
        meta.last_used_at = Utc::now() - chrono::TimeDelta::hours(2);

        // With a small max_age, cache should need revalidation
        match meta.check_freshness(60) {
            CacheAction::Revalidate { headers } => {
                assert_eq!(headers.if_none_match, Some("\"abc123\"".to_string()));
                assert_eq!(headers.if_modified_since, None);
            }
            other => panic!("Expected Revalidate, got {:?}", other),
        }
    }

    #[test]
    fn test_check_freshness_stale_no_headers() {
        let mut meta = CacheMeta::new(
            "https://example.com/schema.eure".to_string(),
            None,
            None,
            "hash".to_string(),
            100,
        );

        // Set last_used_at to the past
        meta.last_used_at = Utc::now() - chrono::TimeDelta::hours(2);

        // Without conditional headers, should fetch fresh
        match meta.check_freshness(60) {
            CacheAction::Fetch => {}
            other => panic!("Expected Fetch, got {:?}", other),
        }
    }
}
