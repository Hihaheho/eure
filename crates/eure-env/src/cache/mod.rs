//! Remote schema caching module.
//!
//! This module provides caching for remote schema files fetched over HTTP(S).
//!
//! # Module Organization
//!
//! - **Core types** (always available): `CacheMeta`, `CacheKeyInfo`, path computation
//! - **Native I/O** (requires `native` feature): `fetch`, `FsStorage`, `gc`
//!
//! # Cache Layout
//!
//! The cache is stored in the platform-specific cache directory (via `directories` crate):
//! - macOS: `~/Library/Caches/dev.eure.eure/schemas/`
//! - Linux: `~/.cache/eure/schemas/`
//! - Windows: `C:\Users\<User>\AppData\Local\eure\eure\cache\schemas\`
//!
//! Override with `$EURE_CACHE_DIR` environment variable.
//!
//! Files are organized with 2-level directory sharding to prevent overcrowding:
//!
//! ```text
//! ~/Library/Caches/dev.eure.eure/schemas/  # (macOS example)
//!   eure.dev/
//!     a1/
//!       b2/
//!         a1b2c3d4-eure-schema.schema.eure       # content
//!         a1b2c3d4-eure-schema.schema.eure.meta  # metadata (JSON)
//! ```
//!
//! # Example (native only)
//!
//! ```no_run
//! use url::Url;
//! use eure_env::cache::{fetch, CacheOptions};
//!
//! let url = Url::parse("https://eure.dev/v0.1.0/schemas/eure-schema.schema.eure").unwrap();
//! let result = fetch(&url, &CacheOptions::default()).unwrap();
//! println!("Content: {}", result.content);
//! println!("From cache: {}", result.from_cache);
//! ```

// Core types (pure computation, always available)
mod meta;
mod path;

pub use meta::{CacheAction, CacheMeta, ConditionalHeaders};
pub use path::{
    CacheKeyInfo, compute_cache_key, compute_content_hash, lock_path, meta_path, url_to_cache_path,
};

// Native I/O (requires filesystem and network)
#[cfg(feature = "native")]
mod error;
#[cfg(feature = "native")]
mod fetch;
#[cfg(feature = "native")]
mod gc;
#[cfg(feature = "native")]
mod storage;

#[cfg(feature = "native")]
pub use error::CacheError;
#[cfg(feature = "native")]
pub use fetch::{CacheOptions, FetchResult, default_cache_dir, fetch};
#[cfg(feature = "native")]
pub use gc::{clean, clean_with_dir, gc, gc_with_dir, parse_duration, parse_size};
#[cfg(feature = "native")]
pub use storage::{CacheEntry, CacheStorage, FsStorage, GcOptions, GcStats};
