//! Garbage collection for cache.

use std::time::Duration;

use super::error::CacheError;
use super::fetch::https_cache_dir;
use super::storage::{CacheStorage, FsStorage, GcOptions, GcStats};

/// Run garbage collection on the HTTPS cache.
///
/// This removes old entries based on the provided options.
pub fn gc(opts: &GcOptions) -> Result<GcStats, CacheError> {
    let cache_dir = https_cache_dir();
    let storage = FsStorage::new(cache_dir);
    storage.gc(opts)
}

/// Run garbage collection with a custom cache directory.
pub fn gc_with_dir(cache_dir: &std::path::Path, opts: &GcOptions) -> Result<GcStats, CacheError> {
    let storage = FsStorage::new(cache_dir.to_path_buf());
    storage.gc(opts)
}

/// Remove all cached entries from the HTTPS cache.
pub fn clean() -> Result<(), CacheError> {
    let cache_dir = https_cache_dir();
    let storage = FsStorage::new(cache_dir);
    storage.clean()
}

/// Remove all cached entries from a custom cache directory.
pub fn clean_with_dir(cache_dir: &std::path::Path) -> Result<(), CacheError> {
    let storage = FsStorage::new(cache_dir.to_path_buf());
    storage.clean()
}

/// Parse a duration string like "30d", "7d", "24h", "1w".
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_str, unit) = if let Some(stripped) = s.strip_suffix('d') {
        (stripped, "d")
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, "h")
    } else if let Some(stripped) = s.strip_suffix('w') {
        (stripped, "w")
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, "m")
    } else if let Some(stripped) = s.strip_suffix('s') {
        (stripped, "s")
    } else {
        // Assume days if no unit
        (s, "d")
    };

    let num: u64 = num_str.parse().ok()?;

    let secs = match unit {
        "s" => num,
        "m" => num * 60,
        "h" => num * 60 * 60,
        "d" => num * 24 * 60 * 60,
        "w" => num * 7 * 24 * 60 * 60,
        _ => return None,
    };

    Some(Duration::from_secs(secs))
}

/// Parse a size string like "512MiB", "1GiB", "100MB".
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Find where the number ends
    let num_end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
        .map(|(i, _)| i)
        .unwrap_or(s.len());

    let num_str = &s[..num_end];
    let unit = s[num_end..].trim();

    let num: f64 = num_str.parse().ok()?;

    let multiplier: u64 = match unit.to_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1000,
        "ki" | "kib" => 1024,
        "m" | "mb" => 1000 * 1000,
        "mi" | "mib" => 1024 * 1024,
        "g" | "gb" => 1000 * 1000 * 1000,
        "gi" | "gib" => 1024 * 1024 * 1024,
        _ => return None,
    };

    Some((num * multiplier as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(
            parse_duration("30d"),
            Some(Duration::from_secs(30 * 24 * 60 * 60))
        );
        assert_eq!(
            parse_duration("7d"),
            Some(Duration::from_secs(7 * 24 * 60 * 60))
        );
        assert_eq!(
            parse_duration("24h"),
            Some(Duration::from_secs(24 * 60 * 60))
        );
        assert_eq!(
            parse_duration("1w"),
            Some(Duration::from_secs(7 * 24 * 60 * 60))
        );
        assert_eq!(parse_duration("60m"), Some(Duration::from_secs(60 * 60)));
        assert_eq!(parse_duration("60s"), Some(Duration::from_secs(60)));
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("512MiB"), Some(512 * 1024 * 1024));
        assert_eq!(parse_size("1GiB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("100MB"), Some(100 * 1000 * 1000));
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size(""), None);
    }
}
