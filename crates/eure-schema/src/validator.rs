//! Schema validation for EURE documents

use crate::schema::*;
use eure_tree::prelude::*;
use eure_tree::tree::InputSpan;
use std::collections::{HashMap, HashSet};

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
    _array_values: HashMap<Vec<String>, Vec<String>>,

    // Current context
    current_object_schema: Option<ObjectSchema>,
    _current_variant: Option<String>,
    in_extension: bool,

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

        // Extract required fields from type definitions
        for (type_name, field_schema) in &schema.types {
            if let Type::Object(obj_schema) = &field_schema.type_expr {
                let type_required: HashSet<String> = obj_schema
                    .fields
                    .iter()
                    .filter(|(_, field)| !field.optional)
                    .map(|(name, _)| name.clone())
                    .collect();
                if !type_required.is_empty() {
                    required_fields.insert(vec![format!("$types.{}", type_name)], type_required);
                }
            }
        }

        // Create cascade field schema if needed
        let cascade_field_schema = if let Some(Type::CascadeType(cascade)) = &schema.cascade_type {
            Some(FieldSchema {
                type_expr: (**cascade).clone(),
                optional: false,
                constraints: Constraints::default(),
                preferences: Preferences::default(),
                serde: SerdeOptions::default(),
                span: None,
            })
        } else {
            None
        };

        Self {
            input,
            schema,
            path_stack: Vec::new(),
            type_stack: Vec::new(),
            section_is_array: Vec::new(),
            seen_fields: HashMap::new(),
            required_fields,
            _array_values: HashMap::new(),
            current_object_schema: None,
            _current_variant: None,
            in_extension: false,
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
                Err(_) => {
                    // Invalid regex pattern in schema - this is a schema error
                    // For now, we'll skip validation
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
        let node_id = value_handle.node_id();

        // Check for terminals
        let children: Vec<_> = tree.children(node_id).collect();
        if let Some(first_child) = children.first()
            && let Some(node_data) = tree.node_data(*first_child)
        {
            match node_data {
                CstNode::Terminal {
                    kind: TerminalKind::Str,
                    ..
                } => return "string".to_string(),
                CstNode::Terminal {
                    kind: TerminalKind::Integer,
                    ..
                } => return "number".to_string(),
                CstNode::Terminal {
                    kind: TerminalKind::True,
                    ..
                }
                | CstNode::Terminal {
                    kind: TerminalKind::False,
                    ..
                } => return "boolean".to_string(),
                CstNode::Terminal {
                    kind: TerminalKind::Null,
                    ..
                } => return "null".to_string(),
                CstNode::Terminal {
                    kind: TerminalKind::LBracket,
                    ..
                } => return "array".to_string(),
                CstNode::Terminal {
                    kind: TerminalKind::LBrace,
                    ..
                } => return "object".to_string(),
                CstNode::Terminal {
                    kind: TerminalKind::Dot,
                    ..
                } => return "path".to_string(),
                _ => {}
            }
        }

        "unknown".to_string()
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
        let needs_section_error = section_path
            .last()
            .and_then(|name| self.find_field_schema(name))
            .and_then(|field_schema| field_schema.preferences.section)
            .map(|prefer| !prefer)
            .unwrap_or(false);

        let needs_array_error = is_array
            && section_path
                .last()
                .and_then(|name| self.find_field_schema(name))
                .and_then(|field_schema| field_schema.preferences.array)
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
        if let Some(last_key) = section_path.last()
            && let Some(field_schema) = self.find_field_schema(last_key)
            && let Type::Object(obj_schema) = &field_schema.type_expr
        {
            self.current_object_schema = Some(obj_schema.clone());
        }

        self.path_stack.extend(section_path);
        self.section_is_array.push(is_array);

        // Visit children
        let result = self.visit_section_super(handle, view, tree);

        // Restore state
        for _ in 0..path_len {
            self.path_stack.pop();
        }
        self.section_is_array.pop();

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

        if let Some(key) = key_path.first() {
            // Skip extension keys
            if key.starts_with('$') {
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

                // Validate the value
                if let Ok(binding_rhs_view) = view.binding_rhs.get_view(tree)
                    && let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view
                    && let Ok(value_binding_view) = value_binding_handle.get_view(tree)
                    && let Ok(value_view) = value_binding_view.value.get_view(tree)
                {
                    let _ = self.visit_value(value_binding_view.value, value_view, tree);
                }

                // Pop type
                self.type_stack.pop();
            } else if self.current_object_schema.is_some() {
                // Unexpected field (only if we have a schema)
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
                        match &type_def.type_expr {
                            Type::String | Type::TypedString(_) | Type::Code(_) => {
                                actual_type == "string"
                            }
                            Type::Number => actual_type == "number",
                            Type::Boolean => actual_type == "boolean",
                            Type::Null => actual_type == "null",
                            Type::Array(_) => actual_type == "array",
                            Type::Object(_) => actual_type == "object",
                            Type::Path => actual_type == "path",
                            _ => true,
                        }
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
            }

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

                            // TODO: Validate individual array elements against elem_type
                            // This would require visiting array elements with the expected type
                        }
                    }
                }
                _ => {}
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
        let node_id = value_handle.node_id();
        for child_id in tree.children(node_id) {
            if let Some(node_data) = tree.node_data(child_id)
                && let CstNode::Terminal {
                    kind: TerminalKind::Str,
                    data,
                } = node_data
                && let Some(s) = tree.get_str(data, self.input)
            {
                return Some(s.to_string());
            }
        }
        None
    }

    fn extract_number_value<F: CstFacade>(
        &self,
        value_handle: ValueHandle,
        tree: &F,
    ) -> Option<f64> {
        let node_id = value_handle.node_id();
        for child_id in tree.children(node_id) {
            if let Some(node_data) = tree.node_data(child_id)
                && let CstNode::Terminal {
                    kind: TerminalKind::Integer,
                    data,
                } = node_data
                && let Some(s) = tree.get_str(data, self.input)
                && let Ok(n) = s.parse::<f64>()
            {
                return Some(n);
            }
        }
        None
    }
}
