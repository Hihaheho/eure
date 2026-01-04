//! Cache storage abstraction.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fs2::FileExt;
use url::Url;

use super::error::CacheError;
use super::meta::CacheMeta;
use super::path::{lock_path, meta_path, url_to_cache_path};

/// A cached entry with content and metadata.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached content
    pub content: String,
    /// Cache metadata
    pub meta: CacheMeta,
    /// Path to the cached file
    pub path: PathBuf,
}

/// Statistics from a GC operation.
#[derive(Debug, Default)]
pub struct GcStats {
    /// Number of files removed
    pub files_removed: usize,
    /// Total bytes freed
    pub bytes_freed: u64,
    /// Number of files kept
    pub files_kept: usize,
}

/// Options for GC operation.
#[derive(Debug, Clone)]
pub struct GcOptions {
    /// Remove entries older than this duration
    pub older_than: Option<Duration>,
    /// Keep total cache size under this limit (in bytes)
    pub max_size: Option<u64>,
    /// Whether we're in offline mode (suppress GC)
    pub offline: bool,
}

impl Default for GcOptions {
    fn default() -> Self {
        Self {
            older_than: Some(Duration::from_secs(30 * 24 * 60 * 60)), // 30 days
            max_size: None,
            offline: false,
        }
    }
}

/// Cache storage trait for future extensibility (e.g., SQLite backend).
pub trait CacheStorage: Send + Sync {
    /// Get a cached entry by URL.
    fn get(&self, url: &Url) -> Result<Option<CacheEntry>, CacheError>;

    /// Store content with metadata.
    fn put(&self, url: &Url, content: &[u8], meta: &CacheMeta) -> Result<PathBuf, CacheError>;

    /// Update last_used_at timestamp.
    fn update_last_used(&self, url: &Url) -> Result<(), CacheError>;

    /// List all cached entries.
    fn list(&self) -> Result<Vec<CacheEntry>, CacheError>;

    /// Remove a cached entry.
    fn remove(&self, url: &Url) -> Result<(), CacheError>;

    /// Run garbage collection.
    fn gc(&self, opts: &GcOptions) -> Result<GcStats, CacheError>;

    /// Remove all cached entries.
    fn clean(&self) -> Result<(), CacheError>;
}

/// File-system based cache storage.
pub struct FsStorage {
    cache_dir: PathBuf,
}

impl FsStorage {
    /// Create a new FsStorage with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Acquire an exclusive lock on a cache entry.
    fn lock(&self, url: &Url) -> Result<FileLock, CacheError> {
        let cache_path = url_to_cache_path(url, &self.cache_dir);
        let lock_file_path = lock_path(&cache_path);

        // Ensure parent directory exists
        if let Some(parent) = lock_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let lock_file = File::create(&lock_file_path)?;

        // Try to acquire lock, with a message if waiting
        if lock_file.try_lock_exclusive().is_err() {
            eprintln!("Waiting for lock on schema cache...");
            lock_file.lock_exclusive()?;
        }

        Ok(FileLock {
            _file: lock_file,
            path: lock_file_path,
        })
    }

    /// Try to read a cache entry from a .meta file path.
    /// Returns None if the path is not a .meta file or if reading fails.
    fn try_read_cache_entry(path: &std::path::Path) -> Option<CacheEntry> {
        // Only process .meta files
        if path.extension().and_then(|e| e.to_str()) != Some("meta") {
            return None;
        }

        // Get the content file path
        let content_path = path.with_extension("");
        if !content_path.exists() {
            return None;
        }

        // Read meta and content files
        let meta_content = fs::read_to_string(path).ok()?;
        let meta = serde_json::from_str::<CacheMeta>(&meta_content).ok()?;
        let content = fs::read_to_string(&content_path).ok()?;

        Some(CacheEntry {
            content,
            meta,
            path: content_path,
        })
    }
}

/// RAII guard for file lock.
struct FileLock {
    _file: File,
    path: PathBuf,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Lock is automatically released when file is dropped
        // Optionally remove the lock file
        let _ = fs::remove_file(&self.path);
    }
}

impl CacheStorage for FsStorage {
    fn get(&self, url: &Url) -> Result<Option<CacheEntry>, CacheError> {
        let cache_path = url_to_cache_path(url, &self.cache_dir);
        let meta_file_path = meta_path(&cache_path);

        if !cache_path.exists() || !meta_file_path.exists() {
            return Ok(None);
        }

        // Read meta
        let meta_content = fs::read_to_string(&meta_file_path)?;
        let meta: CacheMeta = serde_json::from_str(&meta_content)?;

        // Read content
        let content = fs::read_to_string(&cache_path)?;

        Ok(Some(CacheEntry {
            content,
            meta,
            path: cache_path,
        }))
    }

