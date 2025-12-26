//! Error types for TOML to Eure conversion.

use thiserror::Error;

/// Errors that can occur when converting TOML to SourceDocument.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum TomlToEureError {
    /// The TOML key is not a valid Eure identifier.
    #[error("Invalid identifier '{key}': {reason}")]
    InvalidIdentifier { key: String, reason: String },

    /// TOML parse error.
    #[error("Parse error: {message}")]
    ParseError { message: String },

    /// Value decode error.
    #[error("Decode error: {message}")]
    DecodeError { message: String },
}
