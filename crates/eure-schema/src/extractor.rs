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
    _current_object_schema: Option<ObjectSchema>,

    // Extracted schemas
    document_schema: DocumentSchema,
    inline_schemas: HashMap<Vec<String>, FieldSchema>,

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
    _Array(Vec<ExtractedValue>),
    _Object(HashMap<String, ExtractedValue>),
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
            _current_object_schema: None,
            document_schema: DocumentSchema::default(),
            inline_schemas: HashMap::new(),
            has_non_schema_content: false,
            pending_value: None,
        }
    }

    /// Consume the extractor and return the extracted schema
    pub fn extract(self) -> ExtractedSchema {
        ExtractedSchema {
            document_schema: self.document_schema,
            is_pure_schema: !self.has_non_schema_content,
            inline_schemas: self.inline_schemas,
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
                if let Some(ExtractedValue::_Array(values)) = &self.pending_value {
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
                if let Some(ExtractedValue::_Array(values)) = &self.pending_value
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
                if let Some(ExtractedValue::_Array(values)) = &self.pending_value
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
                if self.in_variants_section
                    && self.current_field_schema.is_some()
                    && let Some(repr_str) = self
                        .pending_value
                        .as_ref()
                        .and_then(|v| self.extract_string(v))
                {
                    let repr = match repr_str.as_str() {
                        "tagged" => VariantRepr::Tagged,
                        "inline" => VariantRepr::Tagged, // Inline not yet supported
                        _ => VariantRepr::Tagged,        // Default to tagged
                    };

                    // Apply to current variant schema if we're building one
                    if let Some(schema) = &mut self.current_field_schema
                        && let Type::Variants(ref mut variant_schema) = schema.type_expr
                    {
                        variant_schema.representation = repr;
                    }
                }
            }
            _ => {}
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
        if let Some(schema) = self.current_field_schema.clone() {
            let path = self.current_path();
            if !path.is_empty() && !path[0].starts_with('$') {
                self.inline_schemas.insert(path, schema);
            }
        }
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
        self.path_stack.extend(section_path);

        // Visit children
        let result = self.visit_section_super(handle, view, tree);

        // Pop path
        for _ in 0..path_len {
            self.path_stack.pop();
        }

        // Handle exiting sections
        if self.in_types_section && self.path_stack.len() <= 1 {
            // Save last type definition
            if let Some(name) = self.path_stack.get(1).cloned() {
                self.save_current_type_definition(name);
            }

            if self.path_stack.is_empty() || self.path_stack[0] != "$types" {
                self.in_types_section = false;
            }
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

        if let Some(key) = key_path.first() {
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

            // Handle schema extensions
            if key.starts_with('$') {
                self.handle_schema_extension(key);
            } else if !self.in_types_section && !self.in_variants_section {
                // Regular content
                self.has_non_schema_content = true;
            }

            // If we have inline schema info, save it
            if self.current_field_schema.is_some() && !self.in_types_section {
                self.save_inline_schema();
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
                        self.pending_value = Some(ExtractedValue::String(s.to_string()));
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
                _ => {}
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
            }
        }
    }

    fn visit_value_for_extraction<F: CstFacade>(&mut self, value_handle: ValueHandle, tree: &F) {
        let node_id = value_handle.node_id();

        // Check if it's a path (starts with .)
        let children: Vec<_> = tree.children(node_id).collect();
        if let Some(first_child) = children.first()
            && let Some(node_data) = tree.node_data(*first_child)
            && matches!(
                node_data,
                CstNode::Terminal {
                    kind: TerminalKind::Dot,
                    ..
                }
            )
        {
            // This is a path
            let mut path_str = String::from(".");
            for (i, &child_id) in children.iter().enumerate().skip(1) {
                if let Some(CstNode::Terminal {
                    kind: TerminalKind::Ident,
                    data,
                }) = tree.node_data(child_id)
                    && let Some(ident) = tree.get_str(data, self.input)
                {
                    path_str.push_str(ident);

                    // Check if there's a dot after this
                    if i + 1 < children.len()
                        && let Some(CstNode::Terminal {
                            kind: TerminalKind::Dot,
                            ..
                        }) = tree.node_data(children[i + 1])
                    {
                        path_str.push('.');
                    }
                }
            }
            self.pending_value = Some(ExtractedValue::Path(path_str));
            return;
        }

        // Visit normally for other value types
        if let Ok(view) = value_handle.get_view(tree) {
            let _ = self.visit_value_super(value_handle, view, tree);
        }
    }
}
