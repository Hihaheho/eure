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
use eure_value::data_model::VariantRepr;
use eure_value::document::node::{Node, NodeValue};
use eure_value::document::{EureDocument, NodeId};
use eure_value::identifier::Identifier;
use eure_value::path::{EurePath, PathSegment};
use eure_value::text::Language;
use eure_value::value::{ObjectKey, PrimitiveValue, Tuple, Value};
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
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidationError {
    #[error("Type mismatch: expected {expected}, got {actual} at path {path}")]
    TypeMismatch {
        expected: String,
        actual: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Missing required field '{field}' at path {path}")]
    MissingRequiredField {
        field: String,
        path: String,
        /// Source node ID (parent node) for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Unknown field '{field}' at path {path}")]
    UnknownField {
        field: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Value {value} is out of range at path {path}")]
    OutOfRange {
        value: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("String length {length} is out of bounds at path {path}")]
    StringLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("String does not match pattern '{pattern}' at path {path}")]
    PatternMismatch {
        pattern: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Array length {length} is out of bounds at path {path}")]
    ArrayLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Map size {size} is out of bounds at path {path}")]
    MapSizeOutOfBounds {
        size: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Tuple length mismatch: expected {expected}, got {actual} at path {path}")]
    TupleLengthMismatch {
        expected: usize,
        actual: usize,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Array elements must be unique at path {path}")]
    ArrayNotUnique {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Array must contain required element at path {path}")]
    ArrayMissingContains {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("No variant matched for union at path {path}")]
    NoVariantMatched {
        path: String,
        /// Errors from each variant attempt
        variant_errors: Vec<(String, ValidationError)>,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Multiple variants matched for union at path {path}: {variants:?}")]
    AmbiguousUnion {
        path: String,
        variants: Vec<String>,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Invalid variant tag '{tag}' at path {path}")]
    InvalidVariantTag {
        tag: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Missing $variant extension at path {path}")]
    MissingVariantExtension {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Literal value mismatch at path {path}")]
    LiteralMismatch {
        expected: String,
        actual: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Language mismatch: expected {expected}, got {actual} at path {path}")]
    LanguageMismatch {
        expected: String,
        actual: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Invalid key type at path {path}")]
    InvalidKeyType {
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Integer not a multiple of {divisor} at path {path}")]
    NotMultipleOf {
        divisor: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Undefined type reference '{name}' at path {path}")]
    UndefinedTypeReference {
        name: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },

    #[error("Invalid regex pattern '{pattern}': {error}")]
    InvalidRegexPattern {
        pattern: String,
        error: String,
        /// Source node ID for editor diagnostics (may be None for schema errors)
        node_id: Option<NodeId>,
    },

    #[error("Invalid extension type for '{name}' at path {path}")]
    InvalidExtensionType {
        name: String,
        path: String,
        /// Source node ID for editor diagnostics
        node_id: Option<NodeId>,
    },
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
}

impl<'a> Validator<'a> {
    fn new(document: &'a EureDocument, schema: &'a SchemaDocument) -> Self {
        Self {
            schema,
            document,
            path: EurePath::root(),
            has_holes: false,
            current_node_id: None,
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
        // Handle hole values first - they match any schema but mark document incomplete
        if let NodeValue::Primitive(PrimitiveValue::Hole) = &node.content {
            self.has_holes = true;
            return ValidationResult::success(true);
        }

        // Handle uninitialized nodes as holes too
        if let NodeValue::Uninitialized = &node.content {
            self.has_holes = true;
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

        // Warn about unknown extensions (except well-known ones like $variant)
        for (ext_ident, _) in &node.extensions {
            // Skip well-known extensions used for variant discrimination
            if ext_ident == &identifiers::VARIANT {
                continue;
            }
            // Unknown extensions are allowed but generate warnings
            result.add_warning(ValidationWarning::UnknownExtension {
                name: ext_ident.to_string(),
                path: self.current_path(),
            });
        }

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
                    });
                }
            };
            if !regex.is_match(text.as_str()) {
                return ValidationResult::failure(ValidationError::PatternMismatch {
                    pattern: pattern.clone(),
                    path: self.current_path(),
                    node_id: self.node_id(),
                });
            }
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_integer(&mut self, node: &Node, schema: &IntegerSchema) -> ValidationResult {
        let int_val = match &node.content {
            NodeValue::Primitive(PrimitiveValue::BigInt(i)) => i,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                });
            }
        };

        // Validate range
        if !check_integer_bound(int_val, &schema.min, true) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
            });
        }
        if !check_integer_bound(int_val, &schema.max, false) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
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
            });
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_float(&mut self, node: &Node, schema: &FloatSchema) -> ValidationResult {
        let float_val = match &node.content {
            NodeValue::Primitive(PrimitiveValue::F64(f)) => *f,
            NodeValue::Primitive(PrimitiveValue::F32(f)) => *f as f64,
            NodeValue::Primitive(PrimitiveValue::BigInt(i)) => {
                // Allow integer to be coerced to float
                if let Ok(i64_val) = i64::try_from(i) {
                    i64_val as f64
                } else {
                    return ValidationResult::failure(ValidationError::TypeMismatch {
                        expected: "float".to_string(),
                        actual: "integer (too large)".to_string(),
                        path: self.current_path(),
                        node_id: self.node_id(),
                    });
                }
            }
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "float".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.current_path(),
                    node_id: self.node_id(),
                });
            }
        };

        // Validate range
        if !check_float_bound(float_val, &schema.min, true) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
            });
        }
        if !check_float_bound(float_val, &schema.max, false) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.current_path(),
                node_id: self.node_id(),
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
            }),
        }
    }

    fn validate_literal(&mut self, node: &Node, expected: &Value) -> ValidationResult {
        let actual = node_to_value(self.document, node);
        if values_equal(&actual, expected) {
            ValidationResult::success(self.has_holes)
        } else {
            ValidationResult::failure(ValidationError::LiteralMismatch {
                expected: format!("{:?}", expected),
                actual: format!("{:?}", actual),
                path: self.current_path(),
                node_id: self.node_id(),
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
            });
        }

        // Validate uniqueness
        if schema.unique {
            let values: Vec<Value> = arr
                .0
                .iter()
                .map(|&id| node_to_value(self.document, self.document.node(id)))
                .collect();
            if !are_values_unique(&values) {
                return ValidationResult::failure(ValidationError::ArrayNotUnique {
                    path: self.current_path(),
                    node_id: self.node_id(),
                });
            }
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
            });
        }

        // Validate each key-value pair
        let mut result = ValidationResult::success(self.has_holes);
        for (key, &val_id) in map.0.iter() {
            self.push_path_key(key.clone());

            // Validate key (using temp node since key is not from document)
            let key_value = object_key_to_value(key);
            let key_node = value_to_temp_node(&key_value);
            let key_result = self.validate_temp_node(
                &key_node,
                &self.schema.node(schema.key).content,
                schema.key,
            );
            if !key_result.is_valid {
                result.merge(ValidationResult::failure(ValidationError::InvalidKeyType {
                    path: self.current_path(),
                    node_id: self.node_id(),
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

        // Check for $variant extension
        if let Some(tag) = self.get_extension_as_string(node, &identifiers::VARIANT) {
            if let Some(&variant_schema) = schema.variants.get(&tag) {
                // Validate the node content against the variant schema
                // Push tag as identifier if valid, otherwise as string key
                if let Ok(ident) = tag.parse::<Identifier>() {
                    self.push_path_ident(ident);
                } else {
                    self.push_path_key(ObjectKey::String(tag.clone()));
                }
                let result = self.validate_content(
                    node,
                    &self.schema.node(variant_schema).content,
                    variant_schema,
                );
                self.pop_path();
                return result;
            } else {
                return ValidationResult::failure(ValidationError::InvalidVariantTag {
                    tag,
                    path: self.current_path(),
                    node_id: self.node_id(),
                });
            }
        }

        // Also support PrimitiveValue::Variant for backward compatibility
        if let NodeValue::Primitive(PrimitiveValue::Variant(v)) = &node.content {
            if let Some(&variant_schema) = schema.variants.get(&v.tag) {
                // Push tag as identifier if valid, otherwise as string key
                if let Ok(ident) = v.tag.parse::<Identifier>() {
                    self.push_path_ident(ident);
                } else {
                    self.push_path_key(ObjectKey::String(v.tag.clone()));
                }
                // Convert content to a document for proper validation of complex values
                // Use validate_temp_node since content comes from Value, not the original document
                let content_doc = value_to_document(&v.content);
                let content_node = content_doc.node(content_doc.get_root_id());
                let result = self.validate_temp_node(
                    content_node,
                    &self.schema.node(variant_schema).content,
                    variant_schema,
                );
                self.pop_path();
                return result;
            } else {
                return ValidationResult::failure(ValidationError::InvalidVariantTag {
                    tag: v.tag.clone(),
                    path: self.current_path(),
                    node_id: self.node_id(),
                });
            }
        }

        // No $variant extension found - try untagged matching as fallback
        // (This allows literal variants like `integer` shorthand to work)
        self.validate_union_untagged(node, schema)
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
        NodeValue::Uninitialized => "uninitialized".to_string(),
        NodeValue::Primitive(p) => match p {
            PrimitiveValue::Null => "null".to_string(),
            PrimitiveValue::Bool(_) => "boolean".to_string(),
            PrimitiveValue::BigInt(_) => "integer".to_string(),
            PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => "float".to_string(),
            PrimitiveValue::Text(_) => "text".to_string(),
            PrimitiveValue::Hole => "hole".to_string(),
            PrimitiveValue::Variant(_) => "variant".to_string(),
        },
        NodeValue::Array(_) => "array".to_string(),
        NodeValue::Tuple(_) => "tuple".to_string(),
        NodeValue::Map(_) => "map".to_string(),
    }
}

/// Convert a node to a Value for comparison purposes
fn node_to_value(document: &EureDocument, node: &Node) -> Value {
    match &node.content {
        NodeValue::Uninitialized => Value::Primitive(PrimitiveValue::Null),
        NodeValue::Primitive(p) => Value::Primitive(p.clone()),
        NodeValue::Array(arr) => {
            let values: Vec<Value> = arr
                .0
                .iter()
                .map(|&id| node_to_value(document, document.node(id)))
                .collect();
            Value::Array(eure_value::value::Array(values))
        }
        NodeValue::Tuple(tup) => {
            let values: Vec<Value> = tup
                .0
                .iter()
                .map(|&id| node_to_value(document, document.node(id)))
                .collect();
            Value::Tuple(Tuple(values))
        }
        NodeValue::Map(map) => {
            let entries: std::collections::HashMap<ObjectKey, Value> = map
                .0
                .iter()
                .map(|(k, &id)| (k.clone(), node_to_value(document, document.node(id))))
                .collect();
            Value::Map(eure_value::value::Map(entries.into_iter().collect()))
        }
    }
}

/// Create a temporary node from a value for validation
///
/// Note: For primitive values, this creates a simple node.
/// For complex values (arrays, tuples, maps), this only works correctly
/// when the value will be compared directly (e.g., for literal matching).
/// For schema validation of complex nested values, use `value_to_document` instead.
fn value_to_temp_node(value: &Value) -> Node {
    let content = match value {
        Value::Primitive(p) => NodeValue::Primitive(p.clone()),
        // For complex types, create empty nodes that preserve the type
        // Nested content validation requires using value_to_document
        Value::Array(_) => NodeValue::Array(Default::default()),
        Value::Tuple(_) => NodeValue::Tuple(Default::default()),
        Value::Map(_) => NodeValue::Map(Default::default()),
    };
    Node {
        content,
        extensions: Default::default(),
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

/// Check if two values are equal for literal comparison
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Primitive(pa), Value::Primitive(pb)) => primitives_equal(pa, pb),
        (Value::Array(aa), Value::Array(ab)) => {
            aa.0.len() == ab.0.len()
                && aa
                    .0
                    .iter()
                    .zip(ab.0.iter())
                    .all(|(a, b)| values_equal(a, b))
        }
        (Value::Tuple(ta), Value::Tuple(tb)) => {
            ta.0.len() == tb.0.len()
                && ta
                    .0
                    .iter()
                    .zip(tb.0.iter())
                    .all(|(a, b)| values_equal(a, b))
        }
        (Value::Map(ma), Value::Map(mb)) => {
            ma.0.len() == mb.0.len()
                && ma
                    .0
                    .iter()
                    .all(|(k, v)| mb.0.get(k).is_some_and(|other_v| values_equal(v, other_v)))
        }
        _ => false,
    }
}

/// Check if two primitive values are equal
fn primitives_equal(a: &PrimitiveValue, b: &PrimitiveValue) -> bool {
    match (a, b) {
        (PrimitiveValue::Null, PrimitiveValue::Null) => true,
        (PrimitiveValue::Bool(a), PrimitiveValue::Bool(b)) => a == b,
        (PrimitiveValue::BigInt(a), PrimitiveValue::BigInt(b)) => a == b,
        (PrimitiveValue::F32(a), PrimitiveValue::F32(b)) => (a - b).abs() < f32::EPSILON,
        (PrimitiveValue::F64(a), PrimitiveValue::F64(b)) => (a - b).abs() < f64::EPSILON,
        (PrimitiveValue::Text(a), PrimitiveValue::Text(b)) => a.as_str() == b.as_str(),
        (PrimitiveValue::Hole, PrimitiveValue::Hole) => true,
        (PrimitiveValue::Variant(a), PrimitiveValue::Variant(b)) => {
            a.tag == b.tag && values_equal(&a.content, &b.content)
        }
        _ => false,
    }
}

/// Check if all values in an array are unique
fn are_values_unique(values: &[Value]) -> bool {
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            if values_equal(&values[i], &values[j]) {
                return false;
            }
        }
    }
    true
}

