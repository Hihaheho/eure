//! Conversion from EureDocument to SchemaDocument
//!
//! This module provides functionality to convert EURE documents containing schema definitions
//! (using extensions like $type, $array, $variants, etc.) into SchemaDocument structures.

use crate::{
    ArraySchema, BooleanSchema, CodeSchema, FloatSchema, IntegerSchema, MapSchema, PathSchema,
    RecordSchema, SchemaDocument, SchemaMetadata, SchemaNodeContent, SchemaNodeId, StringSchema,
    TupleSchema, UnknownFieldsPolicy, VariantSchema,
};
use eure_value::data_model::VariantRepr;
use eure_value::document::node::NodeValue;
use eure_value::document::{EureDocument, NodeId};
use eure_value::identifier::Identifier;
use eure_value::path::{EurePath, PathSegment};
use eure_value::value::{ObjectKey, PrimitiveValue};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during document to schema conversion
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConversionError {
    #[error("Invalid type path: {0}")]
    InvalidTypePath(String),

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
}

/// Internal conversion context to track state during conversion
struct ConversionContext<'a> {
    /// The source EureDocument
    doc: &'a EureDocument,
    /// The target SchemaDocument being built
    schema: SchemaDocument,
    /// Mapping from type names to whether they've been processed
    type_definitions: HashMap<String, Option<SchemaNodeId>>,
    /// Current path for error reporting
    current_path: EurePath,
    /// Cascade type from ancestor nodes (applies to leaf nodes without explicit types)
    cascade_type: Option<EurePath>,
}

impl<'a> ConversionContext<'a> {
    fn new(doc: &'a EureDocument) -> Self {
        Self {
            doc,
            schema: SchemaDocument::new(),
            type_definitions: HashMap::new(),
            current_path: EurePath::root(),
            cascade_type: None,
        }
    }

    /// Get an identifier from a string
    fn ident(s: &str) -> Identifier {
        s.parse().expect("Invalid identifier")
    }

    /// Convert a node to a schema node, returning its ID
    fn convert_node(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        // Save the current cascade type for restoration after processing this subtree
        let saved_cascade_type = self.cascade_type.clone();

        // Check for $cascade-type extension on this node
        // If present, it becomes the cascade type for all descendants
        if let Some(cascade_node_id) = node.get_extension(&Self::ident("cascade-type")) {
            let cascade_node = self.doc.node(cascade_node_id);
            if let NodeValue::Primitive(PrimitiveValue::Path(path)) = &cascade_node.content {
                self.cascade_type = Some(path.clone());
            }
        }

        // First, check for type-specifying extensions
        // Priority order:
        // 1. $variant (explicit variant type marker)
        // 2. $type (type path or shorthand)
        // 3. $array (array shorthand)
        // 4. $variants (variant type with named variants)
        // 5. Implicit type from content (uses cascade-type for leaf nodes)

        let result = if let Some(variants_node_id) = node.get_extension(&Self::ident("variants")) {
            // Check for $variants extension (variant type definition using sections)
            self.convert_variants_definition(node_id, variants_node_id)
        } else if let Some(variant_node_id) = node.get_extension(&Self::ident("variant")) {
            // Check for $variant extension (explicit type marker like "array", "map", "string", etc.)
            self.convert_variant_marked_node(node_id, variant_node_id)
        } else if let Some(type_node_id) = node.get_extension(&Self::ident("type")) {
            // Check for $type extension
            self.convert_type_extension(node_id, type_node_id)
        } else if let Some(array_item_node_id) = node.get_extension(&Self::ident("array")) {
            // Check for $array extension (array shorthand)
            self.convert_array_shorthand(node_id, array_item_node_id)
        } else {
            // Check for implicit type from content
            self.convert_implicit_type(node_id)
        };

        // Restore cascade type after processing this subtree
        self.cascade_type = saved_cascade_type;

        result
    }

    /// Convert a node with explicit $variant marker
    fn convert_variant_marked_node(
        &mut self,
        node_id: NodeId,
        variant_node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        let variant_node = self.doc.node(variant_node_id);

        // Get the variant type name from the node's content
        let variant_type = match &variant_node.content {
            NodeValue::Primitive(PrimitiveValue::Path(path)) => {
                // Path like .string, .array, etc.
                if path.0.len() == 1 {
                    if let PathSegment::Ident(ident) = &path.0[0] {
                        ident.as_ref().to_string()
                    } else {
                        return Err(ConversionError::InvalidTypePath(format!("{}", path)));
                    }
                } else {
                    return Err(ConversionError::InvalidTypePath(format!("{}", path)));
                }
            }
            NodeValue::Primitive(PrimitiveValue::String(s)) => s.to_string(),
            _ => {
                return Err(ConversionError::InvalidExtensionValue {
                    extension: "variant".to_string(),
                    path: format!("{}", self.current_path),
                })
            }
        };

        match variant_type.as_str() {
            "string" => self.convert_string_variant(node_id),
            "integer" => self.convert_integer_variant(node_id),
            "float" => self.convert_float_variant(node_id),
            "array" => self.convert_array_variant(node_id),
            "map" => self.convert_map_variant(node_id),
            "tuple" => self.convert_tuple_variant(node_id),
            "union" => self.convert_union_variant(node_id),
            "literal" => self.convert_literal_variant(node_id),
            "path" => self.convert_path_variant(node_id),
            "code" => self.convert_code_variant(node_id),
            _ => Err(ConversionError::InvalidTypePath(variant_type)),
        }
    }

