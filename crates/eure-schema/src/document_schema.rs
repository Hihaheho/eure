//! Schema extraction from EureDocument
//!
//! This module provides functionality to extract schema information from
//! EURE documents using the new EureDocument structure.

use crate::schema::*;
use crate::utils::path_to_display_string;
use eure_value::document::{DocumentKey, EureDocument, Node, NodeContent};
use eure_value::value::{Value, KeyCmpValue, Map, PathSegment, PathKey};
use eure_value::identifier::Identifier;
use indexmap::IndexMap;
use ahash::AHashMap;
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
pub fn document_to_schema(doc: &EureDocument<Value>) -> Result<DocumentSchema, SchemaError> {
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
        if let NodeContent::Value(Value::String(schema_ref)) = &schema_node.content {
            schema.schema_ref = Some(schema_ref.clone());
        }
    }
    
    // Handle root-level cascade type
    if let Some(cascade_node_id) = root.extensions.get(&Identifier::from_str("cascade-type").unwrap()) {
        let cascade_node = doc.get_node(*cascade_node_id);
        if let NodeContent::Value(Value::Path(path)) = &cascade_node.content {
            if let Some(cascade_type) = Type::from_path_segments(&path.0) {
                schema.cascade_types.insert(PathKey::from_segments(&[]), cascade_type);
            }
        }
    }
    
    // Handle global serde options
    if let Some(serde_node_id) = root.extensions.get(&Identifier::from_str("serde").unwrap()) {
        let serde_node = doc.get_node(*serde_node_id);
        if let NodeContent::Map(entries) = &serde_node.content {
            schema.serde_options = SchemaBuilder::extract_serde_options_from_entries(doc, entries);
        }
    }
    
    Ok(schema)
}

