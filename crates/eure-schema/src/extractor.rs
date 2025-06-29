//! Schema extraction from EURE documents

use crate::schema::*;
use eure_tree::prelude::*;
use std::collections::HashMap;

/// Extracts schema information from EURE documents
pub struct SchemaExtractor<'a> {
    // Input string for extracting text
    input: &'a str,

    // Current traversal state
    path_stack: Vec<String>,
    in_types_section: bool,
    in_variants_section: bool,
    current_variant: Option<String>,

    // Schema being built
    current_field_schema: Option<FieldSchema>,

    // Extracted schemas
    document_schema: DocumentSchema,

    // Tracking
    has_non_schema_content: bool,

    // Temporary storage for values
    pending_value: Option<ExtractedValue>,
}

#[derive(Debug, Clone)]
enum ExtractedValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ExtractedValue>),
    Object(HashMap<String, ExtractedValue>),
    Path(String),
    Null,
}

impl<'a> SchemaExtractor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            path_stack: Vec::new(),
            in_types_section: false,
            in_variants_section: false,
            current_variant: None,
            current_field_schema: None,
            document_schema: DocumentSchema::default(),
            has_non_schema_content: false,
            pending_value: None,
        }
    }

    /// Consume the extractor and return the extracted schema
    pub fn extract(mut self) -> ExtractedSchema {
        // If we have inline schemas but no explicit cascade type, set it to Any
        if self.has_non_schema_content && self.document_schema.cascade_type.is_none() && !self.document_schema.root.fields.is_empty() {
            self.document_schema.cascade_type = Some(Type::Any);
        }
        
        ExtractedSchema {
            document_schema: self.document_schema,
            is_pure_schema: !self.has_non_schema_content,
        }
    }

    fn current_path(&self) -> Vec<String> {
        self.path_stack.clone()
    }

    fn handle_schema_extension(&mut self, key: &str) {
        match key {
            "$type" => {
                if let Some(value) = &self.pending_value
                    && let Some(path_str) = self.extract_string(value)
                    && let Some(type_expr) = Type::from_path(&path_str)
                {
                    self.set_current_type(type_expr);
                }
            }
            "$union" => {
                if let Some(ExtractedValue::Array(values)) = &self.pending_value {
                    let types: Vec<Type> = values
                        .iter()
                        .filter_map(|v| self.extract_string(v))
                        .filter_map(|s| Type::from_path(&s))
                        .collect();

                    if !types.is_empty() {
                        self.set_current_type(Type::Union(types));
                    }
                }
            }
            "$array" => {
                if let Some(value) = &self.pending_value
                    && let Some(path_str) = self.extract_string(value)
                    && let Some(element_type) = Type::from_path(&path_str)
                {
                    self.set_current_type(Type::Array(Box::new(element_type)));
                }
            }
            "$optional" => {
                if let Some(ExtractedValue::Boolean(true)) = &self.pending_value
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.optional = true;
                }
            }
            "$cascade-type" => {
                if self.path_stack.is_empty() {
                    // Global cascade type
                    let pending_value = self.pending_value.clone();
                    if let Some(path_str) =
                        pending_value.as_ref().and_then(|v| self.extract_string(v))
                        && let Some(type_expr) = Type::from_path(&path_str)
                    {
                        self.document_schema.cascade_type = Some(type_expr);
                    }
                } else if self.current_field_schema.is_some() {
                    let pending_value = self.pending_value.clone();
                    if let Some(path_str) =
                        pending_value.as_ref().and_then(|v| self.extract_string(v))
                        && let Some(type_expr) = Type::from_path(&path_str)
                        && let Some(schema) = &mut self.current_field_schema
                    {
                        schema.type_expr = Type::CascadeType(Box::new(type_expr));
                    }
                }
            }
            // Constraints
            "$length" => {
                if let Some(ExtractedValue::Array(values)) = &self.pending_value
                    && values.len() == 2
                {
                    let min = self.extract_number(&values[0]).map(|n| n as usize);
                    let max = self.extract_number(&values[1]).map(|n| n as usize);
                    if let Some(schema) = &mut self.current_field_schema {
                        schema.constraints.length = Some((min, max));
                    }
                }
            }
            "$pattern" => {
                if let Some(pattern) = self
                    .pending_value
                    .as_ref()
                    .and_then(|v| self.extract_string(v))
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.constraints.pattern = Some(pattern);
                }
            }
            "$range" => {
                if let Some(ExtractedValue::Array(values)) = &self.pending_value
                    && values.len() == 2
                {
                    let min = self.extract_number(&values[0]);
                    let max = self.extract_number(&values[1]);
                    if let Some(schema) = &mut self.current_field_schema {
                        schema.constraints.range = Some((min, max));
                    }
                }
            }
            "$min-items" => {
                if let Some(n) = self
                    .pending_value
                    .as_ref()
                    .and_then(|v| self.extract_number(v))
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.constraints.min_items = Some(n as usize);
                }
            }
            "$max-items" => {
                if let Some(n) = self
                    .pending_value
                    .as_ref()
                    .and_then(|v| self.extract_number(v))
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.constraints.max_items = Some(n as usize);
                }
            }
            "$unique" => {
                if let Some(ExtractedValue::Boolean(b)) = &self.pending_value
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.constraints.unique = Some(*b);
                }
            }
            // Preferences
            "$prefer.section" => {
                if let Some(ExtractedValue::Boolean(b)) = &self.pending_value
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.preferences.section = Some(*b);
                }
            }
            "$prefer.array" => {
                if let Some(ExtractedValue::Boolean(b)) = &self.pending_value
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.preferences.array = Some(*b);
                }
            }
            // Serde
            "$serde.rename" => {
                if let Some(name) = self
                    .pending_value
                    .as_ref()
                    .and_then(|v| self.extract_string(v))
                    && let Some(schema) = &mut self.current_field_schema
                {
                    schema.serde.rename = Some(name);
                }
            }
            "$serde.rename-all" => {
                if let Some(rule_str) = self
                    .pending_value
                    .as_ref()
                    .and_then(|v| self.extract_string(v))
                    && let Some(rule) = self.parse_rename_rule(&rule_str)
                {
                    if self.path_stack.is_empty() {
                        self.document_schema.serde_options.rename_all = Some(rule);
                    } else if let Some(schema) = &mut self.current_field_schema {
                        schema.serde.rename_all = Some(rule);
                    }
                }
            }
            "$variant-repr" => {
                // Handle variant representation
                // $variant-repr is a union type with these possible values:
                // "untagged" (when type is { $literal = "untagged" })
                // "external" (when type is { $literal = "external" })
                // { tag = "fieldname" } (internal tagging)
                // { tag = "tagfield", content = "contentfield" } (adjacent tagging)
                if self.in_variants_section
                    && self.current_field_schema.is_some()
                    && let Some(value) = self.pending_value.as_ref()
                {
                    let repr = match value {
                        // Handle literal string values (from $literal types)
                        ExtractedValue::String(s) => {
                            match s.as_str() {
                                "untagged" => VariantRepr::Untagged,
                                "external" => VariantRepr::Tagged, // External is the default tagged representation
                                _ => VariantRepr::Tagged, // Unknown string, default to tagged
                            }
                        }
                        // Handle object values for internal/adjacent tagging
                        ExtractedValue::Object(fields) => {
                            if let Some(tag_value) = fields.get("tag") {
                                // Extract the tag field name
                                let tag_field = match tag_value {
                                    ExtractedValue::String(s) => s.clone(),
                                    ExtractedValue::Path(p) => p.strip_prefix('.').unwrap_or(p).to_string(),
                                    _ => "tag".to_string(), // Default if not a string or path
                                };
                                
                                if let Some(content_value) = fields.get("content") {
                                    // Adjacent tagging: { tag = "...", content = "..." }
                                    let content_field = match content_value {
                                        ExtractedValue::String(s) => s.clone(),
                                        ExtractedValue::Path(p) => p.strip_prefix('.').unwrap_or(p).to_string(),
                                        _ => "content".to_string(), // Default if not a string or path
                                    };
                                    VariantRepr::AdjacentlyTagged {
                                        tag: tag_field,
                                        content: content_field,
                                    }
                                } else {
                                    // Internal tagging: { tag = "..." }
                                    VariantRepr::InternallyTagged {
                                        tag: tag_field,
                                    }
                                }
                            } else {
                                // Invalid object structure, default to tagged
                                VariantRepr::Tagged
                            }
                        }
                        _ => VariantRepr::Tagged, // Not a string or object, default to tagged
                    };

                    // Apply to current variant schema if we're building one
                    if let Some(schema) = &mut self.current_field_schema
                        && let Type::Variants(ref mut variant_schema) = schema.type_expr
                    {
                        variant_schema.representation = repr;
                    }
                }
            }
            _ => {
                // Other extensions are not schema-related, ignore them
            }
        }
    }

    fn set_current_type(&mut self, type_expr: Type) {
        if let Some(schema) = &mut self.current_field_schema {
            schema.type_expr = type_expr;
        }
    }

    fn extract_string(&self, value: &ExtractedValue) -> Option<String> {
        match value {
            ExtractedValue::String(s) => Some(s.clone()),
            ExtractedValue::Path(p) => Some(p.clone()),
            _ => None,
        }
    }

    fn extract_number(&self, value: &ExtractedValue) -> Option<f64> {
        match value {
            ExtractedValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn parse_rename_rule(&self, s: &str) -> Option<RenameRule> {
        match s {
            "camelCase" => Some(RenameRule::CamelCase),
            "snake_case" => Some(RenameRule::SnakeCase),
            "kebab-case" => Some(RenameRule::KebabCase),
            "PascalCase" => Some(RenameRule::PascalCase),
            "lowercase" => Some(RenameRule::Lowercase),
            "UPPERCASE" => Some(RenameRule::Uppercase),
            _ => None,
        }
    }

    fn save_current_type_definition(&mut self, name: String) {
        if let Some(schema) = self.current_field_schema.take() {
            self.document_schema.types.insert(name, schema);
        }
    }

    fn save_inline_schema(&mut self) {
        if let Some(schema) = self.current_field_schema.take() {
            let path = self.current_path();
            if !path.is_empty() && !path[0].starts_with('$') {
                // Merge inline schema directly into document schema
                let field_name = path.last().unwrap().clone();
                
                // Merge with existing schema if present
                let merged_schema = if let Some(existing) = self.document_schema.root.fields.get(&field_name) {
                    FieldSchema {
                        type_expr: if matches!(schema.type_expr, Type::Any) {
                            existing.type_expr.clone()
                        } else {
                            schema.type_expr
                        },
                        optional: schema.optional || existing.optional,
                        constraints: Constraints {
                            length: schema.constraints.length.or(existing.constraints.length),
                            pattern: schema.constraints.pattern.or(existing.constraints.pattern.clone()),
                            range: schema.constraints.range.or(existing.constraints.range),
                            exclusive_min: schema.constraints.exclusive_min.or(existing.constraints.exclusive_min),
                            exclusive_max: schema.constraints.exclusive_max.or(existing.constraints.exclusive_max),
                            min_items: schema.constraints.min_items.or(existing.constraints.min_items),
                            max_items: schema.constraints.max_items.or(existing.constraints.max_items),
                            unique: schema.constraints.unique.or(existing.constraints.unique),
                            contains: schema.constraints.contains.or(existing.constraints.contains.clone()),
                        },
                        preferences: Preferences {
                            section: schema.preferences.section.or(existing.preferences.section),
                            array: schema.preferences.array.or(existing.preferences.array),
                        },
                        serde: SerdeOptions {
                            rename: schema.serde.rename.or(existing.serde.rename.clone()),
                            rename_all: schema.serde.rename_all.or(existing.serde.rename_all),
                        },
                        span: schema.span.or(existing.span),
                        default_value: None,
                        description: None,
                    }
                } else {
                    schema
                };
                
                self.document_schema.root.fields.insert(field_name, merged_schema);
            }
        }
        // Reset for next field
        self.current_field_schema = None;
    }
}

impl<F: CstFacade> CstVisitor<F> for SchemaExtractor<'_> {
    type Error = std::convert::Infallible;

    fn visit_section(
        &mut self,
        handle: SectionHandle,
        view: SectionView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Extract section path
        let mut section_path = Vec::new();
        if let Ok(keys_view) = view.keys.get_view(tree) {
            self.extract_keys_path(&keys_view, tree, &mut section_path);
        }

        // Handle $types section
        if !section_path.is_empty() && section_path[0] == "$types" {
            self.in_types_section = true;

            if section_path.len() == 2 {
                // Save previous type definition if any
                if let Some(prev_name) = self.path_stack.get(1) {
                    self.save_current_type_definition(prev_name.clone());
                }

                // Start new type definition
                self.current_field_schema = Some(FieldSchema::default());
            } else if section_path.len() >= 3 {
                // This is a field definition for an object type
                // e.g., @ $types.Person.name
                let type_name = &section_path[1];
                let _field_name = &section_path[2];
                
                // Make sure the type exists and is an object
                if let Some(type_def) = self.document_schema.types.get_mut(type_name) {
                    // Ensure it's an object type
                    if !matches!(type_def.type_expr, Type::Object(_)) {
                        type_def.type_expr = Type::Object(ObjectSchema::default());
                    }
                    
                    // Create field schema for this field
                    self.current_field_schema = Some(FieldSchema::default());
                }
            }
        } else if !section_path.is_empty() && section_path[0] == "$variants" {
            self.in_variants_section = true;

            if section_path.len() == 2 {
                self.current_variant = Some(section_path[1].clone());
            }
        } else if !section_path.is_empty() && !section_path[0].starts_with('$') {
            // Regular content
            self.has_non_schema_content = true;
        }

        // Push path
        let path_len = section_path.len();
        self.path_stack.extend(section_path.clone());

        // Visit children
        let result = self.visit_section_super(handle, view, tree);

        // Handle exiting sections
        if self.in_types_section && self.path_stack.len() == 2 && self.path_stack[0] == "$types" {
            // We're exiting a type definition section
            let type_name = self.path_stack[1].clone();
            self.save_current_type_definition(type_name);
        } else if self.in_types_section && self.path_stack.len() >= 3 && self.path_stack[0] == "$types" {
            // We're exiting a field definition for an object type
            let type_name = &self.path_stack[1];
            let field_name = &self.path_stack[2];
            
            if let Some(field_schema) = self.current_field_schema.take() {
                // Add field to the object type
                if let Some(type_def) = self.document_schema.types.get_mut(type_name)
                    && let Type::Object(ref mut obj_schema) = type_def.type_expr {
                        obj_schema.fields.insert(field_name.clone(), field_schema);
                    }
            }
        }

        // Pop path
        for _ in 0..path_len {
            self.path_stack.pop();
        }

        if self.in_types_section && (self.path_stack.is_empty() || self.path_stack.first().map(|s| s.as_str()) != Some("$types")) {
            self.in_types_section = false;
        }

        if self.in_variants_section && self.path_stack.is_empty() {
            self.in_variants_section = false;
            self.current_variant = None;
        }

        result
    }

    fn visit_binding(
        &mut self,
        _handle: BindingHandle,
        view: BindingView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Extract key
        let mut key_path = Vec::new();
        if let Ok(keys_view) = view.keys.get_view(tree) {
            self.extract_keys_path(&keys_view, tree, &mut key_path);
        }

        if !key_path.is_empty() {
            // First visit value to extract it
            self.pending_value = None;
            if let Ok(binding_rhs_view) = view.binding_rhs.get_view(tree) {
                match binding_rhs_view {
                    BindingRhsView::ValueBinding(value_binding_handle) => {
                        if let Ok(value_binding_view) = value_binding_handle.get_view(tree) {
                            self.visit_value_for_extraction(value_binding_view.value, tree);
                        }
                    }
                    BindingRhsView::TextBinding(text_binding_handle) => {
                        if let Ok(text_binding_view) = text_binding_handle.get_view(tree)
                            && let Ok(text_view) = text_binding_view.text.get_view(tree)
                            && let Ok(data) = text_view.text.get_data(tree)
                            && let Some(text) = tree.get_str(data, self.input)
                        {
                            self.pending_value =
                                Some(ExtractedValue::String(text.trim().to_string()));
                        }
                    }
                    BindingRhsView::SectionBinding(_) => {
                        // Schema definitions don't use section bindings
                    }
                }
            }

            // Check if this is an inline schema extension (e.g., field.$type)
            if key_path.len() >= 2 && key_path.last().unwrap().starts_with('$') {
                // This is an inline schema like field.$type = .string
                let field_path: Vec<String> = key_path[..key_path.len() - 1].to_vec();
                let extension = key_path.last().unwrap();
                
                // Update path stack to point to the field
                let original_path = self.path_stack.clone();
                self.path_stack.extend(field_path.clone());
                
                // Get existing inline schema or create new one
                let full_path = self.current_path();
                let field_name = full_path.last().unwrap().clone();
                self.current_field_schema = self.document_schema.root.fields.get(&field_name).cloned()
                    .or_else(|| Some(FieldSchema::default()));
                
                // Handle the schema extension
                self.handle_schema_extension(extension);
                
                // Save the inline schema
                self.save_inline_schema();
                
                // Clear current field schema for next field
                self.current_field_schema = None;
                
                // Restore original path
                self.path_stack = original_path;
            } else if let Some(key) = key_path.first() {
                // Check if this is a $type binding in a regular section
                if key == "$type" && !self.in_types_section && !self.path_stack.is_empty() && !self.path_stack[0].starts_with('$') {
                    // This is an inline type declaration for a section
                    // e.g., @ person { $type = .$types.Person }
                    let section_path = self.current_path();
                    if let Some(section_name) = section_path.first() {
                        // Create or get field schema for this section
                        self.current_field_schema = self.document_schema.root.fields.get(section_name).cloned()
                            .or_else(|| Some(FieldSchema {
                                type_expr: Type::Any,
                                optional: true,  // Inline types are optional by default
                                constraints: Constraints::default(),
                                preferences: Preferences::default(),
                                serde: SerdeOptions::default(),
                                span: None,
                                default_value: None,
                                description: None,
                            }));
                        
                        // Handle the type declaration
                        self.handle_schema_extension(key);
                        
                        // Save as inline schema
                        if let Some(schema) = self.current_field_schema.take() {
                            self.document_schema.root.fields.insert(section_name.clone(), schema);
                        }
                    }
                } else if key.starts_with('$') {
                    // Regular schema extension
                    // For compound keys like $prefer.section, reconstruct the full key
                    let full_key = key_path.join(".");
                    self.handle_schema_extension(&full_key);
                } else if !self.in_types_section && !self.in_variants_section {
                    // Regular content
                    self.has_non_schema_content = true;
                }
            }
        }

        Ok(())
    }

    fn visit_terminal(
        &mut self,
        _id: CstNodeId,
        kind: TerminalKind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        // Extract terminal values for pending_value
        if self.pending_value.is_none() {
            match kind {
                TerminalKind::Str => {
                    if let Some(s) = tree.get_str(data, self.input) {
                        // Parse the string literal (remove quotes and unescape)
                        let unquoted = s.trim_matches('"');
                        // Basic unescaping for common escape sequences
                        let unescaped = unquoted
                            .replace("\\\\", "\\")  // \\ -> \
                            .replace("\\\"", "\"")  // \" -> "
                            .replace("\\n", "\n")    // \n -> newline
                            .replace("\\r", "\r")    // \r -> carriage return
                            .replace("\\t", "\t");   // \t -> tab
                        self.pending_value = Some(ExtractedValue::String(unescaped));
                    }
                }
                TerminalKind::Integer => {
                    if let Some(s) = tree.get_str(data, self.input)
                        && let Ok(n) = s.parse::<f64>()
                    {
                        self.pending_value = Some(ExtractedValue::Number(n));
                    }
                }
                TerminalKind::True => {
                    self.pending_value = Some(ExtractedValue::Boolean(true));
                }
                TerminalKind::False => {
                    self.pending_value = Some(ExtractedValue::Boolean(false));
                }
                TerminalKind::Null => {
                    self.pending_value = Some(ExtractedValue::Null);
                }
                _ => {
                    // Other terminal types don't produce extractable values
                }
            }
        }

        Ok(())
    }
}

