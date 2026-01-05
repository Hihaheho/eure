//! HTTP fetch with caching support.

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH};
use sha2::{Digest, Sha256};
use url::Url;

use super::error::CacheError;
use super::meta::CacheMeta;
use super::storage::{CacheEntry, CacheStorage, FsStorage};

/// Options for cache operations.
#[derive(Debug, Clone)]
pub struct CacheOptions {
    /// Custom cache directory (overrides default)
    pub cache_dir: Option<std::path::PathBuf>,
    /// Offline mode - only use cache, never fetch
    pub offline: bool,
    /// Force refresh - ignore cache, always fetch
    pub refresh: bool,
    /// Max age for cached entries before revalidation
    pub max_age: Option<Duration>,
    /// Allow non-HTTPS URLs
    pub allow_http: bool,
    /// Maximum file size (default: 8 MiB)
    pub max_file_size: u64,
    /// Request timeout (default: 30s)
    pub timeout: Duration,
}

impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            cache_dir: None,
            offline: false,
            refresh: false,
            max_age: None,
            allow_http: false,
            max_file_size: 8 * 1024 * 1024, // 8 MiB
            timeout: Duration::from_secs(30),
        }
    }
}

/// Result of a fetch operation.
#[derive(Debug)]
pub struct FetchResult {
    /// The fetched content
    pub content: String,
    /// Whether the content came from cache
    pub from_cache: bool,
    /// Cache entry path (if cached)
    pub cache_path: Option<std::path::PathBuf>,
}

/// Get the default cache directory.
pub fn default_cache_dir() -> std::path::PathBuf {
    std::env::var("EURE_CACHE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            directories::ProjectDirs::from("dev", "eure", "eure")
                .map(|p| p.cache_dir().to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from(".cache/eure"))
                .join("schemas")
        })
}

/// Fetch a URL with caching.
pub fn fetch(url: &Url, opts: &CacheOptions) -> Result<FetchResult, CacheError> {
    // Check HTTPS requirement
    if url.scheme() == "http" && !opts.allow_http {
        return Err(CacheError::HttpsRequired(url.to_string()));
    }

    let cache_dir = opts.cache_dir.clone().unwrap_or_else(default_cache_dir);
    let storage = FsStorage::new(cache_dir);

    // Check cache first (unless refresh is set)
    if !opts.refresh {
        if let Some(entry) = storage.get(url)? {
            // Check if cache is still fresh
            if let Some(max_age) = opts.max_age {
                let age = chrono::Utc::now()
                    .signed_duration_since(entry.meta.last_used_at)
                    .to_std()
                    .unwrap_or(Duration::MAX);

                if age < max_age {
                    // Cache is fresh, use it without revalidation
                    storage.update_last_used(url)?;
                    return Ok(FetchResult {
                        content: entry.content,
                        from_cache: true,
                        cache_path: Some(entry.path),
                    });
                }
            }

            // Try conditional GET if we have ETag or Last-Modified
            if entry.meta.etag.is_some() || entry.meta.last_modified.is_some() {
                match try_revalidate(url, &entry, opts) {
                    Ok(RevalidateResult::NotModified) => {
                        storage.update_last_used(url)?;
                        return Ok(FetchResult {
                            content: entry.content,
                            from_cache: true,
                            cache_path: Some(entry.path),
                        });
                    }
                    Ok(RevalidateResult::Modified(content, meta)) => {
                        let path = storage.put(url, content.as_bytes(), &meta)?;
                        return Ok(FetchResult {
                            content,
                            from_cache: false,
                            cache_path: Some(path),
                        });
                    }
                    Err(_) if opts.offline => {
                        // In offline mode, use stale cache on network error
                        storage.update_last_used(url)?;
                        return Ok(FetchResult {
                            content: entry.content,
                            from_cache: true,
                            cache_path: Some(entry.path),
                        });
                    }
                    Err(e) => return Err(e),
                }
            }

            // No conditional headers, but in offline mode - use cache
            if opts.offline {
                storage.update_last_used(url)?;
                return Ok(FetchResult {
                    content: entry.content,
                    from_cache: true,
                    cache_path: Some(entry.path),
                });
            }
        } else if opts.offline {
            // No cache and offline mode
            return Err(CacheError::OfflineCacheMiss(url.to_string()));
        }
    } else if opts.offline {
        // Refresh requested but offline mode
        return Err(CacheError::OfflineCacheMiss(url.to_string()));
    }

    // Fetch fresh content
    let (content, meta) = fetch_fresh(url, opts)?;
    let path = storage.put(url, content.as_bytes(), &meta)?;

    Ok(FetchResult {
        content,
        from_cache: false,
        cache_path: Some(path),
    })
}

enum RevalidateResult {
    NotModified,
    Modified(String, CacheMeta),
}

fn try_revalidate(
    url: &Url,
    entry: &CacheEntry,
    opts: &CacheOptions,
) -> Result<RevalidateResult, CacheError> {
    let client = Client::builder().timeout(opts.timeout).build()?;

    let mut headers = HeaderMap::new();
    if let Some(ref etag) = entry.meta.etag
        && let Ok(value) = HeaderValue::from_str(etag)
    {
        headers.insert(IF_NONE_MATCH, value);
    }
    if let Some(ref last_modified) = entry.meta.last_modified
        && let Ok(value) = HeaderValue::from_str(last_modified)
    {
        headers.insert(IF_MODIFIED_SINCE, value);
    }

    let response = client.get(url.as_str()).headers(headers).send()?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(RevalidateResult::NotModified);
    }

    if !response.status().is_success() {
        return Err(CacheError::Http(response.error_for_status().unwrap_err()));
    }

    // Check content length
    if let Some(content_length) = response.content_length()
        && content_length > opts.max_file_size
    {
        return Err(CacheError::FileTooLarge {
            size: content_length,
            limit: opts.max_file_size,
        });
    }

    // Extract headers before consuming response
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let content = response.text()?;

    // Compute content hash
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash_bytes = hasher.finalize();
    let content_sha256 = hex::encode(&hash_bytes);

    let meta = CacheMeta::new(
        url.to_string(),
        etag,
        last_modified,
        content_sha256,
        content.len() as u64,
    );

    Ok(RevalidateResult::Modified(content, meta))
}

fn fetch_fresh(url: &Url, opts: &CacheOptions) -> Result<(String, CacheMeta), CacheError> {
    let client = Client::builder()
        .timeout(opts.timeout)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let response = client.get(url.as_str()).send()?;

    if !response.status().is_success() {
        return Err(CacheError::Http(response.error_for_status().unwrap_err()));
    }

    // Check content length
    if let Some(content_length) = response.content_length()
        && content_length > opts.max_file_size
    {
        return Err(CacheError::FileTooLarge {
            size: content_length,
            limit: opts.max_file_size,
        });
    }

    // Extract headers before consuming response
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let content = response.text()?;

    // Verify size limit for streaming responses
    if content.len() as u64 > opts.max_file_size {
        return Err(CacheError::FileTooLarge {
            size: content.len() as u64,
            limit: opts.max_file_size,
        });
    }

    // Compute content hash
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash_bytes = hasher.finalize();
    let content_sha256 = hex::encode(&hash_bytes);

    let meta = CacheMeta::new(
        url.to_string(),
        etag,
        last_modified,
        content_sha256,
        content.len() as u64,
    );

    Ok((content, meta))
}

// Simple hex encoding
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: &[u8]) -> String {
        let mut result = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            result.push(HEX_CHARS[(byte >> 4) as usize] as char);
            result.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
        }
        result
    }
}
