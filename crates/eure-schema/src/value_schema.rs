//! Schema extraction from EURE Values
//! 
//! This module provides functionality to extract schema information from
//! EURE documents that have been parsed into Value representations.

use crate::schema::*;
use crate::utils::path_to_display_string;
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
        schema.cascade_type = Type::from_path(&path_to_display_string(path));
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
    types: IndexMap<KeyCmpValue, FieldSchema>,
    root_fields: IndexMap<KeyCmpValue, FieldSchema>,
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
        
        // First pass: collect all schema definitions from extension keys
        let mut field_schemas: AHashMap<String, FieldSchema> = AHashMap::new();
        
        // Look for field.$extension patterns that define schemas
        for (key, _value) in map {
            if let KeyCmpValue::String(field_name) = key {
                // Check if there are any extension keys for this field
                let type_key = KeyCmpValue::String(format!("{}.{}", field_name, "$type"));
                let array_key = KeyCmpValue::String(format!("{}.{}", field_name, "$array"));
                let optional_key = KeyCmpValue::String(format!("{}.{}", field_name, "$optional"));
                let length_key = KeyCmpValue::String(format!("{}.{}", field_name, "$length"));
                let range_key = KeyCmpValue::String(format!("{}.{}", field_name, "$range"));
                let pattern_key = KeyCmpValue::String(format!("{}.{}", field_name, "$pattern"));
                let unique_key = KeyCmpValue::String(format!("{}.{}", field_name, "$unique"));
                let min_items_key = KeyCmpValue::String(format!("{}.{}", field_name, "$min-items"));
                let max_items_key = KeyCmpValue::String(format!("{}.{}", field_name, "$max-items"));
                
                let mut field_schema = FieldSchema::default();
                let mut has_schema = false;
                
                // Check for $type
                if let Some(Value::Path(path)) = map.get(&type_key) {
                    let path_str = path_to_display_string(path);
                    if let Some(type_expr) = Type::from_path(&path_str) {
                        field_schema.type_expr = type_expr;
                        has_schema = true;
                    }
                }
                
                // Check for $array
                if let Some(Value::Path(path)) = map.get(&array_key) {
                    let path_str = path_to_display_string(path);
                    if let Some(elem_type) = Type::from_path(&path_str) {
                        field_schema.type_expr = Type::Array(Box::new(elem_type));
                        has_schema = true;
                    }
                }
                
                // Check for $optional
                if let Some(Value::Bool(b)) = map.get(&optional_key) {
                    field_schema.optional = *b;
                    has_schema = true;
                }
                
                // Check for constraints
                if let Some(val) = map.get(&length_key) {
                    match val {
                        Value::I64(n) => {
                            field_schema.constraints.length = Some((Some(*n as usize), Some(*n as usize)));
                            has_schema = true;
                        }
                        Value::Array(arr) if arr.0.len() == 2 => {
                            let min = Self::extract_usize(&arr.0[0]);
                            let max = Self::extract_usize(&arr.0[1]);
                            field_schema.constraints.length = Some((min, max));
                            has_schema = true;
                        }
                        _ => {}
                    }
                }
                
                if let Some(val) = map.get(&range_key) {
                    if let Value::Array(arr) = val {
                        if arr.0.len() == 2 {
                            let min = Self::extract_f64(&arr.0[0]);
                            let max = Self::extract_f64(&arr.0[1]);
                            field_schema.constraints.range = Some((min, max));
                            has_schema = true;
                        }
                    }
                }
                
                if let Some(Value::String(pattern)) = map.get(&pattern_key) {
                    field_schema.constraints.pattern = Some(pattern.clone());
                    has_schema = true;
                }
                
                if let Some(Value::Bool(b)) = map.get(&unique_key) {
                    field_schema.constraints.unique = Some(*b);
                    has_schema = true;
                }
                
                if let Some(val) = map.get(&min_items_key) {
                    field_schema.constraints.min_items = Self::extract_usize(val);
                    has_schema = true;
                }
                
                if let Some(val) = map.get(&max_items_key) {
                    field_schema.constraints.max_items = Self::extract_usize(val);
                    has_schema = true;
                }
                
                if has_schema {
                    field_schemas.insert(field_name.clone(), field_schema);
                }
            }
        }
        
        // Second pass: process other entries
        for (key, value) in map {
            match key {
                KeyCmpValue::Extension(ext) if ext == "types" => {
                    // Process type definitions
                    if let Value::Map(types_map) = value {
                        self.process_map(&types_map.0, &["$types"])?;
                    }
                }
                KeyCmpValue::Extension(_) => {
                    // Skip other extensions at root level - they are handled in value_to_schema
                    continue;
                }
                KeyCmpValue::MetaExtension(meta_ext) => {
                    // Meta-extension defines schema for corresponding extension
                    if let Some(mut schema) = self.extract_field_schema(meta_ext, value)?
                        && path.is_empty() {
                            // Extension schemas are always optional
                            schema.optional = true;
                            // Store as Extension key (without $$)
                            self.root_fields.insert(
                                KeyCmpValue::Extension(meta_ext.clone()), 
                                schema
                            );
                    }
                }
                KeyCmpValue::String(key_str) => {
                    // Skip if this is a field.$extension pattern
                    if key_str.contains('.') && key_str.split('.').nth(1).map_or(false, |s| s.starts_with('$')) {
                        continue;
                    }
                    
                    // Check if we already have schema from extension keys
                    if let Some(existing_schema) = field_schemas.get(key_str) {
                        if path.is_empty() {
                            self.root_fields.insert(KeyCmpValue::String(key_str.clone()), existing_schema.clone());
                        }
                    } else {
                        // Regular field - check if it has schema definitions
                        if let Some(field_schema) = self.extract_field_schema(key_str, value)?
                            && path.is_empty() {
                                self.root_fields.insert(KeyCmpValue::String(key_str.clone()), field_schema);
                        }
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
                    self.types.insert(KeyCmpValue::String(type_name.clone()), type_schema);
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract variants from a Value Map
    fn extract_variants(&mut self, variants_map: &AHashMap<KeyCmpValue, Value>) -> Result<IndexMap<KeyCmpValue, ObjectSchema>, SchemaError> {
        let mut variants = IndexMap::new();
        
        for (variant_key, variant_value) in variants_map {
            let KeyCmpValue::String(variant_name) = variant_key else {
                continue;
            };
            
            if let Value::Map(variant_map) = variant_value {
                let mut variant_fields = IndexMap::new();
                
                for (field_key, field_value) in &variant_map.0 {
                    match field_key {
                        KeyCmpValue::String(field_name) => {
                            if let Some(field_schema) = self.extract_field_schema(field_name, field_value)? {
                                variant_fields.insert(KeyCmpValue::String(field_name.clone()), field_schema);
                            }
                        }
                        KeyCmpValue::Extension(ext_name) => {
                            if let Some(field_schema) = self.extract_field_schema(ext_name, field_value)? {
                                variant_fields.insert(KeyCmpValue::Extension(ext_name.clone()), field_schema);
                            }
                        }
                        KeyCmpValue::MetaExtension(meta_ext) => {
                            if let Some(schema) = self.extract_field_schema(meta_ext, field_value)? {
                                // Store as Extension key (without $$)
                                variant_fields.insert(KeyCmpValue::Extension(meta_ext.clone()), schema);
                            }
                        }
                        _ => continue,
                    }
                }
                
                variants.insert(KeyCmpValue::String(variant_name.clone()), ObjectSchema {
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
                let mut fields: IndexMap<KeyCmpValue, FieldSchema> = IndexMap::new();
                
                for (key, val) in &map.0 {
                    match key {
                        KeyCmpValue::Extension(ext_name) => match ext_name.as_str() {
                            "type" => {
                                if let Value::Path(path) = val {
                                    let path_str = path_to_display_string(path);
                                    schema.type_expr = Type::from_path(&path_str)
                                        .ok_or(SchemaError::InvalidTypePath(path_str))?;
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
                                            let path_str = path_to_display_string(path);
                                            if let Some(union_type) = Type::from_path(&path_str) {
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
                                    let path_str = path_to_display_string(path);
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
                                fields.insert(KeyCmpValue::String(key_str.clone()), field_schema);
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
                let mut child_fields: IndexMap<KeyCmpValue, FieldSchema> = IndexMap::new();
                
                for (key, val) in &map.0 {
                    match key {
                        KeyCmpValue::Extension(ext_name) => match ext_name.as_str() {
                            "type" => {
                                has_schema_info = true;
                                if let Value::Path(path) = val {
                                    let path_str = path_to_display_string(path);
                                    schema.type_expr = Type::from_path(&path_str)
                                        .ok_or(SchemaError::InvalidTypePath(path_str))?;
                                }
                            }
                            "array" => {
                            has_schema_info = true;
                            if let Value::Path(path) = val {
                                let path_str = path_to_display_string(path);
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
                                if let Value::Array(arr) = val
                                    && arr.0.len() == 2 {
                                        let min = Self::extract_usize(&arr.0[0]);
                                        let max = Self::extract_usize(&arr.0[1]);
                                        if min.is_some() || max.is_some() {
                                            schema.constraints.length = Some((min, max));
                                        }
                                    }
                            }
                            "range" => {
                                has_schema_info = true;
                                if let Value::Array(arr) = val
                                    && arr.0.len() == 2 {
                                        let min = Self::extract_f64(&arr.0[0]);
                                        let max = Self::extract_f64(&arr.0[1]);
                                        if min.is_some() || max.is_some() {
                                            schema.constraints.range = Some((min, max));
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
                                    child_fields.insert(KeyCmpValue::String(key_str.clone()), child_schema);
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
                    if matches!(schema.type_expr, Type::Object(_)) && !child_fields.is_empty()
                        && let Type::Object(ref mut obj_schema) = schema.type_expr {
                            obj_schema.fields = child_fields;
                        }
                    Ok(Some(schema))
                } else {
                    Ok(None)
                }
            }
            Value::Path(path) => {
                // Handle simple type assignments like `id = .string`
                let mut schema = FieldSchema::default();
                let path_str = path_to_display_string(path);
                schema.type_expr = Type::from_path(&path_str)
                    .ok_or(SchemaError::InvalidTypePath(path_str))?;
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