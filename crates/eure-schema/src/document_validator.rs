//! Document-based schema validation
//!
//! This module provides validation of EureDocument against schemas
//! using a simple recursive approach without CST visitors.
//!
//! # Variant Validation Algorithm
//!
//! The variant validation system supports four different representation strategies,
//! each with its own detection and validation logic:
//!
//! ## 1. Tagged Representation
//! 
//! In tagged representation, the variant is determined by a single object key:
//! ```eure
//! @ command {
//!   echo {
//!     message = "Hello"
//!   }
//! }
//! ```
//! Or using the `$variant` extension:
//! ```eure
//! @ command {
//!   $variant: echo
//!   message = "Hello"
//! }
//! ```
//!
//! **Detection**: Look for a single key matching a variant name, or check for `$variant` extension
//! **Validation**: Validate the content under the variant key against the variant's schema
//!
//! ## 2. Internally Tagged Representation
//!
//! The variant is determined by a specific field within the object:
//! ```eure
//! @ event {
//!   type = "click"  # Tag field determines variant
//!   x = 100
//!   y = 200
//! }
//! ```
//!
//! **Detection**: Look for the configured tag field and use its value as the variant name
//! **Validation**: Validate all fields (including the tag) against the variant's schema
//!
//! ## 3. Adjacently Tagged Representation
//!
//! The variant tag and content are in separate fields:
//! ```eure
//! @ message {
//!   type = "text"      # Tag field
//!   data = {           # Content field
//!     content = "Hello"
//!     formatted = true
//!   }
//! }
//! ```
//!
//! **Detection**: Look for the tag field to determine variant
//! **Validation**: Validate the content field's value against the variant's schema
//!
//! ## 4. Untagged Representation
//!
//! The variant is determined by attempting to match the structure:
//! ```eure
//! @ value {
//!   text = "Hello"    # Matches 'text' variant by structure
//!   lang = "en"
//! }
//! ```
//!
//! **Detection**: Try each variant schema until one validates without errors
//! **Validation**: Use the first variant that matches successfully
//!
//! ## Variant Context Tracking
//!
//! The validator maintains a `variant_context` map to track which variant was selected
//! at each path in the document. This is crucial for:
//! - Validating nested fields within variants
//! - Providing accurate error messages
//! - Handling cascade types that apply to variant fields
//!
//! ## Edge Cases and Limitations
//!
//! 1. **Ambiguous Untagged Variants**: When multiple variants could match, the first
//!    valid one is chosen. Order matters in the schema definition.
//!
//! 2. **Recursive Variants**: Currently no depth limit is enforced, which could lead
//!    to stack overflow with deeply nested variant structures.
//!
//! 3. **Performance**: Untagged variant validation creates temporary validators for
//!    each variant attempt, which can be expensive for complex schemas.
//!
//! 4. **Error Reporting**: For untagged variants, if no variant matches, the error
//!    messages may not clearly indicate which variant was expected.

use crate::identifiers;
use crate::schema::*;
use eure_tree::document::{EureDocument, Node, NodeValue, NodeId, DocumentKey};
use eure_value::identifier::Identifier;
use eure_value::value::{KeyCmpValue, PathSegment, PathKey};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::fmt;

/// Severity level for validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Validation error information
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub kind: ValidationErrorKind,
    pub severity: Severity,
    pub node_id: NodeId,
}

/// Different kinds of validation errors
#[derive(Debug, Clone)]
pub enum ValidationErrorKind {
    TypeMismatch {
        expected: String,
        actual: String,
    },
    RequiredFieldMissing {
        field: KeyCmpValue,
        path: Vec<PathSegment>,
    },
    UnexpectedField {
        field: KeyCmpValue,
        path: Vec<PathSegment>,
    },
    InvalidValue(String),
    PatternMismatch {
        pattern: String,
        value: String,
    },
    RangeViolation {
        min: Option<f64>,
        max: Option<f64>,
        value: f64,
    },
    StringLengthViolation {
        min: Option<usize>,
        max: Option<usize>,
        length: usize,
    },
    ArrayLengthViolation {
        min: Option<usize>,
        max: Option<usize>,
        length: usize,
    },
    UnknownType(String),
    UnknownVariant {
        variant: String,
        available: Vec<String>,
    },
    VariantDiscriminatorMissing,
    HoleExists {
        path: Vec<PathSegment>,
    },
    MaxDepthExceeded {
        depth: usize,
        max_depth: usize,
    },
}

/// Validate an EureDocument against a schema
pub fn validate_document(
    document: &EureDocument,
    schema: &DocumentSchema,
) -> Vec<ValidationError> {
    let mut validator = DocumentValidator::new(document, schema);
    validator.validate();
    validator.errors
}

/// Information about a detected variant
#[derive(Debug, Clone)]
struct VariantInfo {
    /// The name of the detected variant
    variant_name: Identifier,
    /// The key used to look up the variant in the schema
    variant_key: KeyCmpValue,
    /// How the variant was detected
    detection_source: VariantDetectionSource,
}

/// Describes how a variant was detected
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum VariantDetectionSource {
    /// Variant was determined from a $variant extension
    Extension,
    /// Variant was determined from object key matching variant name (Tagged repr)
    Tagged,
    /// Variant was determined from a tag field value (InternallyTagged repr)
    InternalTag(String),
    /// Variant was determined from structure matching (Untagged repr)
    Untagged,
}

