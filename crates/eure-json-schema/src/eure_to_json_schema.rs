//! Conversion from EURE Schema to JSON Schema (Draft-07)
//!
//! This module provides functionality to convert EURE Schema documents to JSON Schema format.
//! Since EURE Schema is a superset of JSON Schema with additional features, some constructs
//! cannot be represented in JSON Schema and will result in conversion errors.

use crate::json_schema::*;
use eure_schema::{
    ArraySchema as EureArraySchema, BooleanSchema as EureBooleanSchema, Bound,
    FloatSchema, IntegerSchema as EureIntegerSchema, MapSchema,
    RecordSchema, SchemaDocument, SchemaMetadata as EureMetadata, SchemaNode, SchemaNodeContent,
    SchemaNodeId, StringSchema as EureStringSchema, TupleSchema, UnknownFieldsPolicy,
    VariantSchema,
};
use eure_value::data_model::VariantRepr;
use indexmap::IndexMap;
use num_traits::ToPrimitive;

/// Errors that can occur during EURE Schema to JSON Schema conversion
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    /// EURE Code type cannot be represented in JSON Schema
    CodeTypeNotSupported,

    /// EURE Path type cannot be represented in JSON Schema
    PathTypeNotSupported,

    /// EURE Map type with non-string keys cannot be represented in JSON Schema
    /// JSON Schema only supports string keys in objects
    NonStringMapKeysNotSupported,

    /// BigInt value is too large to fit in i64 for JSON Schema
    BigIntOutOfRange(String),

    /// Float value (NaN or Infinity) cannot be represented in JSON Schema
    InvalidFloatValue(String),

    /// Invalid schema node reference
    InvalidNodeReference(usize),

    /// Circular reference detected (not supported in JSON Schema)
    CircularReference(String),

    /// EURE's untagged variant representation may be ambiguous in JSON Schema
    UntaggedVariantAmbiguous { variant_name: String },

    /// Contains constraint with non-primitive value not supported
    ComplexContainsNotSupported,

    /// Tuple with more constraints than JSON Schema array tuple validation supports
    TupleConstraintsNotSupported,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::CodeTypeNotSupported => {
                write!(f, "EURE Code type is not supported in JSON Schema")
            }
            ConversionError::PathTypeNotSupported => {
                write!(f, "EURE Path type is not supported in JSON Schema")
            }
            ConversionError::NonStringMapKeysNotSupported => {
                write!(
                    f,
                    "EURE Map type with non-string keys cannot be represented in JSON Schema"
                )
            }
            ConversionError::BigIntOutOfRange(val) => {
                write!(f, "BigInt value {} is out of range for JSON Schema i64", val)
            }
            ConversionError::InvalidFloatValue(val) => {
                write!(f, "Invalid float value: {}", val)
            }
            ConversionError::InvalidNodeReference(id) => {
                write!(f, "Invalid schema node reference: {}", id)
            }
            ConversionError::CircularReference(path) => {
                write!(f, "Circular reference detected: {}", path)
            }
            ConversionError::UntaggedVariantAmbiguous { variant_name } => {
                write!(
                    f,
                    "Untagged variant '{}' may be ambiguous in JSON Schema",
                    variant_name
                )
            }
            ConversionError::ComplexContainsNotSupported => {
                write!(
                    f,
                    "Array contains constraint with complex values not supported in JSON Schema"
                )
            }
            ConversionError::TupleConstraintsNotSupported => {
                write!(
                    f,
                    "EURE Tuple constraints are not fully supported in JSON Schema"
                )
            }
        }
    }
}

impl std::error::Error for ConversionError {}

/// Conversion context to track state during conversion
struct ConversionContext<'a> {
    /// The source EURE schema document
    document: &'a SchemaDocument,
    /// Track visited nodes to detect circular references
    visiting: Vec<SchemaNodeId>,
}

