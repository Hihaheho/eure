//! Conversion from EureDocument to SchemaDocument
//!
//! This module provides functionality to convert Eure documents containing schema definitions
//! into SchemaDocument structures.
//!
//! # Schema Syntax
//!
//! Schema types are defined using the following syntax:
//!
//! **Primitives (shorthands via inline code):**
//! - `` `text` ``, `` `integer` ``, `` `float` ``, `` `boolean` ``, `` `null` ``, `` `any` ``
//! - `` `text.rust` ``, `` `text.email` ``, `` `text.plaintext` ``
//!
//! **Primitives with constraints:**
//! ```eure
//! @ field {
//!   $variant = "text"
//!   min-length = 3
//!   max-length = 20
//!   pattern = `^[a-z]+$`
//! }
//! ```
//!
//! **Array:** `` [`text`] `` or `` { $variant = "array", item = `text`, ... } ``
//!
//! **Tuple:** `` (`text`, `integer`) `` or `{ $variant = "tuple", elements = [...] }`
//!
//! **Record:** `` { name = `text`, age = `integer` } ``
//!
//! **Union with named variants:**
//! ```eure
//! @ field {
//!   $variant = "union"
//!   variants.success = { data = `any` }
//!   variants.error = { message = `text` }
//!   variants.error.$ext-type.unambiguous = true  // optional, for catch-all variants
//!   $variant-repr = "untagged"  // optional
//! }
//! ```
//!
//! **Literal:** Any constant value (e.g., `{ = "active", $variant = "literal" }`, `42`, `true`)
//!
//! **Type reference:** `` `$types.my-type` `` or `` `$types.namespace.type` ``

use crate::parse::{
    ParsedArraySchema, ParsedExtTypeSchema, ParsedFloatSchema, ParsedIntegerSchema,
    ParsedMapSchema, ParsedRecordSchema, ParsedSchemaMetadata, ParsedSchemaNode,
    ParsedSchemaNodeContent, ParsedTupleSchema, ParsedUnionSchema, ParsedUnknownFieldsPolicy,
};
use crate::{
    ArraySchema, Bound, ExtTypeSchema, FloatPrecision, FloatSchema, IntegerSchema, MapSchema,
    RecordFieldSchema, RecordSchema, SchemaDocument, SchemaMetadata, SchemaNodeContent,
    SchemaNodeId, TupleSchema, UnionSchema, UnknownFieldsPolicy,
};
use eure_document::document::node::{Node, NodeValue};
use eure_document::document::{EureDocument, NodeId};
use eure_document::identifier::Identifier;
use eure_document::parse::ParseError;
use eure_document::value::ObjectKey;
use indexmap::IndexMap;
use num_bigint::BigInt;
use thiserror::Error;

/// Errors that can occur during document to schema conversion
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConversionError {
    #[error("Invalid type name: {0}")]
    InvalidTypeName(ObjectKey),

    #[error("Unsupported schema construct at path: {0}")]
    UnsupportedConstruct(String),

    #[error("Invalid extension value: {extension} at path {path}")]
    InvalidExtensionValue { extension: String, path: String },

    #[error("Invalid range string: {0}")]
    InvalidRangeString(String),

    #[error("Invalid precision: {0} (expected \"f32\" or \"f64\")")]
    InvalidPrecision(String),

    #[error("Undefined type reference: {0}")]
    UndefinedTypeReference(String),

    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),
}

/// Mapping from schema node IDs to their source document node IDs.
/// Used for propagating origin information for error formatting.
pub type SchemaSourceMap = IndexMap<SchemaNodeId, NodeId>;

/// Internal converter state
struct Converter<'a> {
    doc: &'a EureDocument,
    schema: SchemaDocument,
    /// Track source document NodeId for each schema node
    source_map: SchemaSourceMap,
}

impl<'a> Converter<'a> {
    fn new(doc: &'a EureDocument) -> Self {
        Self {
            doc,
            schema: SchemaDocument::new(),
            source_map: IndexMap::new(),
        }
    }

