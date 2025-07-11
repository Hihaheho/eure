//! Schema extraction from EureDocument
//!
//! This module provides functionality to extract schema information from
//! EURE documents using the new EureDocument structure.

use crate::schema::*;
use crate::utils::path_to_display_string;
use eure_tree::document::{DocumentKey, EureDocument, Node, NodeValue};
use eure_value::value::{KeyCmpValue, PathSegment, PathKey};
use eure_value::identifier::Identifier;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::str::FromStr;

/// Errors that can occur during schema extraction
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Invalid type path: {0}")]
    InvalidTypePath(String),

    #[error("Invalid field in schema: {0}")]
    InvalidField(String),

    #[error("Conflicting type definitions for: {0}")]
    ConflictingTypes(String),

    #[error("Invalid variant definition")]
    InvalidVariant,
}

/// Extract a schema from an EureDocument
pub fn document_to_schema(doc: &EureDocument) -> Result<DocumentSchema, SchemaError> {
    let mut schema = DocumentSchema::default();
    let mut builder = SchemaBuilder::new();

    // Process the root node
    builder.process_node(doc, doc.get_root(), &[])?;

    // Extract built schemas
    schema.types = builder.types;
    schema.root = ObjectSchema {
        fields: builder.root_fields,
        additional_properties: None,
    };
    schema.cascade_types = builder.cascade_types;

    // Check for special root-level extensions
    let root = doc.get_root();

    // Handle schema ref
    if let Some(schema_node_id) = root.extensions.get(&Identifier::from_str("schema").unwrap()) {
        let schema_node = doc.get_node(*schema_node_id);
        if let NodeValue::String { value: schema_ref, .. } = &schema_node.content {
            schema.schema_ref = Some(schema_ref.clone());
        }
    }

    // Handle root-level cascade type
    if let Some(cascade_node_id) = root.extensions.get(&Identifier::from_str("cascade-type").unwrap()) {
        let cascade_node = doc.get_node(*cascade_node_id);
        if let NodeValue::Path { value: path, .. } = &cascade_node.content {
            if let Some(cascade_type) = Type::from_path_segments(&path.0) {
                schema.cascade_types.insert(PathKey::from_segments(&[]), cascade_type);
            }
        }
    }

    // Handle global serde options
    if let Some(serde_node_id) = root.extensions.get(&Identifier::from_str("serde").unwrap()) {
        let serde_node = doc.get_node(*serde_node_id);
        if let NodeValue::Map { entries, .. } = &serde_node.content {
            schema.serde_options = SchemaBuilder::extract_serde_options_from_entries(doc, entries);
        }
    }

    Ok(schema)
}