impl<'a> ConversionContext<'a> {
    fn new(document: &'a SchemaDocument) -> Self {
        Self {
            document,
            visiting: Vec::new(),
        }
    }

    /// Get a node from the document
    fn get_node(&self, id: SchemaNodeId) -> Result<&SchemaNode, ConversionError> {
        self.document
            .nodes
            .get(id.0)
            .ok_or(ConversionError::InvalidNodeReference(id.0))
    }

    /// Mark a node as being visited (for cycle detection)
    fn push_visiting(&mut self, id: SchemaNodeId) -> Result<(), ConversionError> {
        if self.visiting.contains(&id) {
            return Err(ConversionError::CircularReference(format!(
                "Node {} creates a cycle",
                id.0
            )));
        }
        self.visiting.push(id);
        Ok(())
    }

    /// Unmark a node as being visited
    fn pop_visiting(&mut self) {
        self.visiting.pop();
    }
}

/// Convert an EURE SchemaDocument to JSON Schema
///
/// The root schema will be converted, along with all referenced type definitions
/// which will be placed in the `$defs` section of the JSON Schema.
pub fn eure_to_json_schema(doc: &SchemaDocument) -> Result<JsonSchema, ConversionError> {
    let mut ctx = ConversionContext::new(doc);

    // Convert the root schema
    let root_schema = convert_node(&mut ctx, doc.root)?;

    // If there are named types, we need to wrap in a GenericSchema with $defs
    if !doc.types.is_empty() {
        let mut defs = IndexMap::new();

        for (name, node_id) in &doc.types {
            let converted = convert_node(&mut ctx, *node_id)?;
            defs.insert(name.to_string(), converted);
        }

        // Wrap the root schema with definitions
        Ok(wrap_with_definitions(root_schema, defs))
    } else {
        Ok(root_schema)
    }
}

/// Wrap a schema with $defs
fn wrap_with_definitions(
    root: JsonSchema,
    defs: IndexMap<String, JsonSchema>,
) -> JsonSchema {
    // If root is already a Generic schema, we can add defs to it
    if let JsonSchema::Generic(mut generic) = root {
        generic.defs = Some(defs);
        JsonSchema::Generic(generic)
    } else {
        // Otherwise, use allOf to combine root with a schema containing defs
        JsonSchema::AllOf(AllOfSchema {
            schemas: vec![
                root,
                JsonSchema::Generic(GenericSchema {
                    defs: Some(defs),
                    ..Default::default()
                }),
            ],
            metadata: SchemaMetadata::default(),
        })
    }
}

/// Convert a single schema node to JSON Schema
fn convert_node(ctx: &mut ConversionContext, id: SchemaNodeId) -> Result<JsonSchema, ConversionError> {
    ctx.push_visiting(id)?;

    // Clone the content and metadata to avoid borrow checker issues
    let node = ctx.get_node(id)?;
    let content = node.content.clone();
    let metadata = node.metadata.clone();

    let result = convert_schema_content(ctx, &content, &metadata)?;

    ctx.pop_visiting();
    Ok(result)
}

