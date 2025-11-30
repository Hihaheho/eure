//! Document schema validation
//!
//! This module provides functionality to validate Eure values against schema definitions.
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

use crate::{
    ArraySchema, Bound, FloatSchema, IntegerSchema, MapSchema, RecordSchema, SchemaDocument,
    SchemaNodeContent, SchemaNodeId, TextSchema, TupleSchema, TypeReference, UnionSchema,
    UnknownFieldsPolicy,
};
use eure_value::data_model::VariantRepr;
use eure_value::text::Language;
use eure_value::value::{ObjectKey, PrimitiveValue, Tuple, Value};
use num_bigint::BigInt;
use regex::Regex;
use thiserror::Error;

/// Result of validating a value against a schema
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
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidationError {
    #[error("Type mismatch: expected {expected}, got {actual} at path {path}")]
    TypeMismatch {
        expected: String,
        actual: String,
        path: String,
    },

    #[error("Missing required field '{field}' at path {path}")]
    MissingRequiredField { field: String, path: String },

    #[error("Unknown field '{field}' at path {path}")]
    UnknownField { field: String, path: String },

    #[error("Value {value} is out of range at path {path}")]
    OutOfRange { value: String, path: String },

    #[error("String length {length} is out of bounds at path {path}")]
    StringLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
    },

    #[error("String does not match pattern '{pattern}' at path {path}")]
    PatternMismatch { pattern: String, path: String },

    #[error("Array length {length} is out of bounds at path {path}")]
    ArrayLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
    },

    #[error("Map size {size} is out of bounds at path {path}")]
    MapSizeOutOfBounds {
        size: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: String,
    },

    #[error("Tuple length mismatch: expected {expected}, got {actual} at path {path}")]
    TupleLengthMismatch {
        expected: usize,
        actual: usize,
        path: String,
    },

    #[error("Array elements must be unique at path {path}")]
    ArrayNotUnique { path: String },

    #[error("Array must contain required element at path {path}")]
    ArrayMissingContains { path: String },

    #[error("No variant matched for union at path {path}")]
    NoVariantMatched {
        path: String,
        /// Errors from each variant attempt
        variant_errors: Vec<(String, ValidationError)>,
    },

    #[error("Multiple variants matched for union at path {path}: {variants:?}")]
    AmbiguousUnion { path: String, variants: Vec<String> },

    #[error("Invalid variant tag '{tag}' at path {path}")]
    InvalidVariantTag { tag: String, path: String },

    #[error("Literal value mismatch at path {path}")]
    LiteralMismatch {
        expected: String,
        actual: String,
        path: String,
    },

    #[error("Language mismatch: expected {expected}, got {actual} at path {path}")]
    LanguageMismatch {
        expected: String,
        actual: String,
        path: String,
    },

    #[error("Invalid key type at path {path}")]
    InvalidKeyType { path: String },

    #[error("Integer not a multiple of {divisor} at path {path}")]
    NotMultipleOf { divisor: String, path: String },

    #[error("Undefined type reference '{name}' at path {path}")]
    UndefinedTypeReference { name: String, path: String },

    #[error("Invalid regex pattern '{pattern}': {error}")]
    InvalidRegexPattern { pattern: String, error: String },
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
    path: Vec<String>,
    has_holes: bool,
}

impl<'a> Validator<'a> {
    fn new(schema: &'a SchemaDocument) -> Self {
        Self {
            schema,
            path: Vec::new(),
            has_holes: false,
        }
    }

    fn current_path(&self) -> String {
        if self.path.is_empty() {
            "$".to_string()
        } else {
            format!("$.{}", self.path.join("."))
        }
    }

    fn push_path(&mut self, segment: &str) {
        self.path.push(segment.to_string());
    }

    fn push_path_index(&mut self, index: usize) {
        self.path.push(format!("[{}]", index));
    }

    fn pop_path(&mut self) {
        self.path.pop();
    }

    /// Main validation entry point
    fn validate(&mut self, value: &Value, schema_id: SchemaNodeId) -> ValidationResult {
        let node = self.schema.node(schema_id);
        self.validate_content(value, &node.content)
    }

