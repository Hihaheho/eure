//! Conversion from EureDocument to SchemaDocument
//!
//! This module provides functionality to convert Eure documents containing schema definitions
//! into SchemaDocument structures.
//!
//! # Schema Syntax
//!
//! Schema types are defined using the following syntax:
//!
//! **Primitives (shorthands):**
//! - `.text`, `.integer`, `.float`, `.boolean`, `.null`, `.any`, `.path`
//! - `.text.rust`, `.text.email`, `.text.plaintext`
//!
//! **Primitives with constraints:**
//! ```eure
//! @ field {
//!   $variant: text
//!   min-length = 3
//!   max-length = 20
//!   pattern = "^[a-z]+$"
//! }
//! ```
//!
//! **Array:** `[.text]` or `{ $variant: array, item = .text, ... }`
//!
//! **Tuple:** `(.text, .integer)` or `{ $variant: tuple, elements = [...] }`
//!
//! **Record:** `{ name = .text, age = .integer }`
//!
//! **Union with named variants:**
//! ```eure
//! @ field {
//!   $variant: union
//!   variants.success = { data = .any }
//!   variants.error = { message = .text }
//!   $variant-repr = "untagged"  // optional
//!   priority = ["error", "success"]  // optional, for untagged unions
//! }
//! ```
//!
//! **Literal:** Any constant value (e.g., `"active"`, `42`, `true`)
//!
//! **Type reference:** `.$types.my-type` or `.$types.namespace.type`

use crate::{
    ArraySchema, Bound, Description, FloatSchema, IntegerSchema, MapSchema, PathSchema,
    RecordFieldSchema, RecordSchema, SchemaDocument, SchemaNodeContent, SchemaNodeId, TextSchema,
    TupleSchema, TypeReference, UnionSchema, UnknownFieldsPolicy,
};
use eure_value::data_model::VariantRepr;
use eure_value::document::node::{Node, NodeValue};
use eure_value::document::{EureDocument, NodeId};
use eure_value::identifier::Identifier;
use eure_value::path::{EurePath, PathSegment};
use eure_value::value::{ObjectKey, PrimitiveValue, Value};
use num_bigint::BigInt;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during document to schema conversion
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConversionError {
    #[error("Empty type path")]
    EmptyTypePath,

    #[error("Unknown primitive type: {0}")]
    UnknownPrimitiveType(EurePath),

    #[error("Unknown extension in type path: {0}")]
    UnknownExtensionPath(EurePath),

    #[error("Invalid type path: {0}")]
    InvalidTypePath(EurePath),

    #[error("Invalid type name: {0}")]
    InvalidTypeName(ObjectKey),

    #[error("Unsupported schema construct at path: {0}")]
    UnsupportedConstruct(String),

    #[error("Invalid extension value: {extension} at path {path}")]
    InvalidExtensionValue { extension: String, path: String },

    #[error("Missing required extension: {extension} at path {path}")]
    MissingRequiredExtension { extension: String, path: String },

    #[error("Conflicting extensions: {extensions:?} at path {path}")]
    ConflictingExtensions {
        extensions: Vec<String>,
        path: String,
    },

    #[error("Invalid node reference: {0}")]
    InvalidNodeReference(String),

    #[error("Invalid constraint value: {constraint} with value {value}")]
    InvalidConstraintValue { constraint: String, value: String },

    #[error("Invalid range string: {0}")]
    InvalidRangeString(String),

    #[error("Undefined type reference: {0}")]
    UndefinedTypeReference(String),
}

/// Internal converter state
struct Converter<'a> {
    doc: &'a EureDocument,
    schema: SchemaDocument,
}

impl<'a> Converter<'a> {
    fn new(doc: &'a EureDocument) -> Self {
        Self {
            doc,
            schema: SchemaDocument::new(),
        }
    }

    /// Convert the root node and produce the final schema
    fn convert(mut self) -> Result<SchemaDocument, ConversionError> {
        let root_id = self.doc.get_root_id();
        let root_node = self.doc.node(root_id);

        // Convert all type definitions from $types extension
        self.convert_types(root_node)?;

        // Convert root node
        self.schema.root = self.convert_node(root_id)?;

        // Validate all type references exist
        self.validate_type_references()?;

        Ok(self.schema)
    }