/// Internal validator state
struct DocumentValidator<'a> {
    document: &'a EureDocument,
    schema: &'a DocumentSchema,
    errors: Vec<ValidationError>,
    /// Track which fields have been seen at each path
    seen_fields: HashMap<PathKey, HashSet<KeyCmpValue>>,
    /// Track variant context for proper field validation
    variant_context: HashMap<PathKey, String>,
    /// Track variant representation info for each path (for excluding tag fields)
    variant_repr_context: HashMap<PathKey, VariantRepr>,
    /// Current recursion depth for validation
    current_depth: usize,
    /// Maximum allowed recursion depth (prevents stack overflow)
    max_depth: usize,
}

impl<'a> DocumentValidator<'a> {
    fn new(document: &'a EureDocument, schema: &'a DocumentSchema) -> Self {
        Self {
            document,
            schema,
            errors: Vec::new(),
            seen_fields: HashMap::new(),
            variant_context: HashMap::new(),
            variant_repr_context: HashMap::new(),
            current_depth: 0,
            max_depth: 100, // Default max depth to prevent stack overflow
        }
    }

    fn validate(&mut self) {
        // Check if there's a cascade type for the root
        let root_path_key = PathKey::from_segments(&[]);
        let root_id = self.document.get_root_id();
        
        if let Some(cascade_type) = self.schema.cascade_types.get(&root_path_key) {
            // Check if it's a variant cascade type
            if let Type::Variants(variant_schema) = cascade_type {
                // For any variant cascade type at root, validate as variant
                let root_node = self.document.get_node(root_id);
                self.validate_variant(root_id, root_node, &[], variant_schema);
                return;
            }
        }
        
        // Normal object validation
        self.validate_object_fields(root_id, &[], &self.schema.root);
    }

    fn validate_object_fields(
        &mut self,
        node_id: NodeId,
        path: &[PathSegment],
        object_schema: &ObjectSchema,
    ) {
        let node = self.document.get_node(node_id);

        match &node.content {
            NodeValue::Map { entries, .. } => {
                // Validate map entries
                for (key, child_id) in entries {
                    match key {
                        DocumentKey::Ident(ident) => {
                            self.validate_field(
                                *child_id,
                                path,
                                ident,
                                &object_schema.fields,
                            );
                        }
                        DocumentKey::MetaExtension(ident) => {
                            // Handle meta-extension fields
                            self.handle_meta_extension(*child_id, path, ident);
                        }
                        DocumentKey::Value(key_value) => {
                            // Check if this is a quoted field name that matches a schema field
                            if let KeyCmpValue::String(field_str) = key_value {
                                // Check if this quoted field name matches an expected field
                                if let Some(field_schema) = object_schema.fields.get(&KeyCmpValue::String(field_str.clone())) {
                                    // This is a known field with a quoted name
                                    // Track that we've seen this field
                                    let path_key = PathKey::from_segments(path);
                                    self.seen_fields
                                        .entry(path_key)
                                        .or_default()
                                        .insert(KeyCmpValue::String(field_str.clone()));
                                    
                                    // Validate the field value
                                    let mut field_path = path.to_vec();
                                    field_path.push(PathSegment::Value(key_value.clone()));
                                    self.validate_type_with_constraints(*child_id, &field_path, &field_schema.type_expr, &field_schema.constraints);
                                } else if let Some(additional_properties) = &object_schema.additional_properties {
                                    // Not a known field, check additional properties
                                    let mut child_path = path.to_vec();
                                    child_path.push(PathSegment::Value(key_value.clone()));
                                    self.validate_type(*child_id, &child_path, additional_properties);
                                } else {
                                    // Unexpected field
                                    self.add_error(
                                        node_id,
                                        ValidationErrorKind::UnexpectedField {
                                            field: key_value.clone(),
                                            path: path.to_vec(),
                                        },
                                    );
                                }
                            } else {
                                // Non-string value keys
                                if let Some(additional_properties) = &object_schema.additional_properties {
                                    let mut child_path = path.to_vec();
                                    child_path.push(PathSegment::Value(key_value.clone()));
                                    self.validate_type(*child_id, &child_path, additional_properties);
                                } else {
                                    self.add_error(
                                        node_id,
                                        ValidationErrorKind::UnexpectedField {
                                            field: key_value.clone(),
                                            path: path.to_vec(),
                                        },
                                    );
                                }
                            }
                        }
                        DocumentKey::TupleIndex(_) => {
                            self.add_error(
                                node_id,
                                ValidationErrorKind::InvalidValue(
                                    "Tuple index in map context".to_string()
                                ),
                            );
                        }
                    }
                }

                // Also validate extension nodes
                for (ext_ident, ext_node_id) in &node.extensions {
                    self.handle_extension(*ext_node_id, path, ext_ident);
                }

                // Check for missing required fields after processing all present fields
                // This should only be done for valid object nodes
                self.check_missing_fields(path, &object_schema.fields);
            }
            _ => {
                // Non-map at root or object position
                self.add_error(
                    node_id,
                    ValidationErrorKind::TypeMismatch {
                        expected: "object".to_string(),
                        actual: self.node_type_name(node),
                    },
                );
            }
        }
    }