/// Check if a Node represents a pure schema (no data content)
pub fn is_pure_schema_node(doc: &EureDocument, node: &Node) -> bool {
    match &node.content {
        NodeValue::Map { entries, .. } => {
            // Check all entries
            for (key, node_id) in entries {
                match key {
                    DocumentKey::MetaExtension(_) => {
                        // Meta-extension keys are OK for schemas
                        continue;
                    }
                    DocumentKey::Ident(_) | DocumentKey::Value(_) => {
                        let child_node = doc.get_node(*node_id);
                        if !is_schema_or_nested_schema_node(doc, child_node) {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            true
        }
        _ => {
            // Other content types with extensions might be schema definitions
            !node.extensions.is_empty()
        }
    }
}

fn is_schema_or_nested_schema_node(doc: &EureDocument, node: &Node) -> bool {
    // Check if this node has schema-related extensions
    if node.extensions.iter().any(|(ext, _)| {
        ext.as_ref() == "type" || ext.as_ref() == "optional" ||
        ext.as_ref() == "min" || ext.as_ref() == "max" ||
        ext.as_ref() == "pattern" || ext.as_ref() == "values"
    }) {
        return true;
    }

    match &node.content {
        NodeValue::Map { .. } => is_pure_schema_node(doc, node),
        _ => false,
    }
}

/// Helper to build schemas from EureDocument
struct SchemaBuilder {
    types: IndexMap<KeyCmpValue, FieldSchema>,
    root_fields: IndexMap<KeyCmpValue, FieldSchema>,
    cascade_types: HashMap<PathKey, Type>,
}

impl SchemaBuilder {
    fn new() -> Self {
        Self {
            types: IndexMap::new(),
            root_fields: IndexMap::new(),
            cascade_types: HashMap::new(),
        }
    }

    /// Process a node at a given path
    fn process_node(&mut self, doc: &EureDocument, node: &Node, path: &[&str]) -> Result<(), SchemaError> {
        // Check if we're in the types extension namespace
        if !path.is_empty() && path[0] == "types" {
            self.process_types_node(doc, node)?;
            return Ok(());
        }

        // Process extensions first
        for (ext_name, ext_node_id) in &node.extensions {
            let ext_node = doc.get_node(*ext_node_id);

            match ext_name.as_ref() {
                "types" => {
                    // Process type definitions
                    self.process_types_node(doc, ext_node)?;
                }
                "cascade-type" => {
                    // Handle cascade-type at any level
                    if let NodeValue::Path { value: type_path, .. } = &ext_node.content {
                        if let Some(cascade_type) = Type::from_path_segments(&type_path.0) {
                            let path_segments: Vec<PathSegment> = path.iter()
                                .map(|s| PathSegment::Ident(
                                    Identifier::from_str(s).unwrap_or_else(|_| Identifier::from_str("unknown").unwrap())
                                ))
                                .collect();
                            self.cascade_types.insert(PathKey::from_segments(&path_segments), cascade_type);
                        }
                    }
                }
                _ => {
                    // Other extensions might be field schemas
                    if path.is_empty() {
                        if let Some(schema) = self.extract_field_schema_from_node(doc, ext_name.as_ref(), ext_node)? {
                            // Store extension schemas directly by their identifier
                            self.root_fields.insert(
                                KeyCmpValue::MetaExtension(ext_name.clone()),
                                schema
                            );
                        }
                    }
                }
            }
        }

        // Process map content
        if let NodeValue::Map { entries, .. } = &node.content {
            for (key, node_id) in entries {
                let child_node = doc.get_node(*node_id);

                match key {
                    DocumentKey::Ident(ident) => {
                        if path.is_empty() {
                            // Check if this is the types namespace
                            if ident.as_ref() == "types" {
                                // Process as types namespace
                                self.process_types_node(doc, child_node)?;
                            } else {
                                // Regular field at root level
                                if let Some(field_schema) = self.extract_field_schema_from_node(doc, ident.as_ref(), child_node)? {
                                    self.root_fields.insert(KeyCmpValue::String(ident.to_string()), field_schema);
                                }
                            }
                        } else {
                            // For nested paths, recursively process
                            let mut new_path = path.to_vec();
                            new_path.push(ident.as_ref());
                            self.process_node(doc, child_node, &new_path)?;
                        }
                    }
                    DocumentKey::MetaExtension(meta_ext_ident) => {
                        // Meta-extension fields define schemas for extensions
                        if let Some(field_schema) = self.extract_field_schema_from_node(doc, meta_ext_ident.as_ref(), child_node)? {
                            // Store in root_fields as extension schema
                            self.root_fields.insert(
                                KeyCmpValue::MetaExtension(meta_ext_ident.clone()),
                                field_schema
                            );
                        }
                    }
                    _ => {} // Skip other key types
                }
            }
        }

        Ok(())
    }

    /// Process nodes in the types extension namespace
    fn process_types_node(&mut self, doc: &EureDocument, node: &Node) -> Result<(), SchemaError> {
        if let NodeValue::Map { entries, .. } = &node.content {
            for (key, node_id) in entries {
                if let DocumentKey::Ident(type_name) = key {
                    let type_node = doc.get_node(*node_id);
                    if let Some(type_schema) = self.extract_type_definition(doc, type_name.as_ref(), type_node)? {
                        self.types.insert(KeyCmpValue::String(type_name.to_string()), type_schema);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract type definition from a node
    fn extract_type_definition(&self, doc: &EureDocument, type_name: &str, node: &Node) -> Result<Option<FieldSchema>, SchemaError> {
        // A type definition can be:
        // 1. A direct type path: Point = .tuple
        // 2. An object with extensions: User = { $$type = .object, ... }
        // 3. A variant definition with $variants

        match &node.content {
            NodeValue::Path { value: path, .. } => {
                // Direct type reference
                if let Some(type_expr) = Type::from_path_segments(&path.0) {
                    Ok(Some(FieldSchema {
                        type_expr,
                        ..Default::default()
                    }))
                } else {
                    Err(SchemaError::InvalidTypePath(path_to_display_string(path)))
                }
            }
            NodeValue::Map { .. } => {
                // Check if this is a variant type
                if let Some(variants_node_id) = node.extensions.get(&Identifier::from_str("variants").unwrap()) {
                    let variants_node = doc.get_node(*variants_node_id);
                    let variants = self.extract_all_variants_from_node(doc, variants_node)?;

                    // Determine variant representation
                    let mut repr = VariantRepr::Tagged;
                    if let Some(repr_node_id) = node.extensions.get(&Identifier::from_str("variant-repr").unwrap()) {
                        let repr_node = doc.get_node(*repr_node_id);
                        if let NodeValue::String { value: repr_str, .. } = &repr_node.content {
                            repr = match repr_str.as_str() {
                                "external" | "tagged" => VariantRepr::Tagged,
                                "internal" => VariantRepr::InternallyTagged { tag: KeyCmpValue::String("type".to_string()) },
                                "adjacent" => VariantRepr::AdjacentlyTagged {
                                    tag: KeyCmpValue::String("type".to_string()),
                                    content: KeyCmpValue::String("content".to_string())
                                },
                                "untagged" => VariantRepr::Untagged,
                                _ => VariantRepr::Tagged,
                            };
                        }
                    }

                    return Ok(Some(FieldSchema {
                        type_expr: Type::Variants(VariantSchema {
                            variants,
                            representation: repr,
                        }),
                        ..Default::default()
                    }));
                }

                // Check if this is an object type definition with fields
                let mut fields = IndexMap::new();
                let mut has_fields = false;
                
                if let NodeValue::Map { entries, .. } = &node.content {
                    for (key, field_node_id) in entries {
                        if let DocumentKey::Ident(field_name) = key {
                            let field_node = doc.get_node(*field_node_id);
                            if let Some(field_schema) = self.extract_field_schema_from_node(doc, field_name.as_ref(), field_node)? {
                                fields.insert(KeyCmpValue::String(field_name.to_string()), field_schema);
                                has_fields = true;
                            }
                        }
                    }
                }
                
                if has_fields {
                    // This is an object type with fields
                    Ok(Some(FieldSchema {
                        type_expr: Type::Object(ObjectSchema {
                            fields,
                            additional_properties: None,
                        }),
                        ..Default::default()
                    }))
                } else {
                    // Otherwise, extract as regular field schema
                    self.extract_field_schema_from_node(doc, type_name, node)
                }
            }
            _ => Ok(None),
        }
    }

    /// Extract field schema from a node
    fn extract_field_schema_from_node(&self, doc: &EureDocument, _field_name: &str, node: &Node) -> Result<Option<FieldSchema>, SchemaError> {
        let mut schema = FieldSchema::default();
        let mut has_schema = false;

        // Check extensions for schema information
        for (ext_name, ext_node_id) in &node.extensions {
            let ext_node = doc.get_node(*ext_node_id);

            match ext_name.as_ref() {
                "type" => {
                    if let NodeValue::Path { value: path, .. } = &ext_node.content {
                        if let Some(type_expr) = Type::from_path_segments(&path.0) {
                            schema.type_expr = type_expr;
                            has_schema = true;
                        }
                    }
                }
                "optional" => {
                    if let NodeValue::Bool { value: b, .. } = &ext_node.content {
                        schema.optional = *b;
                        has_schema = true;
                    }
                }
                "min" => {
                    if let Some(min_value) = Self::extract_f64_from_node(&ext_node.content) {
                        if let Some((ref mut min, _)) = schema.constraints.range {
                            *min = Some(min_value);
                        } else {
                            schema.constraints.range = Some((Some(min_value), None));
                        }
                        has_schema = true;
                    }
                }
                "max" => {
                    if let Some(max_value) = Self::extract_f64_from_node(&ext_node.content) {
                        if let Some((_, ref mut max)) = schema.constraints.range {
                            *max = Some(max_value);
                        } else {
                            schema.constraints.range = Some((None, Some(max_value)));
                        }
                        has_schema = true;
                    }
                }
                "min-length" => {
                    if let Some(min_len) = Self::extract_usize_from_node(&ext_node.content) {
                        if let Some((ref mut min, _)) = schema.constraints.length {
                            *min = Some(min_len);
                        } else {
                            schema.constraints.length = Some((Some(min_len), None));
                        }
                        has_schema = true;
                    }
                }
                "max-length" => {
                    if let Some(max_len) = Self::extract_usize_from_node(&ext_node.content) {
                        if let Some((_, ref mut max)) = schema.constraints.length {
                            *max = Some(max_len);
                        } else {
                            schema.constraints.length = Some((None, Some(max_len)));
                        }
                        has_schema = true;
                    }
                }
                "pattern" => {
                    if let NodeValue::String { value: s, .. } = &ext_node.content {
                        schema.constraints.pattern = Some(s.clone());
                        has_schema = true;
                    }
                }
                "values" => {
                    // Note: values/enum constraint is not in the current Constraints struct
                    // This would need to be added if enum validation is required
                    has_schema = true;
                }
                "array" => {
                    // Handle array extension: field.$array = .type or field.$array = { inline object }
                    match &ext_node.content {
                        NodeValue::Path { value: path, .. } => {
                            // Array with type reference: field.$array = .$types.Item
                            if let Some(elem_type) = Type::from_path_segments(&path.0) {
                                schema.type_expr = Type::Array(Box::new(elem_type));
                                has_schema = true;
                            }
                        }
                        NodeValue::Map { .. } => {
                            // Array with inline object: field.$array = { id = .number, name = .string }
                            let mut fields = IndexMap::new();
                            if let NodeValue::Map { entries, .. } = &ext_node.content {
                                for (key, field_node_id) in entries {
                                    if let DocumentKey::Ident(field_name) = key {
                                        let field_node = doc.get_node(*field_node_id);
                                        if let Some(field_schema) = self.extract_field_schema_from_node(doc, field_name.as_ref(), field_node)? {
                                            fields.insert(KeyCmpValue::String(field_name.to_string()), field_schema);
                                        }
                                    }
                                }
                            }
                            schema.type_expr = Type::Array(Box::new(Type::Object(ObjectSchema {
                                fields,
                                additional_properties: None,
                            })));
                            has_schema = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Check if the value itself implies a schema
        match &node.content {
            NodeValue::Path { value: path, .. } => {
                // Field with direct type: field = .string
                if !has_schema {
                    if let Some(type_expr) = Type::from_path_segments(&path.0) {
                        schema.type_expr = type_expr;
                        has_schema = true;
                    }
                }
            }
            _ if has_schema => {
                // This is a field with both schema and default value
                if let Some(val) = Self::node_content_to_serde_value(&node.content) {
                    schema.default_value = Some(val);
                }
            }
            NodeValue::Map { .. } if !has_schema => {
                // Check if this is a map containing fields with schemas (i.e., an object schema)
                let mut fields = IndexMap::new();
                let mut has_schema_fields = false;
                
                if let NodeValue::Map { entries, .. } = &node.content {
                    for (key, field_node_id) in entries {
                        if let DocumentKey::Ident(field_name) = key {
                            let field_node = doc.get_node(*field_node_id);
                            if let Some(field_schema) = self.extract_field_schema_from_node(doc, field_name.as_ref(), field_node)? {
                                fields.insert(KeyCmpValue::String(field_name.to_string()), field_schema);
                                has_schema_fields = true;
                            }
                        }
                    }
                }
                
                if has_schema_fields {
                    // This is an object with schema fields
                    schema.type_expr = Type::Object(ObjectSchema {
                        fields,
                        additional_properties: None,
                    });
                    has_schema = true;
                }
            }
            _ => {}
        }

        if has_schema {
            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    /// Extract all variants from a variants node
    fn extract_all_variants_from_node(&self, doc: &EureDocument, variants_node: &Node) -> Result<IndexMap<KeyCmpValue, ObjectSchema>, SchemaError> {
        let mut variants = IndexMap::new();

        if let NodeValue::Map { entries, .. } = &variants_node.content {
            for (key, node_id) in entries {
                if let DocumentKey::Ident(variant_name) = key {
                    let variant_node = doc.get_node(*node_id);
                    let variant_schema = self.extract_variant_schema(doc, variant_name.as_ref(), variant_node)?;
                    variants.insert(KeyCmpValue::String(variant_name.to_string()), variant_schema);
                }
            }
        }

        Ok(variants)
    }

    /// Extract schema for a single variant
    fn extract_variant_schema(&self, doc: &EureDocument, _variant_name: &str, variant_node: &Node) -> Result<ObjectSchema, SchemaError> {
        let mut fields = IndexMap::new();

        if let NodeValue::Map { .. } = &variant_node.content {
            self.extract_variant_fields(doc, &mut fields, variant_node)?;
        }

        Ok(ObjectSchema {
            fields,
            additional_properties: None,
        })
    }

    /// Recursively extract variant fields
    fn extract_variant_fields(
        &self,
        doc: &EureDocument,
        fields: &mut IndexMap<KeyCmpValue, FieldSchema>,
        node: &Node,
    ) -> Result<(), SchemaError> {
        if let NodeValue::Map { entries, .. } = &node.content {
            for (key, node_id) in entries {
                if let DocumentKey::Ident(field_name) = key {
                    let field_node = doc.get_node(*node_id);

                    // Check if this field has an array extension
                    if let Some(array_node_id) = field_node.extensions.get(&Identifier::from_str("array").unwrap()) {
                        let array_node = doc.get_node(*array_node_id);
                        let array_field = self.extract_array_field(doc, array_node)?;
                        fields.insert(KeyCmpValue::String(field_name.to_string()), array_field);
                    } else if let Some(field_schema) = self.extract_field_schema_from_node(doc, field_name.as_ref(), field_node)? {
                        fields.insert(KeyCmpValue::String(field_name.to_string()), field_schema);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract array field schema
    fn extract_array_field(&self, doc: &EureDocument, array_node: &Node) -> Result<FieldSchema, SchemaError> {
        match &array_node.content {
            NodeValue::Path { value: path, .. } => {
                // Simple array type: field.$array = .string
                let elem_type = Type::from_path_segments(&path.0)
                    .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                Ok(FieldSchema {
                    type_expr: Type::Array(Box::new(elem_type)),
                    optional: false,
                    ..Default::default()
                })
            }
            NodeValue::Map { .. } => {
                // Complex array with object elements
                let mut elem_fields = IndexMap::new();
                self.extract_variant_fields(doc, &mut elem_fields, array_node)?;

                Ok(FieldSchema {
                    type_expr: Type::Array(Box::new(Type::Object(ObjectSchema {
                        fields: elem_fields,
                        additional_properties: None,
                    }))),
                    optional: false,
                    ..Default::default()
                })
            }
            _ => Err(SchemaError::InvalidField("Invalid array definition".to_string())),
        }
    }

    /// Helper to extract usize from NodeContent
    fn extract_usize_from_node(content: &NodeValue) -> Option<usize> {
        match content {
            NodeValue::I64 { value, .. } => Some(*value as usize),
            NodeValue::U64 { value, .. } => Some(*value as usize),
            _ => None,
        }
    }

    /// Helper to extract f64 from NodeContent
    fn extract_f64_from_node(content: &NodeValue) -> Option<f64> {
        match content {
            NodeValue::I64 { value, .. } => Some(*value as f64),
            NodeValue::U64 { value, .. } => Some(*value as f64),
            NodeValue::F32 { value, .. } => Some(*value as f64),
            NodeValue::F64 { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Convert NodeContent to serde_json::Value for default values
    fn node_content_to_serde_value(content: &NodeValue) -> Option<serde_json::Value> {
        match content {
            NodeValue::Null { .. } => Some(serde_json::Value::Null),
            NodeValue::Bool { value, .. } => Some(serde_json::Value::Bool(*value)),
            NodeValue::I64 { value, .. } => Some(serde_json::Value::Number((*value).into())),
            NodeValue::U64 { value, .. } => Some(serde_json::Value::Number((*value).into())),
            NodeValue::F32 { value, .. } => serde_json::Number::from_f64(*value as f64).map(serde_json::Value::Number),
            NodeValue::F64 { value, .. } => serde_json::Number::from_f64(*value).map(serde_json::Value::Number),
            NodeValue::String { value, .. } => Some(serde_json::Value::String(value.clone())),
            NodeValue::Code { value, .. } => Some(serde_json::Value::String(value.content.clone())),
            NodeValue::CodeBlock { value, .. } => Some(serde_json::Value::String(value.content.clone())),
            NodeValue::NamedCode { value, .. } => Some(serde_json::Value::String(value.content.clone())),
            NodeValue::Path { value, .. } => Some(serde_json::Value::String(path_to_display_string(value))),
            NodeValue::Hole { .. } => None,
            // For complex types, we'll return None as they require recursive conversion
            NodeValue::Array { .. } | NodeValue::Map { .. } | NodeValue::Tuple { .. } => None,
        }
    }

    /// Helper to extract serde options from node entries
    fn extract_serde_options_from_entries(doc: &EureDocument, entries: &[(DocumentKey, eure_tree::document::NodeId)]) -> SerdeOptions {
        let mut options = SerdeOptions::default();

        for (key, node_id) in entries {
            if let DocumentKey::Ident(ident) = key {
                let node = doc.get_node(*node_id);
                match ident.as_ref() {
                    "rename" => {
                        if let NodeValue::String { value: rename, .. } = &node.content {
                            options.rename = Some(rename.clone());
                        }
                    }
                    "rename-all" => {
                        if let NodeValue::String { value: rename_all, .. } = &node.content {
                            options.rename_all = RenameRule::from_str(rename_all);
                        }
                    }
                    _ => {}
                }
            }
        }

        options
    }
}
