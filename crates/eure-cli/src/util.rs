use clap::ValueEnum;
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