/// Convert an ObjectKey to a Value for validation
fn object_key_to_value(key: &ObjectKey) -> Value {
    match key {
        ObjectKey::Bool(b) => Value::Primitive(PrimitiveValue::Bool(*b)),
        ObjectKey::Number(n) => Value::Primitive(PrimitiveValue::BigInt(n.clone())),
        ObjectKey::String(s) => Value::Primitive(PrimitiveValue::Text(
            eure_value::text::Text::plaintext(s.clone()),
        )),
        ObjectKey::Tuple(t) => {
            let values: Vec<Value> = t.0.iter().map(object_key_to_value).collect();
            Value::Tuple(Tuple(values))
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
/// use eure_value::document::EureDocument;
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

/// Validate a value against a schema (convenience wrapper)
///
/// This converts the value to a temporary document for validation.
/// Use `validate()` directly for better performance with existing documents.
pub fn validate_value(value: &Value, schema: &SchemaDocument) -> ValidationResult {
    let document = value_to_document(value);
    validate(&document, schema)
}

/// Convert a Value to an EureDocument
fn value_to_document(value: &Value) -> EureDocument {
    let mut doc = EureDocument::new();
    let root_id = doc.get_root_id();
    set_node_value(&mut doc, root_id, value);
    doc
}

/// Recursively set a node's value from a Value
fn set_node_value(doc: &mut EureDocument, node_id: NodeId, value: &Value) {
    match value {
        Value::Primitive(p) => {
            doc.node_mut(node_id).content = NodeValue::Primitive(p.clone());
        }
        Value::Array(arr) => {
            doc.node_mut(node_id).content = NodeValue::Array(Default::default());
            for (i, item) in arr.0.iter().enumerate() {
                let child_id = doc.add_array_element(Some(i), node_id).unwrap().node_id;
                set_node_value(doc, child_id, item);
            }
        }
        Value::Tuple(tup) => {
            doc.node_mut(node_id).content = NodeValue::Tuple(Default::default());
            for (i, item) in tup.0.iter().enumerate() {
                let child_id = doc.add_tuple_element(i as u8, node_id).unwrap().node_id;
                set_node_value(doc, child_id, item);
            }
        }
        Value::Map(map) => {
            doc.node_mut(node_id).content = NodeValue::Map(Default::default());
            for (key, val) in map.0.iter() {
                let child_id = doc.add_map_child(key.clone(), node_id).unwrap().node_id;
                set_node_value(doc, child_id, val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SchemaDocument, SchemaNodeContent};
    use eure_value::text::Text;

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
        let doc = create_doc_with_primitive(PrimitiveValue::Hole);

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

        let doc = create_doc_with_primitive(PrimitiveValue::BigInt(BigInt::from(42)));
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

        let doc = create_doc_with_primitive(PrimitiveValue::BigInt(BigInt::from(50)));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::BigInt(BigInt::from(-1)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::BigInt(BigInt::from(101)));
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

        let doc = create_doc_with_primitive(PrimitiveValue::BigInt(BigInt::from(15)));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::BigInt(BigInt::from(13)));
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
        let expected =
            Value::Primitive(PrimitiveValue::Text(Text::plaintext("active".to_string())));
        let (schema, _) = create_simple_schema(SchemaNodeContent::Literal(expected.clone()));

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
        let value = Value::Array(eure_value::value::Array(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(2))),
        ]));
        let result = validate_value(&value, &schema);
        assert!(result.is_valid);

        // Too short
        let value = Value::Array(eure_value::value::Array(vec![]));
        let result = validate_value(&value, &schema);
        assert!(!result.is_valid);

        // Too long
        let value = Value::Array(eure_value::value::Array(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(2))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(3))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(4))),
        ]));
        let result = validate_value(&value, &schema);
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
        let value = Value::Array(eure_value::value::Array(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(2))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(3))),
        ]));
        let result = validate_value(&value, &schema);
        assert!(result.is_valid);

        // Duplicate values
        let value = Value::Array(eure_value::value::Array(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(2))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
        ]));
        let result = validate_value(&value, &schema);
        assert!(!result.is_valid);
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
        let value = Value::Tuple(Tuple(vec![
            Value::Primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string()))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(42))),
        ]));
        let result = validate_value(&value, &schema);
        assert!(result.is_valid);

        // Wrong length
        let value = Value::Tuple(Tuple(vec![Value::Primitive(PrimitiveValue::Text(
            Text::plaintext("hello".to_string()),
        ))]));
        let result = validate_value(&value, &schema);
        assert!(!result.is_valid);

        // Wrong types
        let value = Value::Tuple(Tuple(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(42))),
            Value::Primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string()))),
        ]));
        let result = validate_value(&value, &schema);
        assert!(!result.is_valid);
    }
}