    fn put(&self, url: &Url, content: &[u8], meta: &CacheMeta) -> Result<PathBuf, CacheError> {
        let _lock = self.lock(url)?;

        let cache_path = url_to_cache_path(url, &self.cache_dir);
        let meta_file_path = meta_path(&cache_path);

        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write content atomically using tempfile
        let dir = cache_path.parent().unwrap();
        let mut temp_content = tempfile::NamedTempFile::new_in(dir)?;
        temp_content.write_all(content)?;
        temp_content.persist(&cache_path)?;

        // Write meta atomically
        let mut temp_meta = tempfile::NamedTempFile::new_in(dir)?;
        serde_json::to_writer_pretty(&mut temp_meta, meta)?;
        temp_meta.persist(&meta_file_path)?;

        Ok(cache_path)
    }

    fn update_last_used(&self, url: &Url) -> Result<(), CacheError> {
        let cache_path = url_to_cache_path(url, &self.cache_dir);
        let meta_file_path = meta_path(&cache_path);

        if !meta_file_path.exists() {
            return Ok(());
        }

        // Read, update, write meta
        let meta_content = fs::read_to_string(&meta_file_path)?;
        let mut meta: CacheMeta = serde_json::from_str(&meta_content)?;
        meta.touch();

        let dir = meta_file_path.parent().unwrap();
        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
        serde_json::to_writer_pretty(&mut temp, &meta)?;
        temp.persist(&meta_file_path)?;

        Ok(())
    }

    fn list(&self) -> Result<Vec<CacheEntry>, CacheError> {
        let mut entries = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(entries);
        }

        for entry in walkdir::WalkDir::new(&self.cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Some(cache_entry) = Self::try_read_cache_entry(entry.path()) {
                entries.push(cache_entry);
            }
        }

        Ok(entries)
    }

    fn remove(&self, url: &Url) -> Result<(), CacheError> {
        let _lock = self.lock(url)?;

        let cache_path = url_to_cache_path(url, &self.cache_dir);
        let meta_file_path = meta_path(&cache_path);

        if cache_path.exists() {
            fs::remove_file(&cache_path)?;
        }
        if meta_file_path.exists() {
            fs::remove_file(&meta_file_path)?;
        }

        Ok(())
    }

    fn gc(&self, opts: &GcOptions) -> Result<GcStats, CacheError> {
        // Don't GC in offline mode
        if opts.offline {
            return Ok(GcStats::default());
        }

        let mut stats = GcStats::default();
        let mut entries = self.list()?;

        // Sort by last_used_at (oldest first)
        entries.sort_by(|a, b| a.meta.last_used_at.cmp(&b.meta.last_used_at));

        let now = chrono::Utc::now();

        // Calculate total size
        let mut total_size: u64 = entries.iter().map(|e| e.meta.size_bytes).sum();

        for entry in entries {
            let should_remove = if let Some(older_than) = opts.older_than {
                let age = now.signed_duration_since(entry.meta.last_used_at);
                age > chrono::Duration::from_std(older_than).unwrap_or(chrono::TimeDelta::MAX)
            } else if let Some(max_size) = opts.max_size {
                total_size > max_size
            } else {
                false
            };

            if should_remove {
                // Try to acquire lock without blocking
                let cache_path = &entry.path;
                let lock_file_path = lock_path(cache_path);

                if let Ok(lock_file) = File::create(&lock_file_path) {
                    if lock_file.try_lock_exclusive().is_ok() {
                        // Successfully locked, safe to remove
                        let meta_file_path = meta_path(cache_path);

                        if let Ok(()) = fs::remove_file(cache_path) {
                            let _ = fs::remove_file(&meta_file_path);
                            stats.files_removed += 1;
                            stats.bytes_freed += entry.meta.size_bytes;
                            total_size -= entry.meta.size_bytes;
                        }

                        // Lock is released when lock_file is dropped
                        let _ = fs::remove_file(&lock_file_path);
                    } else {
                        // Entry is in use, skip it
                        stats.files_kept += 1;
                    }
                } else {
                    stats.files_kept += 1;
                }
            } else {
                stats.files_kept += 1;
            }
        }

        Ok(stats)
    }

    fn clean(&self) -> Result<(), CacheError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }
}
