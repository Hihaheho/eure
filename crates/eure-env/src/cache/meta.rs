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
