//! Schema extraction from EURE Values
//! 
//! This module provides functionality to extract schema information from
//! EURE documents that have been parsed into Value representations.

use crate::schema::*;
use crate::utils::path_to_string;
use eure_value::value::{Value, KeyCmpValue};
use indexmap::IndexMap;
use ahash::AHashMap;

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

/// Extract a schema from a Value representation of an EURE document
pub fn value_to_schema(value: &Value) -> Result<DocumentSchema, SchemaError> {
    let Value::Map(map) = value else {
        return Ok(DocumentSchema::default());
    };
    
    let mut schema = DocumentSchema::default();
    let mut builder = SchemaBuilder::new();
    
    // Process the document map
    builder.process_map(&map.0, &[])?;
    
    // Extract built schemas
    schema.types = builder.types;
    schema.root = ObjectSchema {
        fields: builder.root_fields,
        additional_properties: None,
    };
    
    // Check for special root-level keys
    if let Some(Value::String(schema_ref)) = map.0.get(&KeyCmpValue::Extension("schema".to_string())) {
        schema.schema_ref = Some(schema_ref.clone());
    }
    
    // Handle cascade type
    if let Some(Value::Path(path)) = map.0.get(&KeyCmpValue::Extension("cascade-type".to_string())) {
        schema.cascade_type = Type::from_path(&path_to_string(path));
    }
    
    // Handle global serde options
    if let Some(Value::Map(serde_map)) = map.0.get(&KeyCmpValue::Extension("serde".to_string())) {
        schema.serde_options = SchemaBuilder::extract_serde_options(&serde_map.0);
    }
    
    Ok(schema)
}

