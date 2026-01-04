//! Cache path computation.

use sha2::{Digest, Sha256};
use std::path::PathBuf;
use url::Url;

/// Information about a cache key derived from a URL.
#[derive(Debug, Clone)]
pub struct CacheKeyInfo {
    /// Original URL
    pub url: String,
    /// SHA256 hash prefix (8 characters)
    pub hash: String,
    /// Host name from URL
    pub host: String,
    /// Original filename from URL path
    pub filename: String,
    /// Relative path within cache directory
    pub cache_path: String,
}

/// Compute cache key information from a URL.
///
/// The cache path uses 2-level directory sharding to prevent directory overcrowding:
/// `{host}/{hash[0:2]}/{hash[2:4]}/{hash}-{filename}`
///
/// Example:
/// - URL: `https://eure.dev/v0.1.0/schemas/eure-schema.schema.eure`
/// - Path: `eure.dev/a1/b2/a1b2c3d4-eure-schema.schema.eure`
pub fn compute_cache_key(url: &Url) -> CacheKeyInfo {
    let url_str = url.as_str();

    // Compute SHA256 hash of the full URL
    let mut hasher = Sha256::new();
    hasher.update(url_str.as_bytes());
    let hash_bytes = hasher.finalize();
    let hash = hex::encode(&hash_bytes[..4]); // First 8 hex characters (4 bytes)

    // Extract host
    let host = url.host_str().unwrap_or("unknown").to_string();

    // Extract filename from path
    let path = url.path();
    let filename = path
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("index")
        .to_string();

    // Build cache path with 2-level sharding
    let cache_path = format!(
        "{}/{}/{}/{}-{}",
        host,
        &hash[0..2],
        &hash[2..4],
        hash,
        filename
    );

    CacheKeyInfo {
        url: url_str.to_string(),
        hash,
        host,
        filename,
        cache_path,
    }
}

/// Convert URL to full cache file path.
pub fn url_to_cache_path(url: &Url, cache_dir: &std::path::Path) -> PathBuf {
    let key_info = compute_cache_key(url);
    cache_dir.join(&key_info.cache_path)
}

/// Get the meta file path for a cache file.
pub fn meta_path(cache_path: &std::path::Path) -> PathBuf {
    let mut meta = cache_path.as_os_str().to_owned();
    meta.push(".meta");
    PathBuf::from(meta)
}

/// Get the lock file path for a cache file.
pub fn lock_path(cache_path: &std::path::Path) -> PathBuf {
    let mut lock = cache_path.as_os_str().to_owned();
    lock.push(".lock");
    PathBuf::from(lock)
}

/// Compute SHA256 hash of content and return as hex string.
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash_bytes = hasher.finalize();
    hex::encode(&hash_bytes)
}

// Use hex crate for encoding
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cache_key() {
        let url = Url::parse("https://eure.dev/v0.1.0/schemas/eure-schema.schema.eure").unwrap();
        let key = compute_cache_key(&url);

        assert_eq!(key.host, "eure.dev");
        assert_eq!(key.filename, "eure-schema.schema.eure");
        assert_eq!(key.hash.len(), 8);
        assert!(key.cache_path.starts_with("eure.dev/"));
        assert!(key.cache_path.contains(&key.hash));
    }

    #[test]
    fn test_compute_cache_key_root_path() {
        let url = Url::parse("https://example.com/").unwrap();
        let key = compute_cache_key(&url);

        assert_eq!(key.host, "example.com");
        assert_eq!(key.filename, "index");
    }

    #[test]
    fn test_meta_path() {
        let cache_path = PathBuf::from("/cache/eure.dev/a1/b2/a1b2c3d4-schema.eure");
        let meta = meta_path(&cache_path);
        assert_eq!(
            meta,
            PathBuf::from("/cache/eure.dev/a1/b2/a1b2c3d4-schema.eure.meta")
        );
    }

    #[test]
    fn test_lock_path() {
        let cache_path = PathBuf::from("/cache/eure.dev/a1/b2/a1b2c3d4-schema.eure");
        let lock = lock_path(&cache_path);
        assert_eq!(
            lock,
            PathBuf::from("/cache/eure.dev/a1/b2/a1b2c3d4-schema.eure.lock")
        );
    }

    #[test]
    fn test_compute_content_hash() {
        // SHA256 of empty string
        let hash = compute_content_hash("");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // SHA256 of "hello"
        let hash = compute_content_hash("hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_url_to_cache_path() {
        let url = Url::parse("https://eure.dev/schema.eure").unwrap();
        let cache_dir = PathBuf::from("/home/user/.cache/eure/schemas");
        let path = url_to_cache_path(&url, &cache_dir);

        assert!(path.starts_with(&cache_dir));
        assert!(path.to_string_lossy().contains("eure.dev"));
    }
}
