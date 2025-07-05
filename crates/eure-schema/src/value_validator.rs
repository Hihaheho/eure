//! Value-based schema validation
//! 
//! This module provides simple validation of EURE Values against schemas.

use crate::schema::*;
use crate::utils::path_segments_to_display_string;
use eure_value::value::{Value, Map, KeyCmpValue, Array, Tuple, PathSegment};
use eure_tree::tree::InputSpan;
use std::collections::HashSet;
use ahash::AHashMap;
use std::fmt;
use regex::Regex;

/// Severity of validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

/// Types of validation errors
#[derive(Debug, Clone)]
pub enum ValidationErrorKind {
    // Type errors
    TypeMismatch {
        expected: String,
        actual: String,
    },
    UnknownType(String),

    // Field errors
    RequiredFieldMissing {
        field: KeyCmpValue,
        path: Vec<PathSegment>,
    },
    UnexpectedField {
        field: KeyCmpValue,
        path: Vec<PathSegment>,
    },

    // Constraint violations
    StringLengthViolation {
        min: Option<usize>,
        max: Option<usize>,
        actual: usize,
    },
    StringPatternViolation {
        pattern: String,
        value: String,
    },
    NumberRangeViolation {
        min: Option<f64>,
        max: Option<f64>,
        actual: f64,
    },
    ArrayLengthViolation {
        min: Option<usize>,
        max: Option<usize>,
        actual: usize,
    },

    // Variant errors
    VariantDiscriminatorMissing,
    InvalidVariantDiscriminator(String),
    UnknownVariant {
        variant: String,
        available: Vec<String>,
    },
    MissingVariantTag,
    
    // Schema validation errors
    InvalidSchemaPattern {
        pattern: String,
        error: String,
    },
    ArrayUniqueViolation {
        duplicate: String,
    },
    
    // Preference violations (warnings)
    PreferSection {
        path: Vec<PathSegment>,
    },
    PreferArraySyntax {
        path: Vec<PathSegment>,
    },
    
    // Other errors
    InvalidValue(String),
    InternalError(String),
    HoleExists {
        path: Vec<PathSegment>,
    },
}

/// A validation error with severity
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub kind: ValidationErrorKind,
    pub severity: Severity,
    pub span: Option<InputSpan>,
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationErrorKind::TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {expected}, but got {actual}")
            }
            ValidationErrorKind::UnknownType(type_name) => {
                write!(f, "Unknown type: {type_name}")
            }
            ValidationErrorKind::RequiredFieldMissing { field, path } => {
                let location = if path.is_empty() {
                    String::new()
                } else {
                    format!(" at {}", path_segments_to_display_string(path))
                };
                write!(f, "Required field {field:?} is missing{location}")
            }
            ValidationErrorKind::UnexpectedField { field, path } => {
                let location = if path.is_empty() {
                    String::new()
                } else {
                    format!(" at {}", path_segments_to_display_string(path))
                };
                write!(f, "Unexpected field {field:?}{location} not defined in schema")
            }
            ValidationErrorKind::StringLengthViolation { min, max, actual } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "String length must be between {min} and {max} characters, but got {actual}"),
                    (Some(min), None) => write!(f, "String must be at least {min} characters long, but got {actual}"),
                    (None, Some(max)) => write!(f, "String length exceeds maximum {max} characters, but got {actual}"),
                    (None, None) => write!(f, "String length violation"),
                }
            }
            ValidationErrorKind::StringPatternViolation { pattern, value } => {
                write!(f, "String '{value}' does not match pattern /{pattern}/")
            }
            ValidationErrorKind::NumberRangeViolation { min, max, actual } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "Number must be between {min} and {max}, but got {actual}"),
                    (Some(min), None) => write!(f, "Number must be at least {min}, but got {actual}"),
                    (None, Some(max)) => write!(f, "Number exceeds maximum {max}, but got {actual}"),
                    (None, None) => write!(f, "Number range violation"),
                }
            }
            ValidationErrorKind::ArrayLengthViolation { min, max, actual } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "Array must have between {min} and {max} items, but has {actual}"),
                    (Some(min), None) => write!(f, "Array must have at least {min} items, but has {actual}"),
                    (None, Some(max)) => write!(f, "Array exceeds maximum {max} items, but has {actual}"),
                    (None, None) => write!(f, "Array length violation"),
                }
            }
            ValidationErrorKind::VariantDiscriminatorMissing => {
                write!(f, "Variant discriminator field '$variant' is missing")
            }
            ValidationErrorKind::InvalidVariantDiscriminator(value) => {
                write!(f, "Invalid variant discriminator: {value}")
            }
            ValidationErrorKind::UnknownVariant { variant, available } => {
                if available.is_empty() {
                    write!(f, "Unknown variant '{variant}'")
                } else {
                    write!(f, "Unknown variant '{variant}'. Available variants: {}", available.join(", "))
                }
            }
            ValidationErrorKind::MissingVariantTag => {
                write!(f, "Missing variant tag field '$variant'")
            }
            ValidationErrorKind::InvalidSchemaPattern { pattern, error } => {
                write!(f, "Invalid regex pattern '{pattern}': {error}")
            }
            ValidationErrorKind::ArrayUniqueViolation { duplicate } => {
                write!(f, "Array contains duplicate value: {duplicate}")
            }
            ValidationErrorKind::PreferSection { path } => {
                let path_str = path_segments_to_display_string(path);
                write!(f, "Consider using section syntax for '{path_str}' instead of inline binding")
            }
            ValidationErrorKind::PreferArraySyntax { path } => {
                let path_str = path_segments_to_display_string(path);
                write!(f, "Consider using array syntax [] for '{path_str}' instead of repeated fields")
            }
            ValidationErrorKind::InvalidValue(msg) => {
                write!(f, "Invalid value: {msg}")
            }
            ValidationErrorKind::InternalError(msg) => {
                write!(f, "Internal error: {msg}")
            }
            ValidationErrorKind::HoleExists { path } => {
                let path_str = path_segments_to_display_string(path);
                write!(f, "Hole value (!) found at '{path_str}' - holes must be filled with actual values")
            }
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity, self.kind)
    }
}