    fn validate_field(
        &mut self,
        node_id: NodeId,
        path: &[PathSegment],
        field_name: &Identifier,
        expected_fields: &IndexMap<KeyCmpValue, FieldSchema>,
    ) {
        // Check if this is a schema-only field (has schema extensions but no data content)
        let node = self.document.get_node(node_id);
        let is_schema_only = self.is_schema_only_node(node);
        
        // Only track fields that have actual data content
        if !is_schema_only {
            let path_key = PathKey::from_segments(path);
            self.seen_fields
                .entry(path_key)
                .or_default()
                .insert(KeyCmpValue::String(field_name.to_string()));
        }

        let field_key = KeyCmpValue::String(field_name.to_string());
        if let Some(field_schema) = expected_fields.get(&field_key) {
            // Only validate against field schema if this is not a schema-only field
            if !is_schema_only {
                let mut field_path = path.to_vec();
                field_path.push(PathSegment::Ident(field_name.clone()));
                self.validate_type_with_constraints(node_id, &field_path, &field_schema.type_expr, &field_schema.constraints);
            }
        } else if !is_schema_only {
            // Check if this field is a tag field for internally tagged variant
            let path_key = PathKey::from_segments(path);
            let is_tag_field = if let Some(variant_repr) = self.variant_repr_context.get(&path_key) {
                match variant_repr {
                    VariantRepr::InternallyTagged { tag } => {
                        field_key == *tag
                    }
                    VariantRepr::AdjacentlyTagged { tag, content } => {
                        field_key == *tag || field_key == *content
                    }
                    _ => false
                }
            } else {
                false
            };
            
            if is_tag_field {
                // This is a tag field for variant discrimination, not an unexpected field
                // Just skip validation for tag fields
                return;
            }
            
            // Check if there's a cascade type for this path
            if let Some(cascade_type) = self.schema.cascade_types.get(&path_key) {
                // Validate against cascade type
                let mut field_path = path.to_vec();
                field_path.push(PathSegment::Ident(field_name.clone()));
                self.validate_type(node_id, &field_path, cascade_type);
            } else {
                // No cascade type, field is unexpected
                self.add_error(
                    node_id,
                    ValidationErrorKind::UnexpectedField {
                        field: KeyCmpValue::String(field_name.to_string()),
                        path: path.to_vec(),
                    },
                );
            }
        }
    }

    fn validate_type(
        &mut self,
        node_id: NodeId,
        path: &[PathSegment],
        expected_type: &Type,
    ) {
        // Check depth limit to prevent stack overflow
        if self.current_depth >= self.max_depth {
            self.add_error(
                node_id,
                ValidationErrorKind::MaxDepthExceeded {
                    depth: self.current_depth,
                    max_depth: self.max_depth,
                },
            );
            return;
        }

        // Increment depth for this validation
        self.current_depth += 1;
        
        let node = self.document.get_node(node_id);

        // Check for holes first - holes are always invalid except for Type::Any
        if matches!(node.content, NodeValue::Hole { .. }) {
            if !matches!(expected_type, Type::Any) {
                self.add_error(
                    node_id,
                    ValidationErrorKind::HoleExists {
                        path: path.to_vec(),
                    },
                );
                self.current_depth -= 1;
                return;
            }
        }

        match expected_type {
            Type::Null => self.validate_null(node_id, node),
            Type::Boolean => self.validate_bool(node_id, node),
            Type::Number => self.validate_number(node_id, node),
            Type::String => self.validate_string(node_id, node),
            Type::Code(_) => self.validate_code(node_id, node),
            Type::Array(elem_type) => self.validate_array(node_id, node, path, elem_type),
            Type::Tuple(tuple_types) => self.validate_tuple(node_id, node, path, tuple_types),
            Type::Object(object_schema) => {
                self.validate_object(node_id, node, path, object_schema);
            }
            Type::Variants(variant_schema) => {
                self.validate_variant(node_id, node, path, variant_schema);
            }
            Type::TypeRef(type_name) => {
                // Convert Identifier to KeyCmpValue for lookup
                let type_key = KeyCmpValue::String(type_name.to_string());
                if let Some(referenced_type) = self.schema.types.get(&type_key) {
                    self.validate_type_with_constraints(node_id, path, &referenced_type.type_expr, &referenced_type.constraints);
                } else {
                    self.add_error(
                        node_id,
                        ValidationErrorKind::UnknownType(type_name.to_string()),
                    );
                }
            }
            Type::Union(types) => {
                // Try each type in the union
                let mut union_errors = Vec::new();
                for union_type in types {
                    let mut temp_validator = DocumentValidator::new(self.document, self.schema);
                    // Preserve current depth in temp validator
                    temp_validator.current_depth = self.current_depth;
                    temp_validator.max_depth = self.max_depth;
                    temp_validator.validate_type(node_id, path, union_type);
                    if temp_validator.errors.is_empty() {
                        // Valid for this union member
                        self.current_depth -= 1; // Decrement before returning
                        return;
                    }
                    union_errors.extend(temp_validator.errors);
                }
                // None of the union types matched
                self.add_error(
                    node_id,
                    ValidationErrorKind::TypeMismatch {
                        expected: format!("union of {} types", types.len()),
                        actual: self.node_type_name(node),
                    },
                );
            }
            Type::Any => {
                // Any type is always valid, but check for holes
                if matches!(node.content, NodeValue::Hole { .. }) {
                    self.add_error(
                        node_id,
                        ValidationErrorKind::HoleExists {
                            path: path.to_vec(),
                        },
                    );
                }
            }
            Type::Path => self.validate_path(node_id, node),
            Type::CascadeType(inner_type) => {
                // Cascade types validate the inner type
                self.validate_type(node_id, path, inner_type);
            }
        }
        
        // Decrement depth after validation
        self.current_depth -= 1;
    }

