//! Schema extraction from EURE Values
//! 
//! This module provides functionality to extract schema information from
//! EURE documents that have been parsed into Value representations.

use crate::schema::*;
use crate::utils::path_to_display_string;
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
    schema.cascade_types = builder.cascade_types;
    
    // Check for special root-level keys
    if let Some(Value::String(schema_ref)) = map.0.get(&KeyCmpValue::String("$schema".to_string())) {
        schema.schema_ref = Some(schema_ref.clone());
    }
    
    // Handle root-level cascade type
    if let Some(Value::Path(path)) = map.0.get(&KeyCmpValue::String("$cascade-type".to_string())) {
        if let Some(cascade_type) = Type::from_path_segments(&path.0) {
            schema.cascade_types.insert(PathKey::from_segments(&[]), cascade_type);
        }
    }
    
    // Handle global serde options
    if let Some(Value::Map(serde_map)) = map.0.get(&KeyCmpValue::String("$serde".to_string())) {
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
            KeyCmpValue::String(s) => {
                if s.starts_with('$') {
                    // Extension keys (starting with $) are OK for schemas
                    continue;
                } else {
                    // Regular fields should contain schema definitions or be nested objects
                    if !is_schema_or_nested_schema(val) {
                        return false;
                    }
                }
            }
            // This case is now handled above
            _ => {
                return false;
            }
        }
    }
    
    true
}