/// Check if a Node represents a pure schema (no data content)
pub fn is_pure_schema_node(doc: &EureDocument<Value>, node: &Node<Value>) -> bool {
    match &node.content {
        NodeContent::Value(_) => {
            // Values with extensions might be schema definitions
            !node.extensions.is_empty()
        }
        NodeContent::Map(entries) => {
            // Check all entries
            for (key, node_id) in entries {
                match key {
                    DocumentKey::Extension(_) => {
                        // Extension keys are OK for schemas
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
        NodeContent::Array(_) => false,
    }
}

fn is_schema_or_nested_schema_node(doc: &EureDocument<Value>, node: &Node<Value>) -> bool {
    // Check if this node has schema-related extensions
    if node.extensions.iter().any(|(ext, _)| {
        ext.as_ref() == "type" || ext.as_ref() == "optional" || 
        ext.as_ref() == "min" || ext.as_ref() == "max" || 
        ext.as_ref() == "pattern" || ext.as_ref() == "values"
    }) {
        return true;
    }
    
    match &node.content {
        NodeContent::Map(_) => is_pure_schema_node(doc, node),
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
    fn process_node(&mut self, doc: &EureDocument<Value>, node: &Node<Value>, path: &[&str]) -> Result<(), SchemaError> {
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
                    if let NodeContent::Value(Value::Path(type_path)) = &ext_node.content {
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
                        if let Some(schema) = self.extract_field_schema_from_node(doc, ext_name, ext_node)? {
                            // Extension schemas are stored with $ prefix
                            self.root_fields.insert(
                                KeyCmpValue::String(format!("${}", ext_name)),
                                schema
                            );
                        }
                    }
                }
            }
        }
        
        // Process map content
        if let NodeContent::Map(entries) = &node.content {
            for (key, node_id) in entries {
                let child_node = doc.get_node(*node_id);
                
                match key {
                    DocumentKey::Ident(ident) => {
                        let key_str = ident.to_string();
                        
                        // Check for meta-extension fields ($$fieldname)
                        if key_str.starts_with("$$") {
                            let field_name = &key_str[2..];
                            if let Some(mut schema) = self.extract_field_schema_from_node(doc, field_name, child_node)?
                                && path.is_empty() {
                                // Meta-extension schemas define schemas for extension fields
                                schema.optional = true;
                                self.root_fields.insert(
                                    KeyCmpValue::String(format!("${}", field_name)),
                                    schema
                                );
                            }
                        } else if path.is_empty() {
                            // Regular field at root level
                            if let Some(field_schema) = self.extract_field_schema_from_node(doc, &key_str, child_node)? {
                                self.root_fields.insert(KeyCmpValue::String(key_str), field_schema);
                            }
                        }
                    }
                    DocumentKey::Extension(ext_ident) => {
                        // Extension fields are handled above
                        continue;
                    }
                    _ => {} // Skip other key types
                }
            }
        }
        
        Ok(())
    }
    
    /// Process nodes in the types extension namespace
    fn process_types_node(&mut self, doc: &EureDocument<Value>, node: &Node<Value>) -> Result<(), SchemaError> {
        if let NodeContent::Map(entries) = &node.content {
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
    fn extract_type_definition(&self, doc: &EureDocument<Value>, type_name: &str, node: &Node<Value>) -> Result<Option<FieldSchema>, SchemaError> {
        // A type definition can be:
        // 1. A direct type path: Point = .tuple
        // 2. An object with extensions: User = { $$type = .object, ... }
        // 3. A variant definition with $variants
        
        match &node.content {
            NodeContent::Value(Value::Path(path)) => {
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
            NodeContent::Map(_) => {
                // Check if this is a variant type
                if let Some(variants_node_id) = node.extensions.get(&Identifier::from_str("variants").unwrap()) {
                    let variants_node = doc.get_node(*variants_node_id);
                    let variants = self.extract_all_variants_from_node(doc, variants_node)?;
                    
                    // Determine variant representation
                    let mut repr = VariantRepresentation::External;
                    if let Some(repr_node_id) = node.extensions.get(&Identifier::from_str("variant.repr").unwrap()) {
                        let repr_node = doc.get_node(*repr_node_id);
                        if let NodeContent::Value(Value::String(repr_str)) = &repr_node.content {
                            repr = match repr_str.as_str() {
                                "external" => VariantRepresentation::External,
                                "internal" => VariantRepresentation::Internal { tag: "type".to_string() },
                                "adjacent" => VariantRepresentation::Adjacent { 
                                    tag: "type".to_string(), 
                                    content: "content".to_string() 
                                },
                                _ => VariantRepresentation::External,
                            };
                        }
                    }
                    
                    return Ok(Some(FieldSchema {
                        type_expr: Type::Variant(EnumSchema {
                            variants,
                            representation: repr,
                        }),
                        ..Default::default()
                    }));
                }
                
                // Otherwise, extract as regular field schema
                self.extract_field_schema_from_node(doc, type_name, node)
            }
            _ => Ok(None),
        }
    }
    
    /// Extract field schema from a node
    fn extract_field_schema_from_node(&self, doc: &EureDocument<Value>, _field_name: &str, node: &Node<Value>) -> Result<Option<FieldSchema>, SchemaError> {
        let mut schema = FieldSchema::default();
        let mut has_schema = false;
        
        // Check extensions for schema information
        for (ext_name, ext_node_id) in &node.extensions {
            let ext_node = doc.get_node(*ext_node_id);
            
            match ext_name.as_ref() {
                "type" => {
                    if let NodeContent::Value(Value::Path(path)) = &ext_node.content {
                        if let Some(type_expr) = Type::from_path_segments(&path.0) {
                            schema.type_expr = type_expr;
                            has_schema = true;
                        }
                    }
                }
                "optional" => {
                    if let NodeContent::Value(Value::Bool(b)) = &ext_node.content {
                        schema.optional = *b;
                        has_schema = true;
                    }
                }
                "min" => {
                    if let NodeContent::Value(val) = &ext_node.content {
                        schema.min = Self::extract_f64(val);
                        has_schema = true;
                    }
                }
                "max" => {
                    if let NodeContent::Value(val) = &ext_node.content {
                        schema.max = Self::extract_f64(val);
                        has_schema = true;
                    }
                }
                "min-length" => {
                    if let NodeContent::Value(val) = &ext_node.content {
                        schema.min_length = Self::extract_usize(val);
                        has_schema = true;
                    }
                }
                "max-length" => {
                    if let NodeContent::Value(val) = &ext_node.content {
                        schema.max_length = Self::extract_usize(val);
                        has_schema = true;
                    }
                }
                "pattern" => {
                    if let NodeContent::Value(Value::String(s)) = &ext_node.content {
                        schema.pattern = Some(s.clone());
                        has_schema = true;
                    }
                }
                "values" => {
                    if let NodeContent::Array(values) = &ext_node.content {
                        let mut allowed_values = Vec::new();
                        for value_node_id in values {
                            let value_node = doc.get_node(*value_node_id);
                            if let NodeContent::Value(val) = &value_node.content {
                                allowed_values.push(val.clone());
                            }
                        }
                        schema.values = Some(allowed_values);
                        has_schema = true;
                    }
                }
                _ => {}
            }
        }
        
        // Check if the value itself implies a schema
        match &node.content {
            NodeContent::Value(Value::Path(path)) => {
                // Field with direct type: field = .string
                if !has_schema {
                    if let Some(type_expr) = Type::from_path_segments(&path.0) {
                        schema.type_expr = type_expr;
                        has_schema = true;
                    }
                }
            }
            NodeContent::Value(val) if has_schema => {
                // This is a field with both schema and default value
                schema.default = Some(val.clone());
            }
            NodeContent::Map(_) if has_schema => {
                // This is fine - object with schema extensions
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
    fn extract_all_variants_from_node(&self, doc: &EureDocument<Value>, variants_node: &Node<Value>) -> Result<IndexMap<KeyCmpValue, ObjectSchema>, SchemaError> {
        let mut variants = IndexMap::new();
        
        if let NodeContent::Map(entries) = &variants_node.content {
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
    fn extract_variant_schema(&self, doc: &EureDocument<Value>, _variant_name: &str, variant_node: &Node<Value>) -> Result<ObjectSchema, SchemaError> {
        let mut fields = IndexMap::new();
        
        if let NodeContent::Map(entries) = &variant_node.content {
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
        doc: &EureDocument<Value>,
        fields: &mut IndexMap<KeyCmpValue, FieldSchema>,
        node: &Node<Value>,
    ) -> Result<(), SchemaError> {
        if let NodeContent::Map(entries) = &node.content {
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
    fn extract_array_field(&self, doc: &EureDocument<Value>, array_node: &Node<Value>) -> Result<FieldSchema, SchemaError> {
        match &array_node.content {
            NodeContent::Value(Value::Path(path)) => {
                // Simple array type: field.$array = .string
                let elem_type = Type::from_path_segments(&path.0)
                    .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                Ok(FieldSchema {
                    type_expr: Type::Array(Box::new(elem_type)),
                    optional: false,
                    ..Default::default()
                })
            }
            NodeContent::Map(_) => {
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
    
    /// Helper to extract usize from numeric Value
    fn extract_usize(val: &Value) -> Option<usize> {
        match val {
            Value::I64(n) => Some(*n as usize),
            Value::U64(n) => Some(*n as usize),
            _ => None,
        }
    }
    
    /// Helper to extract f64 from numeric Value
    fn extract_f64(val: &Value) -> Option<f64> {
        match val {
            Value::I64(n) => Some(*n as f64),
            Value::U64(n) => Some(*n as f64),
            Value::F32(n) => Some(*n as f64),
            Value::F64(n) => Some(*n),
            _ => None,
        }
    }
    
    /// Helper to extract serde options from node entries
    fn extract_serde_options_from_entries(doc: &EureDocument<Value>, entries: &[(DocumentKey, eure_value::document::NodeId)]) -> SerdeOptions {
        let mut options = SerdeOptions::default();
        
        for (key, node_id) in entries {
            if let DocumentKey::Ident(ident) = key {
                let node = doc.get_node(*node_id);
                match ident.as_ref() {
                    "rename" => {
                        if let NodeContent::Value(Value::String(rename)) = &node.content {
                            options.rename = Some(rename.clone());
                        }
                    }
                    "rename-all" => {
                        if let NodeContent::Value(Value::String(rename_all)) = &node.content {
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