    fn validate_type_with_constraints(
        &mut self,
        node_id: NodeId,
        path: &[PathSegment],
        expected_type: &Type,
        constraints: &Constraints,
    ) {
        // First validate the type
        self.validate_type(node_id, path, expected_type);
        
        // Then apply constraints
        let node = self.document.get_node(node_id);
        match (&node.content, expected_type) {
            (NodeValue::String { value, .. }, Type::String) => {
                // Check string length constraints
                if let Some((min, max)) = &constraints.length {
                    let len = value.len();
                    if let Some(min_len) = min
                        && len < *min_len {
                            self.add_error(
                                node_id,
                                ValidationErrorKind::StringLengthViolation {
                                    min: Some(*min_len),
                                    max: *max,
                                    length: len,
                                },
                            );
                            return;
                        }
                    if let Some(max_len) = max
                        && len > *max_len {
                            self.add_error(
                                node_id,
                                ValidationErrorKind::StringLengthViolation {
                                    min: *min,
                                    max: Some(*max_len),
                                    length: len,
                                },
                            );
                            return;
                        }
                }
                
                // Check pattern constraint
                if let Some(pattern) = &constraints.pattern {
                    let re = match regex::Regex::new(pattern) {
                        Ok(re) => re,
                        Err(_) => {
                            self.add_error(
                                node_id,
                                ValidationErrorKind::InvalidValue(
                                    format!("Invalid regex pattern: {pattern}")
                                ),
                            );
                            return;
                        }
                    };
                    if !re.is_match(value) {
                        self.add_error(
                            node_id,
                            ValidationErrorKind::PatternMismatch {
                                pattern: pattern.clone(),
                                value: value.clone(),
                            },
                        );
                    }
                }
            }
            (NodeValue::Array { .. }, Type::Array(_)) => {
                // Array constraints removed - no length constraints for arrays
            }
            (NodeValue::I64 { value, .. }, Type::Number) => {
                self.check_number_constraints(node_id, *value as f64, constraints);
            }
            (NodeValue::U64 { value, .. }, Type::Number) => {
                self.check_number_constraints(node_id, *value as f64, constraints);
            }
            (NodeValue::F32 { value, .. }, Type::Number) => {
                self.check_number_constraints(node_id, *value as f64, constraints);
            }
            (NodeValue::F64 { value, .. }, Type::Number) => {
                self.check_number_constraints(node_id, *value, constraints);
            }
            _ => {
                // No constraints to check for other types
            }
        }
    }
    
    fn check_number_constraints(&mut self, node_id: NodeId, value: f64, constraints: &Constraints) {
        // Check range constraints
        if let Some((min, max)) = &constraints.range {
            if let Some(min_val) = min
                && value < *min_val {
                    self.add_error(
                        node_id,
                        ValidationErrorKind::RangeViolation {
                            min: Some(*min_val),
                            max: *max,
                            value,
                        },
                    );
                    return;
                }
            if let Some(max_val) = max
                && value > *max_val {
                    self.add_error(
                        node_id,
                        ValidationErrorKind::RangeViolation {
                            min: *min,
                            max: Some(*max_val),
                            value,
                        },
                    );
                    return;
                }
        }
        
        // Check exclusive constraints
        if let Some(min_exclusive) = constraints.exclusive_min
            && value <= min_exclusive {
                self.add_error(
                    node_id,
                    ValidationErrorKind::RangeViolation {
                        min: Some(min_exclusive),
                        max: None,
                        value,
                    },
                );
            }
        if let Some(max_exclusive) = constraints.exclusive_max
            && value >= max_exclusive {
                self.add_error(
                    node_id,
                    ValidationErrorKind::RangeViolation {
                        min: None,
                        max: Some(max_exclusive),
                        value,
                    },
                );
            }
    }

    fn validate_null(&mut self, node_id: NodeId, node: &Node) {
        if !matches!(&node.content, NodeValue::Null { .. }) {
            self.add_error(
                node_id,
                ValidationErrorKind::TypeMismatch {
                    expected: "null".to_string(),
                    actual: self.node_type_name(node),
                },
            );
        }
    }

    fn validate_bool(&mut self, node_id: NodeId, node: &Node) {
        if !matches!(&node.content, NodeValue::Bool { .. }) {
            self.add_error(
                node_id,
                ValidationErrorKind::TypeMismatch {
                    expected: "boolean".to_string(),
                    actual: self.node_type_name(node),
                },
            );
        }
    }

    fn validate_number(&mut self, node_id: NodeId, node: &Node) {
        match &node.content {
            NodeValue::I64 { .. } | NodeValue::U64 { .. } |
            NodeValue::F32 { .. } | NodeValue::F64 { .. } => {}
            _ => {
                self.add_error(
                    node_id,
                    ValidationErrorKind::TypeMismatch {
                        expected: "number".to_string(),
                        actual: self.node_type_name(node),
                    },
                );
            }
        }
    }

    fn validate_string(&mut self, node_id: NodeId, node: &Node) {
        if !matches!(&node.content, NodeValue::String { .. }) {
            self.add_error(
                node_id,
                ValidationErrorKind::TypeMismatch {
                    expected: "string".to_string(),
                    actual: self.node_type_name(node),
                },
            );
        }
    }

    fn validate_code(&mut self, node_id: NodeId, node: &Node) {
        let valid = matches!(
            &node.content,
            NodeValue::Code { .. } | NodeValue::CodeBlock { .. } | NodeValue::NamedCode { .. }
        );

        if !valid {
            self.add_error(
                node_id,
                ValidationErrorKind::TypeMismatch {
                    expected: "code".to_string(),
                    actual: self.node_type_name(node),
                },
            );
        }
    }

