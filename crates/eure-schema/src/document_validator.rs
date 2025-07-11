//! Document-based schema validation
//!
//! This module provides validation of EureDocument against schemas
//! using a simple recursive approach without CST visitors.

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
    HoleExists {
        path: Vec<PathSegment>,
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

/// Internal validator state
struct DocumentValidator<'a> {
    document: &'a EureDocument,
    schema: &'a DocumentSchema,
    errors: Vec<ValidationError>,
    /// Track which fields have been seen at each path
    seen_fields: HashMap<PathKey, HashSet<KeyCmpValue>>,
    /// Track variant context for proper field validation
    variant_context: HashMap<PathKey, String>,
}

impl<'a> DocumentValidator<'a> {
    fn new(document: &'a EureDocument, schema: &'a DocumentSchema) -> Self {
        Self {
            document,
            schema,
            errors: Vec::new(),
            seen_fields: HashMap::new(),
            variant_context: HashMap::new(),
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

        // Check for missing required fields
        self.check_missing_fields(&[], &self.schema.root.fields);
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
                            // Handle dynamic keys
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
        // Track that we've seen this field
        let path_key = PathKey::from_segments(path);
        self.seen_fields
            .entry(path_key)
            .or_default()
            .insert(KeyCmpValue::String(field_name.to_string()));

        let field_key = KeyCmpValue::String(field_name.to_string());
        if let Some(field_schema) = expected_fields.get(&field_key) {
            // Validate against field schema
            let mut field_path = path.to_vec();
            field_path.push(PathSegment::Ident(field_name.clone()));
            self.validate_type_with_constraints(node_id, &field_path, &field_schema.type_expr, &field_schema.constraints);
        } else {
            // Additional properties are handled at the object level, not field level
            // For now, disallow unexpected fields
            self.add_error(
                node_id,
                ValidationErrorKind::UnexpectedField {
                    field: KeyCmpValue::String(field_name.to_string()),
                    path: path.to_vec(),
                },
            );
        }
    }

    fn validate_type(
        &mut self,
        node_id: NodeId,
        path: &[PathSegment],
        expected_type: &Type,
    ) {
        let node = self.document.get_node(node_id);

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
                    temp_validator.validate_type(node_id, path, union_type);
                    if temp_validator.errors.is_empty() {
                        // Valid for this union member
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
            (NodeValue::Array { children, .. }, Type::Array(_)) => {
                // Check array length constraints
                let len = children.len();
                if let Some(min_items) = constraints.min_items
                    && len < min_items {
                        self.add_error(
                            node_id,
                            ValidationErrorKind::ArrayLengthViolation {
                                min: Some(min_items),
                                max: constraints.max_items,
                                length: len,
                            },
                        );
                        return;
                    }
                if let Some(max_items) = constraints.max_items
                    && len > max_items {
                        self.add_error(
                            node_id,
                            ValidationErrorKind::ArrayLengthViolation {
                                min: constraints.min_items,
                                max: Some(max_items),
                                length: len,
                            },
                        );
                    }
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
                    item_path.push(PathSegment::TupleIndex(index as u8));
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
        
        // Check for missing required fields
        self.check_missing_fields(path, &object_schema.fields);
    }

    fn validate_variant(
        &mut self,
        node_id: NodeId,
        node: &Node,
        path: &[PathSegment],
        variant_schema: &VariantSchema,
    ) {
        let path_key = PathKey::from_segments(path);

        // Check if variant was already determined via $variant extension
        let variant_name = if let Some(variant_from_ext) = self.variant_context.get(&path_key) {
            // Variant already known from extension, validate it exists
            let variant_key = KeyCmpValue::String(variant_from_ext.clone());
            if variant_schema.variants.contains_key(&variant_key) {
                Some(Identifier::from_str(variant_from_ext).unwrap_or_else(|_| Identifier::from_str("unknown").unwrap()))
            } else {
                // Invalid variant name
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
                return;
            }
        } else {
            // For Tagged representation, check if there's a $variant extension at this level
            // This handles the case where $variant is used with Tagged representation
            if matches!(&variant_schema.representation, VariantRepr::Tagged) {
                // Debug: Print all extensions on this node
                eprintln!("DEBUG validate_variant: Node extensions: {:?}", 
                    node.extensions.keys().map(|k| k.to_string()).collect::<Vec<_>>());
                
                if let Some(variant_ext_id) = node.extensions.get(&Identifier::from_str("variant").unwrap()) {
                    let variant_node = self.document.get_node(*variant_ext_id);
                    if let NodeValue::String { value, .. } = &variant_node.content {
                        // Store variant context
                        self.variant_context.insert(path_key.clone(), value.clone());
                        // Validate as internally tagged (variant fields at same level)
                        let variant_key = KeyCmpValue::String(value.clone());
                        if let Some(variant_type) = variant_schema.variants.get(&variant_key) {
                            self.validate_object_fields(node_id, path, variant_type);
                            self.check_missing_fields(path, &variant_type.fields);
                            return;
                        } else {
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
                            return;
                        }
                    }
                }
            }
            
            // Try to determine the variant
            match &variant_schema.representation {
            VariantRepr::Tagged => {
                // Look for single key that matches a variant name
                if let NodeValue::Map { entries, .. } = &node.content {
                    if entries.len() == 1 {
                        if let Some((DocumentKey::Ident(key), _)) = entries.first() {
                            let key_cmp = KeyCmpValue::String(key.to_string());
                            if variant_schema.variants.contains_key(&key_cmp) {
                                Some(key.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            VariantRepr::InternallyTagged { tag } => {
                // Look for tag field
                if let NodeValue::Map { entries, .. } = &node.content {
                    entries.iter().find_map(|(key, child_id)| {
                        if let DocumentKey::Ident(field_name) = key {
                            if KeyCmpValue::String(field_name.to_string()) == *tag {
                                let tag_node = self.document.get_node(*child_id);
                                if let NodeValue::String { value, .. } = &tag_node.content {
                                    Identifier::from_str(value).ok()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            }
            VariantRepr::AdjacentlyTagged { tag, content: _ } => {
                // Look for tag field
                if let NodeValue::Map { entries, .. } = &node.content {
                    entries.iter().find_map(|(key, child_id)| {
                        if let DocumentKey::Ident(field_name) = key {
                            if KeyCmpValue::String(field_name.to_string()) == *tag {
                                let tag_node = self.document.get_node(*child_id);
                                if let NodeValue::String { value, .. } = &tag_node.content {
                                    Identifier::from_str(value).ok()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            }
            VariantRepr::Untagged => {
                // Try each variant to see which matches
                for (variant_name, variant_type) in &variant_schema.variants {
                    let mut temp_validator = DocumentValidator::new(self.document, self.schema);
                    temp_validator.validate_type(node_id, path, &Type::Object(variant_type.clone()));
                    if temp_validator.errors.is_empty() {
                        self.variant_context.insert(path_key.clone(), format!("{variant_name:?}"));
                        return; // Valid variant found
                    }
                }
                None
            }
        }
        };

        if let Some(variant_name) = variant_name {
            // Store variant context
            self.variant_context.insert(path_key, variant_name.to_string());

            // Validate variant content
            let variant_key = KeyCmpValue::String(variant_name.to_string());
            if let Some(variant_type) = variant_schema.variants.get(&variant_key) {
                match &variant_schema.representation {
                    VariantRepr::Tagged => {
                        // Content is under the variant key
                        if let NodeValue::Map { entries, .. } = &node.content
                            && let Some((_, content_id)) = entries.first() {
                                let mut variant_path = path.to_vec();
                                variant_path.push(PathSegment::Ident(variant_name));
                                self.validate_type(*content_id, &variant_path, &Type::Object(variant_type.clone()));
                            }
                    }
                    VariantRepr::InternallyTagged { .. } => {
                        // Content is mixed with tag - validate as object but skip the $variant field
                        self.validate_object_fields(node_id, path, variant_type);
                        self.check_missing_fields(path, &variant_type.fields);
                    }
                    VariantRepr::AdjacentlyTagged { content, .. } => {
                        // Content is under content field
                        if let NodeValue::Map { entries, .. } = &node.content
                            && let Some((_, content_id)) = entries.iter()
                                .find(|(k, _)| matches!(k, DocumentKey::Ident(id) if KeyCmpValue::String(id.to_string()) == *content))
                            {
                                self.validate_type(*content_id, path, &Type::Object(variant_type.clone()));
                            }
                    }
                    VariantRepr::Untagged => {
                        // Already validated above
                    }
                }
            }
        } else {
            self.add_error(
                node_id,
                ValidationErrorKind::UnknownVariant {
                    variant: "unknown".to_string(),
                    available: variant_schema.variants.keys()
                        .map(|k| format!("{k:?}"))
                        .collect(),
                },
            );
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
        write!(f, "{}", self.kind)
    }
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ValidationErrorKind::*;
        match self {
            TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {expected}, but got {actual}")
            }
            RequiredFieldMissing { field, .. } => {
                write!(f, "Required field '{field:?}' is missing")
            }
            UnexpectedField { field, .. } => {
                write!(f, "Unexpected field '{field:?}'")
            }
            InvalidValue(msg) => write!(f, "Invalid value: {msg}"),
            PatternMismatch { pattern, value } => {
                write!(f, "Value '{value}' does not match pattern '{pattern}'")
            }
            RangeViolation { min, max, value } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "Value {value} is outside range [{min}, {max}]"),
                    (Some(min), None) => write!(f, "Value {value} is less than minimum {min}"),
                    (None, Some(max)) => write!(f, "Value {value} is greater than maximum {max}"),
                    (None, None) => write!(f, "Value {value} violates range constraint"),
                }
            }
            StringLengthViolation { min, max, length } => {
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, "String must have between {min} and {max} characters, but has {length}"),
                    (Some(min), None) => write!(f, "String must have at least {min} characters, but has {length}"),
                    (None, Some(max)) => write!(f, "String must have at most {max} characters, but has {length}"),
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
                write!(f, "Unknown variant '{}'. Available variants: {}", variant, available.join(", "))
            }
            HoleExists { path } => {
                let path_str = crate::utils::path_segments_to_display_string(path);
                write!(f, "Hole (!) exists at path: {path_str}")
            }
        }
    }
}
