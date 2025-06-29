//! Schema validation for EURE documents

use crate::schema::*;
use eure_tree::prelude::*;
use eure_tree::tree::InputSpan;
use std::collections::{HashMap, HashSet};
use std::fmt;

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
        field: String,
        path: Vec<String>,
    },
    UnexpectedField {
        field: String,
        path: Vec<String>,
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
    ArrayUniqueViolation {
        duplicate: String,
    },
    
    // Schema definition errors
    InvalidSchemaPattern {
        pattern: String,
        error: String,
    },

    // Variant errors
    UnknownVariant {
        variant: String,
        available: Vec<String>,
    },
    MissingVariantTag,

    // Preference violations (warnings)
    PreferSection {
        path: Vec<String>,
    },
    PreferArraySyntax {
        path: Vec<String>,
    },
}

/// Severity of validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A validation error with location and severity
#[derive(Debug)]
pub struct ValidationError {
    pub kind: ValidationErrorKind,
    pub span: InputSpan,
    pub severity: Severity,
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
                    format!(" at {}", path.join("."))
                };
                write!(f, "Required field '{field}' is missing{location}")
            }
            ValidationErrorKind::UnexpectedField { field, path } => {
                let location = if path.is_empty() {
                    String::new()
                } else {
                    format!(" at {}", path.join("."))
                };
                write!(f, "Unexpected field '{field}'{location} not defined in schema")
            }
            ValidationErrorKind::StringLengthViolation { min, max, actual } => {
                match (min, max) {
                    (Some(min), Some(max)) => {
                        write!(f, "String length must be between {min} and {max} characters, but got {actual}")
                    }
                    (Some(min), None) => {
                        write!(f, "String must be at least {min} characters long, but got {actual}")
                    }
                    (None, Some(max)) => {
                        write!(f, "String must be at most {max} characters long, but got {actual}")
                    }
                    (None, None) => {
                        write!(f, "String length violation (actual: {actual})")
                    }
                }
            }
            ValidationErrorKind::StringPatternViolation { pattern, value } => {
                write!(f, "String '{value}' does not match pattern /{pattern}/")
            }
            ValidationErrorKind::NumberRangeViolation { min, max, actual } => {
                match (min, max) {
                    (Some(min), Some(max)) => {
                        write!(f, "Number must be between {min} and {max}, but got {actual}")
                    }
                    (Some(min), None) => {
                        write!(f, "Number must be at least {min}, but got {actual}")
                    }
                    (None, Some(max)) => {
                        write!(f, "Number must be at most {max}, but got {actual}")
                    }
                    (None, None) => {
                        write!(f, "Number range violation (actual: {actual})")
                    }
                }
            }
            ValidationErrorKind::ArrayLengthViolation { min, max, actual } => {
                match (min, max) {
                    (Some(min), Some(max)) => {
                        write!(f, "Array must have between {min} and {max} items, but has {actual}")
                    }
                    (Some(min), None) => {
                        write!(f, "Array must have at least {min} items, but has {actual}")
                    }
                    (None, Some(max)) => {
                        write!(f, "Array must have at most {max} items, but has {actual}")
                    }
                    (None, None) => {
                        write!(f, "Array length violation (actual: {actual})")
                    }
                }
            }
            ValidationErrorKind::ArrayUniqueViolation { duplicate } => {
                write!(f, "Array contains duplicate value: {duplicate}")
            }
            ValidationErrorKind::InvalidSchemaPattern { pattern, error } => {
                write!(f, "Invalid pattern '/{pattern}/': {error}")
            }
            ValidationErrorKind::UnknownVariant { variant, available } => {
                if available.is_empty() {
                    write!(f, "Unknown variant '{variant}'")
                } else {
                    write!(f, "Unknown variant '{variant}'. Available variants: {}", available.join(", "))
                }
            }
            ValidationErrorKind::MissingVariantTag => {
                write!(f, "Missing $variant tag for variant type")
            }
            ValidationErrorKind::PreferSection { path } => {
                let location = if path.is_empty() {
                    String::new()
                } else {
                    format!(" for {}", path.join("."))
                };
                write!(f, "Consider using binding syntax instead of section syntax{location}")
            }
            ValidationErrorKind::PreferArraySyntax { path } => {
                let location = if path.is_empty() {
                    String::new()
                } else {
                    format!(" for {}", path.join("."))
                };
                write!(f, "Consider using explicit array syntax instead of array append syntax{location}")
            }
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{}: {}", severity, self.kind)
    }
}

