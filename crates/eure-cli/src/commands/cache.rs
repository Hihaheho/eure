//! Cache management commands.

use std::path::PathBuf;

use clap::Subcommand;
use eure_env::cache::{
    self, CacheOptions, CacheStorage, FsStorage, GcOptions, clean, clean_with_dir,
    default_cache_dir, gc, gc_with_dir, parse_duration, parse_size,
};
use url::Url;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: CacheCommand,
}

#[derive(Subcommand)]
enum CacheCommand {
    /// List cached entries
    List {
        /// Custom cache directory
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    /// Run garbage collection
    Gc {
        /// Remove entries older than this duration (e.g., "30d", "7d", "24h")
        #[arg(long, default_value = "30d")]
        older_than: Option<String>,
        /// Keep total cache size under this limit (e.g., "512MiB", "1GiB")
        #[arg(long)]
        max_size: Option<String>,
        /// Custom cache directory
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    /// Remove all cached entries
    Clean {
        /// Custom cache directory
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    /// Show cache directory path
    Path,
    /// Fetch a URL and cache it
    Fetch {
        /// URL to fetch
        url: String,
        /// Force refresh (ignore cache)
        #[arg(long)]
        refresh: bool,
        /// Custom cache directory
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
}

pub fn run(args: Args) {
    match args.command {
        CacheCommand::List { cache_dir } => run_list(cache_dir),
        CacheCommand::Gc {
            older_than,
            max_size,
            cache_dir,
        } => run_gc(older_than, max_size, cache_dir),
        CacheCommand::Clean { cache_dir } => run_clean(cache_dir),
        CacheCommand::Path => run_path(),
        CacheCommand::Fetch {
            url,
            refresh,
            cache_dir,
        } => run_fetch(url, refresh, cache_dir),
    }
}

fn run_list(cache_dir: Option<PathBuf>) {
    let dir = cache_dir.unwrap_or_else(default_cache_dir);
    let storage = FsStorage::new(dir);

    match storage.list() {
        Ok(entries) => {
            if entries.is_empty() {
                println!("Cache is empty.");
                return;
            }

            println!(
                "{:<60} {:>10} {:>20}",
                "URL", "SIZE", "LAST USED"
            );
            println!("{}", "-".repeat(92));

            let mut total_size: u64 = 0;
            for entry in &entries {
                let url = if entry.meta.url.len() > 58 {
                    format!("{}...", &entry.meta.url[..55])
                } else {
                    entry.meta.url.clone()
                };
                let size = format_size(entry.meta.size_bytes);
                let last_used = entry.meta.last_used_at.format("%Y-%m-%d %H:%M").to_string();
                println!("{:<60} {:>10} {:>20}", url, size, last_used);
                total_size += entry.meta.size_bytes;
            }

            println!("{}", "-".repeat(92));
            println!(
                "{} entries, {} total",
                entries.len(),
                format_size(total_size)
            );
        }
        Err(e) => {
            eprintln!("Error listing cache: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_gc(older_than: Option<String>, max_size: Option<String>, cache_dir: Option<PathBuf>) {
    let opts = GcOptions {
        older_than: older_than.and_then(|s| parse_duration(&s)),
        max_size: max_size.and_then(|s| parse_size(&s)),
        offline: false,
    };

    let result = match cache_dir {
        Some(dir) => gc_with_dir(&dir, &opts),
        None => gc(&opts),
    };

    match result {
        Ok(stats) => {
            println!(
                "Removed {} files ({}), kept {} files",
                stats.files_removed,
                format_size(stats.bytes_freed),
                stats.files_kept
            );
        }
        Err(e) => {
            eprintln!("Error running GC: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_clean(cache_dir: Option<PathBuf>) {
    let result = match cache_dir {
        Some(dir) => clean_with_dir(&dir),
        None => clean(),
    };

    match result {
        Ok(()) => {
            println!("Cache cleaned.");
        }
        Err(e) => {
            eprintln!("Error cleaning cache: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_path() {
    println!("{}", default_cache_dir().display());
}

fn run_fetch(url_str: String, refresh: bool, cache_dir: Option<PathBuf>) {
    let url = match Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Invalid URL: {}", e);
            std::process::exit(1);
        }
    };

    let opts = CacheOptions {
        cache_dir,
        refresh,
        ..Default::default()
    };

    match cache::fetch(&url, &opts) {
        Ok(result) => {
            if result.from_cache {
                println!("From cache: {}", result.cache_path.unwrap().display());
            } else {
                println!("Fetched and cached: {}", result.cache_path.unwrap().display());
            }
            println!("Content length: {} bytes", result.content.len());
        }
        Err(e) => {
            eprintln!("Error fetching: {}", e);
            std::process::exit(1);
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}
