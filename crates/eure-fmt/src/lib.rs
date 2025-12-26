//! Eure formatter.
//!
//! This crate provides formatting functionality for Eure files, using an
//! IR-based architecture inspired by Wadler's "A Prettier Printer" algorithm.
//!
//! # Architecture
//!
//! The formatter uses a three-stage pipeline:
//!
//! 1. **Parse** - Source text → CST (done externally via `eure-parol`)
//! 2. **Build** - CST → Doc IR (intermediate representation)
//! 3. **Print** - Doc IR → Formatted string
//!
//! # Example
//!
//! ```ignore
//! use eure_fmt::{format, FormatConfig};
//!
//! let input = "a={b=1,c=2}";
//! let config = FormatConfig::default();
//! let formatted = format(input, &config).unwrap();
//! ```

mod builder;
pub mod config;
pub mod doc;
pub mod printer;
pub mod source;

#[cfg(any(feature = "unformat", test))]
pub mod unformat;

pub use config::FormatConfig;
pub use doc::Doc;
pub use source::{build_source_doc, format_source_document};

use eure_tree::Cst;

use builder::FormatBuilder;
use printer::Printer;

/// Error that can occur during formatting.
#[derive(Debug, Clone)]
pub enum FormatError {
    /// Failed to parse the input.
    ParseError(String),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for FormatError {}

/// Result of checking if a file is formatted.
#[derive(Debug, Clone)]
pub enum FormatCheckResult {
    /// The input is already well-formatted.
    WellFormatted,
    /// The input needs formatting.
    NeedsFormatting {
        /// The formatted output.
        formatted: String,
    },
    /// Failed to parse the input.
    ParseError(String),
}

impl FormatCheckResult {
    /// Returns true if the input is well-formatted.
    pub fn is_well_formatted(&self) -> bool {
        matches!(self, FormatCheckResult::WellFormatted)
    }

    /// Returns true if the input needs formatting.
    pub fn needs_formatting(&self) -> bool {
        matches!(self, FormatCheckResult::NeedsFormatting { .. })
    }

    /// Returns true if there was a parse error.
    pub fn is_parse_error(&self) -> bool {
        matches!(self, FormatCheckResult::ParseError(_))
    }
}

/// Format Eure source code using an already-parsed CST.
///
/// This is the lower-level API that works directly with the CST.
pub fn format_cst(input: &str, cst: &Cst, config: &FormatConfig) -> String {
    let builder = FormatBuilder::new(input, cst, config);
    let doc = builder.build(cst);
    Printer::new(config.clone()).print(&doc)
}

/// Build a Doc IR from a CST.
///
/// This is useful for debugging or custom printing.
pub fn build_doc(input: &str, cst: &Cst, config: &FormatConfig) -> Doc {
    let builder = FormatBuilder::new(input, cst, config);
    builder.build(cst)
}

/// A text edit representing a change to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Start offset in bytes.
    pub start: usize,
    /// End offset in bytes.
    pub end: usize,
    /// New text to insert.
    pub new_text: String,
}

/// Compute text edits to transform input into formatted output.
///
/// This is useful for LSP integration where you need incremental edits
/// rather than full file replacement.
pub fn compute_edits(input: &str, formatted: &str) -> Vec<TextEdit> {
    // For now, use a simple full-file replacement
    // A more sophisticated implementation would use a diff algorithm
    if input == formatted {
        Vec::new()
    } else {
        vec![TextEdit {
            start: 0,
            end: input.len(),
            new_text: formatted.to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_format(input: &str) -> String {
        let cst = eure_parol::parse(input).expect("parse failed");
        format_cst(input, &cst, &FormatConfig::default())
    }

    #[test]
    fn test_format_simple() {
        let formatted = parse_and_format("a = 1");
        assert_eq!(formatted, "a = 1\n");
    }

    #[test]
    fn test_format_preserves_content() {
        let input = "name = \"hello\"\nage = 42";
        let formatted = parse_and_format(input);
        assert!(formatted.contains("name"));
        assert!(formatted.contains("\"hello\""));
        assert!(formatted.contains("age"));
        assert!(formatted.contains("42"));
    }

    #[test]
    fn test_format_array() {
        let formatted = parse_and_format("= [1, 2, 3]");
        assert_eq!(formatted, "= [1, 2, 3]\n");
    }

    #[test]
    fn test_format_empty_array() {
        let formatted = parse_and_format("= []");
        assert_eq!(formatted, "= []\n");
    }

    #[test]
    fn test_format_empty_object() {
        let formatted = parse_and_format("= {}");
        assert_eq!(formatted, "= {}\n");
    }

    #[test]
    fn test_compute_edits_no_change() {
        let edits = compute_edits("a = 1\n", "a = 1\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_compute_edits_with_change() {
        let edits = compute_edits("a=1", "a = 1\n");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start, 0);
        assert_eq!(edits[0].end, 3);
        assert_eq!(edits[0].new_text, "a = 1\n");
    }
}
