//! Shared argument definitions.

use std::path::PathBuf;

use eure::query::{CacheOptions, parse_duration};

/// Cache-related command-line arguments.
///
/// Can be embedded in other command Args using `#[command(flatten)]`.
#[derive(clap::Args, Clone, Default)]
pub struct CacheArgs {
    /// Offline mode: only use cached schemas, never fetch from network
    #[arg(long)]
    pub offline: bool,

    /// Force refresh: ignore cache and re-fetch remote schemas
    #[arg(long)]
    pub refresh: bool,

    /// Maximum cache age before revalidation (e.g., "24h", "7d")
    #[arg(long)]
    pub max_age: Option<String>,

    /// Custom cache directory for remote schemas
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,
}

impl CacheArgs {
    /// Build CacheOptions from command-line arguments.
    pub fn to_cache_options(&self) -> CacheOptions {
        CacheOptions {
            cache_dir: self.cache_dir.clone(),
            offline: self.offline,
            refresh: self.refresh,
            max_age: self.max_age.as_ref().and_then(|s| parse_duration(s)),
            ..Default::default()
        }
    }
}