/// Convert schema content with metadata
fn convert_schema_content(
    ctx: &mut ConversionContext,
    content: &SchemaNodeContent,
    metadata: &EureMetadata,
) -> Result<JsonSchema, ConversionError> {
    let json_metadata = convert_metadata(metadata);

    match content {
        SchemaNodeContent::Any => Ok(JsonSchema::Generic(GenericSchema {
            metadata: json_metadata,
            ..Default::default()
        })),

        SchemaNodeContent::String(s) => convert_string_schema(s, json_metadata),

        SchemaNodeContent::Code(_) => Err(ConversionError::CodeTypeNotSupported),

        SchemaNodeContent::Integer(i) => convert_integer_schema(i, json_metadata),

        SchemaNodeContent::Float(f) => convert_float_schema(f, json_metadata),

        SchemaNodeContent::Boolean(b) => convert_boolean_schema(b, json_metadata),

        SchemaNodeContent::Null => Ok(JsonSchema::Typed(TypedSchema::Null(NullSchema {
            metadata: json_metadata,
        }))),

        SchemaNodeContent::Path(_) => Err(ConversionError::PathTypeNotSupported),

        SchemaNodeContent::Array(a) => convert_array_schema(ctx, a, json_metadata),

        SchemaNodeContent::Map(m) => convert_map_schema(ctx, m, json_metadata),

        SchemaNodeContent::Record(r) => convert_record_schema(ctx, r, json_metadata),

        SchemaNodeContent::Tuple(t) => convert_tuple_schema(ctx, t, json_metadata),

        SchemaNodeContent::Union(variants) => convert_union_schema(ctx, variants, json_metadata),

        SchemaNodeContent::Variant(v) => convert_variant_schema(ctx, v, json_metadata),

        SchemaNodeContent::Reference(name) => {
            // Convert to JSON Schema $ref
            Ok(JsonSchema::Reference(ReferenceSchema {
                reference: format!("#/$defs/{}", name),
                metadata: json_metadata,
            }))
        }
    }
}

/// Convert EURE metadata to JSON Schema metadata
fn convert_metadata(eure_meta: &EureMetadata) -> SchemaMetadata {
    SchemaMetadata {
        title: None, // EURE doesn't have title
        description: eure_meta.description.clone(),
    }
}

/// Convert EURE String schema to JSON Schema
fn convert_string_schema(
    eure: &EureStringSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Handle const
    if let Some(const_val) = &eure.r#const {
        return Ok(JsonSchema::Const(ConstSchema {
            value: serde_json::Value::String(const_val.clone()),
            metadata,
        }));
    }

    // Handle enum
    if let Some(enum_vals) = &eure.r#enum {
        return Ok(JsonSchema::Enum(EnumSchema {
            values: enum_vals
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
            metadata,
        }));
    }

    // Regular string schema
    let (min_length, max_length) = match eure.length {
        Some((min, max)) => (Some(min), Some(max)),
        None => (None, None),
    };

    Ok(JsonSchema::Typed(TypedSchema::String(StringSchema {
        min_length,
        max_length,
        pattern: eure.pattern.clone(),
        format: eure.format.clone(),
        default: None,
        metadata,
    })))
}

/// Convert EURE Integer schema to JSON Schema
fn convert_integer_schema(
    eure: &EureIntegerSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Handle const
    if let Some(const_val) = &eure.r#const {
        let val = bigint_to_i64(const_val)?;
        return Ok(JsonSchema::Const(ConstSchema {
            value: serde_json::Value::Number(val.into()),
            metadata,
        }));
    }

    // Handle enum
    if let Some(enum_vals) = &eure.r#enum {
        let values: Result<Vec<_>, _> = enum_vals
            .iter()
            .map(|v| {
                bigint_to_i64(v).map(|i| serde_json::Value::Number(i.into()))
            })
            .collect();
        return Ok(JsonSchema::Enum(EnumSchema {
            values: values?,
            metadata,
        }));
    }

    // Convert bounds
    let (minimum, exclusive_minimum) = match &eure.min {
        Bound::Unbounded => (None, None),
        Bound::Inclusive(val) => (Some(bigint_to_i64(val)?), None),
        Bound::Exclusive(val) => (None, Some(bigint_to_i64(val)?)),
    };

    let (maximum, exclusive_maximum) = match &eure.max {
        Bound::Unbounded => (None, None),
        Bound::Inclusive(val) => (Some(bigint_to_i64(val)?), None),
        Bound::Exclusive(val) => (None, Some(bigint_to_i64(val)?)),
    };

    let multiple_of = eure
        .multiple_of
        .as_ref()
        .map(bigint_to_i64)
        .transpose()?;

    Ok(JsonSchema::Typed(TypedSchema::Integer(IntegerSchema {
        minimum,
        maximum,
        exclusive_minimum,
        exclusive_maximum,
        multiple_of,
        default: None,
        metadata,
    })))
}

