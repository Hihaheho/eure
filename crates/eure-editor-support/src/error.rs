//! Error types for editor support operations

use std::fmt;

/// Errors that can occur during editor support operations
#[derive(Debug)]
pub enum EditorError {
    /// Tree construction failed
    TreeConstruction(String),

    /// Invalid identifier
    InvalidIdentifier(String),

    /// Invalid schema reference
    InvalidSchemaRef(String),

    /// Path parsing failed
    PathParsing(String),

    /// Tuple key conversion not supported
    TupleKeyConversion(String),
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditorError::TreeConstruction(msg) => write!(f, "Tree construction failed: {}", msg),
            EditorError::InvalidIdentifier(msg) => write!(f, "Invalid identifier: {}", msg),
            EditorError::InvalidSchemaRef(msg) => write!(f, "Invalid schema reference: {}", msg),
            EditorError::PathParsing(msg) => write!(f, "Path parsing failed: {}", msg),
            EditorError::TupleKeyConversion(msg) => {
                write!(f, "Tuple key conversion not supported: {}", msg)
            }
        }
    }
}

impl std::error::Error for EditorError {}

/// Result type for editor support operations
pub type Result<T> = std::result::Result<T, EditorError>;