/// Validate a document Value against a schema
pub fn validate_document(doc: &Value, schema: &DocumentSchema) -> Vec<ValidationError> {
    let mut context = ValidationContext {
        schema,
        errors: Vec::new(),
        path: Vec::new(),
        // required_fields_stack: Vec::new(),
    };
    
    // Start validation from the root
    if let Value::Map(map) = doc {
        validate_object_against_schema(map, &schema.root, &mut context);
    } else {
        context.add_error(ValidationErrorKind::TypeMismatch {
            expected: "object".to_string(),
            actual: value_type_name(doc).to_string(),
        });
    }
    
    context.errors
}

/// Validation context that tracks errors and current path
struct ValidationContext<'a> {
    schema: &'a DocumentSchema,
    errors: Vec<ValidationError>,
    path: Vec<PathSegment>,
    // required_fields_stack: Vec<HashSet<String>>, // TODO: Implement nested required field tracking
}

impl<'a> ValidationContext<'a> {
    fn add_error(&mut self, kind: ValidationErrorKind) {
        self.errors.push(ValidationError {
            kind,
            severity: Severity::Error,
            span: None, // Value-based validation doesn't have spans
        });
    }
    
    fn with_path<F, R>(&mut self, segment: PathSegment, f: F) -> R 
    where F: FnOnce(&mut Self) -> R 
    {
        self.path.push(segment);
        let result = f(self);
        self.path.pop();
        result
    }
}

/// Extract the actual value from a possibly wrapped value
/// If the value is a map containing only schema keys and a _value key,
/// return the _value. Otherwise return the original value.
fn extract_actual_value(value: &Value) -> &Value {
    if let Value::Map(map) = value {
        // Check if this map has _value and only schema-related keys
        let has_value_key = map.0.contains_key(&KeyCmpValue::String("_value".to_string()));
        let only_schema_keys = map.0.keys().all(|k| {
            matches!(k, KeyCmpValue::String(s) if s == "_value") ||
            matches!(k, KeyCmpValue::Extension(_)) ||
            matches!(k, KeyCmpValue::MetaExtension(_))
        });
        
        if has_value_key && only_schema_keys {
            // Return the _value
            if let Some(actual_value) = map.0.get(&KeyCmpValue::String("_value".to_string())) {
                return actual_value;
            }
        }
    }
    value
}

