//! Validation error types
//!
//! Two categories of errors:
//! - `ValidationError`: Type errors accumulated during validation (non-fatal)
//! - `ValidatorError`: Internal validator errors that cause fail-fast behavior

use eure_document::document::NodeId;
use eure_document::parse::ParseError;
use eure_document::path::EurePath;
use thiserror::Error;

use crate::SchemaNodeId;

// =============================================================================
// ValidatorError (fail-fast internal errors)
// =============================================================================

/// Internal validator errors that cause immediate failure.
///
/// These represent problems with the validator itself or invalid inputs,
/// not type mismatches in the document being validated.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidatorError {
    /// Undefined type reference in schema
    #[error("undefined type reference: {name}")]
    UndefinedTypeReference { name: String },

    /// Invalid variant tag (parse error)
    #[error("invalid variant tag '{tag}': {reason}")]
    InvalidVariantTag { tag: String, reason: String },

    /// Conflicting variant tags between $variant and repr
    #[error("conflicting variant tags: $variant = {explicit}, repr = {repr}")]
    ConflictingVariantTags { explicit: String, repr: String },

    /// Cross-schema reference not supported
    #[error("cross-schema reference not supported: {namespace}.{name}")]
    CrossSchemaReference { namespace: String, name: String },

    /// Parse error (from eure-document)
    #[error("parse error: {0}")]
    DocumentParseError(#[from] ParseError),

    /// Inner validation errors were already propagated (no additional error needed)
    #[error("inner errors propagated")]
    InnerErrorsPropagated,
}

impl ValidatorError {
    /// Get the underlying ParseError if this is a DocumentParseError variant.
    pub fn as_parse_error(&self) -> Option<&ParseError> {
        match self {
            ValidatorError::DocumentParseError(e) => Some(e),
            _ => None,
        }
    }
}

// =============================================================================
// ValidationError (accumulated type errors)
// =============================================================================

/// Type errors accumulated during validation.
///
/// These represent mismatches between the document and schema.
/// Validation continues after recording these errors.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidationError {
    #[error("Type mismatch: expected {expected}, got {actual} at path {path}")]
    TypeMismatch {
        expected: String,
        actual: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Missing required field '{field}' at path {path}")]
    MissingRequiredField {
        field: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Unknown field '{field}' at path {path}")]
    UnknownField {
        field: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Value {value} is out of range at path {path}")]
    OutOfRange {
        value: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("String length {length} is out of bounds at path {path}")]
    StringLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("String does not match pattern '{pattern}' at path {path}")]
    PatternMismatch {
        pattern: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Array length {length} is out of bounds at path {path}")]
    ArrayLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Map size {size} is out of bounds at path {path}")]
    MapSizeOutOfBounds {
        size: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Tuple length mismatch: expected {expected}, got {actual} at path {path}")]
    TupleLengthMismatch {
        expected: usize,
        actual: usize,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Array elements must be unique at path {path}")]
    ArrayNotUnique {
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Array must contain required element at path {path}")]
    ArrayMissingContains {
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("No variant matched for union at path {path}")]
    NoVariantMatched {
        path: EurePath,
        variant_errors: Vec<(String, ValidationError)>,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Multiple variants matched for union at path {path}: {variants:?}")]
    AmbiguousUnion {
        path: EurePath,
        variants: Vec<String>,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Invalid variant tag '{tag}' at path {path}")]
    InvalidVariantTag {
        tag: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Conflicting variant tags: $variant = {explicit}, repr = {repr} at path {path}")]
    ConflictingVariantTags {
        explicit: String,
        repr: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Literal value mismatch at path {path}")]
    LiteralMismatch {
        expected: String,
        actual: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Language mismatch: expected {expected}, got {actual} at path {path}")]
    LanguageMismatch {
        expected: String,
        actual: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Invalid key type at path {path}")]
    InvalidKeyType {
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Integer not a multiple of {divisor} at path {path}")]
    NotMultipleOf {
        divisor: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Undefined type reference '{name}' at path {path}")]
    UndefinedTypeReference {
        name: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Missing required extension '{extension}' at path {path}")]
    MissingRequiredExtension {
        extension: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    /// Parse error with schema context.
    /// Uses custom display to translate ParseErrorKind to user-friendly messages.
    #[error("{}", format_parse_error(path, error))]
    ParseError {
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
        error: eure_document::parse::ParseError,
    },
}

/// Format a ParseError into a user-friendly validation error message.
fn format_parse_error(path: &EurePath, error: &eure_document::parse::ParseError) -> String {
    use eure_document::parse::ParseErrorKind;
    match &error.kind {
        ParseErrorKind::UnknownVariant(name) => {
            format!("Invalid variant tag '{name}' at path {path}")
        }
        ParseErrorKind::ConflictingVariantTags { explicit, repr } => {
            format!("Conflicting variant tags: $variant = {explicit}, repr = {repr} at path {path}")
        }
        ParseErrorKind::InvalidVariantType(kind) => {
            format!("$variant must be a string, got {kind:?} at path {path}")
        }
        ParseErrorKind::InvalidVariantPath(path_str) => {
            format!("Invalid $variant path syntax: '{path_str}' at path {path}")
        }
        // For other parse errors, use the default display
        _ => format!("{} at path {}", error.kind, path),
    }
}

impl ValidationError {
    /// Get the node IDs associated with this error.
    pub fn node_ids(&self) -> (NodeId, SchemaNodeId) {
        match self {
            Self::TypeMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::MissingRequiredField {
                node_id,
                schema_node_id,
                ..
            }
            | Self::UnknownField {
                node_id,
                schema_node_id,
                ..
            }
            | Self::OutOfRange {
                node_id,
                schema_node_id,
                ..
            }
            | Self::StringLengthOutOfBounds {
                node_id,
                schema_node_id,
                ..
            }
            | Self::PatternMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ArrayLengthOutOfBounds {
                node_id,
                schema_node_id,
                ..
            }
            | Self::MapSizeOutOfBounds {
                node_id,
                schema_node_id,
                ..
            }
            | Self::TupleLengthMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ArrayNotUnique {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ArrayMissingContains {
                node_id,
                schema_node_id,
                ..
            }
            | Self::NoVariantMatched {
                node_id,
                schema_node_id,
                ..
            }
            | Self::AmbiguousUnion {
                node_id,
                schema_node_id,
                ..
            }
            | Self::InvalidVariantTag {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ConflictingVariantTags {
                node_id,
                schema_node_id,
                ..
            }
            | Self::LiteralMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::LanguageMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::InvalidKeyType {
                node_id,
                schema_node_id,
                ..
            }
            | Self::NotMultipleOf {
                node_id,
                schema_node_id,
                ..
            }
            | Self::UndefinedTypeReference {
                node_id,
                schema_node_id,
                ..
            }
            | Self::MissingRequiredExtension {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ParseError {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
        }
    }
}

// =============================================================================
// ValidationWarning
// =============================================================================

/// Warnings generated during validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarning {
    /// Unknown extension on a node
    UnknownExtension { name: String, path: EurePath },
    /// Deprecated field usage
    DeprecatedField { field: String, path: EurePath },
}