    /// Convert all type definitions from $types extension
    fn convert_types(&mut self, node: &Node) -> Result<(), ConversionError> {
        let types_ident: Identifier = "types".parse().unwrap();
        if let Some(types_node_id) = node.extensions.get(&types_ident) {
            let types_node = self.doc.node(*types_node_id);
            if let NodeValue::Map(map) = &types_node.content {
                for (key, &node_id) in map.0.iter() {
                    if let ObjectKey::String(name) = key {
                        let type_name: Identifier = name
                            .parse()
                            .map_err(|_| ConversionError::InvalidTypeName(key.clone()))?;
                        let schema_id = self.convert_node(node_id)?;
                        self.schema.types.insert(type_name, schema_id);
                    } else {
                        return Err(ConversionError::InvalidTypeName(key.clone()));
                    }
                }
            } else {
                return Err(ConversionError::InvalidExtensionValue {
                    extension: "types".to_string(),
                    path: "$types must be a map".to_string(),
                });
            }
        }
        Ok(())
    }

    /// Validate that all type references point to defined types
    fn validate_type_references(&self) -> Result<(), ConversionError> {
        for node in &self.schema.nodes {
            if let SchemaNodeContent::Reference(type_ref) = &node.content
                && type_ref.namespace.is_none()
                && !self.schema.types.contains_key(&type_ref.name)
            {
                return Err(ConversionError::UndefinedTypeReference(
                    type_ref.name.to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Convert a document node to a schema node
    fn convert_node(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        // Check for $variant extension to determine explicit type
        let variant = self.get_variant_extension(node)?;

        match &node.content {
            NodeValue::Uninitialized => {
                // Uninitialized node means incomplete document - always an error
                Err(ConversionError::UnsupportedConstruct(
                    "Incomplete document: uninitialized node".to_string(),
                ))
            }
            NodeValue::Primitive(prim) => self.convert_primitive(prim, node),
            NodeValue::Array(arr) => {
                // Array shorthand: [.type]
                if arr.0.len() == 1 {
                    let item_id = self.convert_node(arr.0[0])?;
                    let schema_id =
                        self.schema
                            .create_node(SchemaNodeContent::Array(ArraySchema {
                                item: item_id,
                                min_length: None,
                                max_length: None,
                                unique: false,
                                contains: None,
                                binding_style: None,
                            }));
                    Ok(schema_id)
                } else {
                    Err(ConversionError::UnsupportedConstruct(
                        "Array with multiple elements".to_string(),
                    ))
                }
            }
            NodeValue::Tuple(tup) => {
                // Tuple shorthand: (.type1, .type2)
                let elements: Vec<SchemaNodeId> = tup
                    .0
                    .iter()
                    .map(|&id| self.convert_node(id))
                    .collect::<Result<_, _>>()?;
                let schema_id = self
                    .schema
                    .create_node(SchemaNodeContent::Tuple(TupleSchema {
                        elements,
                        binding_style: None,
                    }));
                Ok(schema_id)
            }
            NodeValue::Map(map) => {
                // Could be: record, map, union, array, etc. based on $variant
                self.convert_map_node(node_id, map, variant, node)
            }
        }
    }

    /// Get $variant extension value if present
    /// Returns Ok(Some(tag)) for valid single-segment variant or string
    /// Returns Ok(None) if $variant is not present
    /// Returns Err for multi-segment paths (invalid in schema context - type type has no nested unions)
    fn get_variant_extension(&self, node: &Node) -> Result<Option<String>, ConversionError> {
        let variant_ident: Identifier = "variant".parse().unwrap();
        if let Some(&ext_node_id) = node.extensions.get(&variant_ident) {
            let ext_node = self.doc.node(ext_node_id);
            // Simple variant: $variant = .text (parsed as Variant with tag "text")
            if let NodeValue::Primitive(PrimitiveValue::Variant(v)) = &ext_node.content {
                return Ok(Some(v.tag.clone()));
            }
            // Text value: $variant = "union"
            if let NodeValue::Primitive(PrimitiveValue::Text(t)) = &ext_node.content {
                return Ok(Some(t.as_str().to_string()));
            }
            // Path value: $variant = .text or $variant = .ok.ok (multi-segment)
            if let NodeValue::Primitive(PrimitiveValue::Path(path)) = &ext_node.content {
                let segments: Vec<&str> = path
                    .0
                    .iter()
                    .filter_map(|seg| {
                        if let PathSegment::Ident(ident) = seg {
                            Some(ident.as_ref())
                        } else {
                            None
                        }
                    })
                    .collect();

                if segments.len() == 1 {
                    // Single-segment path like .text, .union - equivalent to string notation
                    return Ok(Some(segments[0].to_string()));
                } else if segments.len() > 1 {
                    // Multi-segment path like .ok.ok - invalid in schema context
                    // The type type union doesn't have nested unions
                    return Err(ConversionError::InvalidExtensionValue {
                        extension: "variant".to_string(),
                        path: format!(
                            "nested variant path .{} is invalid in schema context (type type has no nested unions)",
                            segments.join(".")
                        ),
                    });
                }
            }
        }
        Ok(None)
    }

    /// Convert a primitive value to a schema node
    fn convert_primitive(
        &mut self,
        prim: &PrimitiveValue,
        node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        match prim {
            PrimitiveValue::Path(path) => self.convert_path_to_type(path, node),
            PrimitiveValue::Text(t) => {
                // Text literals are always literal types in schema context
                let schema_id =
                    self.schema
                        .create_node(SchemaNodeContent::Literal(Value::Primitive(
                            PrimitiveValue::Text(t.clone()),
                        )));
                Ok(schema_id)
            }
            PrimitiveValue::BigInt(i) => {
                let schema_id =
                    self.schema
                        .create_node(SchemaNodeContent::Literal(Value::Primitive(
                            PrimitiveValue::BigInt(i.clone()),
                        )));
                Ok(schema_id)
            }
            PrimitiveValue::Bool(b) => {
                let schema_id =
                    self.schema
                        .create_node(SchemaNodeContent::Literal(Value::Primitive(
                            PrimitiveValue::Bool(*b),
                        )));
                Ok(schema_id)
            }
            PrimitiveValue::Null => {
                let schema_id =
                    self.schema
                        .create_node(SchemaNodeContent::Literal(Value::Primitive(
                            PrimitiveValue::Null,
                        )));
                Ok(schema_id)
            }
            _ => Err(ConversionError::UnsupportedConstruct(format!(
                "Unsupported primitive value: {:?}",
                prim
            ))),
        }
    }

    /// Convert a path value to a schema type
    fn convert_path_to_type(
        &mut self,
        path: &EurePath,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        match path.0.as_slice() {
            [] => Err(ConversionError::EmptyTypePath),

            // Primitive types: .text, .integer, .float, .boolean, .null, .any, .path
            [PathSegment::Ident(ident)] => {
                let content = match ident.as_ref() {
                    "text" => SchemaNodeContent::Text(TextSchema::default()),
                    "integer" => SchemaNodeContent::Integer(IntegerSchema::default()),
                    "float" => SchemaNodeContent::Float(FloatSchema::default()),
                    "boolean" => SchemaNodeContent::Boolean,
                    "null" => SchemaNodeContent::Null,
                    "any" => SchemaNodeContent::Any,
                    "path" => SchemaNodeContent::Path(PathSchema::default()),
                    _ => return Err(ConversionError::UnknownPrimitiveType(path.clone())),
                };
                Ok(self.schema.create_node(content))
            }

            // Text with language: .text.rust, .text.email, .text.plaintext
            [PathSegment::Ident(first), PathSegment::Ident(lang)] if first.as_ref() == "text" => {
                let content = SchemaNodeContent::Text(TextSchema {
                    language: Some(lang.to_string()),
                    ..Default::default()
                });
                Ok(self.schema.create_node(content))
            }

            // Local type reference: .$types.typename
            [PathSegment::Extension(ext), PathSegment::Ident(type_name)]
                if ext.as_ref() == "types" =>
            {
                let content = SchemaNodeContent::Reference(TypeReference {
                    namespace: None,
                    name: type_name.clone(),
                });
                Ok(self.schema.create_node(content))
            }

            // External type reference: .$types.namespace.typename
            [
                PathSegment::Extension(ext),
                PathSegment::Ident(ns),
                PathSegment::Ident(type_name),
            ] if ext.as_ref() == "types" => {
                let content = SchemaNodeContent::Reference(TypeReference {
                    namespace: Some(ns.to_string()),
                    name: type_name.clone(),
                });
                Ok(self.schema.create_node(content))
            }

            // Unknown extension
            [PathSegment::Extension(_), ..] => {
                Err(ConversionError::UnknownExtensionPath(path.clone()))
            }

            // Catch-all for invalid paths
            _ => Err(ConversionError::InvalidTypePath(path.clone())),
        }
    }

    /// Convert a map node to a schema node (record, map, union, array, etc.)
    fn convert_map_node(
        &mut self,
        node_id: NodeId,
        map: &eure_value::document::node::NodeMap,
        variant: Option<String>,
        node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        match variant.as_deref() {
            // Text type with optional constraints
            Some("text") => self.convert_text_with_constraints(node_id, node),
            Some("integer") => self.convert_integer_with_constraints(node_id, node),
            Some("float") => self.convert_float_with_constraints(node_id, node),
            Some("array") => self.convert_array_with_constraints(node_id, node),
            Some("map") => self.convert_map_type(node_id, node),
            Some("tuple") => self.convert_tuple_with_constraints(node_id, node),
            Some("union") => self.convert_union_type(node_id, node),
            Some("path") => self.convert_path_with_constraints(node_id, node),
            Some("literal") => self.convert_literal_type(node_id, node),
            Some("record") => self.convert_record_type(node_id, node),
            None => {
                // No explicit variant - treat as record
                self.convert_record_type_from_map(map, node)
            }
            Some(other) => Err(ConversionError::UnsupportedConstruct(format!(
                "Unknown variant: {}",
                other
            ))),
        }
    }

    /// Create an empty record
    fn create_empty_record(&mut self) -> Result<SchemaNodeId, ConversionError> {
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Record(RecordSchema::default()));
        Ok(schema_id)
    }

    /// Convert a text type with constraints
    fn convert_text_with_constraints(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);
        let mut text_schema = TextSchema::default();

        if let NodeValue::Map(map) = &node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key {
                    match key_str.as_str() {
                        "language" => {
                            text_schema.language = self.get_string_value(value_id)?;
                        }
                        "min-length" => {
                            text_schema.min_length = self.get_integer_value(value_id)?;
                        }
                        "max-length" => {
                            text_schema.max_length = self.get_integer_value(value_id)?;
                        }
                        "pattern" => {
                            text_schema.pattern = self.get_string_value(value_id)?;
                        }
                        _ => {}
                    }
                }
            }
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Text(text_schema));
        Ok(schema_id)
    }

    /// Convert an integer type with constraints
    fn convert_integer_with_constraints(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);
        let mut int_schema = IntegerSchema::default();

        if let NodeValue::Map(map) = &node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key {
                    match key_str.as_str() {
                        "range" => {
                            let range_str = self.get_string_value(value_id)?.ok_or(
                                ConversionError::InvalidRangeString("missing".to_string()),
                            )?;
                            let (min, max) = parse_integer_range(&range_str)?;
                            int_schema.min = min;
                            int_schema.max = max;
                        }
                        "multiple-of" => {
                            if let Some(v) = self.get_bigint_value(value_id)? {
                                int_schema.multiple_of = Some(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Integer(int_schema));
        Ok(schema_id)
    }

    /// Convert a float type with constraints
    fn convert_float_with_constraints(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);
        let mut float_schema = FloatSchema::default();

        if let NodeValue::Map(map) = &node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key {
                    match key_str.as_str() {
                        "range" => {
                            let range_str = self.get_string_value(value_id)?.ok_or(
                                ConversionError::InvalidRangeString("missing".to_string()),
                            )?;
                            let (min, max) = parse_float_range(&range_str)?;
                            float_schema.min = min;
                            float_schema.max = max;
                        }
                        "multiple-of" => {
                            if let Some(v) = self.get_float_value(value_id)? {
                                float_schema.multiple_of = Some(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Float(float_schema));
        Ok(schema_id)
    }

    /// Convert an array type with constraints
    fn convert_array_with_constraints(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);
        let mut min_length = None;
        let mut max_length = None;
        let mut unique = false;
        let mut contains = None;
        let mut item_id = None;

        if let NodeValue::Map(map) = &node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key {
                    match key_str.as_str() {
                        "item" => {
                            item_id = Some(self.convert_node(value_id)?);
                        }
                        "min-length" => {
                            min_length = self.get_integer_value(value_id)?;
                        }
                        "max-length" => {
                            max_length = self.get_integer_value(value_id)?;
                        }
                        "unique" => {
                            unique = self.get_bool_value(value_id)?.unwrap_or(false);
                        }
                        "contains" => {
                            contains = Some(self.convert_node(value_id)?);
                        }
                        _ => {}
                    }
                }
            }
        }

        let item = item_id.ok_or_else(|| ConversionError::MissingRequiredExtension {
            extension: "item".to_string(),
            path: "array".to_string(),
        })?;

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Array(ArraySchema {
                item,
                min_length,
                max_length,
                unique,
                contains,
                binding_style: None,
            }));
        Ok(schema_id)
    }

    /// Convert a map type
    fn convert_map_type(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);
        let mut key_id = None;
        let mut value_id = None;
        let mut min_size = None;
        let mut max_size = None;

        if let NodeValue::Map(map) = &node.content {
            for (key, &val_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key {
                    match key_str.as_str() {
                        "key" => {
                            key_id = Some(self.convert_node(val_id)?);
                        }
                        "value" => {
                            value_id = Some(self.convert_node(val_id)?);
                        }
                        "min-size" => {
                            min_size = self.get_integer_value(val_id)?;
                        }
                        "max-size" => {
                            max_size = self.get_integer_value(val_id)?;
                        }
                        _ => {}
                    }
                }
            }
        }

        let key = key_id.ok_or_else(|| ConversionError::MissingRequiredExtension {
            extension: "key".to_string(),
            path: "map".to_string(),
        })?;

        let value = value_id.ok_or_else(|| ConversionError::MissingRequiredExtension {
            extension: "value".to_string(),
            path: "map".to_string(),
        })?;

        let schema_id = self.schema.create_node(SchemaNodeContent::Map(MapSchema {
            key,
            value,
            min_size,
            max_size,
        }));
        Ok(schema_id)
    }

    /// Convert a tuple type with constraints
    fn convert_tuple_with_constraints(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);
        let mut elements = Vec::new();

        if let NodeValue::Map(map) = &node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key
                    && key_str == "elements"
                {
                    let elem_node = self.doc.node(value_id);
                    if let NodeValue::Array(arr) = &elem_node.content {
                        for &elem_id in &arr.0 {
                            elements.push(self.convert_node(elem_id)?);
                        }
                    }
                }
            }
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Tuple(TupleSchema {
                elements,
                binding_style: None,
            }));
        Ok(schema_id)
    }

    /// Convert a union type
    fn convert_union_type(
        &mut self,
        node_id: NodeId,
        node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let doc_node = self.doc.node(node_id);
        let mut variants: HashMap<String, SchemaNodeId> = HashMap::new();
        let mut priority = None;
        let mut repr = VariantRepr::External;

        // Check for $variant-repr extension
        let repr_ident: Identifier = "variant-repr".parse().unwrap();
        if let Some(&repr_node_id) = node.extensions.get(&repr_ident) {
            repr = self.parse_variant_repr(repr_node_id)?;
        }

        if let NodeValue::Map(map) = &doc_node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key {
                    if key_str == "variants" {
                        // variants = { name => schema, ... }
                        let variants_node = self.doc.node(value_id);
                        if let NodeValue::Map(variants_map) = &variants_node.content {
                            for (var_key, &var_value_id) in variants_map.0.iter() {
                                if let ObjectKey::String(var_name) = var_key {
                                    let var_schema_id = self.convert_node(var_value_id)?;
                                    variants.insert(var_name.clone(), var_schema_id);
                                }
                            }
                        }
                    } else if key_str.starts_with("variants.") {
                        // variants.name = schema (alternative syntax)
                        let var_name = key_str.strip_prefix("variants.").unwrap().to_string();
                        let var_schema_id = self.convert_node(value_id)?;
                        variants.insert(var_name, var_schema_id);
                    } else if key_str == "priority" {
                        priority = self.get_string_array(value_id)?;
                    }
                }
            }
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Union(UnionSchema {
                variants,
                priority,
                repr,
            }));
        Ok(schema_id)
    }

    /// Parse variant representation from extension
    fn parse_variant_repr(&self, node_id: NodeId) -> Result<VariantRepr, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => {
                let s_str = t.as_str();
                match s_str {
                    "untagged" => Ok(VariantRepr::Untagged),
                    "external" => Ok(VariantRepr::External),
                    _ => Err(ConversionError::InvalidExtensionValue {
                        extension: "variant-repr".to_string(),
                        path: s_str.to_string(),
                    }),
                }
            }
            NodeValue::Map(map) => {
                let mut tag = None;
                let mut content = None;

                for (key, &value_id) in map.0.iter() {
                    if let ObjectKey::String(key_str) = key {
                        match key_str.as_str() {
                            "tag" => {
                                tag = self.get_string_value(value_id)?;
                            }
                            "content" => {
                                content = self.get_string_value(value_id)?;
                            }
                            _ => {}
                        }
                    }
                }

                if let Some(tag_str) = tag {
                    if let Some(content_str) = content {
                        Ok(VariantRepr::Adjacent {
                            tag: tag_str,
                            content: content_str,
                        })
                    } else {
                        Ok(VariantRepr::Internal { tag: tag_str })
                    }
                } else {
                    Err(ConversionError::InvalidExtensionValue {
                        extension: "variant-repr".to_string(),
                        path: "missing tag".to_string(),
                    })
                }
            }
            _ => Err(ConversionError::InvalidExtensionValue {
                extension: "variant-repr".to_string(),
                path: "invalid type".to_string(),
            }),
        }
    }

    /// Convert a path type with constraints
    fn convert_path_with_constraints(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);
        let mut path_schema = PathSchema::default();

        if let NodeValue::Map(map) = &node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key {
                    match key_str.as_str() {
                        "starts-with" => {
                            path_schema.starts_with = self.get_path_value(value_id)?;
                        }
                        "min-length" => {
                            path_schema.min_length = self.get_integer_value(value_id)?;
                        }
                        "max-length" => {
                            path_schema.max_length = self.get_integer_value(value_id)?;
                        }
                        _ => {}
                    }
                }
            }
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Path(path_schema));
        Ok(schema_id)
    }

    /// Convert a literal type
    fn convert_literal_type(
        &mut self,
        node_id: NodeId,
        _node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        // Look for root binding value in the map
        if let NodeValue::Map(map) = &node.content {
            for (key, &value_id) in map.0.iter() {
                if let ObjectKey::String(key_str) = key
                    && key_str.is_empty()
                {
                    // Root binding: { = value, $variant: literal }
                    let value = self.node_to_value(value_id)?;
                    let schema_id = self.schema.create_node(SchemaNodeContent::Literal(value));
                    return Ok(schema_id);
                }
            }
        }

        Err(ConversionError::MissingRequiredExtension {
            extension: "value".to_string(),
            path: "literal".to_string(),
        })
    }

    /// Convert a record type (explicit $variant: record)
    fn convert_record_type(
        &mut self,
        node_id: NodeId,
        node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let doc_node = self.doc.node(node_id);
        if let NodeValue::Map(map) = &doc_node.content {
            self.convert_record_type_from_map(map, node)
        } else {
            self.create_empty_record()
        }
    }

    /// Convert a map to a record type
    fn convert_record_type_from_map(
        &mut self,
        map: &eure_value::document::node::NodeMap,
        node: &Node,
    ) -> Result<SchemaNodeId, ConversionError> {
        let mut properties: HashMap<String, RecordFieldSchema> = HashMap::new();
        let mut unknown_fields = UnknownFieldsPolicy::Deny;

        // Check for $unknown-fields extension
        let unknown_fields_ident: Identifier = "unknown-fields".parse().unwrap();
        if let Some(&ext_node_id) = node.extensions.get(&unknown_fields_ident) {
            unknown_fields = self.parse_unknown_fields_policy(ext_node_id)?;
        }

        for (key, &value_id) in map.0.iter() {
            if let ObjectKey::String(field_name) = key {
                // Skip internal fields like $variant
                if field_name.starts_with('$') {
                    continue;
                }

                let field_schema_id = self.convert_node(value_id)?;

                // Get field metadata from the field node's extensions
                let field_node = self.doc.node(value_id);
                let (optional, description, deprecated, default) =
                    self.extract_field_metadata(field_node)?;

                // Apply metadata to the schema node
                {
                    let schema_node = self.schema.node_mut(field_schema_id);
                    if let Some(desc) = description {
                        schema_node.metadata.description = Some(desc);
                    }
                    schema_node.metadata.deprecated = deprecated;
                    schema_node.metadata.default = default;
                }

                properties.insert(
                    field_name.clone(),
                    RecordFieldSchema {
                        schema: field_schema_id,
                        optional,
                        binding_style: None,
                    },
                );
            }
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Record(RecordSchema {
                properties,
                unknown_fields,
            }));
        Ok(schema_id)
    }

    /// Extract metadata from field node extensions
    fn extract_field_metadata(
        &self,
        node: &Node,
    ) -> Result<(bool, Option<Description>, bool, Option<Value>), ConversionError> {
        let mut optional = false;
        let mut description = None;
        let mut deprecated = false;
        let mut default = None;

        // Check for $optional extension
        let optional_ident: Identifier = "optional".parse().unwrap();
        if let Some(&ext_node_id) = node.extensions.get(&optional_ident) {
            optional = self.get_bool_value(ext_node_id)?.unwrap_or(false);
        }

        // Check for $description extension
        let description_ident: Identifier = "description".parse().unwrap();
        if let Some(&ext_node_id) = node.extensions.get(&description_ident) {
            let ext_node = self.doc.node(ext_node_id);
            if let NodeValue::Primitive(PrimitiveValue::Text(t)) = &ext_node.content {
                description = Some(Description::String(t.as_str().to_string()));
            }
        }

        // Check for $deprecated extension
        let deprecated_ident: Identifier = "deprecated".parse().unwrap();
        if let Some(&ext_node_id) = node.extensions.get(&deprecated_ident) {
            deprecated = self.get_bool_value(ext_node_id)?.unwrap_or(false);
        }

        // Check for $default extension
        let default_ident: Identifier = "default".parse().unwrap();
        if let Some(&ext_node_id) = node.extensions.get(&default_ident) {
            default = Some(self.node_to_value(ext_node_id)?);
        }

        Ok((optional, description, deprecated, default))
    }

    /// Parse unknown fields policy
    fn parse_unknown_fields_policy(
        &mut self,
        node_id: NodeId,
    ) -> Result<UnknownFieldsPolicy, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => match t.as_str() {
                "deny" => Ok(UnknownFieldsPolicy::Deny),
                "allow" => Ok(UnknownFieldsPolicy::Allow),
                _ => Err(ConversionError::InvalidExtensionValue {
                    extension: "unknown-fields".to_string(),
                    path: t.as_str().to_string(),
                }),
            },
            NodeValue::Primitive(PrimitiveValue::Path(_)) => {
                // Schema type for unknown fields
                let schema_id = self.convert_node(node_id)?;
                Ok(UnknownFieldsPolicy::Schema(schema_id))
            }
            _ => Err(ConversionError::InvalidExtensionValue {
                extension: "unknown-fields".to_string(),
                path: "invalid type".to_string(),
            }),
        }
    }

    /// Helper: get integer value from a node
    fn get_integer_value(&self, node_id: NodeId) -> Result<Option<u32>, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::BigInt(i)) => {
                let value: u32 =
                    i.try_into()
                        .map_err(|_| ConversionError::InvalidConstraintValue {
                            constraint: "integer".to_string(),
                            value: i.to_string(),
                        })?;
                Ok(Some(value))
            }
            _ => Ok(None),
        }
    }

    /// Helper: get bigint value from a node
    fn get_bigint_value(&self, node_id: NodeId) -> Result<Option<BigInt>, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::BigInt(i)) => Ok(Some(i.clone())),
            _ => Ok(None),
        }
    }

    /// Helper: get float value from a node
    fn get_float_value(&self, node_id: NodeId) -> Result<Option<f64>, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::F64(f)) => Ok(Some(*f)),
            NodeValue::Primitive(PrimitiveValue::F32(f)) => Ok(Some(*f as f64)),
            NodeValue::Primitive(PrimitiveValue::BigInt(i)) => {
                let value: i64 =
                    i.try_into()
                        .map_err(|_| ConversionError::InvalidConstraintValue {
                            constraint: "float".to_string(),
                            value: i.to_string(),
                        })?;
                Ok(Some(value as f64))
            }
            _ => Ok(None),
        }
    }

    /// Helper: get string value from a node (accepts any Text value)
    fn get_string_value(&self, node_id: NodeId) -> Result<Option<String>, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => Ok(Some(t.as_str().to_string())),
            _ => Ok(None),
        }
    }

    /// Helper: get bool value from a node
    fn get_bool_value(&self, node_id: NodeId) -> Result<Option<bool>, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Bool(b)) => Ok(Some(*b)),
            _ => Ok(None),
        }
    }

    /// Helper: get path value from a node
    fn get_path_value(&self, node_id: NodeId) -> Result<Option<EurePath>, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Path(p)) => Ok(Some(p.clone())),
            _ => Ok(None),
        }
    }

    /// Helper: get string array from a node
    fn get_string_array(&self, node_id: NodeId) -> Result<Option<Vec<String>>, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Array(arr) => {
                let mut strings = Vec::new();
                for &elem_id in &arr.0 {
                    if let Some(s) = self.get_string_value(elem_id)? {
                        strings.push(s);
                    }
                }
                Ok(Some(strings))
            }
            _ => Ok(None),
        }
    }

    /// Convert a document node to a Value for literal types
    fn node_to_value(&self, node_id: NodeId) -> Result<Value, ConversionError> {
        let node = self.doc.node(node_id);
        match &node.content {
            NodeValue::Primitive(prim) => Ok(Value::Primitive(prim.clone())),
            NodeValue::Array(arr) => {
                let values: Vec<Value> = arr
                    .0
                    .iter()
                    .map(|&id| self.node_to_value(id))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Array(eure_value::value::Array(values)))
            }
            NodeValue::Tuple(tup) => {
                let values: Vec<Value> = tup
                    .0
                    .iter()
                    .map(|&id| self.node_to_value(id))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Tuple(eure_value::value::Tuple(values)))
            }
            NodeValue::Map(map) => {
                let mut result = eure_value::value::Map::default();
                for (key, &value_id) in map.0.iter() {
                    let value = self.node_to_value(value_id)?;
                    result.0.insert(key.clone(), value);
                }
                Ok(Value::Map(result))
            }
            NodeValue::Uninitialized => Err(ConversionError::UnsupportedConstruct(
                "Uninitialized node".to_string(),
            )),
        }
    }
}