    /// Validate value against schema content
    fn validate_content(&mut self, value: &Value, content: &SchemaNodeContent) -> ValidationResult {
        // Handle hole values first - they match any schema but mark document incomplete
        if let Value::Primitive(PrimitiveValue::Hole) = value {
            self.has_holes = true;
            return ValidationResult::success(true);
        }

        match content {
            SchemaNodeContent::Any => ValidationResult::success(self.has_holes),
            SchemaNodeContent::Text(text_schema) => self.validate_text(value, text_schema),
            SchemaNodeContent::Integer(int_schema) => self.validate_integer(value, int_schema),
            SchemaNodeContent::Float(float_schema) => self.validate_float(value, float_schema),
            SchemaNodeContent::Boolean => self.validate_boolean(value),
            SchemaNodeContent::Null => self.validate_null(value),
            SchemaNodeContent::Literal(expected) => self.validate_literal(value, expected),
            SchemaNodeContent::Array(array_schema) => self.validate_array(value, array_schema),
            SchemaNodeContent::Map(map_schema) => self.validate_map(value, map_schema),
            SchemaNodeContent::Record(record_schema) => self.validate_record(value, record_schema),
            SchemaNodeContent::Tuple(tuple_schema) => self.validate_tuple(value, tuple_schema),
            SchemaNodeContent::Union(union_schema) => self.validate_union(value, union_schema),
            SchemaNodeContent::Reference(type_ref) => self.validate_reference(value, type_ref),
        }
    }

    // =========================================================================
    // Primitive Type Validation
    // =========================================================================

