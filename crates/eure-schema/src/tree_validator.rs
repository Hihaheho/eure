//! Tree-based schema validation that preserves span information
//!
//! This module provides validation of EURE documents against schemas
//! while preserving span information for error reporting.

use crate::schema::*;
use crate::value_validator::{Severity, ValidationError, ValidationErrorKind};
use eure_tree::{
    document::EureDocument,
    nodes::{BindingRhsView, SectionBodyView, SectionHandle, SectionView},
    prelude::*,
    tree::{CstNodeData, InputSpan, NonTerminalData, RecursiveView, TerminalData},
};
use eure_value::identifier::Identifier;
use eure_value::value::{KeyCmpValue, Map, PathSegment, Value, PathKey};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;


/// A tree-based validator that preserves span information
pub struct SchemaValidator<'a> {
    _input: &'a str,
    schema: &'a DocumentSchema,
    document: &'a EureDocument,
    errors: Vec<ValidationError>,
    current_path: Vec<PathSegment>,
    seen_fields: HashMap<PathKey, HashSet<KeyCmpValue>>,
    /// Track variant context for proper field validation
    variant_context: Option<String>,
    /// Track variant names at specific paths for finalize checking
    variant_at_path: HashMap<PathKey, String>,
    /// Cascade types at all paths (empty PathKey = root)
    cascade_types: &'a HashMap<PathKey, Type>,
}

impl<'a> SchemaValidator<'a> {
    /// Create a new schema validator
    pub fn new(input: &'a str, schema: &'a DocumentSchema, document: &'a EureDocument) -> Self {
        Self {
            _input: input,
            schema,
            document,
            errors: Vec::new(),
            current_path: Vec::new(),
            seen_fields: HashMap::new(),
            variant_context: None,
            variant_at_path: HashMap::new(),
            cascade_types: &schema.cascade_types,
        }
    }

    /// Get the validation errors
    pub fn into_errors(self) -> Vec<ValidationError> {
        self.errors
    }

