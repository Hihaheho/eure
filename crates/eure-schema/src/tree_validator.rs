//! Tree-based schema validation that preserves span information
//! 
//! This module provides validation of EURE documents against schemas
//! while preserving span information for error reporting.

use crate::schema::*;
use crate::value_validator::{ValidationError, ValidationErrorKind, Severity};
use eure_tree::{
    prelude::*,
    tree::{InputSpan, CstNodeData, TerminalData, NonTerminalData},
    value_visitor::Values,
    nodes::{BindingRhsView},
};
use eure_value::value::{Value, Map, KeyCmpValue, PathSegment};
use std::collections::HashSet;

/// A tree-based validator that preserves span information
pub struct SchemaValidator<'a> {
    _input: &'a str,
    schema: &'a DocumentSchema,
    values: &'a Values,
    errors: Vec<ValidationError>,
    current_path: Vec<PathSegment>,
}

impl<'a> SchemaValidator<'a> {
    /// Create a new schema validator
    pub fn new(input: &'a str, schema: &'a DocumentSchema, values: &'a Values) -> Self {
        Self {
            _input: input,
            schema,
            values,
            errors: Vec::new(),
            current_path: Vec::new(),
        }
    }
    
    /// Get the validation errors
    pub fn into_errors(self) -> Vec<ValidationError> {
        self.errors
    }
    
    /// Add an error with span information
    fn add_error(&mut self, kind: ValidationErrorKind, span: Option<InputSpan>) {
        self.errors.push(ValidationError {
            kind,
            severity: Severity::Error,
            span,
        });
    }
    