/// Convert BigInt to i64, returning error if out of range
fn bigint_to_i64(val: &num_bigint::BigInt) -> Result<i64, ConversionError> {
    val.to_i64()
        .ok_or_else(|| ConversionError::BigIntOutOfRange(val.to_string()))
}

/// Convert EURE Float schema to JSON Schema
fn convert_float_schema(
    eure: &FloatSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Validate float values (no NaN or Infinity)
    let validate_float = |f: f64| -> Result<f64, ConversionError> {
        if f.is_nan() || f.is_infinite() {
            Err(ConversionError::InvalidFloatValue(f.to_string()))
        } else {
            Ok(f)
        }
    };

    // Handle const
    if let Some(const_val) = &eure.r#const {
        let val = validate_float(*const_val)?;
        return Ok(JsonSchema::Const(ConstSchema {
            value: serde_json::json!(val),
            metadata,
        }));
    }

    // Handle enum
    if let Some(enum_vals) = &eure.r#enum {
        let values: Result<Vec<_>, _> = enum_vals
            .iter()
            .map(|v| validate_float(*v).map(|f| serde_json::json!(f)))
            .collect();
        return Ok(JsonSchema::Enum(EnumSchema {
            values: values?,
            metadata,
        }));
    }

    // Convert bounds
    let (minimum, exclusive_minimum) = match &eure.min {
        Bound::Unbounded => (None, None),
        Bound::Inclusive(val) => (Some(validate_float(*val)?), None),
        Bound::Exclusive(val) => (None, Some(validate_float(*val)?)),
    };

    let (maximum, exclusive_maximum) = match &eure.max {
        Bound::Unbounded => (None, None),
        Bound::Inclusive(val) => (Some(validate_float(*val)?), None),
        Bound::Exclusive(val) => (None, Some(validate_float(*val)?)),
    };

    Ok(JsonSchema::Typed(TypedSchema::Number(NumberSchema {
        minimum,
        maximum,
        exclusive_minimum,
        exclusive_maximum,
        multiple_of: None, // EURE float doesn't have multiple_of
        default: None,
        metadata,
    })))
}

/// Convert EURE Boolean schema to JSON Schema
fn convert_boolean_schema(
    eure: &EureBooleanSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    if let Some(const_val) = &eure.r#const {
        Ok(JsonSchema::Const(ConstSchema {
            value: serde_json::Value::Bool(*const_val),
            metadata,
        }))
    } else {
        Ok(JsonSchema::Typed(TypedSchema::Boolean(BooleanSchema {
            default: None,
            metadata,
        })))
    }
}

/// Convert EURE Array schema to JSON Schema
fn convert_array_schema(
    ctx: &mut ConversionContext,
    eure: &EureArraySchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let items = Some(Box::new(convert_node(ctx, eure.item)?));

    let contains = if let Some(prim_val) = &eure.contains {
        // JSON Schema contains expects a schema, but EURE has a primitive value
        // We convert this to a const schema
        Some(Box::new(JsonSchema::Const(ConstSchema {
            value: primitive_value_to_json(prim_val)?,
            metadata: SchemaMetadata::default(),
        })))
    } else {
        None
    };

    Ok(JsonSchema::Typed(TypedSchema::Array(ArraySchema {
        items,
        min_items: eure.min_items,
        max_items: eure.max_items,
        unique_items: if eure.unique { Some(true) } else { None },
        contains,
        metadata,
    })))
}