    fn validate_text(&mut self, value: &Value, schema: &TextSchema) -> ValidationResult {
        let text = match value {
            Value::Primitive(PrimitiveValue::Text(t)) => t,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "text".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
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
                        });
                    }
                }
                // Implicit (`...`) can be coerced to any language - always passes
                Language::Implicit => {}
                // Explicit language (lang`...`) must match
                Language::Other(lang) => {
                    if lang != expected_lang {
                        return ValidationResult::failure(ValidationError::LanguageMismatch {
                            expected: expected_lang.clone(),
                            actual: lang.clone(),
                            path: self.current_path(),
                        });
                    }
                }
            }
        }

        // Validate length constraints
        let len = text.as_str().chars().count();
        if let Some(min) = schema.min_length {
            if len < min as usize {
                return ValidationResult::failure(ValidationError::StringLengthOutOfBounds {
                    length: len,
                    min: Some(min),
                    max: schema.max_length,
                    path: self.current_path(),
                });
            }
        }
        if let Some(max) = schema.max_length {
            if len > max as usize {
                return ValidationResult::failure(ValidationError::StringLengthOutOfBounds {
                    length: len,
                    min: schema.min_length,
                    max: Some(max),
                    path: self.current_path(),
                });
            }
        }

        // Validate pattern
        if let Some(pattern) = &schema.pattern {
            let regex = match Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => {
                    return ValidationResult::failure(ValidationError::InvalidRegexPattern {
                        pattern: pattern.clone(),
                        error: e.to_string(),
                    })
                }
            };
            if !regex.is_match(text.as_str()) {
                return ValidationResult::failure(ValidationError::PatternMismatch {
                    pattern: pattern.clone(),
                    path: self.current_path(),
                });
            }
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_integer(&mut self, value: &Value, schema: &IntegerSchema) -> ValidationResult {
        let int_val = match value {
            Value::Primitive(PrimitiveValue::BigInt(i)) => i,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        // Validate range
        if !check_integer_bound(int_val, &schema.min, true) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.current_path(),
            });
        }
        if !check_integer_bound(int_val, &schema.max, false) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.current_path(),
            });
        }

        // Validate multiple-of
        if let Some(divisor) = &schema.multiple_of {
            if int_val % divisor != BigInt::from(0) {
                return ValidationResult::failure(ValidationError::NotMultipleOf {
                    divisor: divisor.to_string(),
                    path: self.current_path(),
                });
            }
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_float(&mut self, value: &Value, schema: &FloatSchema) -> ValidationResult {
        let float_val = match value {
            Value::Primitive(PrimitiveValue::F64(f)) => *f,
            Value::Primitive(PrimitiveValue::F32(f)) => *f as f64,
            Value::Primitive(PrimitiveValue::BigInt(i)) => {
                // Allow integer to be coerced to float
                if let Ok(i64_val) = i64::try_from(i) {
                    i64_val as f64
                } else {
                    return ValidationResult::failure(ValidationError::TypeMismatch {
                        expected: "float".to_string(),
                        actual: "integer (too large)".to_string(),
                        path: self.current_path(),
                    });
                }
            }
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "float".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        // Validate range
        if !check_float_bound(float_val, &schema.min, true) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.current_path(),
            });
        }
        if !check_float_bound(float_val, &schema.max, false) {
            return ValidationResult::failure(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.current_path(),
            });
        }

        // Validate multiple-of
        if let Some(divisor) = &schema.multiple_of {
            if (float_val % divisor).abs() > f64::EPSILON {
                return ValidationResult::failure(ValidationError::NotMultipleOf {
                    divisor: divisor.to_string(),
                    path: self.current_path(),
                });
            }
        }

        ValidationResult::success(self.has_holes)
    }

    fn validate_boolean(&mut self, value: &Value) -> ValidationResult {
        match value {
            Value::Primitive(PrimitiveValue::Bool(_)) => ValidationResult::success(self.has_holes),
            _ => ValidationResult::failure(ValidationError::TypeMismatch {
                expected: "boolean".to_string(),
                actual: value_type_name(value),
                path: self.current_path(),
            }),
        }
    }

    fn validate_null(&mut self, value: &Value) -> ValidationResult {
        match value {
            Value::Primitive(PrimitiveValue::Null) => ValidationResult::success(self.has_holes),
            _ => ValidationResult::failure(ValidationError::TypeMismatch {
                expected: "null".to_string(),
                actual: value_type_name(value),
                path: self.current_path(),
            }),
        }
    }

    fn validate_literal(&mut self, value: &Value, expected: &Value) -> ValidationResult {
        if values_equal(value, expected) {
            ValidationResult::success(self.has_holes)
        } else {
            ValidationResult::failure(ValidationError::LiteralMismatch {
                expected: format!("{:?}", expected),
                actual: format!("{:?}", value),
                path: self.current_path(),
            })
        }
    }

    // =========================================================================
    // Container Type Validation
    // =========================================================================

    fn validate_array(&mut self, value: &Value, schema: &ArraySchema) -> ValidationResult {
        let arr = match value {
            Value::Array(a) => a,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "array".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        let len = arr.0.len();

        // Validate length constraints
        if let Some(min) = schema.min_length {
            if len < min as usize {
                return ValidationResult::failure(ValidationError::ArrayLengthOutOfBounds {
                    length: len,
                    min: Some(min),
                    max: schema.max_length,
                    path: self.current_path(),
                });
            }
        }
        if let Some(max) = schema.max_length {
            if len > max as usize {
                return ValidationResult::failure(ValidationError::ArrayLengthOutOfBounds {
                    length: len,
                    min: schema.min_length,
                    max: Some(max),
                    path: self.current_path(),
                });
            }
        }

        // Validate uniqueness
        if schema.unique && !are_values_unique(&arr.0) {
            return ValidationResult::failure(ValidationError::ArrayNotUnique {
                path: self.current_path(),
            });
        }

        // Validate each item
        let mut result = ValidationResult::success(self.has_holes);
        for (i, item) in arr.0.iter().enumerate() {
            self.push_path_index(i);
            let item_result = self.validate(item, schema.item);
            result.merge(item_result);
            self.pop_path();
        }

        // Validate contains constraint
        if let Some(contains_schema) = schema.contains {
            let mut found = false;
            for item in &arr.0 {
                // Create a temporary validator to test without affecting main state
                let test_result = self.validate(item, contains_schema);
                if test_result.is_valid {
                    found = true;
                    break;
                }
            }
            if !found {
                result.merge(ValidationResult::failure(
                    ValidationError::ArrayMissingContains {
                        path: self.current_path(),
                    },
                ));
            }
        }

        result
    }

    fn validate_map(&mut self, value: &Value, schema: &MapSchema) -> ValidationResult {
        let map = match value {
            Value::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "map".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        let size = map.0.len();

        // Validate size constraints
        if let Some(min) = schema.min_size {
            if size < min as usize {
                return ValidationResult::failure(ValidationError::MapSizeOutOfBounds {
                    size,
                    min: Some(min),
                    max: schema.max_size,
                    path: self.current_path(),
                });
            }
        }
        if let Some(max) = schema.max_size {
            if size > max as usize {
                return ValidationResult::failure(ValidationError::MapSizeOutOfBounds {
                    size,
                    min: schema.min_size,
                    max: Some(max),
                    path: self.current_path(),
                });
            }
        }

        // Validate each key-value pair
        let mut result = ValidationResult::success(self.has_holes);
        for (key, val) in map.0.iter() {
            let key_str = key.to_string();
            self.push_path(&key_str);

            // Validate key
            let key_value = object_key_to_value(key);
            let key_result = self.validate(&key_value, schema.key);
            if !key_result.is_valid {
                result.merge(ValidationResult::failure(ValidationError::InvalidKeyType {
                    path: self.current_path(),
                }));
            }

            // Validate value
            let value_result = self.validate(val, schema.value);
            result.merge(value_result);

            self.pop_path();
        }

        result
    }

    fn validate_record(&mut self, value: &Value, schema: &RecordSchema) -> ValidationResult {
        let map = match value {
            Value::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "record".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
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
                        },
                    ));
                }
            }
        }

        // Validate each field
        for (key, val) in map.0.iter() {
            let field_name = match key {
                ObjectKey::String(s) => s.clone(),
                _ => {
                    result.merge(ValidationResult::failure(ValidationError::InvalidKeyType {
                        path: self.current_path(),
                    }));
                    continue;
                }
            };

            self.push_path(&field_name);

            if let Some(field_schema) = schema.properties.get(&field_name) {
                // Check deprecated
                let node = self.schema.node(field_schema.schema);
                if node.metadata.deprecated {
                    result.add_warning(ValidationWarning::DeprecatedField {
                        field: field_name.clone(),
                        path: self.current_path(),
                    });
                }

                let field_result = self.validate(val, field_schema.schema);
                result.merge(field_result);
            } else {
                // Unknown field - check policy
                match &schema.unknown_fields {
                    UnknownFieldsPolicy::Deny => {
                        result.merge(ValidationResult::failure(ValidationError::UnknownField {
                            field: field_name.clone(),
                            path: self.current_path(),
                        }));
                    }
                    UnknownFieldsPolicy::Allow => {
                        // Allow any value
                    }
                    UnknownFieldsPolicy::Schema(schema_id) => {
                        // Validate against the schema
                        let field_result = self.validate(val, *schema_id);
                        result.merge(field_result);
                    }
                }
            }

            self.pop_path();
        }

        result
    }

    fn validate_tuple(&mut self, value: &Value, schema: &TupleSchema) -> ValidationResult {
        let tuple = match value {
            Value::Tuple(t) => t,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "tuple".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        // Check length matches
        if tuple.0.len() != schema.elements.len() {
            return ValidationResult::failure(ValidationError::TupleLengthMismatch {
                expected: schema.elements.len(),
                actual: tuple.0.len(),
                path: self.current_path(),
            });
        }

        // Validate each element
        let mut result = ValidationResult::success(self.has_holes);
        for (i, (item, &elem_schema)) in tuple.0.iter().zip(schema.elements.iter()).enumerate() {
            self.push_path_index(i);
            let item_result = self.validate(item, elem_schema);
            result.merge(item_result);
            self.pop_path();
        }

        result
    }

    // =========================================================================
    // Union Type Validation
    // =========================================================================

    fn validate_union(&mut self, value: &Value, schema: &UnionSchema) -> ValidationResult {
        match &schema.repr {
            VariantRepr::External => self.validate_union_external(value, schema),
            VariantRepr::Internal { tag } => self.validate_union_internal(value, schema, tag),
            VariantRepr::Adjacent { tag, content } => {
                self.validate_union_adjacent(value, schema, tag, content)
            }
            VariantRepr::Untagged => self.validate_union_untagged(value, schema),
        }
    }

    fn validate_union_external(&mut self, value: &Value, schema: &UnionSchema) -> ValidationResult {
        // External representation: the value is a map with a single key being the variant name
        // Example: { circle = { radius = 5.0 } }
        let map = match value {
            Value::Map(m) => m,
            // Also handle Variant type directly
            Value::Primitive(PrimitiveValue::Variant(v)) => {
                // Check if variant name matches
                if let Some(&variant_schema) = schema.variants.get(&v.tag) {
                    self.push_path(&v.tag);
                    let result = self.validate(&v.content, variant_schema);
                    self.pop_path();
                    return result;
                } else {
                    return ValidationResult::failure(ValidationError::InvalidVariantTag {
                        tag: v.tag.clone(),
                        path: self.current_path(),
                    });
                }
            }
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "union (external)".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        // Must have exactly one key
        if map.0.len() != 1 {
            return ValidationResult::failure(ValidationError::TypeMismatch {
                expected: "union with single variant key".to_string(),
                actual: format!("map with {} keys", map.0.len()),
                path: self.current_path(),
            });
        }

        let (key, val) = map.0.iter().next().unwrap();
        let tag = match key {
            ObjectKey::String(s) => s.clone(),
            _ => {
                return ValidationResult::failure(ValidationError::InvalidKeyType {
                    path: self.current_path(),
                })
            }
        };

        if let Some(&variant_schema) = schema.variants.get(&tag) {
            self.push_path(&tag);
            let result = self.validate(val, variant_schema);
            self.pop_path();
            result
        } else {
            ValidationResult::failure(ValidationError::InvalidVariantTag {
                tag,
                path: self.current_path(),
            })
        }
    }

    fn validate_union_internal(
        &mut self,
        value: &Value,
        schema: &UnionSchema,
        tag_field: &str,
    ) -> ValidationResult {
        // Internal representation: { type = "text", content = "Hello" }
        let map = match value {
            Value::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "union (internal)".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        // Get the tag value
        let tag_key = ObjectKey::String(tag_field.to_string());
        let tag_value = match map.0.get(&tag_key) {
            Some(v) => v,
            None => {
                return ValidationResult::failure(ValidationError::MissingRequiredField {
                    field: tag_field.to_string(),
                    path: self.current_path(),
                })
            }
        };

        let tag = match tag_value {
            Value::Primitive(PrimitiveValue::Text(t)) => t.as_str().to_string(),
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "string tag".to_string(),
                    actual: value_type_name(tag_value),
                    path: self.current_path(),
                })
            }
        };

        if let Some(&variant_schema) = schema.variants.get(&tag) {
            // Validate the entire map against the variant schema
            // The variant schema should be a record that includes all fields except the tag
            self.validate(value, variant_schema)
        } else {
            ValidationResult::failure(ValidationError::InvalidVariantTag {
                tag,
                path: self.current_path(),
            })
        }
    }

    fn validate_union_adjacent(
        &mut self,
        value: &Value,
        schema: &UnionSchema,
        tag_field: &str,
        content_field: &str,
    ) -> ValidationResult {
        // Adjacent representation: { kind = "login", data = { username = "alice" } }
        let map = match value {
            Value::Map(m) => m,
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "union (adjacent)".to_string(),
                    actual: value_type_name(value),
                    path: self.current_path(),
                })
            }
        };

        // Get the tag value
        let tag_key = ObjectKey::String(tag_field.to_string());
        let tag_value = match map.0.get(&tag_key) {
            Some(v) => v,
            None => {
                return ValidationResult::failure(ValidationError::MissingRequiredField {
                    field: tag_field.to_string(),
                    path: self.current_path(),
                })
            }
        };

        let tag = match tag_value {
            Value::Primitive(PrimitiveValue::Text(t)) => t.as_str().to_string(),
            _ => {
                return ValidationResult::failure(ValidationError::TypeMismatch {
                    expected: "string tag".to_string(),
                    actual: value_type_name(tag_value),
                    path: self.current_path(),
                })
            }
        };

        // Get the content value
        let content_key = ObjectKey::String(content_field.to_string());
        let content_value = match map.0.get(&content_key) {
            Some(v) => v,
            None => {
                return ValidationResult::failure(ValidationError::MissingRequiredField {
                    field: content_field.to_string(),
                    path: self.current_path(),
                })
            }
        };

        if let Some(&variant_schema) = schema.variants.get(&tag) {
            self.push_path(content_field);
            let result = self.validate(content_value, variant_schema);
            self.pop_path();
            result
        } else {
            ValidationResult::failure(ValidationError::InvalidVariantTag {
                tag,
                path: self.current_path(),
            })
        }
    }

    fn validate_union_untagged(&mut self, value: &Value, schema: &UnionSchema) -> ValidationResult {
        // Untagged: try each variant, exactly one must match
        let mut matching: Vec<String> = Vec::new();
        let mut failures: Vec<(String, ValidationError)> = Vec::new();

        for (name, &variant_schema) in &schema.variants {
            let result = self.validate(value, variant_schema);
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
                })
            }
        }
    }

    // =========================================================================
    // Type Reference Validation
    // =========================================================================

    fn validate_reference(&mut self, value: &Value, type_ref: &TypeReference) -> ValidationResult {
        // Only handle local references for now
        // External references would require import resolution
        if type_ref.namespace.is_some() {
            // For now, external references are not resolved
            // They would need to be inlined during schema bundling
            return ValidationResult::failure(ValidationError::UndefinedTypeReference {
                name: format!(
                    "{}.{}",
                    type_ref.namespace.as_ref().unwrap(),
                    type_ref.name
                ),
                path: self.current_path(),
            });
        }

        // Look up the type in the schema's types map
        if let Some(&schema_id) = self.schema.types.get(&type_ref.name) {
            self.validate(value, schema_id)
        } else {
            ValidationResult::failure(ValidationError::UndefinedTypeReference {
                name: type_ref.name.to_string(),
                path: self.current_path(),
            })
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get a descriptive name for a value's type
fn value_type_name(value: &Value) -> String {
    match value {
        Value::Primitive(p) => match p {
            PrimitiveValue::Null => "null".to_string(),
            PrimitiveValue::Bool(_) => "boolean".to_string(),
            PrimitiveValue::BigInt(_) => "integer".to_string(),
            PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => "float".to_string(),
            PrimitiveValue::Text(_) => "text".to_string(),
            PrimitiveValue::Hole => "hole".to_string(),
            PrimitiveValue::Variant(_) => "variant".to_string(),
        },
        Value::Array(_) => "array".to_string(),
        Value::Tuple(_) => "tuple".to_string(),
        Value::Map(_) => "map".to_string(),
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
                && aa.0.iter().zip(ab.0.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Tuple(ta), Value::Tuple(tb)) => {
            ta.0.len() == tb.0.len()
                && ta.0.iter().zip(tb.0.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Map(ma), Value::Map(mb)) => {
            ma.0.len() == mb.0.len()
                && ma.0.iter().all(|(k, v)| {
                    mb.0.get(k)
                        .map_or(false, |other_v| values_equal(v, other_v))
                })
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
    // Use a simple O(n^2) comparison for now
    // Could be optimized with hashing for primitive types
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

/// Validate a value against a schema document
///
/// # Arguments
///
/// * `value` - The value to validate
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
/// use eure_value::value::Value;
///
/// let schema = // ... load or convert schema
/// let value = // ... parse value
/// let result = validate(&value, &schema);
///
/// if result.is_valid {
///     println!("Value is valid!");
///     if result.is_complete {
///         println!("Value is also complete (no holes)");
///     }
/// } else {
///     for error in &result.errors {
///         println!("Error: {}", error);
///     }
/// }
/// ```
pub fn validate(value: &Value, schema: &SchemaDocument) -> ValidationResult {
    let mut validator = Validator::new(schema);
    validator.validate(value, schema.root)
}

/// Validate a value against a specific schema node
///
/// This is useful when you want to validate a value against a specific type
/// defined in the schema, rather than the root type.
///
/// # Arguments
///
/// * `value` - The value to validate
/// * `schema` - The schema document
/// * `schema_id` - The ID of the schema node to validate against
pub fn validate_against(
    value: &Value,
    schema: &SchemaDocument,
    schema_id: SchemaNodeId,
) -> ValidationResult {
    let mut validator = Validator::new(schema);
    validator.validate(value, schema_id)
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

    #[test]
    fn test_validate_any() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Any);

        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "hello".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(result.is_valid);
        assert!(result.is_complete);
    }

    #[test]
    fn test_validate_hole() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Any);

        let value = Value::Primitive(PrimitiveValue::Hole);
        let result = validate(&value, &schema);
        assert!(result.is_valid);
        assert!(!result.is_complete); // Holes make the document incomplete
    }

    #[test]
    fn test_validate_text_basic() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema::default()));

        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "hello".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::BigInt(BigInt::from(42)));
        let result = validate(&value, &schema);
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
        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext("ab".to_string())));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);

        // Just right
        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "hello".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        // Too long
        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "hello world!".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_text_pattern() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema {
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        }));

        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "hello".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "Hello123".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_integer() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Integer(IntegerSchema {
            min: Bound::Inclusive(BigInt::from(0)),
            max: Bound::Inclusive(BigInt::from(100)),
            multiple_of: None,
        }));

        let value = Value::Primitive(PrimitiveValue::BigInt(BigInt::from(50)));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::BigInt(BigInt::from(-1)));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);

        let value = Value::Primitive(PrimitiveValue::BigInt(BigInt::from(101)));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_integer_multiple_of() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Integer(IntegerSchema {
            min: Bound::Unbounded,
            max: Bound::Unbounded,
            multiple_of: Some(BigInt::from(5)),
        }));

        let value = Value::Primitive(PrimitiveValue::BigInt(BigInt::from(15)));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::BigInt(BigInt::from(13)));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_float() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Float(FloatSchema {
            min: Bound::Inclusive(0.0),
            max: Bound::Exclusive(1.0),
            multiple_of: None,
        }));

        let value = Value::Primitive(PrimitiveValue::F64(0.5));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::F64(-0.1));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);

        let value = Value::Primitive(PrimitiveValue::F64(1.0));
        let result = validate(&value, &schema);
        assert!(!result.is_valid); // Exclusive bound
    }

    #[test]
    fn test_validate_boolean() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Boolean);

        let value = Value::Primitive(PrimitiveValue::Bool(true));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "true".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_null() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Null);

        let value = Value::Primitive(PrimitiveValue::Null);
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::Bool(false));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_literal() {
        let expected = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "active".to_string(),
        )));
        let (schema, _) = create_simple_schema(SchemaNodeContent::Literal(expected.clone()));

        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "active".to_string(),
        )));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        let value = Value::Primitive(PrimitiveValue::Text(Text::plaintext(
            "inactive".to_string(),
        )));
        let result = validate(&value, &schema);
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
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        // Too short
        let value = Value::Array(eure_value::value::Array(vec![]));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);

        // Too long
        let value = Value::Array(eure_value::value::Array(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(2))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(3))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(4))),
        ]));
        let result = validate(&value, &schema);
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
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        // Duplicate values
        let value = Value::Array(eure_value::value::Array(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(2))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(1))),
        ]));
        let result = validate(&value, &schema);
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
            Value::Primitive(PrimitiveValue::Text(Text::plaintext(
                "hello".to_string(),
            ))),
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(42))),
        ]));
        let result = validate(&value, &schema);
        assert!(result.is_valid);

        // Wrong length
        let value = Value::Tuple(Tuple(vec![Value::Primitive(PrimitiveValue::Text(
            Text::plaintext("hello".to_string()),
        ))]));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);

        // Wrong types
        let value = Value::Tuple(Tuple(vec![
            Value::Primitive(PrimitiveValue::BigInt(BigInt::from(42))),
            Value::Primitive(PrimitiveValue::Text(Text::plaintext(
                "hello".to_string(),
            ))),
        ]));
        let result = validate(&value, &schema);
        assert!(!result.is_valid);
    }
}