/// Validate a Map against an ObjectSchema
fn validate_object_against_schema(
    map: &Map, 
    schema: &ObjectSchema,
    context: &mut ValidationContext
) {
    // Track required fields
    let mut required_fields: HashSet<KeyCmpValue> = schema.fields
        .iter()
        .filter(|(_, field)| !field.optional)
        .map(|(name, _)| name.clone())
        .collect();
    
    // Validate each field in the map
    for (key, value) in &map.0 {
        match key {
            KeyCmpValue::Extension(_) | KeyCmpValue::MetaExtension(_) => {
                // Skip extension keys during validation - they are schema metadata
                continue;
            }
            KeyCmpValue::String(field_name) => {
                // Create appropriate PathSegment based on field name
                use eure_value::identifier::Identifier;
                use std::str::FromStr;
                let path_segment = PathSegment::Ident(Identifier::from_str(field_name).unwrap_or_else(|_| Identifier::from_str("unknown").unwrap()));
                
                // Mark field as seen
                required_fields.remove(&KeyCmpValue::String(field_name.clone()));
                
                // Find schema for this field
                if let Some(field_schema) = schema.fields.get(&KeyCmpValue::String(field_name.clone())) {
                    context.with_path(path_segment, |ctx| {
                        // Extract the actual value (unwrap from _value if needed)
                        let actual_value = extract_actual_value(value);
                        validate_value_against_type(actual_value, &field_schema.type_expr, ctx);
                        
                        // Validate constraints
                        validate_constraints(actual_value, &field_schema.constraints, ctx);
                    });
                } else {
                    // Field not in schema - check cascade type
                    if let Some(cascade_type) = &context.schema.cascade_type {
                        context.with_path(path_segment, |ctx| {
                            let actual_value = extract_actual_value(value);
                            validate_value_against_type(actual_value, cascade_type, ctx);
                        });
                    } else if schema.additional_properties.is_none() {
                        // Unexpected field
                        context.add_error(ValidationErrorKind::UnexpectedField {
                            field: KeyCmpValue::String(field_name.clone()),
                            path: context.path.clone(),
                        });
                    }
                }
            }
            _ => {
                // Non-string keys are not supported in schemas
                continue;
            }
        }
    }
    
    // Check for missing required fields
    for missing_field in required_fields {
        context.add_error(ValidationErrorKind::RequiredFieldMissing {
            field: missing_field,
            path: context.path.clone(),
        });
    }
}

/// Validate a Value against a Type
fn validate_value_against_type(
    value: &Value,
    expected_type: &Type,
    context: &mut ValidationContext
) {
    // Check for holes first - they should always be reported regardless of expected type
    if let Value::Hole = value {
        context.add_error(ValidationErrorKind::HoleExists {
            path: context.path.clone(),
        });
        return;
    }
    
    match (value, expected_type) {
        // Primitive types
        (Value::String(_), Type::String) => {},
        (Value::I64(_) | Value::U64(_) | Value::F64(_) | Value::F32(_), Type::Number) => {},
        (Value::Bool(_), Type::Boolean) => {},
        (Value::Null, Type::Null) => {},
        
        // Any type matches anything
        (_, Type::Any) => {},
        
        // Path type
        (Value::Path(_), Type::Path) => {},
        
        // TypedString
        (Value::String(_), Type::TypedString(_)) => {
            // TODO: Validate typed string format
        },
        (Value::TypedString(_ts), Type::TypedString(_)) => {
            // TODO: Validate typed string type matches
        },
        
        // Code
        (Value::String(_), Type::Code(_)) => {},
        (Value::Code(code), Type::Code(expected_lang)) => {
            if !expected_lang.is_empty() && code.language != *expected_lang {
                context.add_error(ValidationErrorKind::TypeMismatch {
                    expected: format!("code.{expected_lang}"),
                    actual: format!("code.{}", code.language),
                });
            }
        },
        
        // Arrays
        (Value::Array(array), Type::Array(elem_type)) => {
            validate_array(array, elem_type, context);
        },
        (Value::Tuple(tuple), Type::Array(elem_type)) => {
            validate_tuple(tuple, elem_type, context);
        },
        
        // Objects
        (Value::Map(map), Type::Object(obj_schema)) => {
            validate_object_against_schema(map, obj_schema, context);
        },
        
        // Type references
        (value, Type::TypeRef(type_name)) => {
            if let Some(type_def) = context.schema.types.get(type_name) {
                validate_value_against_type(value, &type_def.type_expr, context);
                // Also validate constraints from the type definition
                validate_constraints(value, &type_def.constraints, context);
            } else {
                let type_name_str = match type_name {
                    KeyCmpValue::String(s) => s.clone(),
                    _ => format!("{type_name:?}"),
                };
                context.add_error(ValidationErrorKind::UnknownType(type_name_str));
            }
        },
        
        // Variants
        (Value::Map(map), Type::Variants(variant_schema)) => {
            validate_variant(map, variant_schema, context);
        },
        
        // Union types
        (value, Type::Union(types)) => {
            // Try each type in the union
            let mut all_errors = Vec::new();
            for union_type in types {
                let mut temp_context = ValidationContext {
                    schema: context.schema,
                    errors: Vec::new(),
                    path: context.path.clone(),
                    // required_fields_stack: Vec::new(),
                };
                validate_value_against_type(value, union_type, &mut temp_context);
                if temp_context.errors.is_empty() {
                    // Found a matching type
                    return;
                }
                all_errors.extend(temp_context.errors);
            }
            // None of the union types matched
            context.add_error(ValidationErrorKind::TypeMismatch {
                expected: format!("union of {} types", types.len()),
                actual: value_type_name(value).to_string(),
            });
        },
        
        // Cascade types - validate the inner type
        (value, Type::CascadeType(inner_type)) => {
            validate_value_against_type(value, inner_type, context);
        },
        
        // Type mismatch
        _ => {
            context.add_error(ValidationErrorKind::TypeMismatch {
                expected: type_to_string(expected_type),
                actual: value_type_name(value).to_string(),
            });
        }
    }
}