/// Convert EURE primitive value to JSON value
fn primitive_value_to_json(
    val: &eure_value::value::PrimitiveValue,
) -> Result<serde_json::Value, ConversionError> {
    use eure_value::value::PrimitiveValue;
    match val {
        PrimitiveValue::String(s) => Ok(serde_json::Value::String(s.to_string())),
        PrimitiveValue::BigInt(i) => {
            let val = bigint_to_i64(i)?;
            Ok(serde_json::Value::Number(val.into()))
        }
        PrimitiveValue::F64(f) => {
            if f.is_nan() || f.is_infinite() {
                Err(ConversionError::InvalidFloatValue(f.to_string()))
            } else {
                Ok(serde_json::json!(f))
            }
        }
        PrimitiveValue::F32(f) => {
            if f.is_nan() || f.is_infinite() {
                Err(ConversionError::InvalidFloatValue(f.to_string()))
            } else {
                Ok(serde_json::json!(f))
            }
        }
        PrimitiveValue::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        PrimitiveValue::Null => Ok(serde_json::Value::Null),
        _ => Err(ConversionError::ComplexContainsNotSupported),
    }
}

/// Convert EURE Map schema to JSON Schema
///
/// This is tricky because JSON Schema only supports string keys in objects.
/// If the key type is not String, we return an error.
fn convert_map_schema(
    ctx: &mut ConversionContext,
    eure: &MapSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Check if key is string type
    let key_node = ctx.get_node(eure.key)?;
    if !matches!(key_node.content, SchemaNodeContent::String(_)) {
        return Err(ConversionError::NonStringMapKeysNotSupported);
    }

    // Convert value schema
    let value_schema = convert_node(ctx, eure.value)?;

    // Map becomes an object with additionalProperties
    Ok(JsonSchema::Typed(TypedSchema::Object(ObjectSchema {
        properties: None,
        required: None,
        additional_properties: Some(AdditionalProperties::Schema(Box::new(value_schema))),
        metadata,
    })))
}

/// Convert EURE Record schema to JSON Schema object
fn convert_record_schema(
    ctx: &mut ConversionContext,
    eure: &RecordSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let mut properties = IndexMap::new();
    let mut required = Vec::new();

    for (field_name, node_id) in &eure.properties {
        let is_optional = ctx.get_node(*node_id)?.metadata.optional;
        let field_schema = convert_node(ctx, *node_id)?;

        properties.insert(field_name.clone(), field_schema);

        // If field is not optional, add to required
        if !is_optional {
            required.push(field_name.clone());
        }
    }

    let additional_properties = match &eure.unknown_fields {
        UnknownFieldsPolicy::Deny => Some(AdditionalProperties::Bool(false)),
        UnknownFieldsPolicy::Allow => Some(AdditionalProperties::Bool(true)),
        UnknownFieldsPolicy::Schema(node_id) => {
            let schema = convert_node(ctx, *node_id)?;
            Some(AdditionalProperties::Schema(Box::new(schema)))
        }
    };

    let properties = if properties.is_empty() {
        None
    } else {
        Some(properties)
    };

    let required = if required.is_empty() {
        None
    } else {
        Some(required)
    };

    Ok(JsonSchema::Typed(TypedSchema::Object(ObjectSchema {
        properties,
        required,
        additional_properties,
        metadata,
    })))
}

/// Convert EURE Tuple schema to JSON Schema
///
/// JSON Schema supports tuple validation via array with items as an array of schemas
/// However, this is less well-supported, so we note this as a potential limitation
fn convert_tuple_schema(
    _ctx: &mut ConversionContext,
    _eure: &TupleSchema,
    _metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // JSON Schema Draft-07 supports tuple validation but it's complex
    // For now, we return an error as it's not fully supported
    // Future enhancement: use "items" as array and "additionalItems": false
    Err(ConversionError::TupleConstraintsNotSupported)
}

/// Convert EURE Union to JSON Schema anyOf
fn convert_union_schema(
    ctx: &mut ConversionContext,
    variants: &[SchemaNodeId],
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let schemas: Result<Vec<_>, _> = variants
        .iter()
        .map(|id| convert_node(ctx, *id))
        .collect();

    Ok(JsonSchema::AnyOf(AnyOfSchema {
        schemas: schemas?,
        metadata,
    }))
}