    /// Convert the root node and produce the final schema with source mapping
    fn convert(mut self) -> Result<(SchemaDocument, SchemaSourceMap), ConversionError> {
        let root_id = self.doc.get_root_id();
        let root_node = self.doc.node(root_id);

        // Convert all type definitions from $types extension
        self.convert_types(root_node)?;

        // Convert root node
        self.schema.root = self.convert_node(root_id)?;

        // Validate all type references exist
        self.validate_type_references()?;

        Ok((self.schema, self.source_map))
    }

    /// Convert all type definitions from $types extension
    fn convert_types(&mut self, node: &Node) -> Result<(), ConversionError> {
        let types_ident: Identifier = "types".parse().unwrap();
        if let Some(types_node_id) = node.extensions.get(&types_ident) {
            let types_node = self.doc.node(*types_node_id);
            if let NodeValue::Map(map) = &types_node.content {
                for (key, &node_id) in map.iter() {
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

    /// Convert a document node to a schema node using FromEure trait
    fn convert_node(&mut self, node_id: NodeId) -> Result<SchemaNodeId, ConversionError> {
        // Parse the node using FromEure trait
        let parsed: ParsedSchemaNode = self.doc.parse(node_id)?;

        // Convert the parsed node to final schema
        let content = self.convert_content(parsed.content)?;
        let metadata = self.convert_metadata(parsed.metadata)?;
        let ext_types = self.convert_ext_types(parsed.ext_types)?;

        // Create the final schema node
        let schema_id = self.schema.create_node(content);
        self.schema.node_mut(schema_id).metadata = metadata;
        self.schema.node_mut(schema_id).ext_types = ext_types;

        // Record source mapping for span resolution
        self.source_map.insert(schema_id, node_id);
        Ok(schema_id)
    }

    /// Convert parsed schema node content to final schema node content
    fn convert_content(
        &mut self,
        content: ParsedSchemaNodeContent,
    ) -> Result<SchemaNodeContent, ConversionError> {
        match content {
            ParsedSchemaNodeContent::Any => Ok(SchemaNodeContent::Any),
            ParsedSchemaNodeContent::Boolean => Ok(SchemaNodeContent::Boolean),
            ParsedSchemaNodeContent::Null => Ok(SchemaNodeContent::Null),
            ParsedSchemaNodeContent::Text(schema) => Ok(SchemaNodeContent::Text(schema)),
            ParsedSchemaNodeContent::Reference(type_ref) => {
                Ok(SchemaNodeContent::Reference(type_ref))
            }

            ParsedSchemaNodeContent::Integer(parsed) => Ok(SchemaNodeContent::Integer(
                self.convert_integer_schema(parsed)?,
            )),
            ParsedSchemaNodeContent::Float(parsed) => {
                Ok(SchemaNodeContent::Float(self.convert_float_schema(parsed)?))
            }
            ParsedSchemaNodeContent::Literal(node_id) => {
                Ok(SchemaNodeContent::Literal(self.node_to_document(node_id)?))
            }
            ParsedSchemaNodeContent::Array(parsed) => {
                Ok(SchemaNodeContent::Array(self.convert_array_schema(parsed)?))
            }
            ParsedSchemaNodeContent::Map(parsed) => {
                Ok(SchemaNodeContent::Map(self.convert_map_schema(parsed)?))
            }
            ParsedSchemaNodeContent::Record(parsed) => Ok(SchemaNodeContent::Record(
                self.convert_record_schema(parsed)?,
            )),
            ParsedSchemaNodeContent::Tuple(parsed) => {
                Ok(SchemaNodeContent::Tuple(self.convert_tuple_schema(parsed)?))
            }
            ParsedSchemaNodeContent::Union(parsed) => {
                Ok(SchemaNodeContent::Union(self.convert_union_schema(parsed)?))
            }
        }
    }

    /// Convert parsed integer schema (with range string) to final integer schema (with Bound)
    fn convert_integer_schema(
        &self,
        parsed: ParsedIntegerSchema,
    ) -> Result<IntegerSchema, ConversionError> {
        let (min, max) = if let Some(range_str) = &parsed.range {
            parse_integer_range(range_str)?
        } else {
            (Bound::Unbounded, Bound::Unbounded)
        };

        Ok(IntegerSchema {
            min,
            max,
            multiple_of: parsed.multiple_of,
        })
    }

    /// Convert parsed float schema (with range string) to final float schema (with Bound)
    fn convert_float_schema(
        &self,
        parsed: ParsedFloatSchema,
    ) -> Result<FloatSchema, ConversionError> {
        let (min, max) = if let Some(range_str) = &parsed.range {
            parse_float_range(range_str)?
        } else {
            (Bound::Unbounded, Bound::Unbounded)
        };

        let precision = match parsed.precision.as_deref() {
            Some("f32") => FloatPrecision::F32,
            Some("f64") | None => FloatPrecision::F64,
            Some(other) => {
                return Err(ConversionError::InvalidPrecision(other.to_string()));
            }
        };

        Ok(FloatSchema {
            min,
            max,
            multiple_of: parsed.multiple_of,
            precision,
        })
    }

    /// Convert parsed array schema to final array schema
    fn convert_array_schema(
        &mut self,
        parsed: ParsedArraySchema,
    ) -> Result<ArraySchema, ConversionError> {
        let item = self.convert_node(parsed.item)?;
        let contains = parsed
            .contains
            .map(|id| self.convert_node(id))
            .transpose()?;

        Ok(ArraySchema {
            item,
            min_length: parsed.min_length,
            max_length: parsed.max_length,
            unique: parsed.unique,
            contains,
            binding_style: parsed.binding_style,
        })
    }

    /// Convert parsed map schema to final map schema
    fn convert_map_schema(
        &mut self,
        parsed: ParsedMapSchema,
    ) -> Result<MapSchema, ConversionError> {
        let key = self.convert_node(parsed.key)?;
        let value = self.convert_node(parsed.value)?;

        Ok(MapSchema {
            key,
            value,
            min_size: parsed.min_size,
            max_size: parsed.max_size,
        })
    }

    /// Convert parsed tuple schema to final tuple schema
    fn convert_tuple_schema(
        &mut self,
        parsed: ParsedTupleSchema,
    ) -> Result<TupleSchema, ConversionError> {
        let elements: Vec<SchemaNodeId> = parsed
            .elements
            .iter()
            .map(|&id| self.convert_node(id))
            .collect::<Result<_, _>>()?;

        Ok(TupleSchema {
            elements,
            binding_style: parsed.binding_style,
        })
    }

    /// Convert parsed record schema to final record schema
    fn convert_record_schema(
        &mut self,
        parsed: ParsedRecordSchema,
    ) -> Result<RecordSchema, ConversionError> {
        let mut properties = IndexMap::new();

        for (field_name, field_parsed) in parsed.properties {
            let schema = self.convert_node(field_parsed.schema)?;
            properties.insert(
                field_name,
                RecordFieldSchema {
                    schema,
                    optional: field_parsed.optional,
                    binding_style: field_parsed.binding_style,
                },
            );
        }

        // Convert flatten targets
        let flatten = parsed
            .flatten
            .into_iter()
            .map(|id| self.convert_node(id))
            .collect::<Result<Vec<_>, _>>()?;

        let unknown_fields = self.convert_unknown_fields_policy(parsed.unknown_fields)?;

        Ok(RecordSchema {
            properties,
            flatten,
            unknown_fields,
        })
    }

    /// Convert parsed union schema to final union schema
    fn convert_union_schema(
        &mut self,
        parsed: ParsedUnionSchema,
    ) -> Result<UnionSchema, ConversionError> {
        let mut variants = IndexMap::new();

        for (variant_name, variant_node_id) in parsed.variants {
            let schema = self.convert_node(variant_node_id)?;
            variants.insert(variant_name, schema);
        }

        Ok(UnionSchema {
            variants,
            unambiguous: parsed.unambiguous,
            repr: parsed.repr,
            deny_untagged: parsed.deny_untagged,
        })
    }

    /// Convert parsed unknown fields policy to final policy
    fn convert_unknown_fields_policy(
        &mut self,
        parsed: ParsedUnknownFieldsPolicy,
    ) -> Result<UnknownFieldsPolicy, ConversionError> {
        match parsed {
            ParsedUnknownFieldsPolicy::Deny => Ok(UnknownFieldsPolicy::Deny),
            ParsedUnknownFieldsPolicy::Allow => Ok(UnknownFieldsPolicy::Allow),
            ParsedUnknownFieldsPolicy::Schema(node_id) => {
                let schema = self.convert_node(node_id)?;
                Ok(UnknownFieldsPolicy::Schema(schema))
            }
        }
    }

    /// Convert parsed metadata to final metadata
    fn convert_metadata(
        &mut self,
        parsed: ParsedSchemaMetadata,
    ) -> Result<SchemaMetadata, ConversionError> {
        let default = parsed
            .default
            .map(|id| self.node_to_document(id))
            .transpose()?;

        let examples = parsed
            .examples
            .map(|ids| {
                ids.into_iter()
                    .map(|id| self.node_to_document(id))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;

        Ok(SchemaMetadata {
            description: parsed.description,
            deprecated: parsed.deprecated,
            default,
            examples,
        })
    }

    /// Convert parsed ext types to final ext types
    fn convert_ext_types(
        &mut self,
        parsed: IndexMap<Identifier, ParsedExtTypeSchema>,
    ) -> Result<IndexMap<Identifier, ExtTypeSchema>, ConversionError> {
        let mut result = IndexMap::new();

        for (name, parsed_schema) in parsed {
            let schema = self.convert_node(parsed_schema.schema)?;
            result.insert(
                name,
                ExtTypeSchema {
                    schema,
                    optional: parsed_schema.optional,
                },
            );
        }

        Ok(result)
    }

    /// Extract a subtree as a new EureDocument for literal types
    fn node_to_document(&self, node_id: NodeId) -> Result<EureDocument, ConversionError> {
        let mut new_doc = EureDocument::new();
        let root_id = new_doc.get_root_id();
        self.copy_node_to(&mut new_doc, root_id, node_id)?;
        Ok(new_doc)
    }

    /// Recursively copy a node from source document to destination
    fn copy_node_to(
        &self,
        dest: &mut EureDocument,
        dest_node_id: NodeId,
        src_node_id: NodeId,
    ) -> Result<(), ConversionError> {
        let src_node = self.doc.node(src_node_id);

        // Collect child info before mutating dest
        let children_to_copy: Vec<_> = match &src_node.content {
            NodeValue::Primitive(prim) => {
                dest.set_content(dest_node_id, NodeValue::Primitive(prim.clone()));
                vec![]
            }
            NodeValue::Array(arr) => {
                dest.set_content(dest_node_id, NodeValue::empty_array());
                arr.to_vec()
            }
            NodeValue::Tuple(tup) => {
                dest.set_content(dest_node_id, NodeValue::empty_tuple());
                tup.to_vec()
            }
            NodeValue::Map(map) => {
                dest.set_content(dest_node_id, NodeValue::empty_map());
                map.iter()
                    .map(|(k, &v)| (k.clone(), v))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|(_, v)| v)
                    .collect()
            }
            NodeValue::Hole(_) => {
                return Err(ConversionError::UnsupportedConstruct(
                    "Hole node".to_string(),
                ));
            }
        };

        // Skip ALL extensions during literal value copying.
        // Extensions are schema metadata (like $variant, $deny-untagged, $optional, etc.)
        // and should not be part of the literal value comparison.
        // Literal types compare only the data structure, not metadata.

        // Now copy children based on the type
        let src_node = self.doc.node(src_node_id);
        match &src_node.content {
            NodeValue::Array(_) => {
                for child_id in children_to_copy {
                    let new_child_id = dest
                        .add_array_element(None, dest_node_id)
                        .map_err(|e| ConversionError::UnsupportedConstruct(e.to_string()))?
                        .node_id;
                    self.copy_node_to(dest, new_child_id, child_id)?;
                }
            }
            NodeValue::Tuple(_) => {
                for (index, child_id) in children_to_copy.into_iter().enumerate() {
                    let new_child_id = dest
                        .add_tuple_element(index as u8, dest_node_id)
                        .map_err(|e| ConversionError::UnsupportedConstruct(e.to_string()))?
                        .node_id;
                    self.copy_node_to(dest, new_child_id, child_id)?;
                }
            }
            NodeValue::Map(map) => {
                for (key, &child_id) in map.iter() {
                    let new_child_id = dest
                        .add_map_child(key.clone(), dest_node_id)
                        .map_err(|e| ConversionError::UnsupportedConstruct(e.to_string()))?
                        .node_id;
                    self.copy_node_to(dest, new_child_id, child_id)?;
                }
            }
            _ => {}
        }

        Ok(())
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
/// A tuple of (SchemaDocument, SchemaSourceMap) on success, or a ConversionError on failure.
/// The SchemaSourceMap maps each schema node ID to its source document node ID, which can be
/// used for propagating origin information for error formatting.
///
/// # Examples
///
/// ```ignore
/// use eure::parse_to_document;
/// use eure_schema::convert::document_to_schema;
///
/// let input = r#"
/// name = `text`
/// age = `integer`
/// "#;
///
/// let doc = parse_to_document(input).unwrap();
/// let (schema, source_map) = document_to_schema(&doc).unwrap();
/// ```
pub fn document_to_schema(
    doc: &EureDocument,
) -> Result<(SchemaDocument, SchemaSourceMap), ConversionError> {
    Converter::new(doc).convert()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::{EXT_TYPE, OPTIONAL};
    use eure_document::document::node::NodeMap;
    use eure_document::eure;
    use eure_document::text::Text;
    use eure_document::value::PrimitiveValue;

    /// Create a document with a record containing a single field with $ext-type extension
    fn create_schema_with_field_ext_type(ext_type_content: NodeValue) -> EureDocument {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Create field value: `text`
        let field_value_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::inline_implicit("text"),
        )));

        // Add $ext-type extension to the field
        let ext_type_id = doc.create_node(ext_type_content);
        doc.node_mut(field_value_id)
            .extensions
            .insert(EXT_TYPE.clone(), ext_type_id);

        // Create root as record with field: { name = `text` }
        let mut root_map = NodeMap::default();
        root_map.insert(ObjectKey::String("name".to_string()), field_value_id);
        doc.node_mut(root_id).content = NodeValue::Map(root_map);

        doc
    }

    #[test]
    fn extract_ext_types_not_map() {
        // name.$ext-type = 1 should error, not silently ignore
        // The new parser catches this during parse_record() which expects a map
        let doc = create_schema_with_field_ext_type(NodeValue::Primitive(PrimitiveValue::Integer(
            1.into(),
        )));

        let err = document_to_schema(&doc).unwrap_err();
        use eure_document::parse::ParseErrorKind;
        use eure_document::value::ValueKind;
        assert_eq!(
            err,
            ConversionError::ParseError(ParseError {
                node_id: NodeId(2),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::Map,
                    actual: ValueKind::Integer,
                }
            })
        );
    }

    #[test]
    fn extract_ext_types_invalid_key() {
        // name.$ext-type = { 0 => `text` } should error, not silently ignore
        // The parser catches this during parse_ext_types() -> unknown_fields()
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Create field value: `text`
        let field_value_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::inline_implicit("text"),
        )));

        // Create $ext-type as map with integer key
        // The value's node_id is returned in the error since that's the entry with invalid key
        let ext_type_value_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::inline_implicit("text"),
        )));
        let mut ext_type_map = NodeMap::default();
        ext_type_map.insert(ObjectKey::Number(0.into()), ext_type_value_id);

        let ext_type_id = doc.create_node(NodeValue::Map(ext_type_map));
        doc.node_mut(field_value_id)
            .extensions
            .insert(EXT_TYPE.clone(), ext_type_id);

        // Create root as record
        let mut root_map = NodeMap::default();
        root_map.insert(ObjectKey::String("name".to_string()), field_value_id);
        doc.node_mut(root_id).content = NodeValue::Map(root_map);

        let err = document_to_schema(&doc).unwrap_err();
        use eure_document::parse::ParseErrorKind;
        assert_eq!(
            err,
            ConversionError::ParseError(ParseError {
                // The error points to the value's node_id (the entry with invalid key)
                node_id: ext_type_value_id,
                kind: ParseErrorKind::InvalidKeyType(ObjectKey::Number(0.into()))
            })
        );
    }

    #[test]
    fn extract_ext_types_invalid_optional() {
        // name.$ext-type.desc.$optional = 1 should error, not silently default to false
        // The new parser catches this during field_optional::<bool>() parsing
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Create field value: `text`
        let field_value_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::inline_implicit("text"),
        )));

        // Create ext-type value with invalid $optional = 1
        let ext_type_value_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::inline_implicit("text"),
        )));
        let optional_node_id =
            doc.create_node(NodeValue::Primitive(PrimitiveValue::Integer(1.into())));
        doc.node_mut(ext_type_value_id)
            .extensions
            .insert(OPTIONAL.clone(), optional_node_id);

        // Create $ext-type map
        let mut ext_type_map = NodeMap::default();
        ext_type_map.insert(ObjectKey::String("desc".to_string()), ext_type_value_id);

        let ext_type_id = doc.create_node(NodeValue::Map(ext_type_map));
        doc.node_mut(field_value_id)
            .extensions
            .insert(EXT_TYPE.clone(), ext_type_id);

        // Create root as record
        let mut root_map = NodeMap::default();
        root_map.insert(ObjectKey::String("name".to_string()), field_value_id);
        doc.node_mut(root_id).content = NodeValue::Map(root_map);

        let err = document_to_schema(&doc).unwrap_err();
        use eure_document::parse::ParseErrorKind;
        use eure_document::value::ValueKind;
        assert_eq!(
            err,
            ConversionError::ParseError(ParseError {
                node_id: NodeId(3),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::Bool,
                    actual: ValueKind::Integer,
                }
            })
        );
    }

    #[test]
    fn literal_variant_with_inline_code() {
        // Test: { = `any`, $variant => "literal" } should create Literal(Text("any"))
        // NOT Any (which would happen if $variant is not detected)
        // Note: { = value, $ext => ... } is represented in document model as just the value with extensions
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Create the $variant extension value: "literal"
        let variant_value_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext("literal"),
        )));

        // Set root content to the inline code value directly: `any`
        // (not wrapped in a map, since { = value } unwraps to just value)
        doc.node_mut(root_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::inline_implicit("any")));

        // Add $variant extension
        doc.node_mut(root_id)
            .extensions
            .insert("variant".parse().unwrap(), variant_value_id);

        let (schema, _source_map) =
            document_to_schema(&doc).expect("Schema conversion should succeed");

        // The root should be a Literal, not Any
        let root_content = &schema.node(schema.root).content;
        match root_content {
            SchemaNodeContent::Literal(doc) => {
                // The value should be Text("any")
                match &doc.root().content {
                    NodeValue::Primitive(PrimitiveValue::Text(t)) => {
                        assert_eq!(t.as_str(), "any", "Literal should contain 'any'");
                    }
                    _ => panic!("Expected Literal with Text primitive, got {:?}", doc),
                }
            }
            SchemaNodeContent::Any => {
                panic!("BUG: Got Any instead of Literal - $variant extension not detected!");
            }
            other => panic!("Expected Literal, got {:?}", other),
        }
    }

    #[test]
    fn literal_variant_parsed_from_eure() {
        let doc = eure!({
            = @code("any")
            %variant = "literal"
        });

        let (schema, _source_map) =
            document_to_schema(&doc).expect("Schema conversion should succeed");

        let root_content = &schema.node(schema.root).content;
        match root_content {
            SchemaNodeContent::Literal(doc) => match &doc.root().content {
                NodeValue::Primitive(PrimitiveValue::Text(t)) => {
                    assert_eq!(t.as_str(), "any", "Literal should contain 'any'");
                }
                _ => panic!("Expected Literal with Text primitive, got {:?}", doc),
            },
            SchemaNodeContent::Any => {
                panic!(
                    "BUG: Got Any instead of Literal - $variant extension not respected for primitive"
                );
            }
            other => panic!("Expected Literal, got {:?}", other),
        }
    }

    #[test]
    fn union_with_literal_any_variant() {
        // Test a union like $types.type which has variants including:
        // @variants.any = { = `any`, $variant => "literal" }
        // @variants.literal = `any`
        // The 'any' variant should match only literal "any", not any value.
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Create the 'any' variant value: { = `any`, $variant => "literal" }
        // Note: { = value, $ext => ... } unwraps to just the value with extensions
        let any_variant_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::inline_implicit("any"),
        )));
        // Add $variant => "literal" extension
        let literal_ext = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext("literal"),
        )));
        doc.node_mut(any_variant_node)
            .extensions
            .insert("variant".parse().unwrap(), literal_ext);

        // Create the 'literal' variant value: `any` (type Any)
        let literal_variant_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::inline_implicit("any"),
        )));

        // Create the variants map
        let mut variants_map = NodeMap::default();
        variants_map.insert(ObjectKey::String("any".to_string()), any_variant_node);
        variants_map.insert(
            ObjectKey::String("literal".to_string()),
            literal_variant_node,
        );
        let variants_node = doc.create_node(NodeValue::Map(variants_map));

        // Create root as union
        let union_ext = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext("union"),
        )));
        let untagged_ext = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext("untagged"),
        )));

        // Create root map with variants
        let mut root_map = NodeMap::default();
        root_map.insert(ObjectKey::String("variants".to_string()), variants_node);

        doc.node_mut(root_id).content = NodeValue::Map(root_map);
        doc.node_mut(root_id)
            .extensions
            .insert("variant".parse().unwrap(), union_ext);
        doc.node_mut(root_id)
            .extensions
            .insert("variant-repr".parse().unwrap(), untagged_ext);

        let (schema, _source_map) =
            document_to_schema(&doc).expect("Schema conversion should succeed");

        // Check the union schema
        let root_content = &schema.node(schema.root).content;
        match root_content {
            SchemaNodeContent::Union(union_schema) => {
                // Check 'any' variant is Literal("any"), not Any
                let any_variant_id = union_schema
                    .variants
                    .get("any")
                    .expect("'any' variant missing");
                let any_content = &schema.node(*any_variant_id).content;
                match any_content {
                    SchemaNodeContent::Literal(doc) => match &doc.root().content {
                        NodeValue::Primitive(PrimitiveValue::Text(t)) => {
                            assert_eq!(
                                t.as_str(),
                                "any",
                                "'any' variant should be Literal(\"any\")"
                            );
                        }
                        _ => panic!("'any' variant: expected Text, got {:?}", doc),
                    },
                    SchemaNodeContent::Any => {
                        panic!(
                            "BUG: 'any' variant is Any instead of Literal(\"any\") - $variant extension not detected!"
                        );
                    }
                    other => panic!("'any' variant: expected Literal, got {:?}", other),
                }

                // Check 'literal' variant is Any
                let literal_variant_id = union_schema
                    .variants
                    .get("literal")
                    .expect("'literal' variant missing");
                let literal_content = &schema.node(*literal_variant_id).content;
                match literal_content {
                    SchemaNodeContent::Any => {
                        // Correct: 'literal' variant should be Any
                    }
                    other => panic!("'literal' variant: expected Any, got {:?}", other),
                }
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }
}