    fn validate_path(&mut self, node_id: NodeId, node: &Node) {
        if !matches!(&node.content, NodeValue::Path { .. }) {
            self.add_error(
                node_id,
                ValidationErrorKind::TypeMismatch {
                    expected: "path".to_string(),
                    actual: self.node_type_name(node),
                },
            );
        }
    }

    fn validate_array(
        &mut self,
        node_id: NodeId,
        node: &Node,
        path: &[PathSegment],
        elem_type: &Type,
    ) {
        match &node.content {
            NodeValue::Array { children, .. } => {
                // Validate each item
                for (index, child_id) in children.iter().enumerate() {
                    let mut item_path = path.to_vec();
                    item_path.push(PathSegment::Value(KeyCmpValue::U64(index as u64)));
                    self.validate_type(*child_id, &item_path, elem_type);
                }
            }
            _ => {
                self.add_error(
                    node_id,
                    ValidationErrorKind::TypeMismatch {
                        expected: "array".to_string(),
                        actual: self.node_type_name(node),
                    },
                );
            }
        }
    }

    fn validate_tuple(
        &mut self,
        node_id: NodeId,
        node: &Node,
        path: &[PathSegment],
        tuple_types: &[Type],
    ) {
        match &node.content {
            NodeValue::Tuple { children, .. } => {
                // Check tuple length
                if children.len() != tuple_types.len() {
                    self.add_error(
                        node_id,
                        ValidationErrorKind::InvalidValue(format!(
                            "Tuple expects {} items but got {}",
                            tuple_types.len(),
                            children.len()
                        )),
                    );
                    return;
                }

                // Validate each item
                for (index, (child_id, expected_type)) in
                    children.iter().zip(tuple_types).enumerate()
                {
                    let mut item_path = path.to_vec();
                    // Clamp to u8::MAX to avoid overflow
                    let tuple_index = if index > 255 { 255 } else { index as u8 };
                    item_path.push(PathSegment::TupleIndex(tuple_index));
                    self.validate_type(*child_id, &item_path, expected_type);
                }
            }
            _ => {
                self.add_error(
                    node_id,
                    ValidationErrorKind::TypeMismatch {
                        expected: "tuple".to_string(),
                        actual: self.node_type_name(node),
                    },
                );
            }
        }
    }

    fn validate_object(
        &mut self,
        node_id: NodeId,
        _node: &Node,
        path: &[PathSegment],
        object_schema: &ObjectSchema,
    ) {
        // Use the general object validation which handles fields
        self.validate_object_fields(node_id, path, object_schema);
    }