/// Convert EURE Variant (tagged union) to JSON Schema
///
/// The conversion strategy depends on the variant representation:
/// - External: oneOf with object schemas (each with a single property)
/// - Internal: oneOf with allOf to merge tag and content
/// - Adjacent: oneOf with schemas having tag and content properties
/// - Untagged: Not well-supported in JSON Schema, returns error
fn convert_variant_schema(
    ctx: &mut ConversionContext,
    eure: &VariantSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    match &eure.repr {
        VariantRepr::External => convert_external_variant(ctx, eure, metadata),
        VariantRepr::Internal { tag } => convert_internal_variant(ctx, eure, tag, metadata),
        VariantRepr::Adjacent { tag, content } => {
            convert_adjacent_variant(ctx, eure, tag, content, metadata)
        }
        VariantRepr::Untagged => {
            // Untagged variants are ambiguous in JSON Schema
            // Return error for now
            Err(ConversionError::UntaggedVariantAmbiguous {
                variant_name: eure.variants.keys().next().unwrap_or(&"unknown".to_string()).clone(),
            })
        }
    }
}

/// Convert external variant representation
fn convert_external_variant(
    ctx: &mut ConversionContext,
    eure: &VariantSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let mut schemas = Vec::new();

    for (variant_name, node_id) in &eure.variants {
        let variant_schema = convert_node(ctx, *node_id)?;

        // External: { "variant-name": <schema> }
        let mut properties = IndexMap::new();
        properties.insert(variant_name.clone(), variant_schema);

        let obj = JsonSchema::Typed(TypedSchema::Object(ObjectSchema {
            properties: Some(properties),
            required: Some(vec![variant_name.clone()]),
            additional_properties: Some(AdditionalProperties::Bool(false)),
            metadata: SchemaMetadata::default(),
        }));

        schemas.push(obj);
    }

    Ok(JsonSchema::OneOf(OneOfSchema { schemas, metadata }))
}

/// Convert internal variant representation
fn convert_internal_variant(
    ctx: &mut ConversionContext,
    eure: &VariantSchema,
    tag: &str,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let mut schemas = Vec::new();

    for (variant_name, node_id) in &eure.variants {
        let variant_schema = convert_node(ctx, *node_id)?;

        // Internal: allOf with tag constraint and content schema
        let tag_schema = JsonSchema::Typed(TypedSchema::Object(ObjectSchema {
            properties: Some({
                let mut props = IndexMap::new();
                props.insert(
                    tag.to_string(),
                    JsonSchema::Const(ConstSchema {
                        value: serde_json::Value::String(variant_name.clone()),
                        metadata: SchemaMetadata::default(),
                    }),
                );
                props
            }),
            required: Some(vec![tag.to_string()]),
            additional_properties: None,
            metadata: SchemaMetadata::default(),
        }));

        let combined = JsonSchema::AllOf(AllOfSchema {
            schemas: vec![tag_schema, variant_schema],
            metadata: SchemaMetadata::default(),
        });

        schemas.push(combined);
    }

    Ok(JsonSchema::OneOf(OneOfSchema { schemas, metadata }))
}

