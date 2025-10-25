//! Error types for the eure-schema crate

use thiserror::Error;

/// Errors that can occur when working with value-based schema APIs
#[derive(Debug, Error)]
pub enum ValueError {
    /// Failed to parse the input EURE document
    #[error("Failed to parse input: {0}")]
    ParseError(#[from] eure_parol::parol_runtime::ParolError),

    /// Failed to build the value tree from the parsed CST
    #[error("Failed to build value tree: {0}")]
    VisitorError(#[from] eure_tree::value_visitor::ValueVisitorError),

    /// Failed to extract schema from the document
    #[error("Failed to extract schema: {0}")]
    SchemaError(#[from] crate::document_schema::SchemaError),
}