/// Parse an integer range string (Rust-style or interval notation)
fn parse_integer_range(s: &str) -> Result<(Bound<BigInt>, Bound<BigInt>), ConversionError> {
    let s = s.trim();

    // Try interval notation first: [a, b], (a, b), [a, b), (a, b]
    if s.starts_with('[') || s.starts_with('(') {
        return parse_interval_integer(s);
    }

    // Rust-style: a..b, a..=b, a.., ..b, ..=b
    if let Some(eq_pos) = s.find("..=") {
        let left = &s[..eq_pos];
        let right = &s[eq_pos + 3..];
        let min = if left.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Inclusive(parse_bigint(left)?)
        };
        let max = if right.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Inclusive(parse_bigint(right)?)
        };
        Ok((min, max))
    } else if let Some(dot_pos) = s.find("..") {
        let left = &s[..dot_pos];
        let right = &s[dot_pos + 2..];
        let min = if left.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Inclusive(parse_bigint(left)?)
        };
        let max = if right.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Exclusive(parse_bigint(right)?)
        };
        Ok((min, max))
    } else {
        Err(ConversionError::InvalidRangeString(s.to_string()))
    }
}

/// Parse interval notation for integers: [a, b], (a, b), etc.
fn parse_interval_integer(s: &str) -> Result<(Bound<BigInt>, Bound<BigInt>), ConversionError> {
    let left_inclusive = s.starts_with('[');
    let right_inclusive = s.ends_with(']');

    let inner = &s[1..s.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
    if parts.len() != 2 {
        return Err(ConversionError::InvalidRangeString(s.to_string()));
    }

    let min = if parts[0].is_empty() {
        Bound::Unbounded
    } else if left_inclusive {
        Bound::Inclusive(parse_bigint(parts[0])?)
    } else {
        Bound::Exclusive(parse_bigint(parts[0])?)
    };

    let max = if parts[1].is_empty() {
        Bound::Unbounded
    } else if right_inclusive {
        Bound::Inclusive(parse_bigint(parts[1])?)
    } else {
        Bound::Exclusive(parse_bigint(parts[1])?)
    };

    Ok((min, max))
}

/// Parse a float range string
fn parse_float_range(s: &str) -> Result<(Bound<f64>, Bound<f64>), ConversionError> {
    let s = s.trim();

    // Try interval notation first
    if s.starts_with('[') || s.starts_with('(') {
        return parse_interval_float(s);
    }

    // Rust-style
    if let Some(eq_pos) = s.find("..=") {
        let left = &s[..eq_pos];
        let right = &s[eq_pos + 3..];
        let min = if left.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Inclusive(parse_f64(left)?)
        };
        let max = if right.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Inclusive(parse_f64(right)?)
        };
        Ok((min, max))
    } else if let Some(dot_pos) = s.find("..") {
        let left = &s[..dot_pos];
        let right = &s[dot_pos + 2..];
        let min = if left.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Inclusive(parse_f64(left)?)
        };
        let max = if right.is_empty() {
            Bound::Unbounded
        } else {
            Bound::Exclusive(parse_f64(right)?)
        };
        Ok((min, max))
    } else {
        Err(ConversionError::InvalidRangeString(s.to_string()))
    }
}