/// Convert adjacent variant representation
fn convert_adjacent_variant(
    ctx: &mut ConversionContext,
    eure: &VariantSchema,
    tag: &str,
    content: &str,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let mut schemas = Vec::new();

    for (variant_name, node_id) in &eure.variants {
        let variant_schema = convert_node(ctx, *node_id)?;

        // Adjacent: { "tag": "variant-name", "content": <schema> }
        let mut properties = IndexMap::new();
        properties.insert(
            tag.to_string(),
            JsonSchema::Const(ConstSchema {
                value: serde_json::Value::String(variant_name.clone()),
                metadata: SchemaMetadata::default(),
            }),
        );
        properties.insert(content.to_string(), variant_schema);

        let obj = JsonSchema::Typed(TypedSchema::Object(ObjectSchema {
            properties: Some(properties),
            required: Some(vec![tag.to_string(), content.to_string()]),
            additional_properties: Some(AdditionalProperties::Bool(false)),
            metadata: SchemaMetadata::default(),
        }));

        schemas.push(obj);
    }

    Ok(JsonSchema::OneOf(OneOfSchema { schemas, metadata }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_schema::{
        CodeSchema, IntegerSchema as EureIntegerSchema,
        PathSchema, RecordSchema, SchemaDocument, SchemaNodeContent, StringSchema as EureStringSchema,
        UnknownFieldsPolicy, Bound,
    };
    use std::collections::HashMap;

    #[test]
    fn test_convert_simple_string() {
        let mut doc = SchemaDocument::new();
        doc.root = doc.create_node(SchemaNodeContent::String(EureStringSchema::default()));

        let result = eure_to_json_schema(&doc).unwrap();
        assert!(matches!(
            result,
            JsonSchema::Typed(TypedSchema::String(_))
        ));
    }

    #[test]
    fn test_convert_code_returns_error() {
        let mut doc = SchemaDocument::new();
        doc.root = doc.create_node(SchemaNodeContent::Code(CodeSchema::default()));

        let result = eure_to_json_schema(&doc);
        assert!(matches!(result, Err(ConversionError::CodeTypeNotSupported)));
    }

    #[test]
    fn test_convert_path_returns_error() {
        let mut doc = SchemaDocument::new();
        doc.root = doc.create_node(SchemaNodeContent::Path(PathSchema::default()));

        let result = eure_to_json_schema(&doc);
        assert!(matches!(result, Err(ConversionError::PathTypeNotSupported)));
    }

    #[test]
    fn test_convert_integer_with_bounds() {
        let mut doc = SchemaDocument::new();
        doc.root = doc.create_node(SchemaNodeContent::Integer(EureIntegerSchema {
            min: Bound::Inclusive(0.into()),
            max: Bound::Exclusive(100.into()),
            multiple_of: None,
            r#const: None,
            r#enum: None,
        }));

        let result = eure_to_json_schema(&doc).unwrap();
        match result {
            JsonSchema::Typed(TypedSchema::Integer(schema)) => {
                assert_eq!(schema.minimum, Some(0));
                assert_eq!(schema.exclusive_maximum, Some(100));
            }
            _ => panic!("Expected Integer schema"),
        }
    }

    #[test]
    fn test_convert_record_to_object() {
        let mut doc = SchemaDocument::new();

        let string_id = doc.create_node(SchemaNodeContent::String(EureStringSchema::default()));
        let int_id = doc.create_node(SchemaNodeContent::Integer(EureIntegerSchema::default()));

        let mut properties = HashMap::new();
        properties.insert("name".to_string(), string_id);
        properties.insert("age".to_string(), int_id);

        doc.root = doc.create_node(SchemaNodeContent::Record(RecordSchema {
            properties,
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));

        let result = eure_to_json_schema(&doc).unwrap();
        match result {
            JsonSchema::Typed(TypedSchema::Object(schema)) => {
                assert!(schema.properties.is_some());
                let props = schema.properties.unwrap();
                assert_eq!(props.len(), 2);
                assert!(props.contains_key("name"));
                assert!(props.contains_key("age"));
            }
            _ => panic!("Expected Object schema"),
        }
    }

    #[test]
    fn test_convert_union_to_anyof() {
        let mut doc = SchemaDocument::new();

        let string_id = doc.create_node(SchemaNodeContent::String(EureStringSchema::default()));
        let int_id = doc.create_node(SchemaNodeContent::Integer(EureIntegerSchema::default()));

        doc.root = doc.create_node(SchemaNodeContent::Union(vec![string_id, int_id]));

        let result = eure_to_json_schema(&doc).unwrap();
        match result {
            JsonSchema::AnyOf(schema) => {
                assert_eq!(schema.schemas.len(), 2);
            }
            _ => panic!("Expected AnyOf schema"),
        }
    }
}