/// Validates EURE documents against schemas
pub struct SchemaValidator<'a> {
    // Input string
    input: &'a str,

    // Schema to validate against
    schema: DocumentSchema,

    // Current traversal state
    path_stack: Vec<String>,
    type_stack: Vec<Type>,
    section_is_array: Vec<bool>,

    // Validation state
    seen_fields: HashMap<Vec<String>, HashSet<String>>,
    required_fields: HashMap<Vec<String>, HashSet<String>>,

    // Current context
    current_object_schema: Option<ObjectSchema>,
    current_variants: Option<HashMap<String, ObjectSchema>>,
    selected_variant: Option<String>,
    in_extension: bool,
    expected_type: Option<Type>,

    // Cascade type field schema (cached)
    cascade_field_schema: Option<FieldSchema>,

    // Collected errors
    errors: Vec<ValidationError>,
}

impl<'a> SchemaValidator<'a> {
    pub fn new(input: &'a str, schema: DocumentSchema) -> Self {
        // Extract required fields from schema
        let mut required_fields = HashMap::new();

        // Root level required fields
        let root_required: HashSet<String> = schema
            .root
            .fields
            .iter()
            .filter(|(_, field)| !field.optional)
            .map(|(name, _)| name.clone())
            .collect();
        if !root_required.is_empty() {
            required_fields.insert(vec![], root_required);
        }

        // Note: We don't extract required fields from type definitions
        // Type definitions are just schemas, not actual data to validate

        // Create cascade field schema if needed
        let cascade_field_schema = schema.cascade_type.as_ref().map(|cascade_type| FieldSchema {
                type_expr: cascade_type.clone(),
                optional: false,
                constraints: Constraints::default(),
                preferences: Preferences::default(),
                serde: SerdeOptions::default(),
                span: None,
                default_value: None,
                description: None,
            });

        Self {
            input,
            schema,
            path_stack: Vec::new(),
            type_stack: Vec::new(),
            section_is_array: Vec::new(),
            seen_fields: HashMap::new(),
            required_fields,
            current_object_schema: None,
            current_variants: None,
            selected_variant: None,
            in_extension: false,
            expected_type: None,
            cascade_field_schema,
            errors: Vec::new(),
        }
    }

    /// Consume the validator and return collected errors
    pub fn validate(mut self, tree: &eure_tree::Cst) -> Vec<ValidationError> {
        // Set initial object schema
        self.current_object_schema = Some(self.schema.root.clone());

        // Visit the tree
        // Use the visitor pattern to visit the tree
        let root_handle = tree.root_handle();
        let _ = self.visit_root_handle(root_handle, tree);

        // Check for missing required fields
        self.check_missing_required_fields();

        self.errors
    }

    fn current_path(&self) -> Vec<String> {
        self.path_stack.clone()
    }

    fn add_error(&mut self, kind: ValidationErrorKind, span: InputSpan) {
        let severity = match &kind {
            ValidationErrorKind::PreferSection { .. }
            | ValidationErrorKind::PreferArraySyntax { .. } => Severity::Warning,
            _ => Severity::Error,
        };

        self.errors.push(ValidationError {
            kind,
            span,
            severity,
        });
    }

    fn check_missing_required_fields(&mut self) {
        let required_fields_copy: Vec<_> = self
            .required_fields
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (path, required) in required_fields_copy {
            let seen = self.seen_fields.get(&path).cloned().unwrap_or_default();

            for field in required {
                if !seen.contains(&field) {
                    self.add_error(
                        ValidationErrorKind::RequiredFieldMissing {
                            field: field.clone(),
                            path: path.clone(),
                        },
                        InputSpan::new(0, 0), // No span for missing fields
                    );
                }
            }
        }
    }

    fn find_field_schema(&self, field_name: &str) -> Option<&FieldSchema> {
        // Check current object schema
        if let Some(obj_schema) = &self.current_object_schema
            && let Some(field_schema) = obj_schema.fields.get(field_name)
        {
            return Some(field_schema);
        }

        // Check if this field name corresponds to a type definition
        if self.schema.types.contains_key(field_name) {
            return self.schema.types.get(field_name);
        }

        // Check cascade type
        if let Some(cascade_schema) = &self.cascade_field_schema {
            return Some(cascade_schema);
        }

        None
    }

