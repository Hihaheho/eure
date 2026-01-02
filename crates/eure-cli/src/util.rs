use clap::ValueEnum;
use eure::query_flow::{QueryError, QueryRuntime};
use eure::report::{ErrorReports, format_error_reports};
use std::fs;
use std::io::{self, Read};

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