/// Validate an array
fn validate_array(
    array: &Array,
    elem_type: &Type,
    context: &mut ValidationContext
) {
    for (index, element) in array.0.iter().enumerate() {
        context.with_path(PathSegment::Value(KeyCmpValue::U64(index as u64)), |ctx| {
            validate_value_against_type(element, elem_type, ctx);
        });
    }
}

/// Validate a tuple
fn validate_tuple(
    tuple: &Tuple<Value>,
    elem_type: &Type,
    context: &mut ValidationContext
) {
    for (index, element) in tuple.0.iter().enumerate() {
        if index > 255 {
            context.add_error(ValidationErrorKind::InvalidValue(
                format!("Tuple index {index} exceeds maximum of 255")
            ));
            break;
        }
        
        context.with_path(PathSegment::TupleIndex(index as u8), |ctx| {
            validate_value_against_type(element, elem_type, ctx);
        });
    }
}

/// Validate a variant value
fn validate_variant(
    map: &Map,
    variant_schema: &VariantSchema,
    context: &mut ValidationContext
) {
    // Look for $variant field
    let variant_name = match map.0.get(&KeyCmpValue::Extension("variant".to_string())) {
        Some(Value::String(name)) => name,
        _ => {
            context.add_error(ValidationErrorKind::TypeMismatch {
                expected: "variant with $variant field".to_string(),
                actual: "object without $variant".to_string(),
            });
            return;
        }
    };
    
    // Find the variant schema
    if let Some(variant_obj_schema) = variant_schema.variants.get(&KeyCmpValue::String(variant_name.clone())) {
        // Create a filtered map without the $variant field and other extension keys
        let mut filtered_map = AHashMap::new();
        for (k, v) in &map.0 {
            match k {
                KeyCmpValue::String(_) => {
                    // Include all string keys (data fields)
                    filtered_map.insert(k.clone(), v.clone());
                }
                KeyCmpValue::Extension(_) | KeyCmpValue::MetaExtension(_) => {
                    // Skip all extension keys
                }
                _ => {}
            }
        }
        
        // Validate against the variant's object schema
        validate_object_against_schema(&Map(filtered_map), variant_obj_schema, context);
    } else {
        let available: Vec<String> = variant_schema.variants.keys()
            .filter_map(|k| match k {
                KeyCmpValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        context.add_error(ValidationErrorKind::UnknownVariant {
            variant: variant_name.clone(),
            available,
        });
    }
}

/// Validate constraints on a value
fn validate_constraints(
    value: &Value,
    constraints: &Constraints,
    context: &mut ValidationContext
) {
    match value {
        Value::String(s) => {
            // String length constraints
            if let Some((min, max)) = &constraints.length {
                let len = s.len();
                if let Some(min) = min
                    && len < *min {
                        context.add_error(ValidationErrorKind::StringLengthViolation {
                            min: Some(*min),
                            max: *max,
                            actual: len,
                        });
                    }
                if let Some(max) = max
                    && len > *max {
                        context.add_error(ValidationErrorKind::StringLengthViolation {
                            min: *min,
                            max: Some(*max),
                            actual: len,
                        });
                    }
            }
            
            // Pattern constraint
            if let Some(pattern) = &constraints.pattern {
                match Regex::new(pattern) {
                    Ok(re) => {
                        if !re.is_match(s) {
                            context.add_error(ValidationErrorKind::StringPatternViolation {
                                pattern: pattern.clone(),
                                value: s.clone(),
                            });
                        }
                    }
                    Err(_) => {
                        // Invalid regex pattern - this should be caught during schema validation
                        context.add_error(ValidationErrorKind::InvalidValue(
                            format!("Invalid regex pattern: {pattern}")
                        ));
                    }
                }
            }
        }
        
        Value::I64(n) => validate_number_constraints(*n as f64, constraints, context),
        Value::U64(n) => validate_number_constraints(*n as f64, constraints, context),
        Value::F32(n) => validate_number_constraints(*n as f64, constraints, context),
        Value::F64(n) => validate_number_constraints(*n, constraints, context),
        
        Value::Array(array) => {
            // Array length constraints
            let len = array.0.len();
            if let Some(min) = constraints.min_items
                && len < min {
                    context.add_error(ValidationErrorKind::ArrayLengthViolation {
                        min: Some(min),
                        max: constraints.max_items,
                        actual: len,
                    });
                }
            if let Some(max) = constraints.max_items
                && len > max {
                    context.add_error(ValidationErrorKind::ArrayLengthViolation {
                        min: constraints.min_items,
                        max: Some(max),
                        actual: len,
                    });
                }
        }
        
        _ => {}
    }
}

/// Validate number constraints
fn validate_number_constraints(
    value: f64,
    constraints: &Constraints,
    context: &mut ValidationContext
) {
    if let Some((min, max)) = &constraints.range {
        if let Some(min) = min
            && value < *min {
                context.add_error(ValidationErrorKind::NumberRangeViolation {
                    min: Some(*min),
                    max: *max,
                    actual: value,
                });
            }
        if let Some(max) = max
            && value > *max {
                context.add_error(ValidationErrorKind::NumberRangeViolation {
                    min: *min,
                    max: Some(*max),
                    actual: value,
                });
            }
    }
    
    if let Some(exclusive_min) = constraints.exclusive_min
        && value <= exclusive_min {
            context.add_error(ValidationErrorKind::NumberRangeViolation {
                min: Some(exclusive_min),
                max: None,
                actual: value,
            });
        }
    
    if let Some(exclusive_max) = constraints.exclusive_max
        && value >= exclusive_max {
            context.add_error(ValidationErrorKind::NumberRangeViolation {
                min: None,
                max: Some(exclusive_max),
                actual: value,
            });
        }
}

/// Get the type name of a value for error messages
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::I64(_) | Value::U64(_) | Value::F32(_) | Value::F64(_) => "number",
        Value::String(_) => "string",
        Value::TypedString(_) => "typed-string",
        Value::Code(_) => "code",
        Value::Array(_) => "array",
        Value::Tuple(_) => "tuple",
        Value::Map(_) => "object",
        Value::Variant(_) => "variant",
        Value::Unit => "unit",
        Value::Path(_) => "path",
        Value::Hole => "hole",
    }
}

/// Convert a Type to string for error messages
fn type_to_string(t: &Type) -> String {
    match t {
        Type::String => "string".to_string(),
        Type::Number => "number".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Null => "null".to_string(),
        Type::Any => "any".to_string(),
        Type::Path => "path".to_string(),
        Type::TypedString(kind) => format!("typed-string.{kind:?}"),
        Type::Code(lang) => {
            if lang.is_empty() {
                "code".to_string()
            } else {
                format!("code.{lang}")
            }
        },
        Type::Array(_) => "array".to_string(),
        Type::Object(_) => "object".to_string(),
        Type::Union(_) => "union".to_string(),
        Type::Variants(_) => "variant".to_string(),
        Type::TypeRef(name) => match name {
            KeyCmpValue::String(s) => format!("${s}"),
            _ => format!("${name:?}"),
        },
        Type::CascadeType(_) => "cascade".to_string(),
    }
}