    /// Finalize validation by checking for missing required fields
    pub fn finalize(&mut self) {
        // Check for missing required fields at all levels
        // Start with the root
        self.check_fields_at_path(&[], &self.schema.root.fields, None);

        // Check all nested levels that we've tracked
        self.check_nested_required_fields(&[], &self.schema.root.fields);
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
                CstNodeData::Terminal {
                    data: TerminalData::Input(span),
                    ..
                } => Some(span),
                CstNodeData::NonTerminal {
                    data: NonTerminalData::Input(span),
                    ..
                } => Some(span),
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
        // The root validation will be handled by individual bindings
        // We only check if the root is an object type
        if let Some((value, span_opt)) = self.values.get_eure_with_span(&handle) {
            let span = span_opt
                .copied()
                .or_else(|| self.get_span_from_node(handle.node_id(), tree));

            // Only validate that root is an object, not its contents
            if !matches!(value, Value::Map(_)) {
                self.add_error(
                    ValidationErrorKind::TypeMismatch {
                        expected: "object".to_string(),
                        actual: value_type_name(value).to_string(),
                    },
                    span,
                );
            }
        }

        // Continue visiting children - this will handle field validation
        self.visit_eure_super(handle, view, tree)?;

        // Don't check for missing fields here - it happens too early
        // The check will be done after all visiting is complete

        Ok(())
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
                        // For text bindings, we need to handle them specially
                        // They don't have a value in the Values structure, but we still
                        // need to validate that the field exists in the schema
                        // Return a special marker to indicate this is a text binding
                        Some(&Value::Null)
                    }
                    Ok(BindingRhsView::SectionBinding(_)) => {
                        // Section bindings create an object/map value
                        // We need to track that we've seen this field
                        // Return a placeholder to indicate the field exists
                        Some(&Value::Map(Default::default()))
                    }
                    _ => None,
                };

                if let Some(binding_value) = binding_value {
                    // Handle merged schema+data format
                    // If the value is a map with _value key, extract the actual data value
                    let actual_value = if let Value::Map(map) = binding_value {
                        if let Some(data_value) =
                            map.0.get(&KeyCmpValue::String("_value".to_string()))
                        {
                            data_value
                        } else {
                            binding_value
                        }
                    } else {
                        binding_value
                    };
                    let span = self.get_span_from_node(handle.node_id(), tree);

                    // Look up the field schema based on the path and validate
                    eprintln!(
                        "DEBUG: Looking up field schema for path {:?}, current_path: {:?}, variant_context: {:?}",
                        path, self.current_path, self.variant_context
                    );

                    // If we don't have variant context but we're in a path that might be inside a variant,
                    // we'll let lookup_field_schema try to find the field in any variant

                    let field_schema_opt = self.lookup_field_schema(&path).cloned();
                    let is_extension = self.is_extension_field(&path);

                    if let Some(field_schema) = field_schema_opt {
                        // Track that we've seen this field at its parent level
                        if !path.is_empty() {
                            let field_name = match path.last().unwrap() {
                                PathSegment::Ident(ident) => {
                                    Some(KeyCmpValue::String(ident.as_ref().to_string()))
                                }
                                PathSegment::Array { key, .. } => {
                                    if let Value::String(name) = key {
                                        Some(KeyCmpValue::String(name.clone()))
                                    } else {
                                        None
                                    }
                                }
                                PathSegment::Value(KeyCmpValue::String(s)) => {
                                    // Quoted field names like "a.b.c" are parsed as Value(String)
                                    Some(KeyCmpValue::String(s.clone()))
                                }
                                _ => None,
                            };

                            if let Some(field_name) = field_name {
                                // The parent path is the current path
                                self.seen_fields
                                    .entry(PathKey::from_segments(&self.current_path))
                                    .or_default()
                                    .insert(field_name);
                            }
                        }

                        // Update current path for error reporting
                        let old_path = std::mem::replace(&mut self.current_path, path.clone());

                        // Validate the value against the field schema
                        self.validate_value(actual_value, &field_schema, span);

                        // Restore path
                        self.current_path = old_path;
                    } else if !is_extension {
                        // Field not found - check for cascade types
                        let cascade_type = self.find_cascade_type(&self.current_path).cloned();
                        if let Some(cascade_type) = cascade_type {
                            // Track that we've seen this field at its parent level
                            if !path.is_empty() {
                                let field_name = match path.last().unwrap() {
                                    PathSegment::Ident(ident) => {
                                        Some(KeyCmpValue::String(ident.as_ref().to_string()))
                                    }
                                    PathSegment::Array { key, .. } => {
                                        if let Value::String(name) = key {
                                            Some(KeyCmpValue::String(name.clone()))
                                        } else {
                                            None
                                        }
                                    }
                                    PathSegment::Value(KeyCmpValue::String(s)) => {
                                        Some(KeyCmpValue::String(s.clone()))
                                    }
                                    _ => None,
                                };

                                if let Some(field_name) = field_name {
                                    self.seen_fields
                                        .entry(PathKey::from_segments(&self.current_path))
                                        .or_default()
                                        .insert(field_name);
                                }
                            }

                            // Create a field schema with the cascade type
                            let cascade_field = FieldSchema {
                                type_expr: cascade_type,
                                optional: false,
                                ..Default::default()
                            };

                            // Update current path for error reporting
                            let old_path = std::mem::replace(&mut self.current_path, path.clone());

                            // Validate the value against the cascade type
                            self.validate_value(actual_value, &cascade_field, span);

                            // Restore path
                            self.current_path = old_path;
                        } else {
                            // No cascade type - check if we're in a variant array path
                            let mut found_in_variant = false;

                            if self.variant_context.is_none() && path.len() == 1 {
                                // Check if current path suggests we're inside a variant array element
                                if let Some(field_key) = path[0].as_ident() {
                                    let current_path_clone = self.current_path.clone();
                                    found_in_variant = self
                                        .check_field_in_variant_path(&current_path_clone, field_key);
                                }
                            }

                            if !found_in_variant {
                                // Field not found in schema and not an extension - report as unexpected
                                if let Some(field_key) = path.last().and_then(|seg| match seg {
                                    PathSegment::Ident(name) => {
                                        Some(KeyCmpValue::String(name.as_ref().to_string()))
                                    }
                                    PathSegment::Value(KeyCmpValue::String(s)) => {
                                        Some(KeyCmpValue::String(s.clone()))
                                    }
                                    _ => None,
                                }) {
                                    self.add_error(
                                        ValidationErrorKind::UnexpectedField {
                                            field: field_key,
                                            path: self.current_path.clone(),
                                        },
                                        span,
                                    );
                                }
                            }
                        }
                    }

                    // Continue visiting
                    let old_path = std::mem::replace(&mut self.current_path, path);
                    let result = self.visit_binding_super(handle, view, tree);
                    self.current_path = old_path;

                    return result;
                }
            }
        }

        self.visit_binding_super(handle, view, tree)
    }

    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Sections create fields - track them
        if let Some(key_handles) = self.values.get_keys(&view.keys) {
            // Build the path from key handles
            let mut path = Vec::new();

            // For nested sections within blocks, we need to include the parent path
            // This ensures that @ choice[] inside @ script.actions[] {} maintains full context
            if !self.current_path.is_empty() {
                // If we already have a current path (from a parent section), start with that
                path.extend(self.current_path.clone());
            }

            for key_handle in key_handles {
                if let Some((segment, _)) = self.values.get_key_with_span(key_handle) {
                    path.push(segment.clone());
                }
            }

            // Track all fields in the path at their respective parent levels
            // For example, @ a.b.c.d should track:
            // - field "a" at root level
            // - field "b" at level [a]
            // - field "c" at level [a, b]
            // - field "d" at level [a, b, c]
            for i in 0..path.len() {
                let segment = &path[i];
                let field_name = match segment {
                    PathSegment::Ident(ident) => {
                        Some(KeyCmpValue::String(ident.as_ref().to_string()))
                    }
                    PathSegment::Array { key, .. } => {
                        if let Value::String(name) = key {
                            Some(KeyCmpValue::String(name.clone()))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(field_name) = field_name {
                    // The parent path is everything before this segment
                    let parent_path = path[..i].to_vec();
                    self.seen_fields
                        .entry(PathKey::from_segments(&parent_path))
                        .or_default()
                        .insert(field_name);
                }
            }

            // Save current context
            let old_path = self.current_path.clone();

            // Check if we should preserve variant context
            // The key insight: if we're in a nested array section like script.actions[].choice[],
            // we want to preserve the variant context from the parent actions[] element
            eprintln!(
                "DEBUG: visit_section - path: {:?}, variant_context: {:?}",
                path, self.variant_context
            );

            // Determine if this path is a child of a variant array element
            // For example: script.actions[].choice[] is a child of script.actions[]
            let should_preserve_variant = if path.len() >= 2 {
                // Check if there's a parent array in the path that might have variant context
                let mut found_parent_array = false;
                for i in 0..(path.len() - 1) {
                    if let PathSegment::Array { key, .. } = &path[i] {
                        // For array segments, extract the field name from the key
                        let array_field_name = if let Value::String(field_name) = key {
                            Some(field_name.as_str())
                        } else {
                            None
                        };

                        // The parent path is everything before this array segment
                        let parent_path = &path[0..i];

                        if let Some(field_name) = array_field_name {
                            eprintln!(
                                "DEBUG: Checking if '{field_name}' at position {i} is a variant array"
                            );

                            // Build the path to look up the array field
                            let mut lookup_path = parent_path.to_vec();
                            if !lookup_path.is_empty() {
                                // Remove the last array segment if it exists
                                if matches!(lookup_path.last(), Some(PathSegment::Array { .. })) {
                                    lookup_path.pop();
                                }
                            }

                            // Temporarily set current_path for lookup
                            let temp_path = std::mem::replace(&mut self.current_path, lookup_path);
                            let field_schema = self
                                .lookup_field_schema(&[PathSegment::Ident(
                                    Identifier::from_str(field_name).unwrap(),
                                )])
                                .cloned();
                            self.current_path = temp_path;

                            if let Some(schema) = field_schema
                                && let Type::Array(elem_type) = &schema.type_expr
                            {
                                match elem_type.as_ref() {
                                    Type::Variants(_) => {
                                        found_parent_array = true;
                                        eprintln!(
                                            "DEBUG: Found parent variant array at position {i}"
                                        );
                                        break;
                                    }
                                    Type::TypeRef(type_name) => {
                                        if let Some(type_def) = self.schema.types.get(type_name)
                                            && matches!(&type_def.type_expr, Type::Variants(_))
                                        {
                                            found_parent_array = true;
                                            eprintln!(
                                                "DEBUG: Found parent variant array (via type ref) at position {i}"
                                            );
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                found_parent_array
            } else {
                false
            };

            let should_clear_variant = !should_preserve_variant;

            eprintln!(
                "DEBUG: should_preserve_variant: {should_preserve_variant}, should_clear_variant: {should_clear_variant}"
            );

            let old_variant_context = self.variant_context.clone();
            if should_clear_variant {
                self.variant_context = None;
            }

            // Check if this is a variant array element BEFORE updating current_path
            // For sections like @ tasks[], check if tasks is an array of variants
            let is_variant_element = if let Some(PathSegment::Array { key, .. }) = path.last() {
                // Need to look up the field based on the parent path + field name
                // For script.actions[], we need to find the schema for the actions field
                // First, get the parent path without the array element
                let parent_path = &path[..path.len() - 1];

                // Extract the field name from the array key
                if let Value::String(field_name) = key {
                    // Build path to the array field
                    let mut field_path = parent_path.to_vec();
                    field_path.push(PathSegment::Ident(
                        Identifier::from_str(field_name).unwrap(),
                    ));

                    // Save current path temporarily
                    let temp_path = std::mem::replace(&mut self.current_path, parent_path.to_vec());

                    // Then look up the array field itself
                    let result = if let Some(field_schema) =
                        self.lookup_field_schema(&[PathSegment::Ident(
                            Identifier::from_str(field_name).unwrap(),
                        )]) {
                        if let Type::Array(elem_type) = &field_schema.type_expr {
                            match elem_type.as_ref() {
                                Type::Variants(_) => true,
                                Type::TypeRef(type_name) => {
                                    if let Some(type_def) = self.schema.types.get(type_name) {
                                        matches!(&type_def.type_expr, Type::Variants(_))
                                    } else {
                                        false
                                    }
                                }
                                _ => false,
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // Restore current path
                    self.current_path = temp_path;
                    result
                } else {
                    false
                }
            } else {
                false
            };

            // Now update current path to reflect we're inside this section
            self.current_path = path.clone();

            if is_variant_element {
                // Try to get the eure value from the section body
                match view.section_body.get_view(tree) {
                    Ok(section_body_view) => {
                        match &section_body_view {
                            SectionBodyView::SectionBodyList(_) => {}
                            SectionBodyView::SectionBinding(_) => {}
                            SectionBodyView::Bind(_) => {}
                        }
                        match section_body_view {
                            SectionBodyView::SectionBinding(binding_handle) => {
                                if let Ok(binding_view) = binding_handle.get_view(tree)
                                    && let Some((eure_value, _)) =
                                        self.values.get_eure_with_span(&binding_view.eure)
                                    && let Value::Map(map) = eure_value
                                {
                                    // Look for $variant field
                                    if let Some(Value::String(variant_name)) =
                                        map.0.get(&KeyCmpValue::Extension("variant".to_string()))
                                    {
                                        eprintln!(
                                            "DEBUG: Setting variant context to '{variant_name}' at path {path:?} (SectionBinding)"
                                        );
                                        self.variant_context = Some(variant_name.clone());
                                        // Store the variant name at this path for later checking
                                        self.variant_at_path.insert(
                                            PathKey::from_segments(&path),
                                            variant_name.clone(),
                                        );
                                    }
                                }
                            }
                            SectionBodyView::SectionBodyList(list_handle) => {
                                // Iterate through bindings to find $variant field
                                match list_handle.get_view(tree) {
                                    Ok(Some(list_view)) => {
                                        match list_view.get_all(tree) {
                                            Ok(bindings) => {
                                                for binding_handle in bindings.iter() {
                                                    if let Ok(binding_view) =
                                                        binding_handle.get_view(tree)
                                                        && let Some(key_handles) =
                                                            self.values.get_keys(&binding_view.keys)
                                                        && key_handles.len() == 1
                                                        && let Some((segment, _)) = self
                                                            .values
                                                            .get_key_with_span(&key_handles[0])
                                                        && let PathSegment::Extension(ext) = segment
                                                        && ext.as_ref() == "variant"
                                                    {
                                                        // Found $variant field
                                                        match binding_view
                                                            .binding_rhs
                                                            .get_view(tree)
                                                        {
                                                            Ok(BindingRhsView::ValueBinding(
                                                                value_binding,
                                                            )) => {
                                                                if let Ok(value_binding_view) =
                                                                    value_binding.get_view(tree)
                                                                    && let Some(value) =
                                                                        self.values.get_value(
                                                                            &value_binding_view
                                                                                .value,
                                                                        )
                                                                    && let Value::String(
                                                                        variant_name,
                                                                    ) = value
                                                                {
                                                                    eprintln!(
                                                                        "DEBUG: Setting variant context to '{variant_name}' at path {path:?}"
                                                                    );
                                                                    self.variant_context =
                                                                        Some(variant_name.clone());
                                                                    // Store the variant name at this path for later checking
                                                                    self.variant_at_path.insert(
                                                                        PathKey::from_segments(&path),
                                                                        variant_name.clone(),
                                                                    );
                                                                    break;
                                                                }
                                                            }
                                                            Ok(BindingRhsView::TextBinding(
                                                                text_binding,
                                                            )) => {
                                                                if let Ok(text_binding_view) =
                                                                    text_binding.get_view(tree)
                                                                {
                                                                    // text_binding_view.text is a TextHandle
                                                                    if let Ok(text_view) =
                                                                        text_binding_view
                                                                            .text
                                                                            .get_view(tree)
                                                                    {
                                                                        // Get the text directly from the tree
                                                                        if let Ok(data) = text_view
                                                                            .text
                                                                            .get_data(tree)
                                                                            && let Some(text) = tree
                                                                                .get_str(
                                                                                    data,
                                                                                    self._input,
                                                                                )
                                                                        {
                                                                            let variant_name =
                                                                                text.trim();
                                                                            self.variant_context =
                                                                                Some(
                                                                                    variant_name
                                                                                        .to_string(
                                                                                        ),
                                                                                );
                                                                            // Store the variant name at this path for later checking
                                                                            self.variant_at_path.insert(
                                                                                PathKey::from_segments(&path),
                                                                                variant_name.to_string(),
                                                                            );
                                                                            break;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            Ok(BindingRhsView::SectionBinding(
                                                                _,
                                                            )) => {}
                                                            Err(_e) => {}
                                                        }
                                                    }
                                                }
                                            }
                                            Err(_e) => {}
                                        }
                                    }
                                    Ok(None) => {}
                                    Err(_e) => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_e) => {}
                }
            }

            let result = self.visit_section_super(handle, view, tree);

            // Restore previous path
            self.current_path = old_path;

            // Restore variant context only if we decided to clear it
            if should_clear_variant {
                self.variant_context = old_variant_context;
            }

            return result;
        }

        self.visit_section_super(handle, view, tree)
    }
}

// Extension trait for PathSegment
trait PathSegmentExt {
    fn as_ident(&self) -> Option<&str>;
}

impl PathSegmentExt for PathSegment {
    fn as_ident(&self) -> Option<&str> {
        match self {
            PathSegment::Ident(ident) => Some(ident.as_ref()),
            _ => None,
        }
    }
}

// Helper functions for validation
impl<'a> SchemaValidator<'a> {
    /// Find the most specific cascade type for a given path
    fn find_cascade_type(&self, path: &[PathSegment]) -> Option<&Type> {
        // Check from current path up to root
        let mut check_path = path.to_vec();
        loop {
            let path_key = PathKey::from_segments(&check_path);
            if let Some(cascade_type) = self.cascade_types.get(&path_key) {
                return Some(cascade_type);
            }
            if check_path.is_empty() {
                break;
            }
            check_path.pop();
        }
        None
    }
    /// Check if a field exists in any variant when we're in a variant array path
    fn check_field_in_variant_path(&mut self, path: &[PathSegment], field_name: &str) -> bool {
        eprintln!("DEBUG: check_field_in_variant_path - path: {path:?}, field: {field_name}");

        // Check if we're in a nested array inside a variant array
        // Example path: [script, actions[], choice[]]
        if path.len() >= 2 {
            // Look for a variant array in the path
            for i in 0..path.len() {
                if let PathSegment::Array { key, .. } = &path[i]
                    && let Value::String(array_field_name) = key
                {
                    // Check if this is a variant array
                    let parent_path = &path[0..i];
                    let lookup_path: Vec<PathSegment> = parent_path
                        .iter()
                        .filter(|seg| !matches!(seg, PathSegment::Array { .. }))
                        .cloned()
                        .collect();

                    // Temporarily set current path for lookup
                    let temp_path = std::mem::replace(&mut self.current_path, lookup_path);
                    let array_field_schema = self
                        .lookup_field_schema(&[PathSegment::Ident(
                            Identifier::from_str(array_field_name).unwrap(),
                        )])
                        .cloned();
                    self.current_path = temp_path;

                    if let Some(schema) = array_field_schema
                        && let Type::Array(elem_type) = &schema.type_expr
                    {
                        match elem_type.as_ref() {
                            Type::Variants(variant_schema) => {
                                // This is a variant array - check all variants
                                eprintln!("DEBUG: Found variant array at position {i}");

                                // Now check if any variant has the path from here to the field
                                let remaining_path = &path[i + 1..];

                                for (variant_key, variant_obj) in &variant_schema.variants {
                                    if self.check_field_in_variant_fields(
                                        &variant_obj.fields,
                                        remaining_path,
                                        field_name,
                                    ) {
                                        eprintln!(
                                            "DEBUG: Field '{field_name}' found in variant {variant_key:?}"
                                        );
                                        return true;
                                    }
                                }
                            }
                            Type::TypeRef(type_name) => {
                                if let Some(type_def) = self.schema.types.get(type_name)
                                    && let Type::Variants(variant_schema) = &type_def.type_expr
                                {
                                    eprintln!(
                                        "DEBUG: Found variant array (via type ref) at position {i}"
                                    );

                                    let remaining_path = &path[i + 1..];

                                    for (variant_key, variant_obj) in &variant_schema.variants {
                                        if self.check_field_in_variant_fields(
                                            &variant_obj.fields,
                                            remaining_path,
                                            field_name,
                                        ) {
                                            eprintln!(
                                                "DEBUG: Field '{field_name}' found in variant {variant_key:?}"
                                            );
                                            return true;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if a field exists following a path through variant fields
    fn check_field_in_variant_fields(
        &self,
        fields: &IndexMap<KeyCmpValue, FieldSchema>,
        path: &[PathSegment],
        field_name: &str,
    ) -> bool {
        if path.is_empty() {
            // Direct field lookup
            return fields.contains_key(&KeyCmpValue::String(field_name.to_string()));
        }

        // Navigate through the path
        if let Some(PathSegment::Array { key, .. }) = path.first()
            && let Value::String(array_field_name) = key
        {
            let field_key = KeyCmpValue::String(array_field_name.clone());
            if let Some(field_schema) = fields.get(&field_key)
                && let Type::Array(elem_type) = &field_schema.type_expr
                && let Type::Object(obj_schema) = elem_type.as_ref()
            {
                // Continue with remaining path
                return self.check_field_in_variant_fields(
                    &obj_schema.fields,
                    &path[1..],
                    field_name,
                );
            }
        }

        false
    }

    /// Check if we're currently inside a variant array
    fn is_in_variant_array(&self) -> bool {
        // Check if current_path indicates we're in a variant array
        if self.current_path.is_empty() {
            return false;
        }

        // Check if any segment in the path is a variant array
        let mut current_fields = &self.schema.root.fields;

        for (i, segment) in self.current_path.iter().enumerate() {
            match segment {
                PathSegment::Array { .. } => {
                    // We're inside an array, check if the previous segment was a variant array field
                    if i > 0
                        && let PathSegment::Ident(ident) = &self.current_path[i - 1]
                    {
                        let field_key = KeyCmpValue::String(ident.as_ref().to_string());
                        if let Some(field) = current_fields.get(&field_key)
                            && let Type::Array(elem_type) = &field.type_expr
                        {
                            match elem_type.as_ref() {
                                Type::Variants(_) => return true,
                                Type::TypeRef(type_name) => {
                                    if let Some(type_def) = self.schema.types.get(type_name) {
                                        return matches!(&type_def.type_expr, Type::Variants(_));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                PathSegment::Ident(ident) => {
                    // Navigate deeper into the schema
                    let key = KeyCmpValue::String(ident.as_ref().to_string());
                    if let Some(field) = current_fields.get(&key) {
                        match &field.type_expr {
                            Type::Object(obj_schema) => {
                                current_fields = &obj_schema.fields;
                            }
                            Type::TypeRef(type_name) => {
                                if let Some(type_def) = self.schema.types.get(type_name)
                                    && let Type::Object(obj_schema) = &type_def.type_expr
                                {
                                    current_fields = &obj_schema.fields;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Check if a path points to a variant array element
    fn is_variant_array(&self, path: &[PathSegment]) -> bool {
        if path.is_empty() {
            return false;
        }

        // Combine current_path with the provided path
        let full_path: Vec<PathSegment> = self
            .current_path
            .iter()
            .chain(path.iter())
            .cloned()
            .collect();

        // Navigate through the schema to check if this is a variant array
        let mut current_fields = &self.schema.root.fields;

        for (i, segment) in full_path.iter().enumerate() {
            match segment {
                PathSegment::Ident(ident) => {
                    let key = KeyCmpValue::String(ident.as_ref().to_string());
                    if let Some(field) = current_fields.get(&key) {
                        // Check if this field is an array of variants
                        if let Type::Array(elem_type) = &field.type_expr {
                            // Check if the next segment is an array access
                            if i + 1 < full_path.len()
                                && let PathSegment::Array { .. } = &full_path[i + 1]
                            {
                                // Check if the element type is a variant
                                match elem_type.as_ref() {
                                    Type::Variants(_) => return true,
                                    Type::TypeRef(type_name) => {
                                        // Check if the type reference is a variant
                                        if let Some(type_def) = self.schema.types.get(type_name) {
                                            return matches!(
                                                &type_def.type_expr,
                                                Type::Variants(_)
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Continue navigating for non-array fields
                        match &field.type_expr {
                            Type::Object(obj_schema) => {
                                current_fields = &obj_schema.fields;
                            }
                            Type::TypeRef(type_name) => {
                                if let Some(type_def) = self.schema.types.get(type_name)
                                    && let Type::Object(obj_schema) = &type_def.type_expr
                                {
                                    current_fields = &obj_schema.fields;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                PathSegment::Array { .. } => {
                    // Already checked above
                    continue;
                }
                _ => {}
            }
        }

        false
    }

    /// Look up a field schema based on a path
    fn lookup_field_schema(&self, path: &[PathSegment]) -> Option<&FieldSchema> {
        if path.is_empty() {
            return None;
        }

        // Combine current_path with the provided path to get the full path
        let full_path: Vec<PathSegment> = self
            .current_path
            .iter()
            .chain(path.iter())
            .cloned()
            .collect();

        eprintln!(
            "DEBUG: lookup_field_schema - full_path: {:?}, variant_context: {:?}",
            full_path, self.variant_context
        );

        // Start with root schema
        let mut current_fields = &self.schema.root.fields;
        let mut field_schema: Option<&FieldSchema> = None;

        // Navigate through the full path
        for (i, segment) in full_path.iter().enumerate() {
            match segment {
                PathSegment::Ident(ident) => {
                    let key = KeyCmpValue::String(ident.as_ref().to_string());
                    if let Some(field) = current_fields.get(&key) {
                        field_schema = Some(field);

                        // If this is not the last segment, we need to navigate deeper
                        if i < full_path.len() - 1 {
                            match &field.type_expr {
                                Type::Object(obj_schema) => {
                                    current_fields = &obj_schema.fields;
                                }
                                Type::Array(elem_type) => {
                                    // For arrays, we need to check the next segment
                                    if i + 1 < full_path.len()
                                        && let PathSegment::Array { .. } = &full_path[i + 1]
                                    {
                                        // Handle array element type
                                        eprintln!(
                                            "DEBUG: Processing array segment at position {i}"
                                        );
                                        if let Type::Object(obj_schema) = elem_type.as_ref() {
                                            eprintln!("DEBUG: Array element is Object type");
                                            current_fields = &obj_schema.fields;
                                            continue;
                                        } else if let Type::Variants(variant_schema) =
                                            elem_type.as_ref()
                                        {
                                            eprintln!(
                                                "DEBUG: Array element is Variants type, variant_context: {:?}",
                                                self.variant_context
                                            );
                                            // For variant arrays, use the variant context if available
                                            if let Some(variant_name) = &self.variant_context
                                                && let Some(variant_obj) = variant_schema
                                                    .variants
                                                    .get(&KeyCmpValue::String(variant_name.clone()))
                                            {
                                                current_fields = &variant_obj.fields;
                                                continue;
                                            }
                                            // If no variant context, check if we're looking for a field that exists in any variant
                                            if i == full_path.len() - 2
                                                && let Some(PathSegment::Ident(field_ident)) =
                                                    full_path.get(i + 1)
                                            {
                                                let field_key = KeyCmpValue::String(
                                                    field_ident.as_ref().to_string(),
                                                );
                                                eprintln!(
                                                    "DEBUG: No variant context, checking if field '{field_ident}' exists in any variant"
                                                );

                                                // Check each variant to see if it has this field
                                                for (variant_key, variant_obj) in
                                                    &variant_schema.variants
                                                {
                                                    if let Some(field) =
                                                        variant_obj.fields.get(&field_key)
                                                    {
                                                        eprintln!(
                                                            "DEBUG: Found field '{field_ident}' in variant {variant_key:?}"
                                                        );
                                                        return Some(field);
                                                    }
                                                }
                                            }
                                            return None;
                                        }
                                    }
                                    return None;
                                }
                                Type::TypeRef(type_name) => {
                                    // Look up the type definition
                                    if let Some(type_def) = self.schema.types.get(type_name) {
                                        match &type_def.type_expr {
                                            Type::Object(obj_schema) => {
                                                current_fields = &obj_schema.fields;
                                            }
                                            Type::Variants(variant_schema) => {
                                                // For variant types, we need the variant context
                                                if let Some(variant_name) = &self.variant_context {
                                                    if let Some(variant_obj) = variant_schema
                                                        .variants
                                                        .get(&KeyCmpValue::String(
                                                            variant_name.clone(),
                                                        ))
                                                    {
                                                        current_fields = &variant_obj.fields;
                                                    } else {
                                                        return None;
                                                    }
                                                } else {
                                                    // No variant context available
                                                    return None;
                                                }
                                            }
                                            _ => return None,
                                        }
                                    } else {
                                        return None;
                                    }
                                }
                                _ => return None,
                            }
                        }
                    } else {
                        return None;
                    }
                }
                PathSegment::Array { key, .. } => {
                    // For array segments, we need to look up the array field by its key

                    // Extract the field name from the array key
                    if let Value::String(field_name) = key {
                        let array_key = KeyCmpValue::String(field_name.clone());

                        if let Some(array_field) = current_fields.get(&array_key) {
                            field_schema = Some(array_field);

                            if let Type::Array(elem_type) = &array_field.type_expr {
                                match elem_type.as_ref() {
                                    Type::Object(obj_schema) => {
                                        current_fields = &obj_schema.fields;
                                    }
                                    Type::Variants(variant_schema) => {
                                        // For variant arrays, use the variant context if available
                                        if let Some(variant_name) = &self.variant_context {
                                            if let Some(variant_obj) = variant_schema
                                                .variants
                                                .get(&KeyCmpValue::String(variant_name.clone()))
                                            {
                                                current_fields = &variant_obj.fields;
                                            } else {
                                                return None;
                                            }
                                        } else {
                                            // No variant context - this path cannot be resolved without knowing the variant
                                            return None;
                                        }
                                    }
                                    Type::TypeRef(type_name) => {
                                        // Handle array of type references
                                        if let Some(type_def) = self.schema.types.get(type_name) {
                                            match &type_def.type_expr {
                                                Type::Object(obj_schema) => {
                                                    current_fields = &obj_schema.fields;
                                                }
                                                Type::Variants(variant_schema) => {
                                                    // For variant arrays, use the variant context
                                                    if let Some(variant_name) =
                                                        &self.variant_context
                                                    {
                                                        if let Some(variant_obj) = variant_schema
                                                            .variants
                                                            .get(&KeyCmpValue::String(
                                                                variant_name.clone(),
                                                            ))
                                                        {
                                                            current_fields = &variant_obj.fields;
                                                        } else {
                                                            return None;
                                                        }
                                                    } else {
                                                        return None;
                                                    }
                                                }
                                                _ => return None,
                                            }
                                        } else {
                                            return None;
                                        }
                                    }
                                    _ => return None,
                                }
                            }
                        }
                    }
                    continue;
                }
                PathSegment::Value(KeyCmpValue::String(s)) => {
                    // Quoted field names like "a.b.c" are parsed as Value(String)
                    let key = KeyCmpValue::String(s.clone());
                    if let Some(field) = current_fields.get(&key) {
                        field_schema = Some(field);

                        // If this is not the last segment, we need to navigate deeper
                        if i < full_path.len() - 1 {
                            match &field.type_expr {
                                Type::Object(obj_schema) => {
                                    current_fields = &obj_schema.fields;
                                }
                                Type::TypeRef(type_name) => {
                                    // Look up the type definition
                                    if let Some(type_def) = self.schema.types.get(type_name) {
                                        match &type_def.type_expr {
                                            Type::Object(obj_schema) => {
                                                current_fields = &obj_schema.fields;
                                            }
                                            _ => return None,
                                        }
                                    } else {
                                        return None;
                                    }
                                }
                                _ => return None,
                            }
                        }
                    } else {
                        return None;
                    }
                }
                PathSegment::Extension(_) | PathSegment::MetaExt(_) => {
                    // Extension fields are metadata, not data fields
                    return None;
                }
                _ => return None,
            }
        }

        field_schema
    }

    /// Check if a path represents an extension field
    fn is_extension_field(&self, path: &[PathSegment]) -> bool {
        path.iter()
            .any(|segment| matches!(segment, PathSegment::Extension(_)))
    }

    /// Check for missing required fields recursively
    fn check_nested_required_fields(
        &mut self,
        path: &[PathSegment],
        fields: &IndexMap<KeyCmpValue, FieldSchema>,
    ) {
        // For each field that exists and has been seen, check its nested fields
        let path_key = PathKey::from_segments(path);
        let seen_fields_clone = self.seen_fields.get(&path_key).cloned();
        if let Some(seen_at_level) = seen_fields_clone {
            for (field_key, field_schema) in fields {
                if seen_at_level.contains(field_key) {
                    // This field was seen, check its nested fields if it's an object
                    match &field_schema.type_expr {
                        Type::Object(obj_schema) => {
                            // Build the path to this field
                            let mut nested_path = path.to_vec();
                            match field_key {
                                KeyCmpValue::String(name) => {
                                    nested_path.push(PathSegment::Ident(
                                        Identifier::from_str(name).unwrap(),
                                    ));
                                }
                                _ => continue,
                            }

                            // Check required fields at this nested level
                            self.check_fields_at_path(&nested_path, &obj_schema.fields, None);

                            // Recursively check deeper levels
                            self.check_nested_required_fields(&nested_path, &obj_schema.fields);
                        }
                        Type::Array(elem_type) => {
                            // For arrays, we need to check if the element type is a variant
                            match elem_type.as_ref() {
                                Type::Variants(variant_schema) => {
                                    // This is an array of variants
                                    let mut array_path = path.to_vec();
                                    match field_key {
                                        KeyCmpValue::String(name) => {
                                            array_path.push(PathSegment::Array {
                                                key: Value::String(name.clone()),
                                                index: None,
                                            });
                                        }
                                        _ => continue,
                                    }

                                    // Check variant fields for array elements
                                    self.check_variant_fields_at_path(
                                        &array_path,
                                        variant_schema,
                                        None,
                                    );
                                }
                                Type::TypeRef(type_name) => {
                                    // Check if the type reference is a variant
                                    if let Some(type_def) = self.schema.types.get(type_name)
                                        && let Type::Variants(variant_schema) = &type_def.type_expr
                                    {
                                        let mut array_path = path.to_vec();
                                        match field_key {
                                            KeyCmpValue::String(name) => {
                                                array_path.push(PathSegment::Array {
                                                    key: Value::String(name.clone()),
                                                    index: None,
                                                });
                                            }
                                            _ => continue,
                                        }

                                        // Check variant fields for array elements
                                        self.check_variant_fields_at_path(
                                            &array_path,
                                            variant_schema,
                                            None,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        Type::TypeRef(type_name) => {
                            // Look up the type and check if it's an object or variant
                            if let Some(type_def) = self.schema.types.get(type_name) {
                                match &type_def.type_expr {
                                    Type::Object(obj_schema) => {
                                        let mut nested_path = path.to_vec();
                                        match field_key {
                                            KeyCmpValue::String(name) => {
                                                nested_path.push(PathSegment::Ident(
                                                    Identifier::from_str(name).unwrap(),
                                                ));
                                            }
                                            _ => continue,
                                        }

                                        self.check_fields_at_path(
                                            &nested_path,
                                            &obj_schema.fields,
                                            None,
                                        );
                                        self.check_nested_required_fields(
                                            &nested_path,
                                            &obj_schema.fields,
                                        );
                                    }
                                    Type::Variants(variant_schema) => {
                                        // For variants, we need to check fields for each variant instance
                                        // This handles the case where a field directly references a variant type
                                        let mut nested_path = path.to_vec();
                                        match field_key {
                                            KeyCmpValue::String(name) => {
                                                nested_path.push(PathSegment::Ident(
                                                    Identifier::from_str(name).unwrap(),
                                                ));
                                            }
                                            _ => continue,
                                        }

                                        // Check variant fields at this path
                                        self.check_variant_fields_at_path(
                                            &nested_path,
                                            variant_schema,
                                            None,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Check required fields at a specific path
    fn check_fields_at_path(
        &mut self,
        path: &[PathSegment],
        fields: &IndexMap<KeyCmpValue, FieldSchema>,
        span: Option<InputSpan>,
    ) {
        // Get the set of fields seen at this level
        let seen_at_level = self
            .seen_fields
            .get(&PathKey::from_segments(path))
            .cloned()
            .unwrap_or_default();

        for (key, field_schema) in fields {
            if !field_schema.optional && !seen_at_level.contains(key) {
                self.add_error(
                    ValidationErrorKind::RequiredFieldMissing {
                        field: key.clone(),
                        path: path.to_vec(),
                    },
                    span,
                );
            }

            // TODO: Recursively check nested objects if field was seen
            // This would require traversing into nested object schemas
        }
    }

    /// Check variant fields at a specific path
    fn check_variant_fields_at_path(
        &mut self,
        path: &[PathSegment],
        variant_schema: &VariantSchema,
        span: Option<InputSpan>,
    ) {
        // Look for the variant name stored at this path
        let path_key = PathKey::from_segments(path);

        if let Some(variant_name) = self.variant_at_path.get(&path_key) {
            // We know which variant was used at this path
            if let Some(variant_obj_schema) = variant_schema
                .variants
                .get(&KeyCmpValue::String(variant_name.clone()))
            {
                // Check the required fields for this specific variant
                let seen_at_level = self.seen_fields.get(&path_key).cloned().unwrap_or_default();

                for (field_key, field_schema) in &variant_obj_schema.fields {
                    if !field_schema.optional && !seen_at_level.contains(field_key) {
                        self.add_error(
                            ValidationErrorKind::RequiredFieldMissing {
                                field: field_key.clone(),
                                path: path.to_vec(),
                            },
                            span,
                        );
                    }
                }
            }
        }
    }

    /// Validate an object against an object schema
    fn validate_object(&mut self, map: &Map, schema: &ObjectSchema, span: Option<InputSpan>) {
        // Track required fields
        let mut required_fields: HashSet<KeyCmpValue> = schema
            .fields
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
                        // Validate the field value
                        // Note: The span here is for the entire object, not the specific field
                        // Field-specific spans are handled in visit_binding
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
                KeyCmpValue::Extension(_ext_name) => {
                    // Extension fields are metadata and don't need schema validation
                    // They are used for schema definition itself (e.g., $type, $optional)
                    // The schema extraction phase has already processed these
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
        // Handle merged schema+data format
        // If the value is a map with _value key, use that for validation
        let actual_value = if let Value::Map(map) = value {
            if let Some(data_value) = map.0.get(&KeyCmpValue::String("_value".to_string())) {
                data_value
            } else {
                value
            }
        } else {
            value
        };

        // Check type matching
        if !self.type_matches(actual_value, &schema.type_expr) {
            self.add_error(
                ValidationErrorKind::TypeMismatch {
                    expected: type_to_string(&schema.type_expr),
                    actual: value_type_name(actual_value).to_string(),
                },
                span,
            );
            return; // No point checking constraints if type is wrong
        }

        // Validate constraints
        self.validate_constraints(actual_value, &schema.constraints, span);

        // For complex types, perform deep validation
        match (&schema.type_expr, actual_value) {
            (Type::Array(elem_schema), Value::Array(array)) => {
                // Validate each array element
                for (i, elem) in array.0.iter().enumerate() {
                    let elem_path = PathSegment::Value(KeyCmpValue::U64(i as u64));
                    self.current_path.push(elem_path.clone());

                    let elem_field = FieldSchema {
                        type_expr: *elem_schema.clone(),
                        optional: false,
                        ..Default::default()
                    };
                    self.validate_value(elem, &elem_field, span);

                    self.current_path.pop();
                }
            }
            (Type::Array(elem_schema), Value::Tuple(tuple)) => {
                // Validate each tuple element
                for (i, elem) in tuple.0.iter().enumerate() {
                    if i > 255 {
                        self.add_error(
                            ValidationErrorKind::InvalidValue(
                                "Tuple index exceeds maximum of 255".to_string(),
                            ),
                            span,
                        );
                        break;
                    }
                    let elem_path = PathSegment::TupleIndex(i as u8);
                    self.current_path.push(elem_path);

                    let elem_field = FieldSchema {
                        type_expr: *elem_schema.clone(),
                        optional: false,
                        ..Default::default()
                    };
                    self.validate_value(elem, &elem_field, span);

                    self.current_path.pop();
                }
            }
            (Type::Tuple(elem_types), Value::Tuple(tuple)) => {
                // Validate tuple with specific types for each position
                // First check length matches
                if elem_types.len() != tuple.0.len() {
                    self.add_error(
                        ValidationErrorKind::InvalidValue(format!(
                            "Tuple length mismatch: expected {}, actual: {}",
                            elem_types.len(),
                            tuple.0.len()
                        )),
                        span,
                    );
                }

                // Validate each element with its specific type
                for (i, (elem, expected_type)) in tuple.0.iter().zip(elem_types.iter()).enumerate()
                {
                    if i > 255 {
                        self.add_error(
                            ValidationErrorKind::InvalidValue(
                                "Tuple index exceeds maximum of 255".to_string(),
                            ),
                            span,
                        );
                        break;
                    }
                    let elem_path = PathSegment::TupleIndex(i as u8);
                    self.current_path.push(elem_path);

                    let elem_field = FieldSchema {
                        type_expr: expected_type.clone(),
                        optional: false,
                        ..Default::default()
                    };
                    self.validate_value(elem, &elem_field, span);

                    self.current_path.pop();
                }
            }
            (Type::Object(obj_schema), Value::Map(map)) => {
                // Validate as an object
                self.validate_object(map, obj_schema, span);
            }
            (Type::Variants(variant_schema), Value::Map(map)) => {
                // Check for $variant field
                if let Some(Value::String(variant_name)) =
                    map.0.get(&KeyCmpValue::Extension("variant".to_string()))
                {
                    if let Some(variant_obj_schema) = variant_schema
                        .variants
                        .get(&KeyCmpValue::String(variant_name.clone()))
                    {
                        // Validate the variant fields
                        self.validate_object(map, variant_obj_schema, span);
                    } else {
                        self.add_error(
                            ValidationErrorKind::UnknownVariant {
                                variant: variant_name.clone(),
                                available: variant_schema
                                    .variants
                                    .keys()
                                    .filter_map(|k| match k {
                                        KeyCmpValue::String(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                    .collect(),
                            },
                            span,
                        );
                    }
                } else {
                    self.add_error(ValidationErrorKind::MissingVariantTag, span);
                }
            }
            _ => {
                // Other types don't need deep validation
            }
        }
    }

    /// Validate value constraints
    fn validate_constraints(
        &mut self,
        value: &Value,
        constraints: &crate::schema::Constraints,
        span: Option<InputSpan>,
    ) {
        // String constraints
        if let Value::String(s) = value {
            if let Some((min_opt, max_opt)) = &constraints.length {
                let len = s.len();

                if let Some(min_length) = min_opt
                    && len < *min_length
                {
                    self.add_error(
                        ValidationErrorKind::StringLengthViolation {
                            min: Some(*min_length),
                            max: *max_opt,
                            actual: len,
                        },
                        span,
                    );
                }

                if let Some(max_length) = max_opt
                    && len > *max_length
                {
                    self.add_error(
                        ValidationErrorKind::StringLengthViolation {
                            min: *min_opt,
                            max: Some(*max_length),
                            actual: len,
                        },
                        span,
                    );
                }
            }

            if let Some(pattern) = &constraints.pattern {
                match regex::Regex::new(pattern) {
                    Ok(re) => {
                        if !re.is_match(s) {
                            self.add_error(
                                ValidationErrorKind::StringPatternViolation {
                                    pattern: pattern.clone(),
                                    value: s.clone(),
                                },
                                span,
                            );
                        }
                    }
                    Err(e) => {
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

        // Number constraints
        if let Some(num_value) = match value {
            Value::I64(n) => Some(*n as f64),
            Value::U64(n) => Some(*n as f64),
            Value::F64(n) => Some(*n),
            Value::F32(n) => Some(*n as f64),
            _ => None,
        } {
            // Check inclusive range
            if let Some((min_opt, max_opt)) = &constraints.range {
                if let Some(minimum) = min_opt
                    && num_value < *minimum
                {
                    self.add_error(
                        ValidationErrorKind::NumberRangeViolation {
                            min: Some(*minimum),
                            max: *max_opt,
                            actual: num_value,
                        },
                        span,
                    );
                }

                if let Some(maximum) = max_opt
                    && num_value > *maximum
                {
                    self.add_error(
                        ValidationErrorKind::NumberRangeViolation {
                            min: *min_opt,
                            max: Some(*maximum),
                            actual: num_value,
                        },
                        span,
                    );
                }
            }

            // Check exclusive bounds
            if let Some(exclusive_min) = constraints.exclusive_min
                && num_value <= exclusive_min
            {
                self.add_error(
                    ValidationErrorKind::NumberRangeViolation {
                        min: Some(exclusive_min),
                        max: None,
                        actual: num_value,
                    },
                    span,
                );
            }

            if let Some(exclusive_max) = constraints.exclusive_max
                && num_value >= exclusive_max
            {
                self.add_error(
                    ValidationErrorKind::NumberRangeViolation {
                        min: None,
                        max: Some(exclusive_max),
                        actual: num_value,
                    },
                    span,
                );
            }
        }

        // Array constraints
        let array_len = match value {
            Value::Array(array) => Some(array.0.len()),
            Value::Tuple(tuple) => Some(tuple.0.len()),
            _ => None,
        };

        if let Some(len) = array_len {
            if let Some(min_items) = constraints.min_items
                && len < min_items
            {
                self.add_error(
                    ValidationErrorKind::ArrayLengthViolation {
                        min: Some(min_items),
                        max: constraints.max_items,
                        actual: len,
                    },
                    span,
                );
            }

            if let Some(max_items) = constraints.max_items
                && len > max_items
            {
                self.add_error(
                    ValidationErrorKind::ArrayLengthViolation {
                        min: constraints.min_items,
                        max: Some(max_items),
                        actual: len,
                    },
                    span,
                );
            }

            if let Some(true) = constraints.unique {
                // TODO: Implement unique items check
            }
        }
    }

    /// Check if a value matches a type
    fn type_matches(&self, value: &Value, expected_type: &Type) -> bool {
        match (value, expected_type) {
            // Basic types
            (Value::Null, Type::Null) => true,
            (Value::Bool(_), Type::Boolean) => true,
            (Value::I64(_) | Value::U64(_), Type::Number) => true,
            (Value::F32(_) | Value::F64(_), Type::Number) => true,
            (Value::String(_), Type::String) => true,
            (Value::Array(_), Type::Array(_)) => true,
            (Value::Tuple(_), Type::Array(_)) => true,
            (Value::Tuple(_), Type::Tuple(_)) => true,
            (Value::Map(_), Type::Object(_)) => true,
            (Value::Path(_), Type::Path) => true,
            (_, Type::Any) => true,

            // Code types (both inline/named and code blocks)
            (Value::Code(code) | Value::CodeBlock(code), Type::Code(expected_lang)) => {
                expected_lang
                    .as_ref()
                    .is_none_or(|expected| code.language == *expected)
            }

            // Unions - value matches if it matches any variant
            (_, Type::Union(variants)) => variants
                .iter()
                .any(|variant| self.type_matches(value, variant)),

            // Variants
            (Value::Variant(variant), Type::Variants(schema)) => {
                // Check if the variant tag exists in the schema
                schema
                    .variants
                    .contains_key(&KeyCmpValue::String(variant.tag.clone()))
            }
            (Value::Map(map), Type::Variants(_)) => {
                // Check if map contains $variant field
                map.0
                    .contains_key(&KeyCmpValue::Extension("variant".to_string()))
            }

            // Type references
            (_, Type::TypeRef(name)) => {
                // Look up the type definition
                if let Some(type_def) = self.schema.types.get(name) {
                    self.type_matches(value, &type_def.type_expr)
                } else {
                    false // Type not found
                }
            }

            // Cascade types - handled at a higher level
            (_, Type::CascadeType(_)) => true,

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
        Value::CodeBlock(_) => "code",
        Value::Code(_) => "code",
        Value::Path(_) => "path",
        Value::Variant(_) => "variant",
        Value::Hole => "hole",
    }
}

/// Convert a path to a KeyCmpValue for error reporting
#[allow(dead_code)]
fn path_to_key(path: &[PathSegment]) -> KeyCmpValue {
    if path.is_empty() {
        KeyCmpValue::String("<empty>".to_string())
    } else if path.len() == 1 {
        match &path[0] {
            PathSegment::Ident(ident) => KeyCmpValue::String(ident.as_ref().to_string()),
            PathSegment::Extension(ident) => KeyCmpValue::Extension(ident.as_ref().to_string()),
            PathSegment::MetaExt(ident) => KeyCmpValue::MetaExtension(ident.as_ref().to_string()),
            PathSegment::Value(val) => val.clone(),
            PathSegment::TupleIndex(idx) => KeyCmpValue::U64(*idx as u64),
            PathSegment::Array { .. } => KeyCmpValue::String("<array>".to_string()),
        }
    } else {
        // For multi-segment paths, create a dotted string representation
        let segments: Vec<String> = path
            .iter()
            .map(|seg| match seg {
                PathSegment::Ident(ident) => ident.as_ref().to_string(),
                PathSegment::Extension(ident) => format!("${}", ident.as_ref()),
                PathSegment::MetaExt(ident) => format!("$${}", ident.as_ref()),
                PathSegment::Value(val) => format!("{val:?}"),
                PathSegment::TupleIndex(idx) => format!("[{idx}]"),
                PathSegment::Array { .. } => "[...]".to_string(),
            })
            .collect();
        KeyCmpValue::String(segments.join("."))
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
        Type::Code(lang) => {
            if let Some(lang) = lang {
                format!("code.{lang}")
            } else {
                "code".to_string()
            }
        }
        Type::Array(_) => "array".to_string(),
        Type::Object(_) => "object".to_string(),
        Type::Tuple(types) => {
            let type_strs: Vec<String> = types.iter().map(type_to_string).collect();
            format!("({})", type_strs.join(", "))
        }
        Type::Union(_) => "union".to_string(),
        Type::Variants(_) => "variant".to_string(),
        Type::TypeRef(name) => match name {
            KeyCmpValue::String(s) => format!("${s}"),
            _ => format!("${name:?}"),
        },
        Type::CascadeType(_) => "cascade".to_string(),
    }
}
