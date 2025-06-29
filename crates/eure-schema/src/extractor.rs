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
        // Convert meta-extension prefix back to double dollar for matching
        let normalized_key = if key.starts_with("$̄") {
            key.replacen("$̄", "$$", 1)
        } else {
            key.to_string()
        };
        
        match normalized_key.as_str() {
            "$schema" => {
                if self.path_stack.is_empty() {
                    // Root-level $schema
                    if let Some(value) = &self.pending_value
                        && let Some(schema_ref) = self.extract_string(value)
                    {
                        self.document_schema.schema_ref = Some(schema_ref.to_string());
                    }
                }
                // Note: Non-root $schema references would need additional handling
                // for partial schema application, which is a more complex feature
            }
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
            "$optional" | "$$optional" => {
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
            "$prefer.section" | "$$prefer.section" => {
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
            "$variant-repr" | "$$variant-repr" => {
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
            // Meta-extensions that are schema-related but don't require special handling
            "$$prefer" | "$$prefer.array" |
            "$$serde" | "$$serde.rename" | "$$serde.rename-all" |
            "$$array" | "$$map" | "$$variants" | "$$cascade-type" | 
            "$$json-schema" | "$$literal" | "$$key" | "$$value" => {
                // These are valid schema meta-extensions
                // They define the schema structure itself, not data
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
                
                // If we're inside a $types definition, this variant belongs to that type
                if self.in_types_section && self.path_stack.len() >= 2 && self.path_stack[0] == "$types" {
                    let type_name = &self.path_stack[1];
                    
                    
                    // Get the type from document_schema.types and ensure it's a variant type
                    if let Some(type_def) = self.document_schema.types.get_mut(type_name) {
                        if !matches!(type_def.type_expr, Type::Variants(_)) {
                            type_def.type_expr = Type::Variants(VariantSchema {
                                variants: HashMap::new(),
                                representation: VariantRepr::default(),
                            });
                        }
                        
                        // Add this variant to the schema
                        if let Type::Variants(ref mut variant_schema) = type_def.type_expr {
                            variant_schema.variants.insert(section_path[1].clone(), ObjectSchema::default());
                        }
                        
                        // Set current_field_schema to this type so field collection works
                        self.current_field_schema = Some(type_def.clone());
                    }
                }
            }
        } else if !section_path.is_empty() && !section_path[0].starts_with('$') && !section_path[0].starts_with("$̄") {
            // Don't immediately mark as non-schema content
            // We'll determine this based on the section's contents
            // If it only contains schema field definitions (bindings with type paths),
            // then it's still a pure schema file
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

        if self.in_variants_section && (self.path_stack.len() < 2 || self.path_stack[0] != "$types") {
            
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
                    BindingRhsView::SectionBinding(section_binding_handle) => {
                        // Handle $types.TypeName { ... } syntax
                        if key_path.len() == 2 && key_path[0] == "$types" {
                            // This is a type definition using section binding syntax
                            let type_name = key_path[1].clone();
                            
                            // We'll simulate entering a $types.TypeName section
                            // by directly processing the content
                            if let Ok(section_binding_view) = section_binding_handle.get_view(tree)
                                && let Ok(eure_view) = section_binding_view.eure.get_view(tree) {
                                // Visit this as if it's a @ $types.TypeName section
                                let saved_in_types = self.in_types_section;
                                let saved_in_variants = self.in_variants_section;
                                let saved_path = self.path_stack.clone();
                                
                                self.in_types_section = true;
                                self.in_variants_section = false;
                                self.path_stack = vec!["$types".to_string(), type_name.clone()];
                                
                                // Create a variant schema to collect variants
                                self.current_field_schema = Some(FieldSchema {
                                    type_expr: Type::Variants(VariantSchema {
                                        variants: HashMap::new(),
                                        representation: VariantRepr::default(),
                                    }),
                                    optional: false,
                                    constraints: Constraints::default(),
                                    preferences: Preferences::default(),
                                    serde: SerdeOptions::default(),
                                    span: None,
                                });
                                
                                // Save the current field schema in the types map immediately
                                // so that when we process variants, they can find and update it
                                self.document_schema.types.insert(type_name.clone(), self.current_field_schema.clone().unwrap());
                                self.current_field_schema = None;
                                
                                // Visit the eure content
                                let _ = self.visit_eure(section_binding_view.eure, eure_view, tree);
                                
                                // The type has already been saved to document_schema.types
                                
                                
                                // Restore state
                                self.in_types_section = saved_in_types;
                                self.in_variants_section = saved_in_variants;
                                self.path_stack = saved_path;
                            }
                        } else if !self.in_types_section && !self.in_variants_section {
                            // Regular content with section binding
                            self.has_non_schema_content = true;
                        }
                    }
                }
            }

            // Check if this is an inline schema extension (e.g., field.$type)
            if key_path.len() >= 2 && key_path.last().unwrap().starts_with('$') {
                // This is an inline schema like field.$type = .string or person.email.$optional = true
                let field_path: Vec<String> = key_path[..key_path.len() - 1].to_vec();
                let extension = key_path.last().unwrap();
                
                // Handle nested field schema extensions
                if !field_path.is_empty() {
                    // Check if we're inside a variant section
                    if field_path.len() == 1 && self.in_variants_section && self.current_variant.is_some() {
                        // We're inside a variant, handle specially
                        if let Some(variant_name) = self.current_variant.clone() {
                            let field_name = field_path[0].clone();
                            
                            // Get the type name from path stack
                            if self.in_types_section && self.path_stack.len() >= 2 && self.path_stack[0] == "$types" {
                                let type_name = self.path_stack[1].clone();
                                
                                // Get existing field schema from the variant
                                let existing_schema = self.document_schema.types
                                    .get(&type_name)
                                    .and_then(|type_def| {
                                        if let Type::Variants(ref variant_schema) = type_def.type_expr {
                                            variant_schema.variants.get(&variant_name)
                                                .and_then(|obj| obj.fields.get(&field_name).cloned())
                                        } else {
                                            None
                                        }
                                    });
                                
                                self.current_field_schema = existing_schema.or_else(|| Some(FieldSchema::default()));
                                
                                // Handle the schema extension
                                self.handle_schema_extension(extension);
                                
                                // Save back to the variant
                                if let Some(field_schema) = self.current_field_schema.take()
                                    && let Some(type_def) = self.document_schema.types.get_mut(&type_name)
                                    && let Type::Variants(ref mut variant_schema) = type_def.type_expr
                                    && let Some(obj) = variant_schema.variants.get_mut(&variant_name) {
                                    obj.fields.insert(field_name, field_schema);
                                }
                            }
                        }
                    } else if field_path.len() == 1 && !self.path_stack.is_empty() && !self.path_stack[0].starts_with('$') {
                        // We're inside a regular section, handle as nested field
                        let section_name = self.path_stack[0].clone();
                        let field_name = &field_path[0];
                        
                        // Get the existing field from the section object
                        let existing_schema = self.document_schema.root.fields
                            .get(&section_name)
                            .and_then(|section_schema| {
                                if let Type::Object(ref obj_schema) = section_schema.type_expr {
                                    obj_schema.fields.get(field_name).cloned()
                                } else {
                                    None
                                }
                            });
                        
                        self.current_field_schema = existing_schema.or_else(|| Some(FieldSchema::default()));
                        
                        // Handle the schema extension
                        self.handle_schema_extension(extension);
                        
                        // Save back to the section object
                        if let Some(field_schema) = self.current_field_schema.take() {
                            // Ensure the section exists and is an object type
                            let section_schema = self.document_schema.root.fields
                                .entry(section_name.clone())
                                .or_insert_with(|| FieldSchema {
                                    type_expr: Type::Object(ObjectSchema::default()),
                                    optional: false,
                                    constraints: Constraints::default(),
                                    preferences: Preferences::default(),
                                    serde: SerdeOptions::default(),
                                    span: None,
                                });
                            
                            // Ensure it's an object type
                            if !matches!(section_schema.type_expr, Type::Object(_)) {
                                section_schema.type_expr = Type::Object(ObjectSchema::default());
                            }
                            
                            // Insert the field
                            if let Type::Object(ref mut obj_schema) = section_schema.type_expr {
                                obj_schema.fields.insert(field_name.clone(), field_schema);
                            }
                        }
                    } else {
                        // Handle arbitrary depth: field.$ext, parent.field.$ext, parent.child.field.$ext, etc.
                        // Navigate to find the existing schema
                        let mut existing_schema = None;
                        
                        if field_path.len() == 1 {
                            // Root level field
                            existing_schema = self.document_schema.root.fields.get(&field_path[0]).cloned();
                        } else {
                            // Nested field - navigate through the object hierarchy
                            let root_field = &field_path[0];
                            if let Some(root_schema) = self.document_schema.root.fields.get(root_field) {
                                if let Type::Object(ref obj_schema) = root_schema.type_expr {
                                    let mut current_obj = obj_schema;
                                    let mut found = true;
                                    
                                    // Navigate through intermediate levels
                                    for i in 1..field_path.len() - 1 {
                                        if let Some(intermediate_schema) = current_obj.fields.get(&field_path[i]) {
                                            if let Type::Object(ref next_obj) = intermediate_schema.type_expr {
                                                current_obj = next_obj;
                                            } else {
                                                found = false;
                                                break;
                                            }
                                        } else {
                                            found = false;
                                            break;
                                        }
                                    }
                                    
                                    // Get the final field schema
                                    if found && field_path.len() > 1 {
                                        existing_schema = current_obj.fields.get(&field_path[field_path.len() - 1]).cloned();
                                    }
                                }
                            }
                        }
                        
                        // Set current field schema
                        self.current_field_schema = existing_schema.or_else(|| Some(FieldSchema::default()));
                        
                        // Handle the schema extension
                        self.handle_schema_extension(extension);
                        
                        // Save back to the appropriate location
                        if let Some(schema) = self.current_field_schema.take() {
                            if field_path.len() == 1 {
                                // Root level - save directly
                                self.document_schema.root.fields.insert(field_path[0].clone(), schema);
                            } else {
                                // Nested - navigate and create intermediate objects as needed
                                let root_field = &field_path[0];
                                let root_schema = self.document_schema.root.fields
                                    .entry(root_field.clone())
                                    .or_insert_with(|| FieldSchema {
                                        type_expr: Type::Object(ObjectSchema::default()),
                                        optional: false,
                                        constraints: Constraints::default(),
                                        preferences: Preferences::default(),
                                        serde: SerdeOptions::default(),
                                        span: None,
                                    });
                                
                                // Ensure root is an object
                                if !matches!(root_schema.type_expr, Type::Object(_)) {
                                    root_schema.type_expr = Type::Object(ObjectSchema::default());
                                }
                                
                                // Navigate to the target location
                                if let Type::Object(obj_schema) = &mut root_schema.type_expr {
                                    // Navigate through the path, creating objects as needed
                                    let path_to_traverse = &field_path[1..field_path.len() - 1];
                                    let final_field = field_path[field_path.len() - 1].clone();
                                    
                                    // Use a recursive helper to navigate and create the path
                                    fn ensure_path_and_insert(
                                        current_obj: &mut ObjectSchema,
                                        remaining_path: &[String],
                                        final_field: String,
                                        schema: FieldSchema,
                                    ) {
                                        if remaining_path.is_empty() {
                                            // We've reached the target location
                                            current_obj.fields.insert(final_field, schema);
                                        } else {
                                            // Still have intermediate levels to process
                                            let next_field = &remaining_path[0];
                                            let rest = &remaining_path[1..];
                                            
                                            // Ensure the intermediate field exists and is an object
                                            let intermediate_schema = current_obj.fields
                                                .entry(next_field.clone())
                                                .or_insert_with(|| FieldSchema {
                                                    type_expr: Type::Object(ObjectSchema::default()),
                                                    optional: false,
                                                    constraints: Constraints::default(),
                                                    preferences: Preferences::default(),
                                                    serde: SerdeOptions::default(),
                                                    span: None,
                                                });
                                            
                                            // Ensure it's an object type
                                            if !matches!(intermediate_schema.type_expr, Type::Object(_)) {
                                                intermediate_schema.type_expr = Type::Object(ObjectSchema::default());
                                            }
                                            
                                            // Recurse into the next level
                                            if let Type::Object(ref mut next_obj) = intermediate_schema.type_expr {
                                                ensure_path_and_insert(next_obj, rest, final_field.clone(), schema);
                                            }
                                        }
                                    }
                                    
                                    ensure_path_and_insert(obj_schema, path_to_traverse, final_field, schema);
                                }
                            }
                        }
                    }
                }
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
                            }));
                        
                        // Handle the type declaration
                        self.handle_schema_extension(key);
                        
                        // Save as inline schema
                        if let Some(schema) = self.current_field_schema.take() {
                            self.document_schema.root.fields.insert(section_name.clone(), schema);
                        }
                    }
                } else if key.starts_with('$') || key.starts_with("$̄") {
                    // Regular schema extension or meta-extension
                    // For compound keys like $prefer.section, reconstruct the full key
                    let full_key = key_path.join(".");
                    self.handle_schema_extension(&full_key);
                } else if self.in_variants_section && self.current_variant.is_some() && self.is_type_path_value() {
                    // We're inside a variant definition, collect field schemas
                    // e.g., inside @ $variants.set-text { speaker = .string }
                    if let Some(variant_name) = &self.current_variant {
                        let field_name = key_path[0].clone();
                        
                        
                        // Extract the type from the pending value
                        if let Some(ExtractedValue::Path(path)) = &self.pending_value {
                            if let Some(type_expr) = Type::from_path(path) {
                                // Find the type in document_schema.types
                                if self.in_types_section && self.path_stack.len() >= 2 && self.path_stack[0] == "$types" {
                                    let type_name = &self.path_stack[1];
                                    
                                    if let Some(type_def) = self.document_schema.types.get_mut(type_name) {
                                        if let Type::Variants(ref mut variant_schema) = type_def.type_expr {
                                            if let Some(object_schema) = variant_schema.variants.get_mut(variant_name) {
                                                let field = FieldSchema {
                                                    type_expr,
                                                    optional: false,
                                                    constraints: Constraints::default(),
                                                    preferences: Preferences::default(),
                                                    serde: SerdeOptions::default(),
                                                    span: None,
                                                };
                                                object_schema.fields.insert(field_name, field);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if !self.in_types_section && !self.in_variants_section {
                    // Check if this is a schema definition by looking at the value
                    // e.g., person.name = .string
                    if self.is_type_path_value() {
                        // This is a schema definition using shorthand syntax
                        self.handle_shorthand_schema_definition(&key_path);
                    } else {
                        // Regular content
                        self.has_non_schema_content = true;
                    }
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
    /// Check if the pending value is a type path (e.g., .string, .number, etc.)
    fn is_type_path_value(&self) -> bool {
        if let Some(ExtractedValue::Path(path)) = &self.pending_value {
            Type::from_path(path).is_some()
        } else {
            false
        }
    }

    /// Handle shorthand schema definitions like `person.name = .string`
    fn handle_shorthand_schema_definition(&mut self, key_path: &[String]) {
        if key_path.is_empty() {
            return;
        }

        // Extract the type from the pending value
        let type_expr = if let Some(ExtractedValue::Path(path)) = &self.pending_value {
            Type::from_path(path)
        } else {
            None
        };

        if let Some(type_expr) = type_expr {
            // Check if we're inside a section
            if !self.path_stack.is_empty() {
                // We're inside a section, add the field to that section's object
                let section_name = self.path_stack[0].clone();
                
                // Get or create the section's field schema
                let mut section_schema = self.document_schema.root.fields
                    .get(&section_name)
                    .cloned()
                    .unwrap_or_else(|| FieldSchema {
                        type_expr: Type::Object(ObjectSchema::default()),
                        optional: false,
                        constraints: Constraints::default(),
                        preferences: Preferences::default(),
                        serde: SerdeOptions::default(),
                        span: None,
                    });
                
                // Ensure it's an object type
                if !matches!(section_schema.type_expr, Type::Object(_)) {
                    section_schema.type_expr = Type::Object(ObjectSchema::default());
                }
                
                // Add the field to this object
                if let Type::Object(ref mut obj_schema) = section_schema.type_expr {
                    let field_name = key_path[0].clone();
                    let field_schema = FieldSchema {
                        type_expr,
                        optional: false,
                        constraints: Constraints::default(),
                        preferences: Preferences::default(),
                        serde: SerdeOptions::default(),
                        span: None,
                    };
                    obj_schema.fields.insert(field_name, field_schema);
                }
                
                // Save the section schema back
                self.document_schema.root.fields.insert(section_name, section_schema);
            } else if key_path.len() == 1 {
                // Simple field at root: field = .string
                let field_name = &key_path[0];
                let field_schema = FieldSchema {
                    type_expr,
                    optional: false,
                    constraints: Constraints::default(),
                    preferences: Preferences::default(),
                    serde: SerdeOptions::default(),
                    span: None,
                };
                self.document_schema.root.fields.insert(field_name.clone(), field_schema);
            } else {
                // Nested field: person.name = .string
                // We need to ensure the parent exists as an object type
                let root_field = &key_path[0];
                
                // Get or create the root field schema
                let mut root_schema = self.document_schema.root.fields
                    .get(root_field)
                    .cloned()
                    .unwrap_or_else(|| FieldSchema {
                        type_expr: Type::Object(ObjectSchema::default()),
                        optional: false,
                        constraints: Constraints::default(),
                        preferences: Preferences::default(),
                        serde: SerdeOptions::default(),
                        span: None,
                    });

                // Ensure it's an object type
                if !matches!(root_schema.type_expr, Type::Object(_)) {
                    root_schema.type_expr = Type::Object(ObjectSchema::default());
                }

                // Navigate to the nested field
                if let Type::Object(obj_schema) = &mut root_schema.type_expr {
                    // Use the same recursive approach to avoid borrow checker issues
                    let path_to_traverse = &key_path[1..key_path.len() - 1];
                    let final_field = if key_path.len() > 1 {
                        key_path[key_path.len() - 1].clone()
                    } else {
                        return; // No field to insert
                    };
                    
                    let field_schema = FieldSchema {
                        type_expr,
                        optional: false,
                        constraints: Constraints::default(),
                        preferences: Preferences::default(),
                        serde: SerdeOptions::default(),
                        span: None,
                    };
                    
                    // Use a recursive helper to navigate and create the path
                    fn ensure_path_and_insert_field(
                        current_obj: &mut ObjectSchema,
                        remaining_path: &[String],
                        final_field: String,
                        schema: FieldSchema,
                    ) {
                        if remaining_path.is_empty() {
                            // We've reached the target location
                            current_obj.fields.insert(final_field, schema);
                        } else {
                            // Still have intermediate levels to process
                            let next_field = &remaining_path[0];
                            let rest = &remaining_path[1..];
                            
                            // Ensure the intermediate field exists and is an object
                            let intermediate_schema = current_obj.fields
                                .entry(next_field.clone())
                                .or_insert_with(|| FieldSchema {
                                    type_expr: Type::Object(ObjectSchema::default()),
                                    optional: false,
                                    constraints: Constraints::default(),
                                    preferences: Preferences::default(),
                                    serde: SerdeOptions::default(),
                                    span: None,
                                });
                            
                            // Ensure it's an object type
                            if !matches!(intermediate_schema.type_expr, Type::Object(_)) {
                                intermediate_schema.type_expr = Type::Object(ObjectSchema::default());
                            }
                            
                            // Recurse into the next level
                            if let Type::Object(ref mut next_obj) = intermediate_schema.type_expr {
                                ensure_path_and_insert_field(next_obj, rest, final_field.clone(), schema);
                            }
                        }
                    }
                    
                    ensure_path_and_insert_field(obj_schema, path_to_traverse, final_field, field_schema);
                }

                // Save the updated root schema
                self.document_schema.root.fields.insert(root_field.clone(), root_schema);
            }
        }
    }

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