/// Parse interval notation for floats
fn parse_interval_float(s: &str) -> Result<(Bound<f64>, Bound<f64>), ConversionError> {
    let left_inclusive = s.starts_with('[');
    let right_inclusive = s.ends_with(']');

    let inner = &s[1..s.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
    if parts.len() != 2 {
        return Err(ConversionError::InvalidRangeString(s.to_string()));
    }

    let min = if parts[0].is_empty() {
        Bound::Unbounded
    } else if left_inclusive {
        Bound::Inclusive(parse_f64(parts[0])?)
    } else {
        Bound::Exclusive(parse_f64(parts[0])?)
    };

    let max = if parts[1].is_empty() {
        Bound::Unbounded
    } else if right_inclusive {
        Bound::Inclusive(parse_f64(parts[1])?)
    } else {
        Bound::Exclusive(parse_f64(parts[1])?)
    };

    Ok((min, max))
}

fn parse_bigint(s: &str) -> Result<BigInt, ConversionError> {
    s.parse()
        .map_err(|_| ConversionError::InvalidRangeString(format!("Invalid integer: {}", s)))
}

fn parse_f64(s: &str) -> Result<f64, ConversionError> {
    s.parse()
        .map_err(|_| ConversionError::InvalidRangeString(format!("Invalid float: {}", s)))
}

/// Convert an EureDocument containing schema definitions to a SchemaDocument
///
/// This function traverses the document and extracts schema information from:
/// - Type paths (`.text`, `.integer`, `.text.rust`, etc.)
/// - `$variant` extension for explicit type variants
/// - `variants.*` fields for union variant definitions
/// - Constraint fields (`min-length`, `max-length`, `pattern`, `range`, etc.)
/// - Metadata extensions (`$description`, `$deprecated`, `$default`, `$examples`)
///
/// # Arguments
///
/// * `doc` - The EureDocument containing schema definitions
///
/// # Returns
///
/// A SchemaDocument on success, or a ConversionError on failure
///
/// # Examples
///
/// ```ignore
/// use eure::parse_to_document;
/// use eure_schema::convert::document_to_schema;
///
/// let input = r#"
/// name = .text
/// age = .integer
/// "#;
///
/// let doc = parse_to_document(input).unwrap();
/// let schema = document_to_schema(&doc).unwrap();
/// ```
pub fn document_to_schema(doc: &EureDocument) -> Result<SchemaDocument, ConversionError> {
    Converter::new(doc).convert()
}
