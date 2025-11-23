//! Conversion from EureDocument to SchemaDocument
//!
//! This module provides functionality to convert EURE documents containing schema definitions
//! (using extensions like $type, $array, $variants, etc.) into SchemaDocument structures.

use crate::SchemaDocument;
use eure_value::document::EureDocument;
use thiserror::Error;

/// Errors that can occur during document to schema conversion
#[derive(Debug, Error, Clone)]
pub enum ConversionError {
    #[error("Invalid type path: {0}")]
    InvalidTypePath(String),

    #[error("Unsupported schema construct at path: {0}")]
    UnsupportedConstruct(String),

    #[error("Invalid extension value: {extension} at path {path}")]
    InvalidExtensionValue { extension: String, path: String },

    #[error("Missing required extension: {extension} at path {path}")]
    MissingRequiredExtension { extension: String, path: String },

    #[error("Conflicting extensions: {extensions:?} at path {path}")]
    ConflictingExtensions {
        extensions: Vec<String>,
        path: String,
    },

    #[error("Invalid node reference: {0}")]
    InvalidNodeReference(String),

    #[error("Invalid constraint value: {constraint} with value {value}")]
    InvalidConstraintValue { constraint: String, value: String },
}

/// Convert an EureDocument containing schema definitions to a SchemaDocument
///
/// This function traverses the document and extracts schema information from extension fields
/// like $type, $array, $variants, etc.
///
/// # Arguments
///
/// * `doc` - The EureDocument containing schema definitions
///
/// # Returns
///
/// A SchemaDocument on success, or a ConversionError on failure
///
/// # Examples
///
/// ```ignore
/// use eure::parse_to_document;
/// use eure_schema::convert::document_to_schema;
///
/// let input = r#"
/// name.$type = .string
/// age.$type = .number
/// "#;
///
/// let doc = parse_to_document(input).unwrap();
/// let schema = document_to_schema(&doc).unwrap();
/// ```
pub fn document_to_schema(_doc: &EureDocument) -> Result<SchemaDocument, ConversionError> {
    todo!("Implement EureDocument to SchemaDocument conversion")
}