    fn validate_string_value(&mut self, value: &str, constraints: &Constraints, span: InputSpan) {
        // Check length constraints
        if let Some((min, max)) = constraints.length {
            let len = value.chars().count();

            let violation = match (min, max) {
                (Some(min_len), Some(max_len)) => len < min_len || len > max_len,
                (Some(min_len), None) => len < min_len,
                (None, Some(max_len)) => len > max_len,
                (None, None) => false,
            };

            if violation {
                self.add_error(
                    ValidationErrorKind::StringLengthViolation {
                        min,
                        max,
                        actual: len,
                    },
                    span,
                );
            }
        }

        // Check pattern constraint
        if let Some(pattern) = &constraints.pattern {
            // Compile and check regex pattern
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(value) {
                        self.add_error(
                            ValidationErrorKind::StringPatternViolation {
                                pattern: pattern.clone(),
                                value: value.to_string(),
                            },
                            span,
                        );
                    }
                }
                Err(e) => {
                    // Invalid regex pattern in schema - this is a schema error
                    self.add_error(
                        ValidationErrorKind::InvalidSchemaPattern {
                            pattern: pattern.clone(),
                            error: e.to_string(),
                        },
                        span,
                    );
                }
            }
        }
    }

    fn validate_number_value(&mut self, value: f64, constraints: &Constraints, span: InputSpan) {
        // Check range constraints
        if let Some((min, max)) = constraints.range {
            let violation = match (min, max) {
                (Some(min_val), Some(max_val)) => value < min_val || value > max_val,
                (Some(min_val), None) => value < min_val,
                (None, Some(max_val)) => value > max_val,
                (None, None) => false,
            };

            if violation {
                self.add_error(
                    ValidationErrorKind::NumberRangeViolation {
                        min,
                        max,
                        actual: value,
                    },
                    span,
                );
            }
        }

        // Check exclusive bounds
        if let Some(exclusive_min) = constraints.exclusive_min
            && value <= exclusive_min
        {
            self.add_error(
                ValidationErrorKind::NumberRangeViolation {
                    min: Some(exclusive_min),
                    max: None,
                    actual: value,
                },
                span,
            );
        }

        if let Some(exclusive_max) = constraints.exclusive_max
            && value >= exclusive_max
        {
            self.add_error(
                ValidationErrorKind::NumberRangeViolation {
                    min: None,
                    max: Some(exclusive_max),
                    actual: value,
                },
                span,
            );
        }
    }

    fn infer_value_type<F: CstFacade>(&self, value_handle: ValueHandle, tree: &F) -> String {
        // Use ValueView to properly infer type
        if let Ok(view) = value_handle.get_view(tree) {
            match view {
                ValueView::Strings(_) => "string".to_string(),
                ValueView::Integer(_) => "number".to_string(),
                ValueView::Boolean(_) => "boolean".to_string(),
                ValueView::Null(_) => "null".to_string(),
                ValueView::Array(_) => "array".to_string(),
                ValueView::Object(_) => "object".to_string(),
                ValueView::Path(_) => "path".to_string(),
                ValueView::Code(_) | ValueView::CodeBlock(_) | ValueView::NamedCode(_) => "string".to_string(),
                ValueView::Hole(_) => "unknown".to_string(),
                ValueView::Tuple(_) => "tuple".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }

    fn get_span<F: CstFacade>(&self, node_id: CstNodeId, tree: &F) -> InputSpan {
        if let Some(node_data) = tree.node_data(node_id) {
            match node_data {
                CstNode::Terminal {
                    data: TerminalData::Input(span),
                    ..
                } => span,
                CstNode::NonTerminal {
                    data: NonTerminalData::Input(span),
                    ..
                } => span,
                _ => InputSpan::new(0, 0),
            }
        } else {
            InputSpan::new(0, 0)
        }
    }
}

impl<F: CstFacade> CstVisitor<F> for SchemaValidator<'_> {
    type Error = std::convert::Infallible;

    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Extract section path
        let mut section_path = Vec::new();
        let is_array = if let Ok(keys_view) = view.keys.get_view(tree) {
            self.extract_keys_path(&keys_view, tree, &mut section_path)
        } else {
            false
        };

        // Skip extension sections
        if !section_path.is_empty() && section_path[0].starts_with('$') {
            self.in_extension = true;
            let result = self.visit_section_super(handle, view, tree);
            self.in_extension = false;
            return result;
        }

        // Check section preferences
        let field_schema = section_path
            .last()
            .and_then(|name| self.find_field_schema(name));
            
        // Get preferences, following TypeRef if needed
        let preferences = if let Some(schema) = field_schema {
            match &schema.type_expr {
                Type::TypeRef(type_name) => {
                    // Look up preferences from the type definition
                    self.schema.types.get(type_name)
                        .map(|type_def| &type_def.preferences)
                        .unwrap_or(&schema.preferences)
                }
                _ => &schema.preferences
            }
        } else {
            &Preferences::default()
        };
        
        let needs_section_error = preferences.section
            .map(|prefer| !prefer)
            .unwrap_or(false);

        let needs_array_error = is_array
            && preferences.array
                .map(|prefer| !prefer)
                .unwrap_or(false);

        if needs_section_error {
            self.add_error(
                ValidationErrorKind::PreferSection {
                    path: self.current_path(),
                },
                self.get_span(handle.node_id(), tree),
            );
        }

        if needs_array_error {
            self.add_error(
                ValidationErrorKind::PreferArraySyntax {
                    path: self.current_path(),
                },
                self.get_span(handle.node_id(), tree),
            );
        }

        // Update state
        let path_len = section_path.len();

        // Update current_object_schema based on path
        if let Some(last_key) = section_path.last() {
            // For nested sections, we need to traverse the path
            if section_path.len() >= 2 {
                // Start from root schema
                let mut current_schema = &self.schema.root;
                let mut found_schema = true;
                
                // Traverse path segments except the last one
                for segment in section_path[..section_path.len()-1].iter() {
                    if let Some(field_schema) = current_schema.fields.get(segment) {
                        // Resolve type if it's a TypeRef
                        let resolved_type = match &field_schema.type_expr {
                            Type::TypeRef(type_name) => {
                                self.schema.types.get(type_name)
                                    .map(|type_def| &type_def.type_expr)
                                    .unwrap_or(&field_schema.type_expr)
                            }
                            other => other
                        };
                        
                        // Check if it's an object type
                        if let Type::Object(obj_schema) = resolved_type {
                            current_schema = obj_schema;
                        } else {
                            found_schema = false;
                            break;
                        }
                    } else {
                        found_schema = false;
                        break;
                    }
                }
                
                if found_schema {
                    // Now check if the last segment exists in current_schema
                    if let Some(field_schema) = current_schema.fields.get(last_key) {
                        // Resolve the type (handle TypeRef)
                        let resolved_type = match &field_schema.type_expr {
                            Type::TypeRef(type_name) => {
                                self.schema.types.get(type_name)
                                    .map(|type_def| &type_def.type_expr)
                                    .unwrap_or(&field_schema.type_expr)
                            }
                            other => other
                        };
                        
                        match resolved_type {
                            Type::Object(obj_schema) => {
                                // Clone the object schema first to avoid borrow issues
                                let obj_schema_clone = obj_schema.clone();
                                
                                // Track required fields for this object
                                let object_required: HashSet<String> = obj_schema_clone
                                    .fields
                                    .iter()
                                    .filter(|(_, field)| !field.optional)
                                    .map(|(name, _)| name.clone())
                                    .collect();
                                if !object_required.is_empty() {
                                    // Get the path that will be current after we push section_path
                                    let mut future_path = self.current_path();
                                    future_path.extend(section_path.clone());
                                    self.required_fields.insert(future_path, object_required);
                                }
                                
                                // Now update current_object_schema
                                self.current_object_schema = Some(obj_schema_clone);
                                self.current_variants = None;
                                self.selected_variant = None;
                            }
                            Type::Variants(variant_schema) => {
                                // For variant types, we store the variants and wait for $variant field
                                self.current_variants = Some(variant_schema.variants.clone());
                                self.current_object_schema = None;
                                self.selected_variant = None;
                            }
                            Type::Array(element_type) if is_array => {
                                // For array sections, we need to handle the element type
                                let resolved_element_type = match element_type.as_ref() {
                                    Type::TypeRef(type_name) => {
                                        self.schema.types.get(type_name)
                                            .map(|type_def| &type_def.type_expr)
                                            .unwrap_or(element_type.as_ref())
                                    }
                                    other => other
                                };
                                
                                match resolved_element_type {
                                    Type::Object(obj_schema) => {
                                        self.current_object_schema = Some(obj_schema.clone());
                                        self.current_variants = None;
                                        self.selected_variant = None;
                                    }
                                    Type::Variants(variant_schema) => {
                                        self.current_variants = Some(variant_schema.variants.clone());
                                        self.current_object_schema = None;
                                        self.selected_variant = None;
                                    }
                                    _ => {
                                        self.current_object_schema = None;
                                        self.current_variants = None;
                                        self.selected_variant = None;
                                    }
                                }
                            }
                            _ => {
                                // Not an object or variant type
                                self.current_object_schema = None;
                                self.current_variants = None;
                                self.selected_variant = None;
                            }
                        }
                    }
                }
            } else if let Some(field_schema) = self.find_field_schema(last_key) {
                // Single-segment section path (e.g., @ user)
                // Resolve the type (handle TypeRef)
                let resolved_type = match &field_schema.type_expr {
                    Type::TypeRef(type_name) => {
                        self.schema.types.get(type_name)
                            .map(|type_def| &type_def.type_expr)
                            .unwrap_or(&field_schema.type_expr)
                    }
                    other => other
                };
                
                match resolved_type {
                    Type::Object(obj_schema) => {
                        // Clone the object schema first to avoid borrow issues
                        let obj_schema_clone = obj_schema.clone();
                        
                        // Track required fields for this object
                        let object_required: HashSet<String> = obj_schema_clone
                            .fields
                            .iter()
                            .filter(|(_, field)| !field.optional)
                            .map(|(name, _)| name.clone())
                            .collect();
                        if !object_required.is_empty() {
                            // Get the path that will be current after we push section_path
                            let mut future_path = self.current_path();
                            future_path.extend(section_path.clone());
                            self.required_fields.insert(future_path, object_required);
                        }
                        
                        // Now update current_object_schema
                        self.current_object_schema = Some(obj_schema_clone);
                        self.current_variants = None;
                        self.selected_variant = None;
                    }
                    Type::Variants(variant_schema) => {
                        // For variant types, we store the variants and wait for $variant field
                        self.current_variants = Some(variant_schema.variants.clone());
                        self.current_object_schema = None;
                        self.selected_variant = None;
                    }
                    Type::Array(element_type) if is_array => {
                        // For array sections, we need to handle the element type
                        let resolved_element_type = match element_type.as_ref() {
                            Type::TypeRef(type_name) => {
                                self.schema.types.get(type_name)
                                    .map(|type_def| &type_def.type_expr)
                                    .unwrap_or(element_type.as_ref())
                            }
                            other => other
                        };
                        
                        match resolved_element_type {
                            Type::Object(obj_schema) => {
                                self.current_object_schema = Some(obj_schema.clone());
                                self.current_variants = None;
                                self.selected_variant = None;
                            }
                            Type::Variants(variant_schema) => {
                                self.current_variants = Some(variant_schema.variants.clone());
                                self.current_object_schema = None;
                                self.selected_variant = None;
                            }
                            _ => {
                                self.current_object_schema = None;
                                self.current_variants = None;
                                self.selected_variant = None;
                            }
                        }
                    }
                    _ => {
                        // Not an object or variant type
                        self.current_object_schema = None;
                        self.current_variants = None;
                        self.selected_variant = None;
                    }
                }
            }
        }

        self.path_stack.extend(section_path.clone());
        self.section_is_array.push(is_array);
        
        // Track that this section field has been seen
        if section_path.len() >= 2 {
            // For nested sections like @ person.address, mark "address" as seen in "person"
            let parent_path: Vec<String> = self.path_stack[..self.path_stack.len() - 1].to_vec();
            let field_name = section_path.last().unwrap().clone();
            self.seen_fields
                .entry(parent_path)
                .or_default()
                .insert(field_name);
        } else if section_path.len() == 1 && !section_path[0].starts_with('$') {
            // For root sections, mark them as seen at root level
            let field_name = section_path[0].clone();
            self.seen_fields
                .entry(vec![])
                .or_default()
                .insert(field_name);
        }

        // Save current variant context
        let saved_object_schema = self.current_object_schema.clone();
        let saved_variants = self.current_variants.clone();
        let saved_selected_variant = self.selected_variant.clone();

        // Visit children
        let result = self.visit_section_super(handle, view, tree);

        // Restore state
        for _ in 0..path_len {
            self.path_stack.pop();
        }
        self.section_is_array.pop();
        
        // Restore variant context
        self.current_object_schema = saved_object_schema;
        self.current_variants = saved_variants;
        self.selected_variant = saved_selected_variant;

        result
    }

    fn visit_binding(
        &mut self,
        handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Skip if in extension
        if self.in_extension {
            return Ok(());
        }

        // Extract key
        let mut key_path = Vec::new();
        if let Ok(keys_view) = view.keys.get_view(tree) {
            self.extract_keys_path(&keys_view, tree, &mut key_path);
        }

        if !key_path.is_empty() {
            let key = &key_path[0];
            
            // Check if this is a $variant field in a variant context
            if key == "$variant" && self.current_variants.is_some() {
                // Handle variant selection
                if let Ok(binding_rhs_view) = view.binding_rhs.get_view(tree)
                    && let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view
                    && let Ok(value_binding_view) = value_binding_handle.get_view(tree)
                    && let Ok(value_view) = value_binding_view.value.get_view(tree)
                {
                    // Extract the variant name
                    if let ValueView::Strings(_) = value_view {
                        if let Some(variant_name) = self.extract_string_value(value_binding_view.value, tree) {
                            // Validate variant exists and update current schema
                            if let Some(variants) = &self.current_variants {
                                if let Some(variant_object_schema) = variants.get(&variant_name) {
                                    // Set the selected variant and its object schema
                                    self.selected_variant = Some(variant_name);
                                    self.current_object_schema = Some(variant_object_schema.clone());
                                } else {
                                    // Unknown variant
                                    let available: Vec<String> = variants.keys().cloned().collect();
                                    self.add_error(
                                        ValidationErrorKind::UnknownVariant {
                                            variant: variant_name,
                                            available,
                                        },
                                        self.get_span(handle.node_id(), tree),
                                    );
                                }
                            }
                        }
                    }
                }
                return Ok(());
            }
            
            // Skip if any part of the key path is an extension (but not $variant)
            if key_path.iter().any(|k| k.starts_with('$')) {
                return Ok(());
            }

            // Track seen field
            let current_path = self.current_path();
            self.seen_fields
                .entry(current_path)
                .or_default()
                .insert(key.clone());

            // Find field schema
            if let Some(field_schema) = self.find_field_schema(key) {
                // Push expected type
                self.type_stack.push(field_schema.type_expr.clone());
                
                // Push key to path stack for constraint validation
                self.path_stack.push(key.clone());

                // Validate the value
                if let Ok(binding_rhs_view) = view.binding_rhs.get_view(tree)
                    && let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view
                    && let Ok(value_binding_view) = value_binding_handle.get_view(tree)
                    && let Ok(value_view) = value_binding_view.value.get_view(tree)
                {
                    let _ = self.visit_value(value_binding_view.value, value_view, tree);
                }

                // Pop key from path stack
                self.path_stack.pop();
                
                // Pop type
                self.type_stack.pop();
            } else if self.current_object_schema.is_some() && self.cascade_field_schema.is_none() {
                // Unexpected field (only if we have a schema and no cascade type)
                self.add_error(
                    ValidationErrorKind::UnexpectedField {
                        field: key.clone(),
                        path: self.current_path(),
                    },
                    self.get_span(handle.node_id(), tree),
                );
            }
        }

        Ok(())
    }

    fn visit_value(
        &mut self,
        handle: ValueHandle,
        view: ValueView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some(expected_type) = self.type_stack.last().cloned() {
            let actual_type = self.infer_value_type(handle, tree);

            // Basic type checking
            let type_matches = match &expected_type {
                Type::Any => true,
                Type::String => actual_type == "string",
                Type::Number => actual_type == "number",
                Type::Boolean => actual_type == "boolean",
                Type::Null => actual_type == "null",
                Type::Array(_) => actual_type == "array",
                Type::Object(_) => actual_type == "object",
                Type::Path => actual_type == "path",
                Type::Union(types) => {
                    // Check if actual type matches any in union
                    types.iter().any(|t| match t {
                        Type::String => actual_type == "string",
                        Type::Number => actual_type == "number",
                        Type::Boolean => actual_type == "boolean",
                        Type::Null => actual_type == "null",
                        _ => false,
                    })
                }
                Type::TypedString(_kind) => {
                    actual_type == "string" // TypedString is still a string at runtime
                }
                Type::Code(_) => actual_type == "string", // Code is stored as string
                Type::TypeRef(type_name) => {
                    // Look up the type definition
                    if let Some(type_def) = self.schema.types.get(type_name) {
                        // Recursively check against the referenced type
                        let mut temp_type_stack = self.type_stack.clone();
                        temp_type_stack.push(type_def.type_expr.clone());
                        let old_stack = std::mem::replace(&mut self.type_stack, temp_type_stack);
                        
                        let type_matches = match &type_def.type_expr {
                            Type::String | Type::TypedString(_) | Type::Code(_) => {
                                actual_type == "string"
                            }
                            Type::Number => actual_type == "number",
                            Type::Boolean => actual_type == "boolean",
                            Type::Null => actual_type == "null",
                            Type::Array(_) => actual_type == "array",
                            Type::Object(_) => actual_type == "object",
                            Type::Path => actual_type == "path",
                            Type::Union(types) => {
                                // Check if actual type matches any in union
                                types.iter().any(|t| match t {
                                    Type::String => actual_type == "string",
                                    Type::Number => actual_type == "number",
                                    Type::Boolean => actual_type == "boolean",
                                    Type::Null => actual_type == "null",
                                    _ => false,
                                })
                            }
                            Type::Any => true,
                            Type::TypeRef(_) => true, // Nested TypeRef - would need recursive handling
                            Type::Variants(_) => actual_type == "object",
                            Type::CascadeType(_) => true,
                        };
                        
                        self.type_stack = old_stack;
                        type_matches
                    } else {
                        false // Unknown type reference
                    }
                }
                Type::Variants(_variant_schema) => {
                    // For variants, check if we have the discriminator field
                    actual_type == "object"
                }
                Type::CascadeType(_) => true, // Cascade type matches anything
            };

            if !type_matches {
                let expected = format!("{expected_type:?}");
                let span = self.get_span(handle.node_id(), tree);
                self.add_error(
                    ValidationErrorKind::TypeMismatch {
                        expected,
                        actual: actual_type,
                    },
                    span,
                );
            } else {
                // Additional validation based on type
                match &expected_type {
                    Type::String => {
                        if let Some(str_value) = self.extract_string_value(handle, tree)
                            && let Some(key) = self.path_stack.last()
                            && let Some(field_schema) = self.find_field_schema(key)
                        {
                            let constraints = field_schema.constraints.clone();
                            self.validate_string_value(
                                &str_value,
                                &constraints,
                                self.get_span(handle.node_id(), tree),
                            );
                        }
                    }
                    Type::Number => {
                        if let Some(num_value) = self.extract_number_value(handle, tree)
                            && let Some(key) = self.path_stack.last()
                            && let Some(field_schema) = self.find_field_schema(key)
                        {
                            let constraints = field_schema.constraints.clone();
                            self.validate_number_value(
                                num_value,
                                &constraints,
                                self.get_span(handle.node_id(), tree),
                            );
                        }
                    }
                    Type::Array(_elem_type) => {
                        // Validate array constraints
                        let array_constraints = self
                            .path_stack
                            .last()
                            .and_then(|key| self.find_field_schema(key))
                            .map(|schema| schema.constraints.clone());

                        if let Some(constraints) = array_constraints {
                            // Check array length constraints
                            if let Ok(_array_view) = handle.get_view(tree) {
                                // Count array elements
                                let mut element_count = 0;
                                let node_id = handle.node_id();
                                for _ in tree.children(node_id) {
                                    element_count += 1;
                                }

                                // Adjust for brackets
                                if element_count >= 2 {
                                    element_count -= 2; // Remove [ and ]
                                }

                                if let Some(min) = constraints.min_items
                                    && element_count < min
                                {
                                    self.add_error(
                                        ValidationErrorKind::ArrayLengthViolation {
                                            min: Some(min),
                                            max: constraints.max_items,
                                            actual: element_count,
                                        },
                                        self.get_span(handle.node_id(), tree),
                                    );
                                }

                                if let Some(max) = constraints.max_items
                                    && element_count > max
                                {
                                    self.add_error(
                                        ValidationErrorKind::ArrayLengthViolation {
                                            min: constraints.min_items,
                                            max: Some(max),
                                            actual: element_count,
                                        },
                                        self.get_span(handle.node_id(), tree),
                                    );
                                }

                                // Validate individual array elements against elem_type
                                if let Ok(ValueView::Array(array_handle)) = handle.get_view(tree)
                                    && let Ok(array_view) = array_handle.get_view(tree) {
                                        self.validate_array_elements(array_view, _elem_type, tree);
                                    }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        self.visit_value_super(handle, view, tree)
    }

    fn visit_terminal(
        &mut self,
        _id: CstNodeId,
        _kind: TerminalKind,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        // Terminal validation is handled in visit_value
        Ok(())
    }
}

impl SchemaValidator<'_> {
    fn extract_keys_path<F: CstFacade>(
        &mut self,
        keys_view: &KeysView,
        tree: &F,
        path: &mut Vec<String>,
    ) -> bool {
        let mut is_array = false;

        // Extract the first key
        if let Ok(key_view) = keys_view.key.get_view(tree) {
            // Check if key has array marker
            if let Ok(key_opt_view) = key_view.key_opt.get_view(tree)
                && key_opt_view.is_some()
            {
                is_array = true;
            }
            self.extract_key(key_view, tree, path);
        }

        // Extract remaining keys if any
        if let Ok(Some(mut current_view)) = keys_view.keys_list.get_view(tree) {
            // Iterate over keys list manually
            loop {
                if let Ok(key_view) = current_view.key.get_view(tree) {
                    // Check if key has array marker
                    if let Ok(key_opt_view) = key_view.key_opt.get_view(tree)
                        && key_opt_view.is_some()
                    {
                        is_array = true;
                    }
                    self.extract_key(key_view, tree, path);
                }

                // Get next item in the list
                match current_view.keys_list.get_view(tree) {
                    Ok(Some(next_view)) => current_view = next_view,
                    _ => break,
                }
            }
        }

        is_array
    }

    fn extract_key<F: CstFacade>(&self, key_view: KeyView, tree: &F, path: &mut Vec<String>) {
        if let Ok(key_base_view) = key_view.key_base.get_view(tree) {
            match key_base_view {
                KeyBaseView::Ident(ident_handle) => {
                    if let Ok(ident_view) = ident_handle.get_view(tree)
                        && let Ok(data) = ident_view.ident.get_data(tree)
                        && let Some(s) = tree.get_str(data, self.input)
                    {
                        path.push(s.to_string());
                    }
                }
                KeyBaseView::Str(str_handle) => {
                    if let Ok(str_view) = str_handle.get_view(tree)
                        && let Ok(data) = str_view.str.get_data(tree)
                        && let Some(s) = tree.get_str(data, self.input)
                    {
                        // Remove quotes and parse string literal
                        let string_value = s.trim_matches('"').to_string();
                        path.push(string_value);
                    }
                }
                KeyBaseView::ExtensionNameSpace(ext_handle) => {
                    if let Ok(ext_view) = ext_handle.get_view(tree)
                        && let Ok(ident_view) = ext_view.ident.get_view(tree)
                        && let Ok(data) = ident_view.ident.get_data(tree)
                        && let Some(s) = tree.get_str(data, self.input)
                    {
                        path.push(format!("${s}"));
                    }
                }
                KeyBaseView::Integer(_) => {
                    // Integer keys are not used in schema paths
                }
                KeyBaseView::Null(_) => {
                    path.push("null".to_string());
                }
                KeyBaseView::True(_) => {
                    path.push("true".to_string());
                }
                KeyBaseView::False(_) => {
                    path.push("false".to_string());
                }
                KeyBaseView::MetaExtKey(meta_ext_handle) => {
                    if let Ok(meta_ext_view) = meta_ext_handle.get_view(tree)
                        && let Ok(ident_view) = meta_ext_view.ident.get_view(tree)
                            && let Ok(data) = ident_view.ident.get_data(tree)
                                && let Some(s) = tree.get_str(data, self.input) {
                                    path.push(format!("$̄{s}"));
                                }
                }
            }
        }
    }

    fn _current_field_schema(&self) -> Option<&FieldSchema> {
        // Get the field schema for the current position
        if let Some(key) = self.path_stack.last() {
            self.find_field_schema(key)
        } else {
            None
        }
    }

    fn extract_string_value<F: CstFacade>(
        &self,
        value_handle: ValueHandle,
        tree: &F,
    ) -> Option<String> {
        if let Ok(value_view) = value_handle.get_view(tree)
            && let ValueView::Strings(strings_handle) = value_view
                && let Ok(strings_view) = strings_handle.get_view(tree)
                    && let Ok(str_view) = strings_view.str.get_view(tree)
                    && let Ok(data) = str_view.str.get_data(tree)
                    && let Some(s) = tree.get_str(data, self.input)
                {
                    // Parse the string literal (remove quotes and unescape)
                    let unquoted = s.trim_matches('"');
                    return Some(unquoted.to_string());
                }
        None
    }

    fn extract_number_value<F: CstFacade>(
        &self,
        value_handle: ValueHandle,
        tree: &F,
    ) -> Option<f64> {
        if let Ok(value_view) = value_handle.get_view(tree)
            && let ValueView::Integer(integer_handle) = value_view
                && let Ok(integer_view) = integer_handle.get_view(tree)
                    && let Ok(data) = integer_view.integer.get_data(tree)
                    && let Some(s) = tree.get_str(data, self.input)
                    && let Ok(n) = s.parse::<f64>()
                {
                    return Some(n);
                }
        None
    }

    fn validate_array_elements<F: CstFacade>(
        &mut self,
        array_view: ArrayView,
        elem_type: &Type,
        tree: &F,
    ) {
        // Check if array has elements
        if let Ok(Some(array_opt)) = array_view.array_opt.get_view(tree)
            && let Ok(array_elements_view) = array_opt.get_view(tree) {
                // Validate first element
                self.validate_array_element(array_elements_view.value, elem_type, tree);
                
                // Validate remaining elements if any
                if let Ok(Some(tail_opt)) = array_elements_view.array_elements_opt.get_view(tree)
                    && let Ok(tail_view) = tail_opt.get_view(tree) {
                        self.validate_array_elements_tail(tail_view, elem_type, tree);
                    }
            }
    }
    
    fn validate_array_element<F: CstFacade>(
        &mut self,
        value_handle: ValueHandle,
        expected_type: &Type,
        tree: &F,
    ) {
        // Store the current expected type for this element
        let prev_expected = self.expected_type.clone();
        self.expected_type = Some(expected_type.clone());
        
        // Visit the value to validate it
        if let Ok(view) = value_handle.get_view(tree) {
            let _ = self.visit_value(value_handle, view, tree);
        }
        
        // Restore previous expected type
        self.expected_type = prev_expected;
    }
    
    fn validate_array_elements_tail<F: CstFacade>(
        &mut self,
        tail_view: ArrayElementsTailView,
        elem_type: &Type,
        tree: &F,
    ) {
        // Check if there are more elements
        if let Ok(Some(elements_handle)) = tail_view.array_elements_tail_opt.get_view(tree) {
            // The elements_handle is ArrayElementsHandle
            if let Ok(elements_view) = elements_handle.get_view(tree) {
                // Validate this element
                self.validate_array_element(elements_view.value, elem_type, tree);
                
                // Continue with remaining elements
                if let Ok(Some(more_tail)) = elements_view.array_elements_opt.get_view(tree)
                    && let Ok(more_tail_view) = more_tail.get_view(tree) {
                        self.validate_array_elements_tail(more_tail_view, elem_type, tree);
                    }
            }
        }
    }
}
