//! Document schema validation
//!
//! This module provides functionality to validate Eure documents against schema definitions.
//!
//! # Validation Result
//!
//! Validation returns two flags:
//! - `is_valid`: No type errors (holes are allowed)
//! - `is_complete`: No type errors AND no holes
//!
//! # Union Type Checking (oneOf Semantics)
//!
//! Union types require exactly one variant to match:
//! - If no variant matches: error with the "closest" failure
//! - If exactly one matches: success
//! - If multiple match: ambiguity error (unless priority is set)
//!
//! # Hole Values
//!
//! The hole value (`!`) represents an unfilled placeholder:
//! - Type checking: Holes match any schema (always pass)
//! - Completeness: Documents containing holes are valid but not complete
//!
//! # Extension Validation
//!
//! Extensions on nodes are validated against:
//! - Schema-defined extensions (`$ext-type.X`)
//! - Built-in extensions (e.g., `$variant` for unions)
//! - Unknown extensions: valid but emit a warning

use crate::{
    ArraySchema, Bound, FloatSchema, IntegerSchema, MapSchema, RecordSchema, SchemaDocument,
    SchemaNodeContent, SchemaNodeId, TextSchema, TupleSchema, TypeReference, UnionSchema,
    UnknownFieldsPolicy, identifiers,
};
use eure_document::data_model::VariantRepr;
use eure_document::document::node::{Node, NodeValue};
use eure_document::document::{EureDocument, NodeId};
use eure_document::identifier::Identifier;
use eure_document::parse::VariantPath;
use eure_document::path::{EurePath, PathSegment};
use eure_document::text::Language;
use eure_document::value::{ObjectKey, PrimitiveValue};
use num_bigint::BigInt;
use regex::Regex;
use thiserror::Error;