    /// Detect which variant is being used based on the node structure and schema
    fn detect_variant(
        &self,
        node: &Node,
        path: &[PathSegment],
        variant_schema: &VariantSchema,
    ) -> Option<VariantInfo> {
        let path_key = PathKey::from_segments(path);

        // Check if variant was already determined via $variant extension
        if let Some(variant_from_ext) = self.variant_context.get(&path_key) {
            // Variant already known from extension, validate it exists
            let variant_key = KeyCmpValue::String(variant_from_ext.clone());
            if variant_schema.variants.contains_key(&variant_key) {
                return Some(VariantInfo {
                    variant_name: Identifier::from_str(variant_from_ext)
                        .unwrap_or_else(|_| identifiers::UNKNOWN.clone()),
                    variant_key,
                    detection_source: VariantDetectionSource::Extension,
                });
            }
            // Invalid variant name - will be handled by validate_variant
            return None;
        }

        // For Tagged representation, check if there's a $variant extension at this level
        if matches!(&variant_schema.representation, VariantRepr::Tagged) {
            if let Some(variant_ext_id) = node.extensions.get(&identifiers::VARIANT) {
                let variant_node = self.document.get_node(*variant_ext_id);
                if let NodeValue::String { value, .. } = &variant_node.content {
                    let variant_key = KeyCmpValue::String(value.clone());
                    if variant_schema.variants.contains_key(&variant_key) {
                        if let Ok(variant_name) = Identifier::from_str(value) {
                            return Some(VariantInfo {
                                variant_name,
                                variant_key,
                                detection_source: VariantDetectionSource::Extension,
                            });
                        }
                    }
                }
            }
        }

        // Try to determine the variant based on representation
        match &variant_schema.representation {
            VariantRepr::Tagged => {
                // Look for single key that matches a variant name
                if let NodeValue::Map { entries, .. } = &node.content {
                    if entries.len() == 1 {
                        if let Some((DocumentKey::Ident(key), _)) = entries.first() {
                            let key_cmp = KeyCmpValue::String(key.to_string());
                            if variant_schema.variants.contains_key(&key_cmp) {
                                return Some(VariantInfo {
                                    variant_name: key.clone(),
                                    variant_key: key_cmp,
                                    detection_source: VariantDetectionSource::Tagged,
                                });
                            }
                        }
                    }
                }
            }
            VariantRepr::InternallyTagged { tag } => {
                // Look for tag field
                if let NodeValue::Map { entries, .. } = &node.content {
                    for (key, child_id) in entries {
                        if let DocumentKey::Ident(field_name) = key {
                            if KeyCmpValue::String(field_name.to_string()) == *tag {
                                let tag_node = self.document.get_node(*child_id);
                                if let NodeValue::String { value, .. } = &tag_node.content {
                                    let variant_key = KeyCmpValue::String(value.clone());
                                    if variant_schema.variants.contains_key(&variant_key) {
                                        if let Ok(variant_name) = Identifier::from_str(value) {
                                            return Some(VariantInfo {
                                                variant_name,
                                                variant_key,
                                                detection_source: VariantDetectionSource::InternalTag(
                                                    match tag {
                                                        KeyCmpValue::String(s) => s.clone(),
                                                        _ => format!("{:?}", tag),
                                                    },
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            VariantRepr::AdjacentlyTagged { tag, .. } => {
                // Look for tag field
                if let NodeValue::Map { entries, .. } = &node.content {
                    for (key, child_id) in entries {
                        if let DocumentKey::Ident(field_name) = key {
                            if KeyCmpValue::String(field_name.to_string()) == *tag {
                                let tag_node = self.document.get_node(*child_id);
                                if let NodeValue::String { value, .. } = &tag_node.content {
                                    let variant_key = KeyCmpValue::String(value.clone());
                                    if variant_schema.variants.contains_key(&variant_key) {
                                        if let Ok(variant_name) = Identifier::from_str(value) {
                                            return Some(VariantInfo {
                                                variant_name,
                                                variant_key,
                                                detection_source: VariantDetectionSource::InternalTag(
                                                    match tag {
                                                        KeyCmpValue::String(s) => s.clone(),
                                                        _ => format!("{:?}", tag),
                                                    },
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            VariantRepr::Untagged => {
                // For untagged, we'll need to check structure matching
                // We'll implement a lightweight check here instead of full validation
                if let Some((variant_key, _)) = self.find_matching_untagged_variant(node, variant_schema) {
                    if let KeyCmpValue::String(variant_str) = &variant_key {
                        if let Ok(variant_name) = Identifier::from_str(variant_str) {
                            return Some(VariantInfo {
                                variant_name,
                                variant_key,
                                detection_source: VariantDetectionSource::Untagged,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Find which untagged variant matches the node structure (lightweight check)
    fn find_matching_untagged_variant<'b>(
        &self,
        node: &Node,
        variant_schema: &'b VariantSchema,
    ) -> Option<(KeyCmpValue, &'b ObjectSchema)> {
        // For untagged variants, try each variant and return the first that matches
        if let NodeValue::Map { entries, .. } = &node.content {
            let node_fields: HashSet<String> = entries
                .iter()
                .filter_map(|(k, _)| match k {
                    DocumentKey::Ident(id) => Some(id.to_string()),
                    _ => None,
                })
                .collect();

            // Try each variant in order
            for (variant_key, variant_type) in &variant_schema.variants {
                // First do a quick check: all required fields must be present
                let required_fields_present = variant_type.fields.iter().all(|(field_key, field_schema)| {
                    if !field_schema.optional {
                        match field_key {
                            KeyCmpValue::String(s) => node_fields.contains(s),
                            _ => false,
                        }
                    } else {
                        true
                    }
                });

                if !required_fields_present {
                    continue; // Skip this variant, required fields missing
                }

                // For untagged variants, we should accept the first variant that could possibly match
                // Even if there are extra fields or type mismatches, the variant detection should succeed
                // The actual validation will happen later and report specific errors
                return Some((variant_key.clone(), variant_type));
            }
        }
        None
    }

    /// Validate the content of a detected variant
    fn validate_variant_content(
        &mut self,
        node_id: NodeId,
        node: &Node,
        path: &[PathSegment],
        variant_schema: &VariantSchema,
        variant_info: &VariantInfo,
    ) {
        let path_key = PathKey::from_segments(path);
        
        // Store variant context for nested validation
        self.variant_context.insert(path_key.clone(), variant_info.variant_name.to_string());
        
        // Store variant representation for field validation (to exclude tag fields)
        self.variant_repr_context.insert(path_key.clone(), variant_schema.representation.clone());

        // Get the variant type schema
        if let Some(variant_type) = variant_schema.variants.get(&variant_info.variant_key) {
            match &variant_schema.representation {
                VariantRepr::Tagged => {
                    match &variant_info.detection_source {
                        VariantDetectionSource::Extension => {
                            // When detected via $variant extension with Tagged representation,
                            // validate fields at the same level
                            self.validate_object_fields(node_id, path, variant_type);
                        }
                        _ => {
                            // Content is under the variant key
                            if let NodeValue::Map { entries, .. } = &node.content
                                && let Some((_, content_id)) = entries.first() {
                                    let mut variant_path = path.to_vec();
                                    variant_path.push(PathSegment::Ident(variant_info.variant_name.clone()));
                                    self.validate_type(*content_id, &variant_path, &Type::Object(variant_type.clone()));
                                }
                        }
                    }
                }
                VariantRepr::InternallyTagged { .. } => {
                    // Content is mixed with tag - validate as object
                    self.validate_object_fields(node_id, path, variant_type);
                }
                VariantRepr::AdjacentlyTagged { content, .. } => {
                    // Content is under content field
                    if let NodeValue::Map { entries, .. } = &node.content {
                        if let Some((_, content_id)) = entries.iter()
                            .find(|(k, _)| matches!(k, DocumentKey::Ident(id) if KeyCmpValue::String(id.to_string()) == *content))
                        {
                            self.validate_type(*content_id, path, &Type::Object(variant_type.clone()));
                        } else {
                            // Content field is missing - report error
                            self.add_error(
                                node_id,
                                ValidationErrorKind::RequiredFieldMissing {
                                    field: content.clone(),
                                    path: path.to_vec(),
                                },
                            );
                        }
                    }
                }
                VariantRepr::Untagged => {
                    // Content validation for untagged variant
                    self.validate_object_fields(node_id, path, variant_type);
                }
            }
        }
    }

    /// Validates a variant type against its schema.
    ///
    /// This is the main entry point for variant validation. It performs two steps:
    /// 1. Detection: Determines which variant is being used based on the representation
    /// 2. Validation: Validates the content against the detected variant's schema
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node being validated
    /// * `node` - The node containing the variant data
    /// * `path` - The path to this node in the document
    /// * `variant_schema` - The schema defining available variants and their representation
    ///
    /// # Variant Detection Process
    ///
    /// The detection process varies by representation:
    /// - **Tagged**: Looks for a single key matching a variant name or `$variant` extension
    /// - **InternallyTagged**: Checks the configured tag field's value
    /// - **AdjacentlyTagged**: Reads the tag field to determine variant
    /// - **Untagged**: Tries each variant until one validates successfully
    ///
    /// # Error Handling
    ///
    /// Errors are added to `self.errors` for:
    /// - Unknown variant names
    /// - Missing required variant tags
    /// - Type mismatches in variant fields
    /// - Unexpected fields in strict variants
    fn validate_variant(
        &mut self,
        node_id: NodeId,
        node: &Node,
        path: &[PathSegment],
        variant_schema: &VariantSchema,
    ) {
        // Step 1: Detect which variant is being used
        let variant_info = self.detect_variant(node, path, variant_schema);
        
        // Step 2: Validate based on detection result
        match variant_info {
            Some(info) => {
                // Variant detected - validate its content
                self.validate_variant_content(node_id, node, path, variant_schema, &info);
            }
            None => {
                // Check if we had an invalid variant from context
                let path_key = PathKey::from_segments(path);
                if let Some(variant_from_ext) = self.variant_context.get(&path_key) {
                    // We had a variant but it wasn't valid
                    self.add_error(
                        node_id,
                        ValidationErrorKind::UnknownVariant {
                            variant: variant_from_ext.clone(),
                            available: variant_schema.variants.keys()
                                .map(|k| match k {
                                    KeyCmpValue::String(s) => s.clone(),
                                    _ => format!("{k:?}")
                                })
                                .collect(),
                        },
                    );
                } else {
                    // Check if this is an internally tagged or adjacently tagged variant with an invalid tag value
                    let has_invalid_tag = match &variant_schema.representation {
                        VariantRepr::InternallyTagged { tag } | VariantRepr::AdjacentlyTagged { tag, .. } => {
                            // Check if tag field exists with an invalid value
                            if let NodeValue::Map { entries, .. } = &node.content {
                                entries.iter().any(|(key, child_id)| {
                                    if let DocumentKey::Ident(field_name) = key {
                                        if KeyCmpValue::String(field_name.to_string()) == *tag {
                                            let tag_node = self.document.get_node(*child_id);
                                            if let NodeValue::String { value, .. } = &tag_node.content {
                                                let variant_key = KeyCmpValue::String(value.clone());
                                                if !variant_schema.variants.contains_key(&variant_key) {
                                                    // Tag exists but value is invalid
                                                    self.add_error(
                                                        node_id,
                                                        ValidationErrorKind::UnknownVariant {
                                                            variant: value.clone(),
                                                            available: variant_schema.variants.keys()
                                                                .map(|k| match k {
                                                                    KeyCmpValue::String(s) => s.clone(),
                                                                    _ => format!("{k:?}")
                                                                })
                                                                .collect(),
                                                        },
                                                    );
                                                    return true;
                                                }
                                            }
                                        }
                                    }
                                    false
                                })
                            } else {
                                false
                            }
                        }
                        _ => false
                    };
                    
                    if !has_invalid_tag {
                        if matches!(variant_schema.representation, VariantRepr::Untagged) {
                            // For untagged variants, report that no variant matched
                            self.add_error(
                                node_id,
                                ValidationErrorKind::UnknownVariant {
                                    variant: "no matching variant".to_string(),
                                    available: variant_schema.variants.keys()
                                        .map(|k| match k {
                                            KeyCmpValue::String(s) => s.clone(),
                                            _ => format!("{k:?}")
                                        })
                                        .collect(),
                                },
                            );
                        } else {
                            // For tagged variants, report missing discriminator
                            self.add_error(
                                node_id,
                                ValidationErrorKind::VariantDiscriminatorMissing,
                            );
                        }
                    }
                }
            }
        }
    }

    fn handle_extension(&mut self, node_id: NodeId, path: &[PathSegment], ident: &Identifier) {
        match ident.as_ref() {
            "cascade-type" => {
                // Handle cascade type extension
                let node = self.document.get_node(node_id);
                if let NodeValue::Path { value, .. } = &node.content
                    && let Some(_cascade_type) = Type::from_path_segments(&value.0) {
                        // This would be used to affect validation of nested fields
                        // For now, we just acknowledge it exists
                    }
            }
            "variant" => {
                // Handle variant discriminator
                let node = self.document.get_node(node_id);
                if let NodeValue::String { value, .. } = &node.content {
                    let path_key = PathKey::from_segments(path);
                    self.variant_context.insert(path_key, value.clone());
                }
            }
            _ => {
                // Other extensions are allowed but not validated
            }
        }
    }

    fn handle_meta_extension(&mut self, _node_id: NodeId, _path: &[PathSegment], _ident: &Identifier) {
        // Meta-extensions are schema definitions, not validated in document validation
    }

    fn check_missing_fields(
        &mut self,
        path: &[PathSegment],
        expected_fields: &IndexMap<KeyCmpValue, FieldSchema>,
    ) {
        let path_key = PathKey::from_segments(path);
        let seen_fields_set = self.seen_fields.get(&path_key).cloned();

        for (field_name, field_schema) in expected_fields {
            let is_seen = seen_fields_set
                .as_ref()
                .is_some_and(|s| s.contains(field_name));

            if !is_seen && !field_schema.optional {
                // Need a dummy NodeId for missing fields - use root
                let root_id = self.document.get_root_id();
                self.add_error(
                    root_id,
                    ValidationErrorKind::RequiredFieldMissing {
                        field: field_name.clone(),
                        path: path.to_vec(),
                    },
                );
            }
        }
    }

    fn is_schema_only_node(&self, node: &Node) -> bool {
        // A node is schema-only if it has schema extensions but no actual data content
        let has_schema_extensions = node.extensions.iter().any(|(ext, _)| {
            matches!(ext.as_ref(), 
                "type" | "optional" | "min" | "max" | "pattern" | 
                "min-length" | "max-length" | "length" | "range" |
                "union" | "variants" | "cascade-type" | "array" |
                "enum" | "values" | "default" | "unique" | "contains"
            )
        });
        
        if !has_schema_extensions {
            return false;
        }
        
        // Check if the node has non-schema content
        match &node.content {
            NodeValue::Map { entries, .. } => {
                // A map with only extension entries is schema-only
                entries.is_empty()
            }
            NodeValue::Null { .. } => true, // Null with schema extensions is schema-only
            _ => false, // Other content types with data are not schema-only
        }
    }

    fn node_type_name(&self, node: &Node) -> String {
        match &node.content {
            NodeValue::Null { .. } => "null",
            NodeValue::Bool { .. } => "boolean",
            NodeValue::I64 { .. } => "i64",
            NodeValue::U64 { .. } => "u64",
            NodeValue::F32 { .. } => "f32",
            NodeValue::F64 { .. } => "f64",
            NodeValue::String { .. } => "string",
            NodeValue::Code { .. } | NodeValue::CodeBlock { .. } | NodeValue::NamedCode { .. } => "code",
            NodeValue::Path { .. } => "path",
            NodeValue::Hole { .. } => "hole",
            NodeValue::Array { .. } => "array",
            NodeValue::Map { .. } => "object",
            NodeValue::Tuple { .. } => "tuple",
        }.to_string()
    }

    fn add_error(&mut self, node_id: NodeId, kind: ValidationErrorKind) {
        self.errors.push(ValidationError {
            kind,
            severity: Severity::Error,
            node_id,
        });
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.severity {
            Severity::Error => write!(f, "error: {}", self.kind),
            Severity::Warning => write!(f, "warning: {}", self.kind),
        }
    }
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ValidationErrorKind::*;
        match self {
            TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {expected}, but got {actual}")
            }
            RequiredFieldMissing { field, path } => {
                if path.is_empty() {
                    write!(f, "Required field {field:?} is missing")
                } else {
                    let path_str = crate::utils::path_segments_to_display_string(path);
                    write!(f, "Required field {field:?} is missing at {path_str}")
                }
            }
            UnexpectedField { field, path } => {
                let path_str = crate::utils::path_segments_to_display_string(path);
                write!(f, "Unexpected field {field:?} at {path_str} not defined in schema")
            }
            InvalidValue(msg) => write!(f, "Invalid value: {msg}"),
            PatternMismatch { pattern, value } => {
                write!(f, "String '{value}' does not match pattern /{pattern}/")
            }
            RangeViolation { min, max, value } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "Number must be between {min} and {max}, but got {value}"),
                    (Some(min), None) => write!(f, "Number must be at least {min}, but got {value}"),
                    (None, Some(max)) => write!(f, "Number must be at most {max}, but got {value}"),
                    (None, None) => write!(f, "Number {value} violates range constraint"),
                }
            }
            StringLengthViolation { min, max, length } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "String length must be between {min} and {max} characters, but got {length}"),
                    (Some(min), None) => write!(f, "String must be at least {min} characters long, but got {length}"),
                    (None, Some(max)) => write!(f, "String must be at most {max} characters long, but got {length}"),
                    (None, None) => write!(f, "String length {length} violates constraint"),
                }
            }
            ArrayLengthViolation { min, max, length } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "Array must have between {min} and {max} items, but has {length}"),
                    (Some(min), None) => write!(f, "Array must have at least {min} items, but has {length}"),
                    (None, Some(max)) => write!(f, "Array must have at most {max} items, but has {length}"),
                    (None, None) => write!(f, "Array length {length} violates constraint"),
                }
            }
            UnknownType(type_name) => write!(f, "Unknown type: {type_name}"),
            UnknownVariant { variant, available } => {
                if available.is_empty() {
                    write!(f, "Unknown variant '{}'", variant)
                } else {
                    write!(f, "Unknown variant '{}'. Available variants: {}", variant, available.join(", "))
                }
            }
            VariantDiscriminatorMissing => {
                write!(f, "Variant discriminator field '$variant' is missing")
            }
            HoleExists { path } => {
                let path_str = crate::utils::path_segments_to_display_string(path);
                write!(f, "Hole (!) exists at path: {path_str}")
            }
            MaxDepthExceeded { depth, max_depth } => {
                write!(f, "Maximum validation depth of {max_depth} exceeded at depth {depth}")
            }
        }
    }
}
