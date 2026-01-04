use std::fs;
use std::io::{self, Read};
use std::sync::Arc;

use anyhow::anyhow;
use clap::ValueEnum;
use eure::query::{TextFile, TextFileContent, fetch_url};
use eure::query_flow::{DurabilityLevel, Query, QueryError, QueryRuntime};
use eure::report::{ErrorReports, format_error_reports};

/// Read input from file path or stdin.
/// - `None` or `Some("-")` reads from stdin
/// - `Some(path)` reads from file
pub fn read_input(file: Option<&str>) -> Result<String, String> {
    match file {
        None | Some("-") => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| format!("Error reading from stdin: {e}"))?;
            Ok(buffer)
        }
        Some(path) => fs::read_to_string(path).map_err(|e| format!("Error reading file: {e}")),
    }
}

/// Helper to get display path for error messages
pub fn display_path(file: Option<&str>) -> &str {
    file.unwrap_or("<stdin>")
}

/// Variant representation format for JSON conversion
#[derive(ValueEnum, Clone, Debug)]
pub enum VariantFormat {
    /// Default: {"variant-name": {...}}
    External,
    /// {"type": "variant-name", ...fields...}
    Internal,
    /// {"type": "variant-name", "content": {...}}
    Adjacent,
    /// Just the content without variant information
    Untagged,
}

/// Handle query errors by printing formatted error reports and exiting.
///
/// This function provides unified error handling for CLI commands:
/// - If the error contains `ErrorReports`, formats and prints them
/// - Otherwise prints the error message directly
pub fn handle_query_error(runtime: &QueryRuntime, e: QueryError) -> ! {
    if let Some(reports) = e.downcast_ref::<ErrorReports>() {
        eprintln!(
            "{}",
            format_error_reports(runtime, reports, true).expect("file content should be loaded")
        );
    } else {
        eprintln!("Error: {e}");
    }
    std::process::exit(1);
}

/// Run a query with automatic file loading on suspend.
///
/// This helper enables single-query patterns for CLI commands by automatically
/// loading files when the query suspends waiting for assets.
///
/// Uses query-flow's suspend/resume mechanism:
/// 1. Execute query
/// 2. If query suspends (waiting for assets), load pending files from disk
/// 3. Retry query until it completes or errors
///
/// Returns the Arc-wrapped query result.
pub fn run_query_with_file_loading<Q, R>(
    runtime: &QueryRuntime,
    query: Q,
) -> Result<Arc<R>, QueryError>
where
    Q: Query<Output = R> + Clone,
{
    loop {
        match runtime.query(query.clone()) {
            Ok(result) => return Ok(result),
            Err(QueryError::Suspend { .. }) => {
                // Load pending file assets from disk or network
                for pending in runtime.pending_assets() {
                    if let Some(file) = pending.key::<TextFile>() {
                        match file {
                            TextFile::Local(path) => {
                                let content = TextFileContent(fs::read_to_string(path.as_ref())?);
                                runtime.resolve_asset(
                                    file.clone(),
                                    content,
                                    DurabilityLevel::Static,
                                );
                            }
                            TextFile::Remote(url) => match fetch_url(url) {
                                Ok(content) => {
                                    runtime.resolve_asset(
                                        file.clone(),
                                        content,
                                        DurabilityLevel::Static,
                                    );
                                }
                                Err(e) => {
                                    runtime.resolve_asset_error::<TextFile>(
                                        file.clone(),
                                        anyhow!("Failed to fetch {}: {}", url, e),
                                        DurabilityLevel::Static,
                                    );
                                }
                            },
                        }
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }
}