    /// Get span from a node handle
    fn get_span_from_node<F: CstFacade>(&self, node_id: CstNodeId, tree: &F) -> Option<InputSpan> {
        if let Some(node_data) = tree.node_data(node_id) {
            match node_data {
                CstNodeData::Terminal { data: TerminalData::Input(span), .. } => Some(span),
                CstNodeData::NonTerminal { data: NonTerminalData::Input(span), .. } => Some(span),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl<'a, F: CstFacade> CstVisitor<F> for SchemaValidator<'a> {
    type Error = std::convert::Infallible;
    
    fn visit_eure(
        &mut self,
        handle: EureHandle,
        view: EureView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Get the value from ValueVisitor's mappings
        if let Some((value, span_opt)) = self.values.get_eure_with_span(&handle) {
            // Get span from the node or from stored span
            let span = span_opt.copied().or_else(|| self.get_span_from_node(handle.node_id(), tree));
            
            // Validate the document root
            if let Value::Map(map) = value {
                self.validate_object(map, &self.schema.root, span);
            } else {
                self.add_error(
                    ValidationErrorKind::TypeMismatch {
                        expected: "object".to_string(),
                        actual: value_type_name(value).to_string(),
                    },
                    span,
                );
            }
        }
        
        // Continue visiting children
        self.visit_eure_super(handle, view, tree)
    }
    
    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Bindings represent field assignments, we can validate them here
        // Get the key path and value
        // Get the key handles from the keys
        {
            if let Some(key_handles) = self.values.get_keys(&view.keys) {
                // Build the path from key handles
                let mut path = Vec::new();
                for key_handle in key_handles {
                    if let Some((segment, _)) = self.values.get_key_with_span(key_handle) {
                        path.push(segment.clone());
                    }
                }
                
                // Get the value for this binding
                // First we need to get the value from the binding_rhs
                let binding_value = match view.binding_rhs.get_view(tree) {
                    Ok(BindingRhsView::ValueBinding(value_binding_handle)) => {
                        if let Ok(value_binding_view) = value_binding_handle.get_view(tree) {
                            self.values.get_value(&value_binding_view.value)
                        } else {
                            None
                        }
                    }
                    Ok(BindingRhsView::TextBinding(_text_binding_handle)) => {
                        // For text bindings, we'd need to extract the text value
                        // For now, we'll skip text binding validation
                        None
                    }
                    Ok(BindingRhsView::SectionBinding(_)) => {
                        // Section bindings are handled separately
                        None
                    }
                    _ => None,
                };
                
                if let Some(_value) = binding_value {
                    let _span = self.get_span_from_node(handle.node_id(), tree);
                    
                    // TODO: Look up the field schema based on the path and validate
                    // For now, we'll just store the path for context
                    let old_path = std::mem::replace(&mut self.current_path, path);
                    
                    // Continue visiting
                    let result = self.visit_binding_super(handle, view, tree);
                    
                    // Restore path
                    self.current_path = old_path;
                    
                    return result;
                }
            }
        }
        
        self.visit_binding_super(handle, view, tree)
    }
}

// Helper functions for validation
impl<'a> SchemaValidator<'a> {
    /// Validate an object against an object schema
    fn validate_object(&mut self, map: &Map, schema: &ObjectSchema, span: Option<InputSpan>) {
        // Track required fields
        let mut required_fields: HashSet<KeyCmpValue> = schema.fields
            .iter()
            .filter(|(_, field)| !field.optional)
            .map(|(name, _)| name.clone())
            .collect();
        
        // Validate each field in the map
        for (key, value) in &map.0 {
            match key {
                KeyCmpValue::String(_field_name) => {
                    // Remove from required fields
                    required_fields.remove(key);
                    
                    // Find schema for this field
                    if let Some(field_schema) = schema.fields.get(key) {
                        // TODO: Get proper span for this specific field
                        self.validate_value(value, field_schema, span);
                    } else if schema.additional_properties.is_none() {
                        // Unexpected field
                        self.add_error(
                            ValidationErrorKind::UnexpectedField {
                                field: key.clone(),
                                path: self.current_path.clone(),
                            },
                            span,
                        );
                    }
                }
                KeyCmpValue::Extension(_) => {
                    // Extension fields are handled separately
                    // TODO: Validate against extension schemas
                }
                _ => {
                    // Other key types - validate if schema allows
                }
            }
        }
        
        // Check for missing required fields
        for missing_field in required_fields {
            self.add_error(
                ValidationErrorKind::RequiredFieldMissing {
                    field: missing_field,
                    path: self.current_path.clone(),
                },
                span,
            );
        }
    }
    
    /// Validate a value against a field schema
    fn validate_value(&mut self, value: &Value, schema: &FieldSchema, span: Option<InputSpan>) {
        // TODO: Implement full validation logic
        // For now, just check basic type matching
        if !self.type_matches(value, &schema.type_expr) {
            self.add_error(
                ValidationErrorKind::TypeMismatch {
                    expected: type_to_string(&schema.type_expr),
                    actual: value_type_name(value).to_string(),
                },
                span,
            );
        }
    }
    
    /// Check if a value matches a type
    fn type_matches(&self, value: &Value, expected_type: &Type) -> bool {
        match (value, expected_type) {
            (Value::Null, Type::Null) => true,
            (Value::Bool(_), Type::Boolean) => true,
            (Value::I64(_) | Value::U64(_), Type::Number) => true,
            (Value::F32(_) | Value::F64(_), Type::Number) => true,
            (Value::String(_), Type::String) => true,
            (Value::Array(_), Type::Array(_)) => true,
            (Value::Map(_), Type::Object(_)) => true,
            (_, Type::Any) => true,
            // TODO: Handle other type matches
            _ => false,
        }
    }
}

/// Get the type name of a value for error messages
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::I64(_) | Value::U64(_) => "number",
        Value::F32(_) | Value::F64(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Map(_) => "object",
        Value::Tuple(_) => "tuple",
        Value::Unit => "unit",
        Value::Code(_) => "code",
        Value::TypedString(_) => "typed-string",
        Value::Path(_) => "path",
        Value::Variant(_) => "variant",
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