/// Helper to build schemas from Values
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
            self.process_types_map(map)?;
            return Ok(());
        }
        
        // Process each entry in the map
        for (key, value) in map {
            match key {
                KeyCmpValue::String(key_str) => {
                    if key_str == "$types" {
                        // Process type definitions
                        if let Value::Map(types_map) = value {
                            self.process_types_map(&types_map.0)?;
                        }
                    } else if key_str == "$cascade-type" {
                        // Handle cascade-type at any level
                        if let Value::Path(type_path) = value {
                            if let Some(cascade_type) = Type::from_path_segments(&type_path.0) {
                                // Convert string path to PathSegment path
                                let path_segments: Vec<PathSegment> = path.iter()
                                    .map(|s| PathSegment::Ident(
                                        eure_value::identifier::Identifier::from_str(s)
                                            .unwrap_or_else(|_| eure_value::identifier::Identifier::from_str("unknown").unwrap())
                                    ))
                                    .collect();
                                self.cascade_types.insert(PathKey::from_segments(&path_segments), cascade_type);
                            }
                        }
                    } else if key_str.starts_with("$$") {
                        // Meta-extension defines schema for corresponding extension
                        let meta_ext = &key_str[2..];
                        if let Some(mut schema) = self.extract_field_schema(meta_ext, value)?
                            && path.is_empty() {
                                // Extension schemas are always optional
                                schema.optional = true;
                                // Store as extension key (with $ prefix)
                                self.root_fields.insert(
                                    KeyCmpValue::String(format!("${}", meta_ext)), 
                                    schema
                                );
                        }
                    } else if key_str.starts_with('$') {
                        // Skip other extensions at root level - they are handled in value_to_schema
                        continue;
                    } else if path.is_empty() {
                        // Regular field at root level
                        if let Some(field_schema) = self.extract_field_schema(key_str, value)? {
                            self.root_fields.insert(KeyCmpValue::String(key_str.clone()), field_schema);
                        }
                    }
                }
                // String case is handled above
                _ => {} // Skip other key types
            }
        }
        
        Ok(())
    }
    
    /// Process entries in the $types namespace
    fn process_types_map(&mut self, map: &AHashMap<KeyCmpValue, Value>) -> Result<(), SchemaError> {
        // Direct children of $types are type definitions
        for (key, value) in map {
            let KeyCmpValue::String(type_name) = key else {
                continue;
            };
            
            if let Some(type_schema) = self.extract_type_definition(type_name, value)? {
                self.types.insert(KeyCmpValue::String(type_name.clone()), type_schema);
            }
        }
        
        Ok(())
    }
    
    /// Extract all variants from entries that start with $variants
    fn extract_all_variants(&self, all_entries: &AHashMap<KeyCmpValue, Value>) -> Result<IndexMap<KeyCmpValue, ObjectSchema>, SchemaError> {
        let mut variants = IndexMap::new();
        
        // The $variants map contains all variant-related definitions
        if let Some(Value::Map(variants_map)) = all_entries.get(&KeyCmpValue::String("$variants".to_string())) {
            // Process each variant in the variants map
            for (variant_key, variant_value) in &variants_map.0 {
                if let KeyCmpValue::String(variant_name) = variant_key {
                    // Process this variant's definition
                    let variant_schema = self.extract_variant_schema(variant_name, variant_value)?;
                    variants.insert(KeyCmpValue::String(variant_name.clone()), variant_schema);
                }
            }
        }
        
        Ok(variants)
    }
    
    /// Extract schema for a single variant, handling nested field definitions
    fn extract_variant_schema(&self, _variant_name: &str, variant_value: &Value) -> Result<ObjectSchema, SchemaError> {
        let mut fields = IndexMap::new();
        
        if let Value::Map(variant_map) = variant_value {
            // Process all fields in this variant
            self.extract_variant_fields(&mut fields, variant_map, &[])?;
        }
        
        Ok(ObjectSchema {
            fields,
            additional_properties: None,
        })
    }
    
    /// Recursively extract variant fields, handling nested structures and $array
    fn extract_variant_fields(
        &self,
        fields: &mut IndexMap<KeyCmpValue, FieldSchema>,
        map: &Map,
        path: &[String],
    ) -> Result<(), SchemaError> {
        for (key, value) in &map.0 {
            match key {
                KeyCmpValue::String(field_name) => {
                    // Regular field - could be a direct field or nested structure
                    if let Value::Map(nested_map) = value {
                        // Check if this map contains $array extension
                        if nested_map.0.contains_key(&KeyCmpValue::String("$array".to_string())) {
                            // This field is an array
                            if let Some(array_value) = nested_map.0.get(&KeyCmpValue::String("$array".to_string())) {
                                match array_value {
                                    Value::Path(path) => {
                                        // Simple array type: lines.$array = .string
                                        let elem_type = Type::from_path_segments(&path.0)
                                            .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                                        let array_field = FieldSchema {
                                            type_expr: Type::Array(Box::new(elem_type)),
                                            optional: false,
                                            ..Default::default()
                                        };
                                        fields.insert(KeyCmpValue::String(field_name.clone()), array_field);
                                    }
                                    Value::Map(array_elem_map) => {
                                        // Complex array with object elements: choice.$array = { text = .string, value = .string }
                                        let mut elem_fields = IndexMap::new();
                                        for (elem_key, elem_value) in &array_elem_map.0 {
                                            if let KeyCmpValue::String(elem_field_name) = elem_key
                                                && let Some(elem_field_schema) = self.extract_field_schema(elem_field_name, elem_value)? {
                                                    elem_fields.insert(KeyCmpValue::String(elem_field_name.clone()), elem_field_schema);
                                                }
                                        }
                                        
                                        let array_field = FieldSchema {
                                            type_expr: Type::Array(Box::new(Type::Object(ObjectSchema {
                                                fields: elem_fields,
                                                additional_properties: None,
                                            }))),
                                            optional: false,
                                            ..Default::default()
                                        };
                                        
                                        fields.insert(KeyCmpValue::String(field_name.clone()), array_field);
                                    }
                                    _ => {
                                        // Other value types for $array are not supported
                                    }
                                }
                            }
                        } else if let Some(field_schema) = self.extract_field_schema(field_name, value)? {
                            // This map contains schema definitions (like $type), treat it as a field definition
                            fields.insert(KeyCmpValue::String(field_name.clone()), field_schema);
                        } else {
                            // Regular nested object - recurse into it
                            let mut new_path = path.to_vec();
                            new_path.push(field_name.clone());
                            self.extract_variant_fields(fields, nested_map, &new_path)?;
                        }
                    } else {
                        // Direct field definition
                        if let Some(field_schema) = self.extract_field_schema(field_name, value)? {
                            fields.insert(KeyCmpValue::String(field_name.clone()), field_schema);
                        }
                    }
                }
                KeyCmpValue::Extension(ext_name) => {
                    // Handle extension fields in variants
                    if let Some(field_schema) = self.extract_field_schema(ext_name, value)? {
                        fields.insert(KeyCmpValue::Extension(ext_name.clone()), field_schema);
                    }
                }
                _ => continue,
            }
        }
        
        Ok(())
    }
    
    /// Extract variants from a Value Map
    fn extract_variants(&mut self, variants_map: &AHashMap<KeyCmpValue, Value>) -> Result<IndexMap<KeyCmpValue, ObjectSchema>, SchemaError> {
        let mut variants = IndexMap::new();
        
        // First pass: collect direct variant definitions
        for (variant_key, variant_value) in variants_map {
            let key_str = match variant_key {
                KeyCmpValue::String(s) => s.clone(),
                KeyCmpValue::String(s) if s == "$variants" => {
                    // Handle the case where we have a direct $variants map
                    if let Value::Map(direct_variants) = variant_value {
                        // Process all direct variants in this map
                        for (vk, vv) in &direct_variants.0 {
                            if let KeyCmpValue::String(variant_name) = vk
                                && let Value::Map(variant_map) = vv {
                                    let mut variant_fields = IndexMap::new();
                                    
                                    for (field_key, field_value) in &variant_map.0 {
                                        match field_key {
                                            KeyCmpValue::String(field_name) => {
                                                if let Some(field_schema) = self.extract_field_schema(field_name, field_value)? {
                                                    variant_fields.insert(KeyCmpValue::String(field_name.clone()), field_schema);
                                                }
                                            }
                                            KeyCmpValue::String(s) if s.starts_with('$') && !s.starts_with("$$") => {
                                                if let Some(field_schema) = self.extract_field_schema(&s[1..], field_value)? {
                                                    variant_fields.insert(KeyCmpValue::String(s.clone()), field_schema);
                                                }
                                            }
                                            KeyCmpValue::String(s) if s.starts_with("$$") => {
                                                if let Some(schema) = self.extract_field_schema(&s[2..], field_value)? {
                                                    // Store as extension key (with $ prefix)
                                                    variant_fields.insert(KeyCmpValue::String(format!("${}", &s[2..])), schema);
                                                }
                                            }
                                            _ => continue,
                                        }
                                    }
                                    
                                    eprintln!("DEBUG: First pass - creating variant '{}' with {} fields", variant_name, variant_fields.len());
                                    for (k, v) in &variant_fields {
                                        eprintln!("  - Field {:?}: {:?}", k, v.type_expr);
                                    }
                                    
                                    variants.insert(KeyCmpValue::String(variant_name.clone()), ObjectSchema {
                                        fields: variant_fields,
                                        additional_properties: None,
                                    });
                                }
                        }
                    }
                    continue;
                }
                _ => continue,
            };
            
            // For string keys that might have full paths like "section.variants.variant-name"
            // Extract just the variant name
            let variant_name = if let Some(pos) = key_str.rfind('.') {
                let prefix = &key_str[..pos];
                if prefix.ends_with("variants") {
                    &key_str[pos + 1..]
                } else {
                    // This is a nested field definition
                    continue;
                }
            } else {
                &key_str
            };
            
            // Check if this is a nested variant field definition
            if variant_name.contains('.') {
                // Skip in first pass, handle in second pass
                continue;
            }
            
            if let Value::Map(variant_map) = variant_value {
                let mut variant_fields = IndexMap::new();
                
                for (field_key, field_value) in &variant_map.0 {
                    match field_key {
                        KeyCmpValue::String(field_name) => {
                            if let Some(field_schema) = self.extract_field_schema(field_name, field_value)? {
                                variant_fields.insert(KeyCmpValue::String(field_name.clone()), field_schema);
                            }
                        }
                        KeyCmpValue::String(s) if s.starts_with('$') => {
                            // Skip extension fields - they are metadata, not data fields
                            continue;
                        }
                        _ => continue,
                    }
                }
                
                eprintln!("DEBUG: First pass - creating variant '{}' with {} fields", key_str, variant_fields.len());
                for (k, v) in &variant_fields {
                    eprintln!("  - Field {:?}: {:?}", k, v.type_expr);
                }
                
                variants.insert(KeyCmpValue::String(key_str.clone()), ObjectSchema {
                    fields: variant_fields,
                    additional_properties: None,
                });
            }
        }
        
        // Second pass: handle nested variant field definitions (process these first)
        let mut nested_fields: IndexMap<String, IndexMap<String, FieldSchema>> = IndexMap::new();
        
        for (variant_key, variant_value) in variants_map {
            let key_str = match variant_key {
                KeyCmpValue::String(s) => s.clone(),
                _ => continue,
            };
            
            // Parse the key to extract variant name and field path
            // Keys might be like "section.variants.set-choices.choice.$array"
            let parts: Vec<&str> = key_str.split('.').collect();
            
            // Find the "variants" part and extract what comes after
            let mut variant_idx = None;
            for (i, part) in parts.iter().enumerate() {
                if *part == "variants" && i + 1 < parts.len() {
                    variant_idx = Some(i);
                    break;
                }
            }
            
            if let Some(idx) = variant_idx
                && idx + 2 < parts.len() {
                    // We have at least "variants.variant-name.field"
                    let variant_name = parts[idx + 1];
                    let field_parts = &parts[idx + 2..];
                    let field_path = field_parts.join(".");
                
                eprintln!("DEBUG: Processing nested variant field: key={key_str}, variant={variant_name}, field_path={field_path}");
                
                // Collect nested fields for this variant
                if !nested_fields.contains_key(variant_name) {
                    nested_fields.insert(variant_name.to_string(), IndexMap::new());
                }
                
                // Handle the nested field definition
                if let Some(variant_fields) = nested_fields.get_mut(variant_name) {
                    // Parse the field path to check for modifiers like $array
                    if field_path.ends_with(".$array") {
                        // Extract the field name without the .$array suffix
                        let field_name = &field_path[..field_path.len() - 7]; // Remove ".$array"
                        
                        // The value should be a map defining the array element schema
                        if let Value::Map(elem_map) = variant_value {
                            // Extract the element schema from the map
                            let mut elem_fields = IndexMap::new();
                            for (elem_key, elem_value) in &elem_map.0 {
                                if let KeyCmpValue::String(elem_field_name) = elem_key
                                    && let Some(elem_field_schema) = self.extract_field_schema(elem_field_name, elem_value)? {
                                        elem_fields.insert(KeyCmpValue::String(elem_field_name.clone()), elem_field_schema);
                                    }
                            }
                            
                            // Create an array field with object elements
                            let array_field = FieldSchema {
                                type_expr: Type::Array(Box::new(Type::Object(ObjectSchema {
                                    fields: elem_fields,
                                    additional_properties: None,
                                }))),
                                optional: false,
                                ..Default::default()
                            };
                            
                            variant_fields.insert(field_name.to_string(), array_field);
                        }
                    } else if let Some(field_schema) = self.extract_field_schema(&field_path, variant_value)? {
                        // Regular nested field without modifiers
                        variant_fields.insert(field_path.to_string(), field_schema);
                    }
                }
            }
        }
        
        // Merge nested fields into the variants
        for (variant_name, fields) in nested_fields {
            eprintln!("DEBUG: Merging nested fields for variant '{variant_name}'");
            
            if let Some(variant_obj) = variants.get_mut(&KeyCmpValue::String(variant_name.clone())) {
                // Add nested fields to existing variant
                eprintln!("  - Found existing variant with {} fields", variant_obj.fields.len());
                for (field_name, field_schema) in fields {
                    eprintln!("  - Adding nested field '{}' with type {:?}", field_name, field_schema.type_expr);
                    variant_obj.fields.insert(KeyCmpValue::String(field_name), field_schema);
                }
                eprintln!("  - Variant now has {} fields", variant_obj.fields.len());
            } else {
                // Create new variant with only the nested fields
                eprintln!("  - Creating new variant");
                let mut variant_fields = IndexMap::new();
                for (field_name, field_schema) in fields {
                    variant_fields.insert(KeyCmpValue::String(field_name), field_schema);
                }
                variants.insert(KeyCmpValue::String(variant_name), ObjectSchema {
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
                
                // First check if this type has variants by collecting all variant-related entries
                let mut has_variants = false;
                let mut all_variant_entries = AHashMap::new();
                
                
                for (key, val) in &map.0 {
                    if let KeyCmpValue::String(s) = key
                        && s.starts_with("$variants") {
                            has_variants = true;
                            // Store with the extension name as key
                            all_variant_entries.insert(KeyCmpValue::String(s.clone()), val.clone());
                        }
                }
                
                
                if has_variants {
                    let variants = self.extract_all_variants(&all_variant_entries)?;
                    schema.type_expr = Type::Variants(VariantSchema {
                        variants,
                        representation: VariantRepr::default(),
                    });
                }
                
                for (key, val) in &map.0 {
                    match key {
                        KeyCmpValue::String(s) if s.starts_with('$') => match s.as_str() {
                            "$type" => {
                                if !has_variants {  // Only set type if not already set to Variants
                                    if let Value::Path(path) = val {
                                        schema.type_expr = Type::from_path_segments(&path.0)
                                            .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                                    }
                                }
                            }
                            ext if ext.starts_with("variants") => {
                                // Already handled above
                            }
                            "union" => {
                                if let Value::Array(arr) = val {
                                    let mut union_types = Vec::new();
                                    for elem in &arr.0 {
                                        if let Value::Path(path) = elem
                                            && let Some(union_type) = Type::from_path_segments(&path.0) {
                                                union_types.push(union_type);
                                            }
                                    }
                                    if !union_types.is_empty() {
                                        schema.type_expr = Type::Union(union_types);
                                    }
                                }
                            }
                            "array" => {
                                if let Value::Path(path) = val {
                                    let elem_type = Type::from_path_segments(&path.0)
                                        .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
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
            Value::Path(path) => {
                // Handle simple type assignments like `Point = .string`
                let mut schema = FieldSchema::default();
                schema.type_expr = Type::from_path_segments(&path.0)
                    .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                Ok(Some(schema))
            }
            Value::Tuple(tuple) => {
                // Handle tuple type definitions like `Point = (.number, .string, .number)`
                let mut element_types = Vec::new();
                for elem in &tuple.0 {
                    if let Value::Path(path) = elem {
                        let elem_type = Type::from_path_segments(&path.0)
                            .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                        element_types.push(elem_type);
                    } else {
                        return Err(SchemaError::InvalidField("Tuple type definition must contain only type paths".to_string()));
                    }
                }
                
                let mut schema = FieldSchema::default();
                schema.type_expr = Type::Tuple(element_types);
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
                                    schema.type_expr = Type::from_path_segments(&path.0)
                                        .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                                }
                            }
                            "array" => {
                            has_schema_info = true;
                            match val {
                                Value::Path(path) => {
                                    // Simple array type: $array = .string
                                    let elem_type = Type::from_path_segments(&path.0)
                                        .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                                    schema.type_expr = Type::Array(Box::new(elem_type));
                                }
                                Value::Map(elem_map) => {
                                    // Complex array with object elements: $array = { text = .string, value = .string }
                                    let mut elem_fields = IndexMap::new();
                                    for (elem_key, elem_value) in &elem_map.0 {
                                        if let KeyCmpValue::String(elem_field_name) = elem_key
                                            && let Some(elem_field_schema) = self.extract_field_schema(elem_field_name, elem_value)? {
                                                elem_fields.insert(KeyCmpValue::String(elem_field_name.clone()), elem_field_schema);
                                            }
                                    }
                                    schema.type_expr = Type::Array(Box::new(Type::Object(ObjectSchema {
                                        fields: elem_fields,
                                        additional_properties: None,
                                    })));
                                }
                                _ => {} // Other value types for $array are not supported
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
                schema.type_expr = Type::from_path_segments(&path.0)
                    .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                Ok(Some(schema))
            }
            Value::Tuple(tuple) => {
                // Handle tuple type definitions like (.number, .string, .number)
                // This represents a fixed-length tuple with specific types for each position
                let mut element_types = Vec::new();
                for elem in &tuple.0 {
                    if let Value::Path(path) = elem {
                        let elem_type = Type::from_path_segments(&path.0)
                            .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                        element_types.push(elem_type);
                    } else {
                        return Err(SchemaError::InvalidField("Tuple type definition must contain only type paths".to_string()));
                    }
                }
                
                // Create a tuple type schema
                let mut schema = FieldSchema::default();
                schema.type_expr = Type::Tuple(element_types.clone());
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
                    KeyCmpValue::String(s) if s == "_value" => {
                        false
                    }, // _value means data
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