impl SchemaExtractor<'_> {
    fn extract_keys_path<F: CstFacade>(
        &mut self,
        keys_view: &KeysView,
        tree: &F,
        path: &mut Vec<String>,
    ) {
        // Extract the first key
        if let Ok(key_view) = keys_view.key.get_view(tree) {
            self.extract_key(key_view, tree, path);
        }

        // Extract remaining keys if any
        if let Ok(Some(mut current_view)) = keys_view.keys_list.get_view(tree) {
            // Iterate over keys list manually
            loop {
                if let Ok(key_view) = current_view.key.get_view(tree) {
                    self.extract_key(key_view, tree, path);
                }

                // Get next item in the list
                match current_view.keys_list.get_view(tree) {
                    Ok(Some(next_view)) => current_view = next_view,
                    _ => break,
                }
            }
        }
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

    fn visit_value_for_extraction<F: CstFacade>(&mut self, value_handle: ValueHandle, tree: &F) {
        // Try to get the value view first
        if let Ok(view) = value_handle.get_view(tree) {
            match view {
                ValueView::Path(path_handle) => {
                    // Extract path value
                    if let Ok(path_view) = path_handle.get_view(tree)
                        && let Ok(keys_view) = path_view.keys.get_view(tree)
                    {
                        let mut path_segments = Vec::new();
                        self.extract_keys_path(&keys_view, tree, &mut path_segments);
                        
                        // Build path string with dots
                        let path_str = format!(".{}", path_segments.join("."));
                        self.pending_value = Some(ExtractedValue::Path(path_str));
                    }
                }
                ValueView::Array(array_handle) => {
                    // Extract array elements
                    let mut values = Vec::new();
                    
                    if let Ok(array_view) = array_handle.get_view(tree) {
                        // Check if array has elements
                        if let Ok(Some(array_opt_handle)) = array_view.array_opt.get_view(tree) {
                            // Process array elements
                            if let Ok(array_elements_view) = array_opt_handle.get_view(tree) {
                                self.extract_array_elements(array_elements_view, tree, &mut values);
                            }
                        }
                    }
                    
                    self.pending_value = Some(ExtractedValue::Array(values));
                }
                ValueView::Object(object_handle) => {
                    // Extract object fields
                    let mut fields = HashMap::new();
                    
                    if let Ok(object_view) = object_handle.get_view(tree) {
                        // Process object fields
                        self.extract_object_fields(object_view.object_list, tree, &mut fields);
                    }
                    
                    self.pending_value = Some(ExtractedValue::Object(fields));
                }
                _ => {
                    // Visit normally for other value types
                    let _ = self.visit_value_super(value_handle, view, tree);
                }
            }
        }
    }
    
    fn extract_object_fields<F: CstFacade>(
        &mut self,
        object_list_handle: ObjectListHandle,
        tree: &F,
        _fields: &mut HashMap<String, ExtractedValue>,
    ) {
        // Visit the object normally to trigger terminal extraction
        // This approach works because we're only extracting simple values for schema extensions
        // Complex object extraction would require building a HashMap from the fields,
        // but for schema purposes, we only need to extract individual field values
        // We need to get the node data for the object list
        if let Some(node_data) = tree.node_data(object_list_handle.node_id())
            && let eure_tree::tree::CstNodeData::NonTerminal { data, .. } = node_data {
                let _ = self.visit_non_terminal_super(
                    object_list_handle.node_id(),
                    eure_tree::node_kind::NonTerminalKind::ObjectList,
                    data,
                    tree
                );
            }
    }
    
    fn extract_array_elements<F: CstFacade>(
        &mut self,
        array_elements_view: ArrayElementsView,
        tree: &F,
        values: &mut Vec<ExtractedValue>,
    ) {
        // Extract first element
        self.pending_value = None;
        self.visit_value_for_extraction(array_elements_view.value, tree);
        if let Some(value) = self.pending_value.take() {
            values.push(value);
        }
        
        // Extract remaining elements
        if let Ok(Some(tail_opt)) = array_elements_view.array_elements_opt.get_view(tree)
            && let Ok(tail_view) = tail_opt.get_view(tree) {
                self.extract_array_elements_tail(tail_view, tree, values);
            }
    }
    
    fn extract_array_elements_tail<F: CstFacade>(
        &mut self,
        tail_view: ArrayElementsTailView,
        tree: &F,
        values: &mut Vec<ExtractedValue>,
    ) {
        // Check if there are more elements
        if let Ok(Some(elements_handle)) = tail_view.array_elements_tail_opt.get_view(tree) {
            // Extract the elements
            if let Ok(elements_view) = elements_handle.get_view(tree) {
                self.extract_array_elements(elements_view, tree, values);
            }
        }
    }
}
