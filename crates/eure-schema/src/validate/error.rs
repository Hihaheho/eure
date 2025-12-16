//! Validation error types
//!
//! Two categories of errors:
//! - `ValidationError`: Type errors accumulated during validation (non-fatal)
//! - `ValidatorError`: Internal validator errors that cause fail-fast behavior

use eure_document::document::NodeId;
use eure_document::parse::ParseError;
use eure_document::path::EurePath;
use eure_document::value::ObjectKey;
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
// BestVariantMatch (for union error reporting)
// =============================================================================

/// Information about the best matching variant in a failed union validation.
///
/// When an untagged union validation fails, this structure captures detailed
/// information about which variant came closest to matching, enabling better
/// error diagnostics.
///
/// # Selection Criteria
///
/// The "best" variant is selected based on:
/// 1. **Depth**: Errors deeper in the structure indicate better match (got further before failing)
/// 2. **Error count**: Fewer errors indicate closer match
/// 3. **Error priority**: Higher priority errors (like MissingRequiredField) indicate clearer mismatches
///
/// # Nested Unions
///
/// For nested unions like `Result<Option<T>, E>`, the error field itself may be a
/// `NoVariantMatched` error, creating a hierarchical error structure that shows
/// the full path of variant attempts.
#[derive(Debug, Clone, PartialEq)]
pub struct BestVariantMatch {
    /// Name of the variant that matched best
    pub variant_name: String,
    /// Primary error from this variant (may be nested NoVariantMatched)
    pub error: Box<ValidationError>,
    /// All errors collected from this variant attempt
    pub all_errors: Vec<ValidationError>,
    /// Depth metric (path length of deepest error)
    pub depth: usize,
    /// Number of errors
    pub error_count: usize,
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

    /// No variant matched in an untagged union validation.
    ///
    /// This error occurs when all variants of a union are tried and none succeeds.
    /// When available, `best_match` provides detailed information about which variant
    /// came closest to matching and why it failed.
    ///
    /// For tagged unions (with `$variant` or `VariantRepr`), validation errors are
    /// reported directly instead of wrapping them in `NoVariantMatched`.
    #[error("{}", format_no_variant_matched(path, best_match))]
    NoVariantMatched {
        path: EurePath,
        /// Best matching variant (None if no variants were tried)
        best_match: Option<Box<BestVariantMatch>>,
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

    #[error("Variant '{variant}' requires explicit $variant tag at path {path}")]
    RequiresExplicitVariant {
        variant: String,
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
        /// The key that has the wrong type
        key: ObjectKey,
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

/// Format NoVariantMatched error with best match information.
fn format_no_variant_matched(
    path: &EurePath,
    best_match: &Option<Box<BestVariantMatch>>,
) -> String {
    match best_match {
        Some(best) => {
            let mut msg = format!(
                "No variant matched for union at path {path}, most close variant is '{}': {}",
                best.variant_name, best.error
            );
            if best.all_errors.len() > 1 {
                msg.push_str(&format!(" (and {} more errors)", best.all_errors.len() - 1));
            }
            msg
        }
        None => format!("No variant matched for union at path {path}"),
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
            | Self::RequiresExplicitVariant {
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

    /// Calculate the depth of this error (path length).
    ///
    /// Deeper errors indicate that validation got further into the structure
    /// before failing, suggesting a better match.
    pub fn depth(&self) -> usize {
        match self {
            Self::TypeMismatch { path, .. }
            | Self::MissingRequiredField { path, .. }
            | Self::UnknownField { path, .. }
            | Self::OutOfRange { path, .. }
            | Self::StringLengthOutOfBounds { path, .. }
            | Self::PatternMismatch { path, .. }
            | Self::ArrayLengthOutOfBounds { path, .. }
            | Self::MapSizeOutOfBounds { path, .. }
            | Self::TupleLengthMismatch { path, .. }
            | Self::ArrayNotUnique { path, .. }
            | Self::ArrayMissingContains { path, .. }
            | Self::NoVariantMatched { path, .. }
            | Self::AmbiguousUnion { path, .. }
            | Self::InvalidVariantTag { path, .. }
            | Self::ConflictingVariantTags { path, .. }
            | Self::RequiresExplicitVariant { path, .. }
            | Self::LiteralMismatch { path, .. }
            | Self::LanguageMismatch { path, .. }
            | Self::InvalidKeyType { path, .. }
            | Self::NotMultipleOf { path, .. }
            | Self::UndefinedTypeReference { path, .. }
            | Self::MissingRequiredExtension { path, .. }
            | Self::ParseError { path, .. } => path.0.len(),
        }
    }

    /// Get priority score for error type (higher = more indicative of mismatch).
    ///
    /// Used for selecting the "best" variant error when multiple variants fail
    /// with similar depth and error counts.
    pub fn priority_score(&self) -> u8 {
        match self {
            Self::MissingRequiredField { .. } => 90,
            Self::TypeMismatch { .. } => 80,
            Self::TupleLengthMismatch { .. } => 70,
            Self::LiteralMismatch { .. } => 70,
            Self::InvalidVariantTag { .. } => 65,
            Self::NoVariantMatched { .. } => 60, // Nested union mismatch
            Self::UnknownField { .. } => 50,
            Self::MissingRequiredExtension { .. } => 50,
            Self::ParseError { .. } => 40, // Medium priority
            Self::OutOfRange { .. } => 30,
            Self::StringLengthOutOfBounds { .. } => 30,
            Self::PatternMismatch { .. } => 30,
            Self::ArrayLengthOutOfBounds { .. } => 30,
            Self::MapSizeOutOfBounds { .. } => 30,
            Self::NotMultipleOf { .. } => 30,
            Self::ArrayNotUnique { .. } => 25,
            Self::ArrayMissingContains { .. } => 25,
            Self::InvalidKeyType { .. } => 20,
            Self::LanguageMismatch { .. } => 20,
            Self::AmbiguousUnion { .. } => 0, // Not a mismatch
            Self::ConflictingVariantTags { .. } => 0, // Configuration error
            Self::UndefinedTypeReference { .. } => 0, // Configuration error
            Self::RequiresExplicitVariant { .. } => 0, // Configuration error
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
