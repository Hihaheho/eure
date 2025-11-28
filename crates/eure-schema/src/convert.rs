//! Conversion from EureDocument to SchemaDocument
//!
//! This module provides functionality to convert EURE documents containing schema definitions
//! into SchemaDocument structures.
//!
//! # Schema Syntax
//!
//! Schema types are defined using the following syntax:
//!
//! **Primitives (shorthands):**
//! - `.string`, `.integer`, `.float`, `.boolean`, `.null`, `.any`, `.path`
//! - `.code`, `.code.rust`, `.code.javascript`, `.code.email`
//!
//! **Primitives with constraints:**
//! ```eure
//! @ field {
//!   $variant: string
//!   min-length = 3
//!   max-length = 20
//!   pattern = "^[a-z]+$"
//! }
//! ```
//!
//! **Array:** `[.string]` or `{ $variant: array, item = .string, ... }`
//!
//! **Tuple:** `(.string, .integer)` or `{ $variant: tuple, elements = [...] }`
//!
//! **Record:** `{ name = .string, age = .integer }`
//!
//! **Union with named variants:**
//! ```eure
//! @ field {
//!   $variant: union
//!   variants.success = { data = .any }
//!   variants.error = { message = .string }
//!   $variant-repr = "untagged"  // optional
//!   priority = ["error", "success"]  // optional, for untagged unions
//! }
//! ```
//!
//! **Literal:** Any constant value (e.g., `"active"`, `42`, `true`)
//!
//! **Type reference:** `.$types.my-type` or `.$types.namespace.type`

use crate::SchemaDocument;
use eure_value::document::EureDocument;
use thiserror::Error;

/// Errors that can occur during document to schema conversion
#[derive(Debug, Error, Clone, PartialEq)]
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

    #[error("Invalid range string: {0}")]
    InvalidRangeString(String),
}

/// Convert an EureDocument containing schema definitions to a SchemaDocument
///
/// This function traverses the document and extracts schema information from:
/// - Type paths (`.string`, `.integer`, `.code.rust`, etc.)
/// - `$variant` extension for explicit type variants
/// - `variants.*` fields for union variant definitions
/// - Constraint fields (`min-length`, `max-length`, `pattern`, `range`, etc.)
/// - Metadata extensions (`$description`, `$deprecated`, `$default`, `$examples`)
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
/// name = .string
/// age = .integer
/// "#;
///
/// let doc = parse_to_document(input).unwrap();
/// let schema = document_to_schema(&doc).unwrap();
/// ```
pub fn document_to_schema(_doc: &EureDocument) -> Result<SchemaDocument, ConversionError> {
    todo!("Implement EureDocument to SchemaDocument conversion")
}