/// Check if a Value represents a pure schema (no data content)
pub fn is_pure_schema(value: &Value) -> bool {
    let Value::Map(map) = value else {
        return false;
    };
    
    // A pure schema only contains schema definitions, no actual data
    for (key, val) in &map.0 {
        match key {
            KeyCmpValue::Extension(_) | KeyCmpValue::MetaExtension(_) => {
                // Extension keys are OK for schemas
                continue;
            }
            KeyCmpValue::String(_) => {
                // Regular fields should contain schema definitions or be nested objects
                if !is_schema_or_nested_schema(val) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    
    true
}

/// Helper to build schemas from Values
struct SchemaBuilder {
    types: IndexMap<String, FieldSchema>,
    root_fields: IndexMap<String, FieldSchema>,
}

impl SchemaBuilder {
    fn new() -> Self {
        Self {
            types: IndexMap::new(),
            root_fields: IndexMap::new(),
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
    
    /// Helper to extract serde options from a map
    fn extract_serde_options(serde_map: &AHashMap<KeyCmpValue, Value>) -> SerdeOptions {
        let mut options = SerdeOptions::default();
        
        if let Some(Value::String(rename)) = serde_map.get(&KeyCmpValue::String("rename".to_string())) {
            options.rename = Some(rename.clone());
        }
        if let Some(Value::String(rename_all)) = serde_map.get(&KeyCmpValue::String("rename-all".to_string())) {
            options.rename_all = RenameRule::from_str(rename_all);
        }
        
        options
    }
    
    /// Process a map at a given path
    fn process_map(&mut self, map: &AHashMap<KeyCmpValue, Value>, path: &[&str]) -> Result<(), SchemaError> {
        // Check if we're in the $types namespace
        if !path.is_empty() && path[0] == "$types" {
            self.process_types_map(map, &path[1..])?;
            return Ok(());
        }
        
        // Process each entry in the map
        for (key, value) in map {
            match key {
                KeyCmpValue::Extension(ext) if ext == "types" => {
                    // Process type definitions
                    if let Value::Map(types_map) = value {
                        self.process_map(&types_map.0, &["$types"])?;
                    }
                }
                KeyCmpValue::Extension(_) | KeyCmpValue::MetaExtension(_) => {
                    // Skip other extension keys at root level for now
                    continue;
                }
                KeyCmpValue::String(key_str) => {
                    // Regular field - check if it has schema definitions
                    if let Some(field_schema) = self.extract_field_schema(key_str, value)?
                        && path.is_empty() {
                            self.root_fields.insert(key_str.clone(), field_schema);
                    }
                }
                _ => {} // Skip other key types
            }
        }
        
        Ok(())
    }
    
    /// Process entries in the $types namespace
    fn process_types_map(&mut self, map: &AHashMap<KeyCmpValue, Value>, path: &[&str]) -> Result<(), SchemaError> {
        if path.is_empty() {
            // Direct children of $types are type definitions
            for (key, value) in map {
                let KeyCmpValue::String(type_name) = key else {
                    continue;
                };
                
                if let Some(type_schema) = self.extract_type_definition(type_name, value)? {
                    self.types.insert(type_name.clone(), type_schema);
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract variants from a Value Map
    fn extract_variants(&mut self, variants_map: &AHashMap<KeyCmpValue, Value>) -> Result<IndexMap<String, ObjectSchema>, SchemaError> {
        let mut variants = IndexMap::new();
        
        for (variant_key, variant_value) in variants_map {
            let KeyCmpValue::String(variant_name) = variant_key else {
                continue;
            };
            
            if let Value::Map(variant_map) = variant_value {
                let mut variant_fields = IndexMap::new();
                
                for (field_key, field_value) in &variant_map.0 {
                    let KeyCmpValue::String(field_name) = field_key else {
                        continue;
                    };
                    
                    if let Some(field_schema) = self.extract_field_schema(field_name, field_value)? {
                        variant_fields.insert(field_name.clone(), field_schema);
                    }
                }
                
                variants.insert(variant_name.clone(), ObjectSchema {
                    fields: variant_fields,
                    additional_properties: None,
                });
            }
        }
        
        Ok(variants)
    }
    
    /// Extract a type definition from a Value
    fn extract_type_definition(&mut self, _type_name: &str, value: &Value) -> Result<Option<FieldSchema>, SchemaError> {
        match value {
            Value::Map(map) => {
                let mut schema = FieldSchema::default();
                let mut fields = IndexMap::new();
                
                for (key, val) in &map.0 {
                    match key {
                        KeyCmpValue::Extension(ext_name) => match ext_name.as_str() {
                            "type" => {
                                if let Value::Path(path) = val {
                                    schema.type_expr = Type::from_path(&path_to_string(path))
                                        .ok_or_else(|| SchemaError::InvalidTypePath(path_to_string(path)))?;
                                }
                            }
                            "variants" => {
                                if let Value::Map(variants_map) = val {
                                    let variants = self.extract_variants(&variants_map.0)?;
                                    schema.type_expr = Type::Variants(VariantSchema {
                                        variants,
                                        representation: VariantRepr::default(),
                                    });
                                }
                            }
                            "union" => {
                                if let Value::Array(arr) = val {
                                    let mut union_types = Vec::new();
                                    for elem in &arr.0 {
                                        if let Value::Path(path) = elem {
                                            if let Some(union_type) = Type::from_path(&path_to_string(path)) {
                                                union_types.push(union_type);
                                            }
                                        }
                                    }
                                    if !union_types.is_empty() {
                                        schema.type_expr = Type::Union(union_types);
                                    }
                                }
                            }
                            "array" => {
                                if let Value::Path(path) = val {
                                    let path_str = path_to_string(path);
                                    let elem_type = Type::from_path(&path_str)
                                        .ok_or_else(|| SchemaError::InvalidTypePath(path_str.clone()))?;
                                    schema.type_expr = Type::Array(Box::new(elem_type));
                                }
                            }
                            "optional" => {
                                if let Value::Bool(b) = val {
                                    schema.optional = *b;
                                }
                            }
                            "min-items" => {
                                schema.constraints.min_items = Self::extract_usize(val);
                            }
                            "max-items" => {
                                schema.constraints.max_items = Self::extract_usize(val);
                            }
                            "serde" => {
                                if let Value::Map(serde_map) = val {
                                    schema.serde = Self::extract_serde_options(&serde_map.0);
                                }
                            }
                            _ => {} // Skip other extensions for now
                        },
                        KeyCmpValue::String(key_str) => {
                            // Check if this is a nested map that might contain $variants
                            if let Value::Map(nested_map) = val {
                                // Look for $variants extension key in the nested map
                                if let Some(Value::Map(variants_map)) = nested_map.0.get(&KeyCmpValue::Extension("variants".to_string())) {
                                    // Found nested variants, use common extraction logic
                                    let variants = self.extract_variants(&variants_map.0)?;
                                    schema.type_expr = Type::Variants(VariantSchema {
                                        variants,
                                        representation: VariantRepr::default(),
                                    });
                                    continue; // Don't process this as a regular field
                                }
                            }
                            
                            // Regular field within type definition
                            if let Some(field_schema) = self.extract_field_schema(key_str, val)? {
                                fields.insert(key_str.clone(), field_schema);
                            }
                        }
                        _ => {} // Skip other key types
                    }
                }
                
                // Handle object types
                match &mut schema.type_expr {
                    Type::Object(obj_schema) => {
                        // If it's already an object type, add the fields to it
                        obj_schema.fields = fields;
                    }
                    Type::Any if !fields.is_empty() => {
                        // If we found fields but no explicit type, make it an object
                        schema.type_expr = Type::Object(ObjectSchema {
                            fields,
                            additional_properties: None,
                        });
                    }
                    _ => {
                        // For other types, if we have fields, that's an error
                        // but we'll ignore it for now
                    }
                }
                
                Ok(Some(schema))
            }
            _ => Ok(None),
        }
    }
    
    /// Extract schema information from a field value
    fn extract_field_schema(&self, _field_name: &str, value: &Value) -> Result<Option<FieldSchema>, SchemaError> {
        match value {
            Value::Map(map) => {
                let mut schema = FieldSchema::default();
                let mut has_schema_info = false;
                let mut child_fields = IndexMap::new();
                
                for (key, val) in &map.0 {
                    match key {
                        KeyCmpValue::Extension(ext_name) => match ext_name.as_str() {
                            "type" => {
                                has_schema_info = true;
                                if let Value::Path(path) = val {
                                    schema.type_expr = Type::from_path(&path_to_string(path))
                                        .ok_or_else(|| SchemaError::InvalidTypePath(path_to_string(path)))?;
                                }
                            }
                            "array" => {
                            has_schema_info = true;
                            if let Value::Path(path) = val {
                                let path_str = path_to_string(path);
                                let elem_type = Type::from_path(&path_str)
                                    .ok_or_else(|| SchemaError::InvalidTypePath(path_str.clone()))?;
                                schema.type_expr = Type::Array(Box::new(elem_type));
                            }
                            }
                            "optional" => {
                                has_schema_info = true;
                                if let Value::Bool(b) = val {
                                    schema.optional = *b;
                                }
                            }
                            "length" => {
                                has_schema_info = true;
                                if let Value::Array(arr) = val {
                                    if arr.0.len() == 2 {
                                        let min = Self::extract_usize(&arr.0[0]);
                                        let max = Self::extract_usize(&arr.0[1]);
                                        if min.is_some() || max.is_some() {
                                            schema.constraints.length = Some((min, max));
                                        }
                                    }
                                }
                            }
                            "range" => {
                                has_schema_info = true;
                                if let Value::Array(arr) = val {
                                    if arr.0.len() == 2 {
                                        let min = Self::extract_f64(&arr.0[0]);
                                        let max = Self::extract_f64(&arr.0[1]);
                                        if min.is_some() || max.is_some() {
                                            schema.constraints.range = Some((min, max));
                                        }
                                    }
                                }
                            }
                            "pattern" => {
                            has_schema_info = true;
                            if let Value::String(pattern) = val {
                                    schema.constraints.pattern = Some(pattern.clone());
                                }
                            }
                            "rename" => {
                            has_schema_info = true;
                            if let Value::String(name) = val {
                                    schema.serde.rename = Some(name.clone());
                                }
                            }
                            "serde" => {
                            // has_schema_info is not set here because $serde is metadata, not type info
                            if let Value::Map(serde_map) = val {
                                schema.serde = Self::extract_serde_options(&serde_map.0);
                            }
                        }
                            "prefer" => {
                                // Handle nested prefer options
                                if let Value::Map(prefer_map) = val {
                                    if let Some(Value::Bool(b)) = prefer_map.0.get(&KeyCmpValue::String("section".to_string())) {
                                        schema.preferences.section = Some(*b);
                                    }
                                    if let Some(Value::Bool(b)) = prefer_map.0.get(&KeyCmpValue::String("array".to_string())) {
                                        schema.preferences.array = Some(*b);
                                    }
                                }
                            }
                            "_value" => {
                                // Skip the special _value key - it's handled separately
                            }
                            _ => {} // Skip other extension keys
                        },
                        KeyCmpValue::String(key_str) => {
                            if key_str == "_value" {
                                // Skip the special _value key
                            } else {
                                // Regular field - check if it has schema definitions
                                if let Some(child_schema) = self.extract_field_schema(key_str, val)? {
                                    child_fields.insert(key_str.clone(), child_schema);
                                }
                            }
                        }
                        _ => {} // Skip other key types
                    }
                }
                
                // If we found child fields with schemas but no explicit type,
                // treat this as an implicit object
                if !has_schema_info && !child_fields.is_empty() {
                    schema.type_expr = Type::Object(ObjectSchema {
                        fields: child_fields,
                        additional_properties: None,
                    });
                    Ok(Some(schema))
                } else if has_schema_info {
                    // If type is already object and we have child fields, merge them
                    if matches!(schema.type_expr, Type::Object(_)) && !child_fields.is_empty() {
                        if let Type::Object(ref mut obj_schema) = schema.type_expr {
                            obj_schema.fields = child_fields;
                        }
                    }
                    Ok(Some(schema))
                } else {
                    Ok(None)
                }
            }
            Value::Path(path) => {
                // Handle simple type assignments like `id = .string`
                let mut schema = FieldSchema::default();
                schema.type_expr = Type::from_path(&path_to_string(path))
                    .ok_or_else(|| SchemaError::InvalidTypePath(path_to_string(path)))?;
                Ok(Some(schema))
            }
            _ => Ok(None),
        }
    }
}

/// Check if a value represents a schema definition
fn is_schema_definition(value: &Value) -> bool {
    match value {
        Value::Map(map) => {
            // Check if the map contains schema-related keys
            map.0.keys().any(|k| {
                matches!(k, KeyCmpValue::Extension(s) if 
                    s == "type" || s == "array" || s == "optional" || 
                    s == "variants" || s == "length" || s == "range")
            })
        }
        _ => false,
    }
}

/// Check if a value represents a schema definition or a nested schema object
fn is_schema_or_nested_schema(value: &Value) -> bool {
    match value {
        Value::Map(map) => {
            // If it has schema keys, check if it ALSO has data
            if is_schema_definition(value) {
                // Check if there are non-schema fields with non-schema values
                for (key, val) in &map.0 {
                    match key {
                        KeyCmpValue::Extension(_) | KeyCmpValue::MetaExtension(_) => {
                            // Extension keys are fine
                            continue;
                        }
                        KeyCmpValue::String(s) if s == "_value" => {
                            // _value means this contains data
                            return false;
                        }
                        KeyCmpValue::String(_) => {
                            // Regular field - check if its value is schema or data
                            if !is_schema_or_nested_schema(val) {
                                // Found a data field, so this is mixed content
                                return false;
                            }
                        }
                        _ => return false,
                    }
                }
                // All non-extension fields contain schemas
                return true;
            }
            
            // Otherwise, check if all its children are schema definitions
            // This handles cases like `@ script` where script is an object containing field schemas
            map.0.iter().all(|(key, val)| {
                match key {
                    KeyCmpValue::Extension(_) | KeyCmpValue::MetaExtension(_) => true,
                    KeyCmpValue::String(s) if s == "_value" => false, // _value means data
                    KeyCmpValue::String(_) => is_schema_or_nested_schema(val),
                    _ => false,
                }
            })
        }
        // Primitive values in a schema context (like `.string`) are schema references
        Value::Path(_) => true,
        // Boolean values can be schema metadata (e.g., for $optional)
        Value::Bool(_) => true,
        _ => false,
    }
}