/// Result of validating a document against a schema
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// No type errors (holes are allowed)
    pub is_valid: bool,
    /// No type errors AND no holes
    pub is_complete: bool,
    /// Type errors encountered during validation
    pub errors: Vec<ValidationError>,
    /// Warnings (e.g., unknown extensions)
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success(has_holes: bool) -> Self {
        Self {
            is_valid: true,
            is_complete: !has_holes,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result with an error
    pub fn failure(error: ValidationError) -> Self {
        Self {
            is_valid: false,
            is_complete: false,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result with multiple errors
    pub fn failures(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            is_complete: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
        }
        if !other.is_complete {
            self.is_complete = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
}

/// Validation error types
///
/// Each error variant includes:
/// - Descriptive fields for the error message
/// - `path`: The document path where the error occurred
/// - `node_id`: Optional NodeId for source location lookup in editors
/// - `schema_node_id`: Optional SchemaNodeId for looking up where the constraint is defined
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidationError {
    #[error("Type mismatch: expected {expected}, got {actual} at path {path}")]
    TypeMismatch {
        expected: String,
        actual: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the type constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Missing required field '{field}' at path {path}")]
    MissingRequiredField {
        field: String,
        path: String,
        /// Source node ID (parent node) for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the required field is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Unknown field '{field}' at path {path}")]
    UnknownField {
        field: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the record is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Value {value} is out of range at path {path}")]
    OutOfRange {
        value: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the range constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("String length {length} is out of bounds at path {path}")]
    StringLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the length constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("String does not match pattern '{pattern}' at path {path}")]
    PatternMismatch {
        pattern: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the pattern constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Array length {length} is out of bounds at path {path}")]
    ArrayLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the length constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Map size {size} is out of bounds at path {path}")]
    MapSizeOutOfBounds {
        size: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the size constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Tuple length mismatch: expected {expected}, got {actual} at path {path}")]
    TupleLengthMismatch {
        expected: usize,
        actual: usize,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the tuple is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Array elements must be unique at path {path}")]
    ArrayNotUnique {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the unique constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Array must contain required element at path {path}")]
    ArrayMissingContains {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the contains constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("No variant matched for union at path {path}")]
    NoVariantMatched {
        path: String,
        /// Errors from each variant attempt
        variant_errors: Vec<(String, ValidationError)>,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the union is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Multiple variants matched for union at path {path}: {variants:?}")]
    AmbiguousUnion {
        path: String,
        variants: Vec<String>,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the union is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Invalid variant tag '{tag}' at path {path}")]
    InvalidVariantTag {
        tag: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the union is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Missing $variant extension at path {path}")]
    MissingVariantExtension {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the union is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Literal value mismatch at path {path}")]
    LiteralMismatch {
        expected: String,
        actual: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the literal is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Language mismatch: expected {expected}, got {actual} at path {path}")]
    LanguageMismatch {
        expected: String,
        actual: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the language constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Invalid key type at path {path}")]
    InvalidKeyType {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the map key type is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Integer not a multiple of {divisor} at path {path}")]
    NotMultipleOf {
        divisor: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the multiple-of constraint is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Undefined type reference '{name}' at path {path}")]
    UndefinedTypeReference {
        name: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the reference is used
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Invalid regex pattern '{pattern}': {error}")]
    InvalidRegexPattern {
        pattern: String,
        error: String,
        /// Source node ID for editor diagnostics (may be None for schema errors)
        node_id: Option<NodeId>,
        /// Schema node ID where the pattern is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Invalid extension type for '{name}' at path {path}")]
    InvalidExtensionType {
        name: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the extension type is defined
        schema_node_id: Option<SchemaNodeId>,
    },

    #[error("Missing required extension '{extension}' at path {path}")]
    MissingRequiredExtension {
        extension: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
        /// Schema node ID where the extension is required
        schema_node_id: Option<SchemaNodeId>,
    },
}

impl ValidationError {
    /// Get both document and schema node IDs for error location
    pub fn node_ids(&self) -> (Option<NodeId>, Option<SchemaNodeId>) {
        match self {
            Self::TypeMismatch {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::MissingRequiredField {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::UnknownField {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::OutOfRange {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::StringLengthOutOfBounds {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::PatternMismatch {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::ArrayLengthOutOfBounds {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::MapSizeOutOfBounds {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::TupleLengthMismatch {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::ArrayNotUnique {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::ArrayMissingContains {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::NoVariantMatched {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::AmbiguousUnion {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::InvalidVariantTag {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::MissingVariantExtension {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::LiteralMismatch {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::LanguageMismatch {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::InvalidKeyType {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::NotMultipleOf {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::UndefinedTypeReference {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::InvalidRegexPattern {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::InvalidExtensionType {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
            Self::MissingRequiredExtension {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
        }
    }
}

/// Validation warnings
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarning {
    /// Unknown extension encountered
    UnknownExtension { name: String, path: String },
    /// Deprecated field used
    DeprecatedField { field: String, path: String },
}

/// Internal validator state
struct Validator<'a> {
    schema: &'a SchemaDocument,
    document: &'a EureDocument,
    path: EurePath,
    has_holes: bool,
    /// Current node being validated - used for error location reporting
    /// None when validating temporary nodes (e.g., variant content converted from Value)
    current_node_id: Option<NodeId>,
    /// Current schema node being validated against - used for error location reporting
    current_schema_node_id: Option<SchemaNodeId>,
}

impl<'a> Validator<'a> {
    fn new(document: &'a EureDocument, schema: &'a SchemaDocument) -> Self {
        Self {
            schema,
            document,
            path: EurePath::root(),
            has_holes: false,
            current_node_id: None,
            current_schema_node_id: None,
        }
    }

    fn current_path(&self) -> String {
        if self.path.is_root() {
            "$".to_string()
        } else {
            format!("${}", self.path)
        }
    }

    /// Get current node ID for error reporting
    fn node_id(&self) -> Option<NodeId> {
        self.current_node_id
    }

    /// Get current schema node ID for error reporting
    fn schema_node_id(&self) -> Option<SchemaNodeId> {
        self.current_schema_node_id
    }

    /// Push an identifier path segment (for record field names that are valid identifiers)
    fn push_path_ident(&mut self, ident: Identifier) {
        self.path.0.push(PathSegment::Ident(ident));
    }

    /// Push an arbitrary key path segment (for map keys that may not be valid identifiers)
    fn push_path_key(&mut self, key: ObjectKey) {
        self.path.0.push(PathSegment::Value(key));
    }

    /// Push an array index path segment
    fn push_path_index(&mut self, index: usize) {
        self.path.0.push(PathSegment::ArrayIndex(Some(index)));
    }

    /// Push a tuple index path segment
    fn push_path_tuple_index(&mut self, index: u8) {
        self.path.0.push(PathSegment::TupleIndex(index));
    }

    /// Push an extension path segment
    fn push_path_extension(&mut self, ident: Identifier) {
        self.path.0.push(PathSegment::Extension(ident));
    }

    fn pop_path(&mut self) {
        self.path.0.pop();
    }

    /// Get extension value as a string (for variant tags)
    fn get_extension_as_string(&self, node: &Node, ident: &Identifier) -> Option<String> {
        let ext_node_id = node.extensions.get(ident)?;
        let ext_node = self.document.node(*ext_node_id);
        match &ext_node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => Some(t.as_str().to_string()),
            _ => None,
        }
    }

    /// Main validation entry point
    fn validate(&mut self, node_id: NodeId, schema_id: SchemaNodeId) -> ValidationResult {
        self.current_node_id = Some(node_id);
        let node = self.document.node(node_id);
        let schema_node = self.schema.node(schema_id);
        self.validate_content(node, &schema_node.content, schema_id)
    }

    /// Validate a temporary node (no source location)
    fn validate_temp_node(
        &mut self,
        node: &Node,
        content: &SchemaNodeContent,
        schema_id: SchemaNodeId,
    ) -> ValidationResult {
        let prev_node_id = self.current_node_id;
        self.current_node_id = None;
        let result = self.validate_content(node, content, schema_id);
        self.current_node_id = prev_node_id;
        result
    }

    /// Validate node content against schema content
    fn validate_content(
        &mut self,
        node: &Node,
        content: &SchemaNodeContent,
        schema_id: SchemaNodeId,
    ) -> ValidationResult {
        // Track current schema node for error reporting
        let prev_schema_node_id = self.current_schema_node_id;
        self.current_schema_node_id = Some(schema_id);

        if let NodeValue::Hole(_) = &node.content {
            self.has_holes = true;
            self.current_schema_node_id = prev_schema_node_id;
            return ValidationResult::success(true);
        }

        let mut result = match content {
            SchemaNodeContent::Any => ValidationResult::success(self.has_holes),
            SchemaNodeContent::Text(text_schema) => self.validate_text(node, text_schema),
            SchemaNodeContent::Integer(int_schema) => self.validate_integer(node, int_schema),
            SchemaNodeContent::Float(float_schema) => self.validate_float(node, float_schema),
            SchemaNodeContent::Boolean => self.validate_boolean(node),
            SchemaNodeContent::Null => self.validate_null(node),
            SchemaNodeContent::Literal(expected) => self.validate_literal(node, expected),
            SchemaNodeContent::Array(array_schema) => self.validate_array(node, array_schema),
            SchemaNodeContent::Map(map_schema) => self.validate_map(node, map_schema),
            SchemaNodeContent::Record(record_schema) => {
                self.validate_record(node, record_schema, schema_id)
            }
            SchemaNodeContent::Tuple(tuple_schema) => self.validate_tuple(node, tuple_schema),
            SchemaNodeContent::Union(union_schema) => self.validate_union(node, union_schema),
            SchemaNodeContent::Reference(type_ref) => self.validate_reference(node, type_ref),
        };

        // Validate extensions against schema-defined ext_types
        let schema_node = self.schema.node(schema_id);
        let ext_types = &schema_node.ext_types;

        // Check for missing required extensions
        for (ext_ident, ext_schema) in ext_types {
            if !ext_schema.optional && !node.extensions.contains_key(ext_ident) {
                result.merge(ValidationResult::failure(
                    ValidationError::MissingRequiredExtension {
                        extension: ext_ident.to_string(),
                        path: self.current_path(),
                        node_id: self.current_node_id,
                        schema_node_id: self.schema_node_id(),
                    },
                ));
            }
        }

        // Validate present extensions
        for (ext_ident, &ext_node_id) in &node.extensions {
            // Skip well-known extensions used for variant discrimination
            if ext_ident == &identifiers::VARIANT {
                continue;
            }

            // Check if this extension is defined in schema
            if let Some(ext_schema) = ext_types.get(ext_ident) {
                // Validate the extension value against its schema
                self.push_path_extension(ext_ident.clone());
                let ext_node = self.document.node(ext_node_id);
                let ext_schema_node = self.schema.node(ext_schema.schema);
                let ext_result =
                    self.validate_content(ext_node, &ext_schema_node.content, ext_schema.schema);
                result.merge(ext_result);
                self.pop_path();
            } else {
                // Unknown extensions are allowed but generate warnings
                result.add_warning(ValidationWarning::UnknownExtension {
                    name: ext_ident.to_string(),
                    path: self.current_path(),
                });
            }
        }

        // Restore previous schema node id
        self.current_schema_node_id = prev_schema_node_id;
        result
    }

    // =========================================================================
    // Primitive Type Validation
    // =========================================================================

    fn validate_text(&mut self, node: &Node, schema: &TextSchema) -> ValidationResult {
        let text = match &node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => t,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "text".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        // Validate language constraint
        if let Some(expected_lang) = &schema.language {
            match &text.language {
                // Plaintext ("...") matches schema with language=None or language="plaintext"
                Language::Plaintext => {
                    if expected_lang != "plaintext" && !expected_lang.is_empty() {
                        return ValidationResult::failure(ValidationError::LanguageMismatch {
                            expected: expected_lang.clone(),
                            actual: "plaintext".to_string(),
                            path: self.current_path(),
                            node_id: self.node_id(),
                            schema_node_id: self.schema_node_id(),
                        });
                    }
                }
                // Implicit (`...`) can be coerced to any language - always passes
                Language::Implicit => {}
                // Explicit language (lang`...`) must match
                Language::Other(lang) => {
                    if lang != expected_lang.as_str() {
                        return ValidationResult::failure(ValidationError::LanguageMismatch {
                            expected: expected_lang.clone(),
                            actual: lang.clone(),
                            path: self.current_path(),
                            node_id: self.node_id(),
                            schema_node_id: self.schema_node_id(),
                        });
                    }
                }
            }
        }

        // Validate length constraints
        let len = text.as_str().chars().count();
        if let Some(min) = schema.min_length
            && len < min as usize
        {
            return ValidationResult::failure(ValidationError::StringLengthOutOfBounds {
                length: len,
                min: Some(min),
                max: schema.max_length,
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }
        if let Some(max) = schema.max_length
            && len > max as usize
        {
            return ValidationResult::failure(ValidationError::StringLengthOutOfBounds {
                length: len,
                min: schema.min_length,
                max: Some(max),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate pattern
        if let Some(pattern) = &schema.pattern {
            let regex = match Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => {
                    return ValidationResult::failure(ValidationError::InvalidRegexPattern {
                        pattern: pattern.clone(),
                        error: e.to_string(),
                        node_id: self.node_id(),
                        schema_node_id: self.schema_node_id(),
                    });
                }
            };
            if !regex.is_match(text.as_str()) {
                return ValidationResult::failure(ValidationError::PatternMismatch {
                    pattern: pattern.clone(),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_integer(&mut self, node: &Node, schema: &IntegerSchema) -> ValidationResult {
        let int_val = match &node.content {
            NodeValue::Primitive(PrimitiveValue::Integer(i)) => i,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        // Validate range
        if !check_integer_bound(int_val, &schema.min, true) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }
        if !check_integer_bound(int_val, &schema.max, false) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate multiple-of
        if let Some(divisor) = &schema.multiple_of
            && int_val % divisor != BigInt::from(0)
        {
            return ValidationResult::failure(ValidationError::NotMultipleOf {
                divisor: divisor.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_float(&mut self, node: &Node, schema: &FloatSchema) -> ValidationResult {
        let float_val = match &node.content {
            NodeValue::Primitive(PrimitiveValue::F64(f)) => *f,
            NodeValue::Primitive(PrimitiveValue::F32(f)) => *f as f64,
            NodeValue::Primitive(PrimitiveValue::Integer(i)) => {
                // Allow integer to be coerced to float
                if let Ok(i64_val) = i64::try_from(i) {
                    i64_val as f64
                } else {
                    return ValidationResult::failure(ValidationError::TypeMismatch {
                        expected: "float".to_string(),
                        actual: "integer (too large)".to_string(),
                        path: self.current_path(),
                        node_id: self.node_id(),
                        schema_node_id: self.schema_node_id(),
                    });
                }
            }
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "float".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        // Validate range
        if !check_float_bound(float_val, &schema.min, true) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }
        if !check_float_bound(float_val, &schema.max, false) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate multiple-of
        if let Some(divisor) = &schema.multiple_of
            && (float_val % divisor).abs() > f64::EPSILON
        {
            return ValidationResult::failure(ValidationError::NotMultipleOf {
                divisor: divisor.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_boolean(&mut self, node: &Node) -> ValidationResult {
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Bool(_)) => {
                ValidationResult::success(self.has_holes)
            }
            _ => ValidationResult::failure(ValidationError::TypeMismatch {
                expected: "boolean".to_string(),
                actual: node_type_name(&node.content),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            }),
        }
    }

    fn validate_null(&mut self, node: &Node) -> ValidationResult {
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Null) => ValidationResult::success(self.has_holes),
            _ => ValidationResult::failure(ValidationError::TypeMismatch {
                expected: "null".to_string(),
                actual: node_type_name(&node.content),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            }),
        }
    }

    fn validate_literal(&mut self, _node: &Node, expected: &EureDocument) -> ValidationResult {
        // Create a document from the current node for comparison
        let node_id = self
            .node_id()
            .expect("node_id should be set during validation");
        let actual = node_subtree_to_document(self.document, node_id);
        if actual == *expected {
            ValidationResult::success(self.has_holes)
        } else {
            ValidationResult::failure(ValidationError::LiteralMismatch {
                expected: format!("{:?}", expected),
                actual: format!("{:?}", actual),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            })
        }
    }

    // =========================================================================
    // Container Type Validation
    // =========================================================================

    fn validate_array(&mut self, node: &Node, schema: &ArraySchema) -> ValidationResult {
        let arr = match &node.content {
            NodeValue::Array(a) => a,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "array".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        let len = arr.0.len();

        // Validate length constraints
        if let Some(min) = schema.min_length
            && len < min as usize
        {
            return ValidationResult::failure(ValidationError::ArrayLengthOutOfBounds {
                length: len,
                min: Some(min),
                max: schema.max_length,
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }
        if let Some(max) = schema.max_length
            && len > max as usize
        {
            return ValidationResult::failure(ValidationError::ArrayLengthOutOfBounds {
                length: len,
                min: schema.min_length,
                max: Some(max),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate uniqueness
        if schema.unique && !are_nodes_unique(self.document, &arr.0) {
            return ValidationResult::failure(ValidationError::ArrayNotUnique {
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate each item
        let mut result = ValidationResult::success(self.has_holes);
        for (i, &item_id) in arr.0.iter().enumerate() {
            self.push_path_index(i);
            let item_result = self.validate(item_id, schema.item);
            result.merge(item_result);
            self.pop_path();
        }

        // Validate contains constraint
        if let Some(contains_schema) = schema.contains {
            let mut found = false;
            for &item_id in &arr.0 {
                let test_result = self.validate(item_id, contains_schema);
                if test_result.is_valid {
                    found = true;
                    break;
                }
            }
            if !found {
                result.merge(ValidationResult::failure(
                    ValidationError::ArrayMissingContains {
                        path: self.current_path(),
                        node_id: self.node_id(),
                        schema_node_id: self.schema_node_id(),
                    },
                ));
            }
        }

        result
    }

    fn validate_map(&mut self, node: &Node, schema: &MapSchema) -> ValidationResult {
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "map".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        let size = map.0.len();

        // Validate size constraints
        if let Some(min) = schema.min_size
            && size < min as usize
        {
            return ValidationResult::failure(ValidationError::MapSizeOutOfBounds {
                size,
                min: Some(min),
                max: schema.max_size,
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }
        if let Some(max) = schema.max_size
            && size > max as usize
        {
            return ValidationResult::failure(ValidationError::MapSizeOutOfBounds {
                size,
                min: schema.min_size,
                max: Some(max),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate each key-value pair
        let mut result = ValidationResult::success(self.has_holes);
        for (key, &val_id) in map.0.iter() {
            self.push_path_key(key.clone());

            // Validate key (using temp document since key is not from document)
            let key_doc = object_key_to_document(key);
            let key_node = key_doc.root();
            let key_result = self.validate_temp_node(
                key_node,
                &self.schema.node(schema.key).content,
                schema.key,
            );
            if !key_result.is_valid {
                result.merge(ValidationResult::failure(ValidationError::InvalidKeyType {
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                }));
            }

            // Validate value
            let value_result = self.validate(val_id, schema.value);
            result.merge(value_result);

            self.pop_path();
        }

        result
    }

    fn validate_record(
        &mut self,
        node: &Node,
        schema: &RecordSchema,
        _schema_id: SchemaNodeId,
    ) -> ValidationResult {
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "record".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        let mut result = ValidationResult::success(self.has_holes);

        // Check required fields
        for (field_name, field_schema) in &schema.properties {
            if !field_schema.optional {
                let key = ObjectKey::String(field_name.clone());
                if !map.0.contains_key(&key) {
                    result.merge(ValidationResult::failure(
                        ValidationError::MissingRequiredField {
                            field: field_name.clone(),
                            path: self.current_path(),
                            node_id: self.node_id(),
                            schema_node_id: self.schema_node_id(),
                        },
                    ));
                }
            }
        }

        // Validate each field
        for (key, &val_id) in map.0.iter() {
            let field_name = match key {
                ObjectKey::String(s) => s.clone(),
                _ => {
                    result.merge(ValidationResult::failure(ValidationError::InvalidKeyType {
                        path: self.current_path(),
                        node_id: self.node_id(),
                        schema_node_id: self.schema_node_id(),
                    }));
                    continue;
                }
            };

            // Try to parse as identifier, fall back to string key
            if let Ok(ident) = field_name.parse::<Identifier>() {
                self.push_path_ident(ident);
            } else {
                self.push_path_key(ObjectKey::String(field_name.clone()));
            }

            if let Some(field_schema) = schema.properties.get(&field_name) {
                // Check deprecated
                let schema_node = self.schema.node(field_schema.schema);
                if schema_node.metadata.deprecated {
                    result.add_warning(ValidationWarning::DeprecatedField {
                        field: field_name.clone(),
                        path: self.current_path(),
                    });
                }

                let field_result = self.validate(val_id, field_schema.schema);
                result.merge(field_result);
            } else {
                // Unknown field - check policy
                match &schema.unknown_fields {
                    UnknownFieldsPolicy::Deny => {
                        result.merge(ValidationResult::failure(ValidationError::UnknownField {
                            field: field_name.clone(),
                            path: self.current_path(),
                            node_id: self.node_id(),
                            schema_node_id: self.schema_node_id(),
                        }));
                    }
                    UnknownFieldsPolicy::Allow => {
                        // Allow any value
                    }
                    UnknownFieldsPolicy::Schema(schema_id) => {
                        // Validate against the schema
                        let field_result = self.validate(val_id, *schema_id);
                        result.merge(field_result);
                    }
                }
            }

            self.pop_path();
        }

        result
    }

    fn validate_tuple(&mut self, node: &Node, schema: &TupleSchema) -> ValidationResult {
        let tuple = match &node.content {
            NodeValue::Tuple(t) => t,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "tuple".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        // Check length matches
        if tuple.0.len() != schema.elements.len() {
            return ValidationResult::failure(ValidationError::TupleLengthMismatch {
                expected: schema.elements.len(),
                actual: tuple.0.len(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate each element
        let mut result = ValidationResult::success(self.has_holes);
        for (i, (&item_id, &elem_schema)) in tuple.0.iter().zip(schema.elements.iter()).enumerate()
        {
            self.push_path_tuple_index(i as u8);
            let item_result = self.validate(item_id, elem_schema);
            result.merge(item_result);
            self.pop_path();
        }

        result
    }

    // =========================================================================
    // Union Type Validation
    // =========================================================================

    fn validate_union(&mut self, node: &Node, schema: &UnionSchema) -> ValidationResult {
        match &schema.repr {
            VariantRepr::External => self.validate_union_external(node, schema),
            VariantRepr::Internal { tag } => self.validate_union_internal(node, schema, tag),
            VariantRepr::Adjacent { tag, content } => {
                self.validate_union_adjacent(node, schema, tag, content)
            }
            VariantRepr::Untagged => self.validate_union_untagged(node, schema),
        }
    }

    fn validate_union_external(&mut self, node: &Node, schema: &UnionSchema) -> ValidationResult {
        // External representation in Eure uses $variant extension
        // Example: { $variant = "circle", radius = 5.0 }
        // Nested unions: { $variant = "ok.some.left", value = 42 }

        // Check for $variant extension
        if let Some(tag) = self.get_extension_as_string(node, &identifiers::VARIANT) {
            // Parse as variant path for nested union support
            let path: VariantPath = tag.as_str().into();
            return self.validate_union_with_path(node, schema, &path);
        }

        // No $variant extension found - try untagged matching as fallback
        // (This allows literal variants like `integer` shorthand to work)
        self.validate_union_untagged(node, schema)
    }

    /// Validate a union with a variant path (supports nested unions).
    ///
    /// For a path like `ok.some.left`:
    /// 1. Find the "ok" variant in the current union schema
    /// 2. If "ok"'s schema is also a union, recursively validate with "some.left"
    /// 3. Continue until the path is exhausted
    fn validate_union_with_path(
        &mut self,
        node: &Node,
        schema: &UnionSchema,
        path: &VariantPath,
    ) -> ValidationResult {
        let Some((first, rest)) = path.split_first() else {
            // Empty path - shouldn't happen, but handle gracefully
            return self.validate_union_untagged(node, schema);
        };

        // Find the variant schema for the first segment
        let Some(&variant_schema_id) = schema.variants.get(first) else {
            return ValidationResult::failure(ValidationError::InvalidVariantTag {
                tag: path.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        };

        // Push path segment
        if let Ok(ident) = first.parse::<Identifier>() {
            self.push_path_ident(ident);
        } else {
            self.push_path_key(ObjectKey::String(first.to_string()));
        }

        let variant_schema_node = self.schema.node(variant_schema_id);

        let result = if let Some(rest_path) = rest {
            // There are more segments - the variant must be a union
            match &variant_schema_node.content {
                SchemaNodeContent::Union(inner_union) => {
                    // Recursively validate with the remaining path
                    self.validate_union_with_path(node, inner_union, &rest_path)
                }
                SchemaNodeContent::Reference(type_ref) => {
                    // Resolve the reference and check if it's a union
                    // TODO: Cross-schema references not yet supported for nested unions
                    if let Some(resolved_id) = self.schema.get_type(&type_ref.name) {
                        let resolved_node = self.schema.node(resolved_id);
                        if let SchemaNodeContent::Union(inner_union) = &resolved_node.content {
                            self.validate_union_with_path(node, inner_union, &rest_path)
                        } else {
                            // Not a union - remaining path is invalid
                            ValidationResult::failure(ValidationError::InvalidVariantTag {
                                tag: path.to_string(),
                                path: self.current_path(),
                                node_id: self.node_id(),
                                schema_node_id: Some(variant_schema_id),
                            })
                        }
                    } else {
                        // Unresolved reference
                        ValidationResult::failure(ValidationError::InvalidVariantTag {
                            tag: path.to_string(),
                            path: self.current_path(),
                            node_id: self.node_id(),
                            schema_node_id: Some(variant_schema_id),
                        })
                    }
                }
                _ => {
                    // Not a union - remaining path is invalid
                    ValidationResult::failure(ValidationError::InvalidVariantTag {
                        tag: path.to_string(),
                        path: self.current_path(),
                        node_id: self.node_id(),
                        schema_node_id: Some(variant_schema_id),
                    })
                }
            }
        } else {
            // No more segments - validate the node against this variant's schema
            self.validate_content(node, &variant_schema_node.content, variant_schema_id)
        };

        self.pop_path();
        result
    }

    fn validate_union_internal(
        &mut self,
        node: &Node,
        schema: &UnionSchema,
        tag_field: &str,
    ) -> ValidationResult {
        // Internal representation: { type = "text", content = "Hello" }
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "union (internal)".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        // Get the tag value
        let tag_key = ObjectKey::String(tag_field.to_string());
        let tag_node_id = match map.0.get(&tag_key) {
            Some(&id) => id,
            None => {
                return ValidationResult::failure(ValidationError::MissingRequiredField {
                    field: tag_field.to_string(),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        let tag_node = self.document.node(tag_node_id);
        let tag = match &tag_node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => t.as_str().to_string(),
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "string tag".to_string(),
                    actual: node_type_name(&tag_node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        if let Some(&variant_schema) = schema.variants.get(&tag) {
            // Validate the entire node against the variant schema
            self.validate_content(
                node,
                &self.schema.node(variant_schema).content,
                variant_schema,
            )
        } else {
            ValidationResult::failure(ValidationError::InvalidVariantTag {
                tag,
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            })
        }
    }

    fn validate_union_adjacent(
        &mut self,
        node: &Node,
        schema: &UnionSchema,
        tag_field: &str,
        content_field: &str,
    ) -> ValidationResult {
        // Adjacent representation: { kind = "login", data = { username = "alice" } }
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "union (adjacent)".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        // Get the tag value
        let tag_key = ObjectKey::String(tag_field.to_string());
        let tag_node_id = match map.0.get(&tag_key) {
            Some(&id) => id,
            None => {
                return ValidationResult::failure(ValidationError::MissingRequiredField {
                    field: tag_field.to_string(),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        let tag_node = self.document.node(tag_node_id);
        let tag = match &tag_node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => t.as_str().to_string(),
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "string tag".to_string(),
                    actual: node_type_name(&tag_node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        // Get the content value
        let content_key = ObjectKey::String(content_field.to_string());
        let content_node_id = match map.0.get(&content_key) {
            Some(&id) => id,
            None => {
                return ValidationResult::failure(ValidationError::MissingRequiredField {
                    field: content_field.to_string(),
                    path: self.current_path(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        };

        if let Some(&variant_schema) = schema.variants.get(&tag) {
            // Push content field as identifier if valid, otherwise as string key
            if let Ok(ident) = content_field.parse::<Identifier>() {
                self.push_path_ident(ident);
            } else {
                self.push_path_key(ObjectKey::String(content_field.to_string()));
            }
            let result = self.validate(content_node_id, variant_schema);
            self.pop_path();
            result
        } else {
            ValidationResult::failure(ValidationError::InvalidVariantTag {
                tag,
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            })
        }
    }

    fn validate_union_untagged(&mut self, node: &Node, schema: &UnionSchema) -> ValidationResult {
        // Untagged: try each variant, exactly one must match
        let mut matching: Vec<String> = Vec::new();
        let mut failures: Vec<(String, ValidationError)> = Vec::new();

        for (name, &variant_schema) in &schema.variants {
            let result = self.validate_content(
                node,
                &self.schema.node(variant_schema).content,
                variant_schema,
            );
            if result.is_valid {
                matching.push(name.clone());
            } else if let Some(err) = result.errors.into_iter().next() {
                failures.push((name.clone(), err));
            }
        }

        match matching.len() {
            0 => {
                // No match - return closest error
                ValidationResult::failure(ValidationError::NoVariantMatched {
                    path: self.current_path(),
                    variant_errors: failures,
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                })
            }
            1 => ValidationResult::success(self.has_holes),
            _ => {
                // Multiple matches - check priority
                if let Some(priority) = &schema.priority {
                    for name in priority {
                        if matching.contains(name) {
                            return ValidationResult::success(self.has_holes);
                        }
                    }
                }
                ValidationResult::failure(ValidationError::AmbiguousUnion {
                    path: self.current_path(),
                    variants: matching,
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                })
            }
        }
    }

    // =========================================================================
    // Type Reference Validation
    // =========================================================================

    fn validate_reference(&mut self, node: &Node, type_ref: &TypeReference) -> ValidationResult {
        // Only handle local references for now
        if type_ref.namespace.is_some() {
            return ValidationResult::failure(ValidationError::UndefinedTypeReference {
                name: format!("{}.{}", type_ref.namespace.as_ref().unwrap(), type_ref.name),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Look up the type in the schema's types map
        if let Some(&schema_id) = self.schema.types.get(&type_ref.name) {
            self.validate_content(node, &self.schema.node(schema_id).content, schema_id)
        } else {
            ValidationResult::failure(ValidationError::UndefinedTypeReference {
                name: type_ref.name.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            })
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get a descriptive name for a node's content type
fn node_type_name(content: &NodeValue) -> String {
    match content {
        NodeValue::Hole(_) => "hole".to_string(),
        NodeValue::Primitive(p) => match p {
            PrimitiveValue::Null => "null".to_string(),
            PrimitiveValue::Bool(_) => "boolean".to_string(),
            PrimitiveValue::Integer(_) => "integer".to_string(),
            PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => "float".to_string(),
            PrimitiveValue::Text(_) => "text".to_string(),
        },
        NodeValue::Array(_) => "array".to_string(),
        NodeValue::Tuple(_) => "tuple".to_string(),
        NodeValue::Map(_) => "map".to_string(),
    }
}

/// Extract a subtree as a new EureDocument for comparison purposes
fn node_subtree_to_document(document: &EureDocument, node_id: NodeId) -> EureDocument {
    let mut new_doc = EureDocument::new();
    let root_id = new_doc.get_root_id();
    copy_node_to(document, &mut new_doc, root_id, node_id);
    new_doc
}

/// Recursively copy a node from source document to destination
fn copy_node_to(
    src: &EureDocument,
    dest: &mut EureDocument,
    dest_node_id: NodeId,
    src_node_id: NodeId,
) {
    let src_node = src.node(src_node_id);

    // Collect child info before mutating dest
    let (children, is_map) = match &src_node.content {
        NodeValue::Hole(_) | NodeValue::Primitive(_) => {
            dest.set_content(dest_node_id, src_node.content.clone());
            return;
        }
        NodeValue::Array(arr) => {
            dest.set_content(dest_node_id, NodeValue::empty_array());
            (arr.0.to_vec(), false)
        }
        NodeValue::Tuple(tup) => {
            dest.set_content(dest_node_id, NodeValue::empty_tuple());
            (tup.0.to_vec(), false)
        }
        NodeValue::Map(map) => {
            dest.set_content(dest_node_id, NodeValue::empty_map());
            (map.0.values().copied().collect::<Vec<_>>(), true)
        }
    };

    // Collect extension info
    let extensions_to_copy: Vec<_> = src_node
        .extensions
        .iter()
        .map(|(k, &v)| (k.clone(), v))
        .collect();

    // Copy children based on type
    let src_node = src.node(src_node_id);
    match &src_node.content {
        NodeValue::Array(_) => {
            for child_id in children {
                let new_child_id = dest.add_array_element(None, dest_node_id).unwrap().node_id;
                copy_node_to(src, dest, new_child_id, child_id);
            }
        }
        NodeValue::Tuple(_) => {
            for (index, child_id) in children.into_iter().enumerate() {
                let new_child_id = dest
                    .add_tuple_element(index as u8, dest_node_id)
                    .unwrap()
                    .node_id;
                copy_node_to(src, dest, new_child_id, child_id);
            }
        }
        NodeValue::Map(map) if is_map => {
            for (key, &child_id) in map.0.iter() {
                let new_child_id = dest
                    .add_map_child(key.clone(), dest_node_id)
                    .unwrap()
                    .node_id;
                copy_node_to(src, dest, new_child_id, child_id);
            }
        }
        _ => {}
    }

    // Copy extensions
    for (ext_name, ext_node_id) in extensions_to_copy {
        let new_ext_id = dest.add_extension(ext_name, dest_node_id).unwrap().node_id;
        copy_node_to(src, dest, new_ext_id, ext_node_id);
    }
}

/// Check if an integer value satisfies a bound
fn check_integer_bound(value: &BigInt, bound: &Bound<BigInt>, is_min: bool) -> bool {
    match bound {
        Bound::Unbounded => true,
        Bound::Inclusive(b) => {
            if is_min {
                value >= b
            } else {
                value <= b
            }
        }
        Bound::Exclusive(b) => {
            if is_min {
                value > b
            } else {
                value < b
            }
        }
    }
}

/// Check if a float value satisfies a bound
fn check_float_bound(value: f64, bound: &Bound<f64>, is_min: bool) -> bool {
    match bound {
        Bound::Unbounded => true,
        Bound::Inclusive(b) => {
            if is_min {
                value >= *b
            } else {
                value <= *b
            }
        }
        Bound::Exclusive(b) => {
            if is_min {
                value > *b
            } else {
                value < *b
            }
        }
    }
}

/// Check if all nodes in an array are unique
fn are_nodes_unique(document: &EureDocument, node_ids: &[NodeId]) -> bool {
    // Compare by creating temporary documents for each node
    let docs: Vec<_> = node_ids
        .iter()
        .map(|&id| node_subtree_to_document(document, id))
        .collect();
    // FIXME: Inefficient and not idiomatic.
    for i in 0..docs.len() {
        for j in (i + 1)..docs.len() {
            if docs[i] == docs[j] {
                return false;
            }
        }
    }
    true
}

/// Convert an ObjectKey to a document for validation
fn object_key_to_document(key: &ObjectKey) -> EureDocument {
    match key {
        ObjectKey::Bool(b) => EureDocument::new_primitive(PrimitiveValue::Bool(*b)),
        ObjectKey::Number(n) => EureDocument::new_primitive(PrimitiveValue::Integer(n.clone())),
        ObjectKey::String(s) => EureDocument::new_primitive(PrimitiveValue::Text(
            eure_document::text::Text::plaintext(s.clone()),
        )),
        ObjectKey::Tuple(t) => {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();
            doc.set_content(root_id, NodeValue::empty_tuple());
            for (i, item) in t.0.iter().enumerate() {
                let child_id = doc.add_tuple_element(i as u8, root_id).unwrap().node_id;
                let child_doc = object_key_to_document(item);
                copy_node_to(&child_doc, &mut doc, child_id, child_doc.get_root_id());
            }
            doc
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Validate an Eure document against a schema document
///
/// # Arguments
///
/// * `document` - The Eure document to validate
/// * `schema` - The schema document to validate against
///
/// # Returns
///
/// A `ValidationResult` containing:
/// - `is_valid`: true if there are no type errors (holes are allowed)
/// - `is_complete`: true if there are no type errors AND no holes
/// - `errors`: list of validation errors
/// - `warnings`: list of warnings (e.g., deprecated fields, unknown extensions)
///
/// # Example
///
/// ```ignore
/// use eure_schema::validate::validate;
/// use eure_document::document::EureDocument;
///
/// let schema = // ... load or convert schema
/// let document = // ... parse document
/// let result = validate(&document, &schema);
///
/// if result.is_valid {
///     println!("Document is valid!");
///     if result.is_complete {
///         println!("Document is also complete (no holes)");
///     }
/// } else {
///     for error in &result.errors {
///         println!("Error: {}", error);
///     }
/// }
/// ```
pub fn validate(document: &EureDocument, schema: &SchemaDocument) -> ValidationResult {
    let mut validator = Validator::new(document, schema);
    validator.validate(document.get_root_id(), schema.root)
}

/// Validate a specific node in the document against a specific schema node
///
/// This is useful when you want to validate a specific part of the document
/// against a specific type defined in the schema.
///
/// # Arguments
///
/// * `document` - The Eure document
/// * `node_id` - The ID of the node to validate
/// * `schema` - The schema document
/// * `schema_id` - The ID of the schema node to validate against
pub fn validate_node(
    document: &EureDocument,
    node_id: NodeId,
    schema: &SchemaDocument,
    schema_id: SchemaNodeId,
) -> ValidationResult {
    let mut validator = Validator::new(document, schema);
    validator.validate(node_id, schema_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArraySchema, IntegerSchema, SchemaDocument, SchemaNodeContent, TextSchema, TupleSchema,
    };
    use eure_document::document::node::NodeValue;
    use eure_document::text::Text;
    use num_bigint::BigInt;

    fn create_simple_schema(content: SchemaNodeContent) -> (SchemaDocument, SchemaNodeId) {
        let mut schema = SchemaDocument::new();
        let id = schema.create_node(content);
        schema.root = id;
        (schema, id)
    }

    fn create_doc_with_primitive(value: PrimitiveValue) -> EureDocument {
        EureDocument::new_primitive(value)
    }

    #[test]
    fn test_validate_any() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Any);
        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
        assert!(result.is_complete);
    }

    #[test]
    fn test_validate_hole() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Any);
        let mut doc = EureDocument::new();
        doc.node_mut(doc.get_root_id()).content = NodeValue::hole();

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
        assert!(!result.is_complete); // Holes make the document incomplete
    }

    #[test]
    fn test_validate_text_basic() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema::default()));

        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(42)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_text_length() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema {
            min_length: Some(3),
            max_length: Some(10),
            ..Default::default()
        }));

        // Too short
        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("ab".to_string())));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);

        // Just right
        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        // Too long
        let doc = create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext(
            "hello world!".to_string(),
        )));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_text_pattern() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema {
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        }));

        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext(
            "Hello123".to_string(),
        )));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_integer() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Integer(IntegerSchema {
            min: Bound::Inclusive(BigInt::from(0)),
            max: Bound::Inclusive(BigInt::from(100)),
            multiple_of: None,
        }));

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(50)));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(-1)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(101)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_integer_multiple_of() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Integer(IntegerSchema {
            min: Bound::Unbounded,
            max: Bound::Unbounded,
            multiple_of: Some(BigInt::from(5)),
        }));

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(15)));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(13)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_float() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Float(FloatSchema {
            min: Bound::Inclusive(0.0),
            max: Bound::Exclusive(1.0),
            multiple_of: None,
        }));

        let doc = create_doc_with_primitive(PrimitiveValue::F64(0.5));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::F64(-0.1));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::F64(1.0));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid); // Exclusive bound
    }

    #[test]
    fn test_validate_boolean() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Boolean);

        let doc = create_doc_with_primitive(PrimitiveValue::Bool(true));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("true".to_string())));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_null() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Null);

        let doc = create_doc_with_primitive(PrimitiveValue::Null);
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Bool(false));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_literal() {
        let expected = EureDocument::new_primitive(PrimitiveValue::Text(Text::plaintext(
            "active".to_string(),
        )));
        let (schema, _) = create_simple_schema(SchemaNodeContent::Literal(expected));

        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("active".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext(
            "inactive".to_string(),
        )));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    /// Helper to create an array document
    fn create_array_doc(values: Vec<PrimitiveValue>) -> EureDocument {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.set_content(root, NodeValue::empty_array());
        for val in values {
            let child = doc.add_array_element(None, root).unwrap().node_id;
            doc.set_content(child, NodeValue::Primitive(val));
        }
        doc
    }

    #[test]
    fn test_validate_array() {
        let mut schema = SchemaDocument::new();
        let item_schema = schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let array_schema = schema.create_node(SchemaNodeContent::Array(ArraySchema {
            item: item_schema,
            min_length: Some(1),
            max_length: Some(3),
            unique: false,
            contains: None,
            binding_style: None,
        }));
        schema.root = array_schema;

        // Valid array
        let doc = create_array_doc(vec![
            PrimitiveValue::Integer(BigInt::from(1)),
            PrimitiveValue::Integer(BigInt::from(2)),
        ]);
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        // Too short
        let doc = create_array_doc(vec![]);
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);

        // Too long
        let doc = create_array_doc(vec![
            PrimitiveValue::Integer(BigInt::from(1)),
            PrimitiveValue::Integer(BigInt::from(2)),
            PrimitiveValue::Integer(BigInt::from(3)),
            PrimitiveValue::Integer(BigInt::from(4)),
        ]);
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_array_unique() {
        let mut schema = SchemaDocument::new();
        let item_schema = schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let array_schema = schema.create_node(SchemaNodeContent::Array(ArraySchema {
            item: item_schema,
            min_length: None,
            max_length: None,
            unique: true,
            contains: None,
            binding_style: None,
        }));
        schema.root = array_schema;

        // Unique values
        let doc = create_array_doc(vec![
            PrimitiveValue::Integer(BigInt::from(1)),
            PrimitiveValue::Integer(BigInt::from(2)),
            PrimitiveValue::Integer(BigInt::from(3)),
        ]);
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        // Duplicate values
        let doc = create_array_doc(vec![
            PrimitiveValue::Integer(BigInt::from(1)),
            PrimitiveValue::Integer(BigInt::from(2)),
            PrimitiveValue::Integer(BigInt::from(1)),
        ]);
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    /// Helper to create a tuple document
    fn create_tuple_doc(values: Vec<PrimitiveValue>) -> EureDocument {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.set_content(root, NodeValue::empty_tuple());
        for (i, val) in values.into_iter().enumerate() {
            let child = doc.add_tuple_element(i as u8, root).unwrap().node_id;
            doc.set_content(child, NodeValue::Primitive(val));
        }
        doc
    }

    #[test]
    fn test_validate_tuple() {
        let mut schema = SchemaDocument::new();
        let text_schema = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_schema = schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let tuple_schema = schema.create_node(SchemaNodeContent::Tuple(TupleSchema {
            elements: vec![text_schema, int_schema],
            binding_style: None,
        }));
        schema.root = tuple_schema;

        // Valid tuple
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Text(Text::plaintext("hello".to_string())),
            PrimitiveValue::Integer(BigInt::from(42)),
        ]);
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        // Wrong length
        let doc = create_tuple_doc(vec![PrimitiveValue::Text(Text::plaintext(
            "hello".to_string(),
        ))]);
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);

        // Wrong types
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Integer(BigInt::from(42)),
            PrimitiveValue::Text(Text::plaintext("hello".to_string())),
        ]);
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }
}