    /// Convert a node with $type extension
    fn convert_type_extension(
        &mut self,
        node_id: NodeId,
        type_node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        let type_node = self.doc.node(type_node_id);

        match &type_node.content {
            NodeValue::Primitive(PrimitiveValue::Path(path)) => {
                self.convert_type_path(node_id, path)
            }
            _ => Err(ConversionError::InvalidExtensionValue {
                extension: "type".to_string(),
                path: format!("{}", self.current_path),
            }),
        }
    }

    /// Convert a type path like .string, .integer, .code.rust, .$types.typename
    fn convert_type_path(
        &mut self,
        node_id: NodeId,
        path: &EurePath,
    ) -> Result<SchemaNodeId, ConversionError> {
        if path.0.is_empty() {
            return Err(ConversionError::InvalidTypePath("empty path".to_string()));
        }

        match &path.0[0] {
            PathSegment::Ident(ident) => {
                let type_name = ident.as_ref();
                match type_name {
                    "string" => self.create_string_schema(node_id),
                    "integer" => self.create_integer_schema(node_id),
                    "float" => self.create_float_schema(node_id),
                    "boolean" => self.create_boolean_schema(node_id),
                    "null" => self.create_null_schema(node_id),
                    "any" => self.create_any_schema(node_id),
                    "path" => self.create_path_schema(node_id),
                    "code" => {
                        // Check for language specifier (.code.rust, .code.email, etc.)
                        let language = if path.0.len() > 1 {
                            if let PathSegment::Ident(lang_ident) = &path.0[1] {
                                Some(lang_ident.as_ref().to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        self.create_code_schema_with_language(node_id, language)
                    }
                    _ => Err(ConversionError::InvalidTypePath(format!("{}", path))),
                }
            }
            PathSegment::Extension(ident) => {
                // Check for $types reference
                if ident.as_ref() == "types" {
                    self.convert_type_reference(path)
                } else {
                    Err(ConversionError::InvalidTypePath(format!("{}", path)))
                }
            }
            _ => Err(ConversionError::InvalidTypePath(format!("{}", path))),
        }
    }

    /// Convert a type reference like .$types.typename
    fn convert_type_reference(&mut self, path: &EurePath) -> Result<SchemaNodeId, ConversionError> {
        if path.0.len() < 2 {
            return Err(ConversionError::InvalidTypePath(format!("{}", path)));
        }

        // Get the type name from the path
        let type_name = match &path.0[1] {
            PathSegment::Ident(ident) => ident.as_ref().to_string(),
            _ => return Err(ConversionError::InvalidTypePath(format!("{}", path))),
        };

        // Create a reference node
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Reference(Self::ident(&type_name)));
        Ok(schema_id)
    }

    /// Convert array shorthand ($array = .string)
    fn convert_array_shorthand(
        &mut self,
        node_id: NodeId,
        item_node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        // Convert the item type
        let item_schema_id = self.convert_node(item_node_id)?;

        // Get array constraints from the node's extensions
        let node = self.doc.node(node_id);
        let min_items = self.get_integer_extension(node_id, "min-items")?;
        let max_items = self.get_integer_extension(node_id, "max-items")?;
        let unique = self.get_bool_extension(node_id, "unique")?.unwrap_or(false);

        let array_schema = ArraySchema {
            item: item_schema_id,
            min_items,
            max_items,
            unique,
            contains: None, // TODO: handle contains
        };

        let schema_id = self.schema.create_node(SchemaNodeContent::Array(array_schema));

        // Apply metadata
        self.apply_metadata(node_id, schema_id)?;

        Ok(schema_id)
    }

    /// Convert implicit type from node content
    fn convert_implicit_type(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        match &node.content {
            NodeValue::Uninitialized => {
                // Uninitialized with no type extension - could be an empty record or Any
                // If the node is a section (created via @ syntax), treat as empty record
                // We detect this by checking if we came from a section path
                // For now, treat uninitialized as empty record since sections create uninitialized nodes
                self.convert_empty_record(node_id)
            }
            NodeValue::Map(map) => {
                // A map is a record with fields
                self.convert_record(node_id, map)
            }
            NodeValue::Array(array) => {
                // Array shorthand: [.string] means array of string
                self.convert_array_literal(node_id, array)
            }
            NodeValue::Tuple(tuple) => {
                // Tuple shorthand: (.string, .integer) means tuple of string and integer
                self.convert_tuple_literal(node_id, tuple)
            }
            NodeValue::Primitive(prim) => {
                // A primitive value is a literal type
                self.convert_literal_value(node_id, prim)
            }
        }
    }

    /// Convert an uninitialized node to an empty record or cascade type
    fn convert_empty_record(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        // If there's an active cascade-type, use that instead of empty record
        // This applies cascade-type to leaf nodes without explicit types
        if let Some(cascade_type) = self.cascade_type.clone() {
            return self.convert_type_path(node_id, &cascade_type);
        }

        let record_schema = RecordSchema::default();
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Record(record_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    /// Convert a node with $variants extension (variant type definition)
    fn convert_variants_definition(
        &mut self,
        node_id: NodeId,
        variants_node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        let variants_node = self.doc.node(variants_node_id);

        // The $variants node should have map content where each key is a variant name
        // and each value is the variant's schema
        let variants_map = match &variants_node.content {
            NodeValue::Map(map) => map,
            NodeValue::Uninitialized => {
                // Empty variants - check for extensions on the variants node
                // The variants may be defined as sections like @$variants.click { ... }
                // In this case, we need to collect extensions from the variants_node
                let mut variants = HashMap::new();
                for (ext_name, &ext_node_id) in variants_node.extensions.iter() {
                    let variant_schema_id = self.convert_node(ext_node_id)?;
                    variants.insert(ext_name.as_ref().to_string(), variant_schema_id);
                }

                if variants.is_empty() {
                    return Err(ConversionError::InvalidExtensionValue {
                        extension: "variants".to_string(),
                        path: format!("{}", self.current_path),
                    });
                }

                // Get variant repr from node extensions
                let node = self.doc.node(node_id);
                let repr = self.get_variant_repr(node_id)?;

                let variant_schema = VariantSchema { variants, repr };
                let schema_id = self
                    .schema
                    .create_node(SchemaNodeContent::Variant(variant_schema));
                self.apply_metadata(node_id, schema_id)?;
                return Ok(schema_id);
            }
            _ => {
                return Err(ConversionError::InvalidExtensionValue {
                    extension: "variants".to_string(),
                    path: format!("{}", self.current_path),
                })
            }
        };

        // Convert each variant from the map
        let mut variants = HashMap::new();
        for (key, &variant_node_id) in variants_map.iter() {
            let variant_name = match key {
                ObjectKey::String(s) => s.clone(),
                _ => {
                    return Err(ConversionError::UnsupportedConstruct(format!(
                        "Non-string variant name at {}",
                        self.current_path
                    )))
                }
            };

            let variant_schema_id = self.convert_node(variant_node_id)?;
            variants.insert(variant_name, variant_schema_id);
        }

        // Get variant repr from node extensions
        let repr = self.get_variant_repr(node_id)?;

        let variant_schema = VariantSchema { variants, repr };
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Variant(variant_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    /// Get variant representation from $variant-repr extension
    fn get_variant_repr(&self, node_id: NodeId) -> Result<VariantRepr, ConversionError> {
        let node = self.doc.node(node_id);

        if let Some(repr_node_id) = node.get_extension(&Self::ident("variant-repr")) {
            let repr_node = self.doc.node(repr_node_id);

            match &repr_node.content {
                NodeValue::Primitive(PrimitiveValue::String(s)) => {
                    if s == "untagged" {
                        Ok(VariantRepr::Untagged)
                    } else if s == "external" {
                        Ok(VariantRepr::External)
                    } else {
                        Err(ConversionError::InvalidExtensionValue {
                            extension: "variant-repr".to_string(),
                            path: format!("{}", self.current_path),
                        })
                    }
                }
                NodeValue::Map(map) => {
                    // Check for tag and content fields
                    let tag = map
                        .get(&ObjectKey::String("tag".to_string()))
                        .and_then(|id| {
                            let node = self.doc.node(id);
                            if let NodeValue::Primitive(PrimitiveValue::String(s)) = &node.content {
                                Some(s.to_string())
                            } else {
                                None
                            }
                        });

                    let content = map
                        .get(&ObjectKey::String("content".to_string()))
                        .and_then(|id| {
                            let node = self.doc.node(id);
                            if let NodeValue::Primitive(PrimitiveValue::String(s)) = &node.content {
                                Some(s.to_string())
                            } else {
                                None
                            }
                        });

                    match (tag, content) {
                        (Some(tag), Some(content)) => Ok(VariantRepr::Adjacent { tag, content }),
                        (Some(tag), None) => Ok(VariantRepr::Internal { tag }),
                        _ => Err(ConversionError::InvalidExtensionValue {
                            extension: "variant-repr".to_string(),
                            path: format!("{}", self.current_path),
                        }),
                    }
                }
                _ => Err(ConversionError::InvalidExtensionValue {
                    extension: "variant-repr".to_string(),
                    path: format!("{}", self.current_path),
                }),
            }
        } else {
            // Default is external
            Ok(VariantRepr::External)
        }
    }

    /// Convert a map node to a record schema
    fn convert_record(
        &mut self,
        node_id: NodeId,
        map: &eure_value::document::node::NodeMap,
    ) -> Result<SchemaNodeId, ConversionError> {
        let mut properties = HashMap::new();

        for (key, &child_id) in map.iter() {
            let field_name = match key {
                ObjectKey::String(s) => s.clone(),
                _ => {
                    return Err(ConversionError::UnsupportedConstruct(format!(
                        "Non-string key in record at {}",
                        self.current_path
                    )))
                }
            };

            // Skip extension-like keys (starting with $)
            if field_name.starts_with('$') {
                continue;
            }

            // Convert the child node
            let child_schema_id = self.convert_node(child_id)?;
            properties.insert(field_name, child_schema_id);
        }

        // Get unknown fields policy from extensions
        let node = self.doc.node(node_id);
        let unknown_fields = if let Some(unknown_node_id) =
            node.get_extension(&Self::ident("unknown-fields"))
        {
            self.convert_unknown_fields_policy(unknown_node_id)?
        } else {
            UnknownFieldsPolicy::Deny
        };

        let record_schema = RecordSchema {
            properties,
            unknown_fields,
        };

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Record(record_schema));

        // Apply metadata
        self.apply_metadata(node_id, schema_id)?;

        Ok(schema_id)
    }

    /// Convert an array literal to an array schema
    fn convert_array_literal(
        &mut self,
        node_id: NodeId,
        array: &eure_value::document::node::NodeArray,
    ) -> Result<SchemaNodeId, ConversionError> {
        if array.0.is_empty() {
            return Err(ConversionError::InvalidExtensionValue {
                extension: "array".to_string(),
                path: format!("{}", self.current_path),
            });
        }

        // Array shorthand: [.string] - first element is the item type
        let item_schema_id = self.convert_node(array.0[0])?;

        let array_schema = ArraySchema {
            item: item_schema_id,
            min_items: None,
            max_items: None,
            unique: false,
            contains: None,
        };

        let schema_id = self.schema.create_node(SchemaNodeContent::Array(array_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    /// Convert a tuple literal to a tuple schema
    fn convert_tuple_literal(
        &mut self,
        node_id: NodeId,
        tuple: &eure_value::document::node::NodeTuple,
    ) -> Result<SchemaNodeId, ConversionError> {
        let mut items = Vec::new();

        for &child_id in &tuple.0 {
            let child_schema_id = self.convert_node(child_id)?;
            items.push(child_schema_id);
        }

        let tuple_schema = TupleSchema { items };
        let schema_id = self.schema.create_node(SchemaNodeContent::Tuple(tuple_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    /// Convert a primitive value to a literal schema
    fn convert_literal_value(
        &mut self,
        node_id: NodeId,
        prim: &PrimitiveValue,
    ) -> Result<SchemaNodeId, ConversionError> {
        let content = match prim {
            PrimitiveValue::String(s) => {
                // String literal - create a string schema with const value
                SchemaNodeContent::String(StringSchema {
                    r#const: Some(s.to_string()),
                    ..Default::default()
                })
            }
            PrimitiveValue::Bool(b) => SchemaNodeContent::Boolean(BooleanSchema {
                r#const: Some(*b),
            }),
            PrimitiveValue::BigInt(n) => SchemaNodeContent::Integer(IntegerSchema {
                r#const: Some(n.clone()),
                ..Default::default()
            }),
            PrimitiveValue::F64(f) => SchemaNodeContent::Float(FloatSchema {
                r#const: Some(*f),
                ..Default::default()
            }),
            PrimitiveValue::F32(f) => SchemaNodeContent::Float(FloatSchema {
                r#const: Some(*f as f64),
                ..Default::default()
            }),
            PrimitiveValue::Null => SchemaNodeContent::Null,
            PrimitiveValue::Path(path) => {
                // Check if this is a type path shorthand
                return self.convert_type_path(node_id, path);
            }
            PrimitiveValue::Code(_) | PrimitiveValue::Hole | PrimitiveValue::Variant(_) => {
                return Err(ConversionError::UnsupportedConstruct(format!(
                    "Unsupported primitive at {}",
                    self.current_path
                )))
            }
        };

        let schema_id = self.schema.create_node(content);
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    // ==========================================================================
    // Schema creation helpers
    // ==========================================================================

    fn create_string_schema(
        &mut self,
        node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::String(StringSchema::default()));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn create_integer_schema(
        &mut self,
        node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn create_float_schema(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Float(FloatSchema::default()));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn create_boolean_schema(
        &mut self,
        node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        let const_value = self.get_bool_extension(node_id, "const")?;
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Boolean(BooleanSchema {
                r#const: const_value,
            }));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn create_null_schema(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let schema_id = self.schema.create_node(SchemaNodeContent::Null);
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn create_any_schema(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let schema_id = self.schema.create_node(SchemaNodeContent::Any);
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn create_path_schema(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let const_value = self.get_path_extension(node_id, "const")?;
        let schema_id = self.schema.create_node(SchemaNodeContent::Path(PathSchema {
            r#const: const_value,
        }));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn create_code_schema_with_language(
        &mut self,
        node_id: NodeId,
        language: Option<String>,
    ) -> Result<SchemaNodeId, ConversionError> {
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Code(CodeSchema { language }));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    // ==========================================================================
    // Variant type converters
    // ==========================================================================

    fn convert_string_variant(
        &mut self,
        node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        // String variant fields are record fields, not extensions
        let min_length = self.get_integer_field(node_id, "min-length")?;
        let max_length = self.get_integer_field(node_id, "max-length")?;
        let pattern = self.get_string_field(node_id, "pattern")?;
        let format = self.get_string_field(node_id, "format")?;

        let length = match (min_length, max_length) {
            (Some(min), Some(max)) => Some((min, max)),
            (Some(min), None) => Some((min, u32::MAX)),
            (None, Some(max)) => Some((0, max)),
            (None, None) => None,
        };

        let string_schema = StringSchema {
            length,
            pattern,
            format,
            r#const: None,
            r#enum: None,
        };

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::String(string_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_integer_variant(
        &mut self,
        node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        // Integer variant fields are record fields, not extensions
        let range = self.get_string_field(node_id, "range")?;
        let multiple_of = self.get_bigint_field(node_id, "multiple-of")?;

        let (min, max) = if let Some(range_str) = range {
            parse_integer_range(&range_str)?
        } else {
            (crate::Bound::Unbounded, crate::Bound::Unbounded)
        };

        let integer_schema = IntegerSchema {
            min,
            max,
            multiple_of,
            r#const: None,
            r#enum: None,
        };

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Integer(integer_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_float_variant(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        // Float variant fields are record fields, not extensions
        let range = self.get_string_field(node_id, "range")?;

        let (min, max) = if let Some(range_str) = range {
            parse_float_range(&range_str)?
        } else {
            (crate::Bound::Unbounded, crate::Bound::Unbounded)
        };

        let float_schema = FloatSchema {
            min,
            max,
            r#const: None,
            r#enum: None,
        };

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Float(float_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_array_variant(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        // Get the item type from the "item" field
        let item_node_id = self.get_map_child(node_id, "item").ok_or_else(|| {
            ConversionError::MissingRequiredExtension {
                extension: "item".to_string(),
                path: format!("{}", self.current_path),
            }
        })?;

        let item_schema_id = self.convert_node(item_node_id)?;

        let min_items = self.get_integer_field(node_id, "min-length")?;
        let max_items = self.get_integer_field(node_id, "max-length")?;
        let unique = self.get_bool_field(node_id, "unique")?.unwrap_or(false);
        let contains = self.get_primitive_field(node_id, "contains")?;

        let array_schema = ArraySchema {
            item: item_schema_id,
            min_items,
            max_items,
            unique,
            contains,
        };

        let schema_id = self.schema.create_node(SchemaNodeContent::Array(array_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_map_variant(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        // Get the key type from the "key" field
        let key_node_id = self.get_map_child(node_id, "key").ok_or_else(|| {
            ConversionError::MissingRequiredExtension {
                extension: "key".to_string(),
                path: format!("{}", self.current_path),
            }
        })?;

        // Get the value type from the "value" field
        let value_node_id = self.get_map_child(node_id, "value").ok_or_else(|| {
            ConversionError::MissingRequiredExtension {
                extension: "value".to_string(),
                path: format!("{}", self.current_path),
            }
        })?;

        let key_schema_id = self.convert_node(key_node_id)?;
        let value_schema_id = self.convert_node(value_node_id)?;

        let min_pairs = self.get_integer_field(node_id, "min-size")?;
        let max_pairs = self.get_integer_field(node_id, "max-size")?;

        let map_schema = MapSchema {
            key: key_schema_id,
            value: value_schema_id,
            min_pairs,
            max_pairs,
        };

        let schema_id = self.schema.create_node(SchemaNodeContent::Map(map_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_tuple_variant(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        // Get the elements from the "elements" field
        let elements_node_id = self.get_map_child(node_id, "elements").ok_or_else(|| {
            ConversionError::MissingRequiredExtension {
                extension: "elements".to_string(),
                path: format!("{}", self.current_path),
            }
        })?;

        let elements_node = self.doc.node(elements_node_id);
        let array = match &elements_node.content {
            NodeValue::Array(arr) => arr,
            _ => {
                return Err(ConversionError::InvalidExtensionValue {
                    extension: "elements".to_string(),
                    path: format!("{}", self.current_path),
                })
            }
        };

        let mut items = Vec::new();
        for &child_id in &array.0 {
            let child_schema_id = self.convert_node(child_id)?;
            items.push(child_schema_id);
        }

        let tuple_schema = TupleSchema { items };
        let schema_id = self.schema.create_node(SchemaNodeContent::Tuple(tuple_schema));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_union_variant(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let node = self.doc.node(node_id);

        // Get the variants from the "variants" field
        let variants_node_id = self.get_map_child(node_id, "variants").ok_or_else(|| {
            ConversionError::MissingRequiredExtension {
                extension: "variants".to_string(),
                path: format!("{}", self.current_path),
            }
        })?;

        let variants_node = self.doc.node(variants_node_id);
        let variants_map = match &variants_node.content {
            NodeValue::Map(map) => map,
            _ => {
                return Err(ConversionError::InvalidExtensionValue {
                    extension: "variants".to_string(),
                    path: format!("{}", self.current_path),
                })
            }
        };

        // Collect all variant schemas
        let mut variant_schemas = Vec::new();
        for (_, &child_id) in variants_map.iter() {
            let child_schema_id = self.convert_node(child_id)?;
            variant_schemas.push(child_schema_id);
        }

        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Union(variant_schemas));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_literal_variant(
        &mut self,
        node_id: NodeId,
    ) -> Result<SchemaNodeId, ConversionError> {
        // Get the literal value from the "value" field
        let value_node_id = self
            .get_map_child(node_id, "value")
            .ok_or_else(|| ConversionError::MissingRequiredExtension {
                extension: "value".to_string(),
                path: format!("{}", self.current_path),
            })?;

        let value_node = self.doc.node(value_node_id);
        match &value_node.content {
            NodeValue::Primitive(prim) => self.convert_literal_value(value_node_id, prim),
            _ => Err(ConversionError::InvalidExtensionValue {
                extension: "literal".to_string(),
                path: format!("{}", self.current_path),
            }),
        }
    }

    fn convert_path_variant(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        // TODO: Implement path constraints (starts-with, length-min, length-max)
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Path(PathSchema::default()));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    fn convert_code_variant(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        let language = self.get_string_field(node_id, "language")?;
        let schema_id = self
            .schema
            .create_node(SchemaNodeContent::Code(CodeSchema { language }));
        self.apply_metadata(node_id, schema_id)?;
        Ok(schema_id)
    }

    // ==========================================================================
    // Unknown fields policy
    // ==========================================================================

    fn convert_unknown_fields_policy(
        &mut self,
        node_id: NodeId,
    ) -> Result<UnknownFieldsPolicy, ConversionError> {
        let node = self.doc.node(node_id);

        match &node.content {
            NodeValue::Primitive(PrimitiveValue::String(s)) => match s.as_str() {
                "deny" => Ok(UnknownFieldsPolicy::Deny),
                "allow" => Ok(UnknownFieldsPolicy::Allow),
                _ => Err(ConversionError::InvalidExtensionValue {
                    extension: "unknown-fields".to_string(),
                    path: format!("{}", self.current_path),
                }),
            },
            NodeValue::Primitive(PrimitiveValue::Path(_)) => {
                // Type schema for unknown fields
                let schema_id = self.convert_node(node_id)?;
                Ok(UnknownFieldsPolicy::Schema(schema_id))
            }
            _ => Err(ConversionError::InvalidExtensionValue {
                extension: "unknown-fields".to_string(),
                path: format!("{}", self.current_path),
            }),
        }
    }

    // ==========================================================================
    // Metadata handling
    // ==========================================================================

    fn apply_metadata(
        &mut self,
        node_id: NodeId,
        schema_id: SchemaNodeId,
    ) -> Result<(), ConversionError> {
        let node = self.doc.node(node_id);

        let mut metadata = SchemaMetadata::default();

        // Check for $optional
        if let Some(optional_node_id) = node.get_extension(&Self::ident("optional")) {
            let optional_node = self.doc.node(optional_node_id);
            if let NodeValue::Primitive(PrimitiveValue::Bool(b)) = &optional_node.content {
                metadata.optional = *b;
            }
        }

        // Check for $description
        if let Some(desc_node_id) = node.get_extension(&Self::ident("description")) {
            let desc_node = self.doc.node(desc_node_id);
            if let NodeValue::Primitive(PrimitiveValue::String(s)) = &desc_node.content {
                metadata.description = Some(s.to_string());
            }
        }

        // Check for $deprecated
        if let Some(depr_node_id) = node.get_extension(&Self::ident("deprecated")) {
            let depr_node = self.doc.node(depr_node_id);
            if let NodeValue::Primitive(PrimitiveValue::Bool(b)) = &depr_node.content {
                metadata.deprecated = *b;
            }
        }

        // Check for $default
        if let Some(default_node_id) = node.get_extension(&Self::ident("default")) {
            let default_node = self.doc.node(default_node_id);
            if let NodeValue::Primitive(prim) = &default_node.content {
                metadata.default = Some(prim.clone());
            }
        }

        // Check for $prefer.section - section is a map child, not an extension
        if let Some(prefer_node_id) = node.get_extension(&Self::ident("prefer")) {
            if let Some(section_node_id) = self.get_map_child(prefer_node_id, "section") {
                let section_node = self.doc.node(section_node_id);
                if let NodeValue::Primitive(PrimitiveValue::Bool(b)) = &section_node.content {
                    metadata.prefer_section = *b;
                }
            }
        }

        // Check for $rename
        if let Some(rename_node_id) = node.get_extension(&Self::ident("rename")) {
            let rename_node = self.doc.node(rename_node_id);
            if let NodeValue::Primitive(PrimitiveValue::String(s)) = &rename_node.content {
                metadata.rename = Some(s.to_string());
            }
        }

        // Check for $rename-all
        if let Some(rename_all_node_id) = node.get_extension(&Self::ident("rename-all")) {
            let rename_all_node = self.doc.node(rename_all_node_id);
            if let NodeValue::Primitive(PrimitiveValue::String(s)) = &rename_all_node.content {
                metadata.rename_all = Some(s.to_string());
            }
        }

        self.schema.node_mut(schema_id).metadata = metadata;
        Ok(())
    }

    // ==========================================================================
    // Helper methods for getting extensions and fields
    // ==========================================================================

    fn get_integer_extension(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<u32>, ConversionError> {
        let node = self.doc.node(node_id);
        if let Some(ext_node_id) = node.get_extension(&Self::ident(name)) {
            let ext_node = self.doc.node(ext_node_id);
            if let NodeValue::Primitive(PrimitiveValue::BigInt(n)) = &ext_node.content {
                let val: u32 = n
                    .try_into()
                    .map_err(|_| ConversionError::InvalidConstraintValue {
                        constraint: name.to_string(),
                        value: n.to_string(),
                    })?;
                return Ok(Some(val));
            }
        }
        Ok(None)
    }

    fn get_bool_extension(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<bool>, ConversionError> {
        let node = self.doc.node(node_id);
        if let Some(ext_node_id) = node.get_extension(&Self::ident(name)) {
            let ext_node = self.doc.node(ext_node_id);
            if let NodeValue::Primitive(PrimitiveValue::Bool(b)) = &ext_node.content {
                return Ok(Some(*b));
            }
        }
        Ok(None)
    }

    fn get_string_extension(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<String>, ConversionError> {
        let node = self.doc.node(node_id);
        if let Some(ext_node_id) = node.get_extension(&Self::ident(name)) {
            let ext_node = self.doc.node(ext_node_id);
            if let NodeValue::Primitive(PrimitiveValue::String(s)) = &ext_node.content {
                return Ok(Some(s.to_string()));
            }
        }
        Ok(None)
    }

    fn get_bigint_extension(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<num_bigint::BigInt>, ConversionError> {
        let node = self.doc.node(node_id);
        if let Some(ext_node_id) = node.get_extension(&Self::ident(name)) {
            let ext_node = self.doc.node(ext_node_id);
            if let NodeValue::Primitive(PrimitiveValue::BigInt(n)) = &ext_node.content {
                return Ok(Some(n.clone()));
            }
        }
        Ok(None)
    }

    fn get_path_extension(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<EurePath>, ConversionError> {
        let node = self.doc.node(node_id);
        if let Some(ext_node_id) = node.get_extension(&Self::ident(name)) {
            let ext_node = self.doc.node(ext_node_id);
            if let NodeValue::Primitive(PrimitiveValue::Path(p)) = &ext_node.content {
                return Ok(Some(p.clone()));
            }
        }
        Ok(None)
    }

    fn get_map_child(&self, node_id: NodeId, key: &str) -> Option<NodeId> {
        let node = self.doc.node(node_id);
        if let NodeValue::Map(map) = &node.content {
            map.get(&ObjectKey::String(key.to_string()))
        } else {
            None
        }
    }

    fn get_integer_field(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<u32>, ConversionError> {
        if let Some(field_node_id) = self.get_map_child(node_id, name) {
            let field_node = self.doc.node(field_node_id);
            if let NodeValue::Primitive(PrimitiveValue::BigInt(n)) = &field_node.content {
                let val: u32 =
                    n.try_into()
                        .map_err(|_| ConversionError::InvalidConstraintValue {
                            constraint: name.to_string(),
                            value: n.to_string(),
                        })?;
                return Ok(Some(val));
            }
        }
        Ok(None)
    }

    fn get_bool_field(&self, node_id: NodeId, name: &str) -> Result<Option<bool>, ConversionError> {
        if let Some(field_node_id) = self.get_map_child(node_id, name) {
            let field_node = self.doc.node(field_node_id);
            if let NodeValue::Primitive(PrimitiveValue::Bool(b)) = &field_node.content {
                return Ok(Some(*b));
            }
        }
        Ok(None)
    }

    fn get_string_field(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<String>, ConversionError> {
        if let Some(field_node_id) = self.get_map_child(node_id, name) {
            let field_node = self.doc.node(field_node_id);
            if let NodeValue::Primitive(PrimitiveValue::String(s)) = &field_node.content {
                return Ok(Some(s.to_string()));
            }
        }
        Ok(None)
    }

    fn get_bigint_field(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<num_bigint::BigInt>, ConversionError> {
        if let Some(field_node_id) = self.get_map_child(node_id, name) {
            let field_node = self.doc.node(field_node_id);
            if let NodeValue::Primitive(PrimitiveValue::BigInt(n)) = &field_node.content {
                return Ok(Some(n.clone()));
            }
        }
        Ok(None)
    }

    fn get_primitive_field(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<Option<PrimitiveValue>, ConversionError> {
        if let Some(field_node_id) = self.get_map_child(node_id, name) {
            let field_node = self.doc.node(field_node_id);
            if let NodeValue::Primitive(prim) = &field_node.content {
                return Ok(Some(prim.clone()));
            }
        }
        Ok(None)
    }

    // ==========================================================================
    // Type definitions processing
    // ==========================================================================

    fn process_type_definitions(&mut self) -> Result<(), ConversionError> {
        let root = self.doc.root();

        // Look for $types extension on the root
        if let Some(types_node_id) = root.get_extension(&Self::ident("types")) {
            let types_node = self.doc.node(types_node_id);

            if let NodeValue::Map(map) = &types_node.content {
                // First pass: register all type names
                for (key, _) in map.iter() {
                    if let ObjectKey::String(type_name) = key {
                        self.type_definitions.insert(type_name.clone(), None);
                    }
                }

                // Second pass: convert all types
                for (key, &type_node_id) in map.iter() {
                    if let ObjectKey::String(type_name) = key {
                        let schema_id = self.convert_node(type_node_id)?;
                        self.type_definitions
                            .insert(type_name.clone(), Some(schema_id));
                        self.schema
                            .register_type(Self::ident(type_name), schema_id);
                    }
                }
            }
        }

        Ok(())
    }

    // ==========================================================================
    // Main conversion entry point
    // ==========================================================================

    fn convert(mut self) -> Result<SchemaDocument, ConversionError> {
        // First, process type definitions
        self.process_type_definitions()?;

        // Convert the root node
        let root_id = self.doc.get_root_id();
        let root_schema_id = self.convert_node(root_id)?;

        // Set the root
        self.schema.root = root_schema_id;

        Ok(self.schema)
    }
}

// ==========================================================================
// Range parsing helpers
// ==========================================================================

fn parse_integer_range(
    range_str: &str,
) -> Result<(crate::Bound<num_bigint::BigInt>, crate::Bound<num_bigint::BigInt>), ConversionError> {
    // Support both Rust-style and interval notation
    // Rust-style: "0..100", "0..=100", "0..", "..100", "..=100"
    // Interval: "[0, 100]", "[0, 100)", "(0, 100]", "(0, 100)", "[0, )", "(, 100]"

    let range_str = range_str.trim();

    // Check for interval notation
    if range_str.starts_with('[') || range_str.starts_with('(') {
        return parse_interval_integer(range_str);
    }

    // Rust-style range
    parse_rust_style_integer_range(range_str)
}

fn parse_rust_style_integer_range(
    range_str: &str,
) -> Result<(crate::Bound<num_bigint::BigInt>, crate::Bound<num_bigint::BigInt>), ConversionError> {
    let inclusive_end = range_str.contains("..=");
    let parts: Vec<&str> = if inclusive_end {
        range_str.split("..=").collect()
    } else {
        range_str.split("..").collect()
    };

    if parts.len() != 2 {
        return Err(ConversionError::InvalidConstraintValue {
            constraint: "range".to_string(),
            value: range_str.to_string(),
        });
    }

    let min = if parts[0].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: num_bigint::BigInt = parts[0].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        crate::Bound::Inclusive(n)
    };

    let max = if parts[1].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: num_bigint::BigInt = parts[1].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        if inclusive_end {
            crate::Bound::Inclusive(n)
        } else {
            crate::Bound::Exclusive(n)
        }
    };

    Ok((min, max))
}

fn parse_interval_integer(
    range_str: &str,
) -> Result<(crate::Bound<num_bigint::BigInt>, crate::Bound<num_bigint::BigInt>), ConversionError> {
    let chars: Vec<char> = range_str.chars().collect();
    if chars.len() < 3 {
        return Err(ConversionError::InvalidConstraintValue {
            constraint: "range".to_string(),
            value: range_str.to_string(),
        });
    }

    let left_inclusive = chars[0] == '[';
    let right_inclusive = *chars.last().unwrap() == ']';

    // Extract the inner part without brackets
    let inner = &range_str[1..range_str.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

    if parts.len() != 2 {
        return Err(ConversionError::InvalidConstraintValue {
            constraint: "range".to_string(),
            value: range_str.to_string(),
        });
    }

    let min = if parts[0].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: num_bigint::BigInt = parts[0].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        if left_inclusive {
            crate::Bound::Inclusive(n)
        } else {
            crate::Bound::Exclusive(n)
        }
    };

    let max = if parts[1].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: num_bigint::BigInt = parts[1].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        if right_inclusive {
            crate::Bound::Inclusive(n)
        } else {
            crate::Bound::Exclusive(n)
        }
    };

    Ok((min, max))
}

fn parse_float_range(
    range_str: &str,
) -> Result<(crate::Bound<f64>, crate::Bound<f64>), ConversionError> {
    let range_str = range_str.trim();

    // Check for interval notation
    if range_str.starts_with('[') || range_str.starts_with('(') {
        return parse_interval_float(range_str);
    }

    // Rust-style range
    parse_rust_style_float_range(range_str)
}

fn parse_rust_style_float_range(
    range_str: &str,
) -> Result<(crate::Bound<f64>, crate::Bound<f64>), ConversionError> {
    let inclusive_end = range_str.contains("..=");
    let parts: Vec<&str> = if inclusive_end {
        range_str.split("..=").collect()
    } else {
        range_str.split("..").collect()
    };

    if parts.len() != 2 {
        return Err(ConversionError::InvalidConstraintValue {
            constraint: "range".to_string(),
            value: range_str.to_string(),
        });
    }

    let min = if parts[0].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: f64 = parts[0].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        crate::Bound::Inclusive(n)
    };

    let max = if parts[1].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: f64 = parts[1].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        if inclusive_end {
            crate::Bound::Inclusive(n)
        } else {
            crate::Bound::Exclusive(n)
        }
    };

    Ok((min, max))
}

fn parse_interval_float(
    range_str: &str,
) -> Result<(crate::Bound<f64>, crate::Bound<f64>), ConversionError> {
    let chars: Vec<char> = range_str.chars().collect();
    if chars.len() < 3 {
        return Err(ConversionError::InvalidConstraintValue {
            constraint: "range".to_string(),
            value: range_str.to_string(),
        });
    }

    let left_inclusive = chars[0] == '[';
    let right_inclusive = *chars.last().unwrap() == ']';

    // Extract the inner part without brackets
    let inner = &range_str[1..range_str.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

    if parts.len() != 2 {
        return Err(ConversionError::InvalidConstraintValue {
            constraint: "range".to_string(),
            value: range_str.to_string(),
        });
    }

    let min = if parts[0].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: f64 = parts[0].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        if left_inclusive {
            crate::Bound::Inclusive(n)
        } else {
            crate::Bound::Exclusive(n)
        }
    };

    let max = if parts[1].is_empty() {
        crate::Bound::Unbounded
    } else {
        let n: f64 = parts[1].parse().map_err(|_| {
            ConversionError::InvalidConstraintValue {
                constraint: "range".to_string(),
                value: range_str.to_string(),
            }
        })?;
        if right_inclusive {
            crate::Bound::Inclusive(n)
        } else {
            crate::Bound::Exclusive(n)
        }
    };

    Ok((min, max))
}

/// Convert an EureDocument containing schema definitions to a SchemaDocument
///
/// This function traverses the document and extracts schema information from extension fields
/// like $type, $array, $variants, etc.
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
/// name.$type = .string
/// age.$type = .number
/// "#;
///
/// let doc = parse_to_document(input).unwrap();
/// let schema = document_to_schema(&doc).unwrap();
/// ```
pub fn document_to_schema(doc: &EureDocument) -> Result<SchemaDocument, ConversionError> {
    let context = ConversionContext::new(doc);
    context.convert()
}
