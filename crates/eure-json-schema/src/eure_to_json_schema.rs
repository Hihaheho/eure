//! Conversion from Eure Schema to JSON Schema (2020-12)
//!
//! This module provides functionality to convert Eure Schema documents to JSON Schema format.
//! Since Eure Schema is a superset of JSON Schema with additional features, some constructs
//! cannot be represented in JSON Schema and will result in conversion errors.

use crate::json_schema::*;
use eure_document::data_model::VariantRepr;
use eure_document::document::EureDocument;
use eure_json::Config as JsonConfig;
use eure_schema::{
    ArraySchema as EureArraySchema, Bound, Description, FloatSchema,
    IntegerSchema as EureIntegerSchema, MapSchema, RecordSchema, SchemaDocument,
    SchemaMetadata as EureMetadata, SchemaNode, SchemaNodeContent, SchemaNodeId, TextSchema,
    TupleSchema, UnionSchema, UnknownFieldsPolicy,
};
use indexmap::IndexMap;
use num_traits::ToPrimitive;

/// Convert an EureDocument to a JSON value
fn document_to_json(doc: &EureDocument) -> Result<serde_json::Value, ConversionError> {
    Ok(eure_json::document_to_value(doc, &JsonConfig::default())?)
}

/// Errors that can occur during Eure Schema to JSON Schema conversion
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ConversionError {
    /// Eure Hole type cannot be represented in JSON Schema
    #[error("Eure Hole type cannot be represented in JSON Schema")]
    HoleNotSupported,

    /// Eure Hole in literal value cannot be represented in JSON Schema
    #[error("Eure Hole in literal value cannot be represented in JSON Schema")]
    HoleInLiteral,

    /// Eure Map type with non-string keys cannot be represented in JSON Schema
    #[error("Eure Map with non-string keys cannot be represented in JSON Schema")]
    NonStringMapKeysNotSupported,

    /// BigInt value is too large to fit in i64 for JSON Schema
    #[error("BigInt value {0} is out of range for JSON Schema i64")]
    BigIntOutOfRange(String),

    /// Float value (NaN or Infinity) cannot be represented in JSON Schema
    #[error("Invalid float value: {0}")]
    InvalidFloatValue(String),

    /// Invalid schema node reference
    #[error("Invalid schema node reference: {0}")]
    InvalidNodeReference(usize),

    /// Circular reference detected (not supported in JSON Schema)
    #[error("Circular reference detected: {0}")]
    CircularReference(String),

    /// JSON conversion error (from eure-json)
    #[error(transparent)]
    JsonConversion(#[from] eure_json::EureToJsonError),

    /// Invalid default value type
    #[error("Invalid default value: expected {expected}, got {actual}")]
    InvalidDefaultValue {
        expected: &'static str,
        actual: String,
    },
}

/// Conversion context to track state during conversion
struct ConversionContext<'a> {
    /// The source Eure schema document
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

/// Convert an Eure SchemaDocument to JSON Schema
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
fn wrap_with_definitions(root: JsonSchema, defs: IndexMap<String, JsonSchema>) -> JsonSchema {
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
fn convert_node(
    ctx: &mut ConversionContext,
    id: SchemaNodeId,
) -> Result<JsonSchema, ConversionError> {
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
    eure_meta: &EureMetadata,
) -> Result<JsonSchema, ConversionError> {
    let json_metadata = convert_metadata(eure_meta)?;

    match content {
        SchemaNodeContent::Any => Ok(JsonSchema::Generic(GenericSchema {
            metadata: json_metadata,
            ..Default::default()
        })),

        SchemaNodeContent::Text(t) => convert_text_schema(t, eure_meta, json_metadata),

        SchemaNodeContent::Integer(i) => convert_integer_schema(i, eure_meta, json_metadata),

        SchemaNodeContent::Float(f) => convert_float_schema(f, eure_meta, json_metadata),

        SchemaNodeContent::Boolean => convert_boolean_schema(eure_meta, json_metadata),

        SchemaNodeContent::Null => Ok(JsonSchema::Typed(TypedSchema::Null(NullSchema {
            metadata: json_metadata,
        }))),

        SchemaNodeContent::Array(a) => convert_array_schema(ctx, a, json_metadata),

        SchemaNodeContent::Map(m) => convert_map_schema(ctx, m, json_metadata),

        SchemaNodeContent::Record(r) => convert_record_schema(ctx, r, json_metadata),

        SchemaNodeContent::Tuple(t) => convert_tuple_schema(ctx, t, json_metadata),

        SchemaNodeContent::Union(u) => convert_union_schema(ctx, u, json_metadata),

        SchemaNodeContent::Reference(ref_type) => {
            // Convert to JSON Schema $ref
            Ok(JsonSchema::Reference(ReferenceSchema {
                reference: format!("#/$defs/{}", ref_type.name),
                metadata: json_metadata,
            }))
        }

        SchemaNodeContent::Literal(val) => Ok(JsonSchema::Const(ConstSchema {
            value: document_to_json(val)?,
            metadata: json_metadata,
        })),
    }
}

/// Convert Eure metadata to JSON Schema metadata
fn convert_metadata(eure_meta: &EureMetadata) -> Result<SchemaMetadata, ConversionError> {
    let examples = eure_meta
        .examples
        .as_ref()
        .map(|examples| {
            examples
                .iter()
                .map(document_to_json)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    Ok(SchemaMetadata {
        title: None, // Eure doesn't have title
        description: eure_meta.description.as_ref().map(|d| match d {
            Description::String(s) => s.clone(),
            Description::Markdown(s) => s.clone(),
        }),
        deprecated: if eure_meta.deprecated {
            Some(true)
        } else {
            None
        },
        examples,
    })
}

/// Known JSON Schema format names (Draft 2020-12)
const JSON_SCHEMA_FORMATS: &[&str] = &[
    "date-time",
    "date",
    "time",
    "duration",
    "email",
    "idn-email",
    "hostname",
    "idn-hostname",
    "ipv4",
    "ipv6",
    "uri",
    "uri-reference",
    "iri",
    "iri-reference",
    "uuid",
    "uri-template",
    "json-pointer",
    "relative-json-pointer",
    "regex",
];

/// Convert Eure Text schema to JSON Schema
///
/// Text (which unifies the old String and Code types) maps to JSON Schema string type.
/// If the language matches a known JSON Schema format, it's mapped to the format field.
fn convert_text_schema(
    eure: &TextSchema,
    eure_meta: &EureMetadata,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Map language to format if it's a known JSON Schema format
    let format = eure.language.as_ref().and_then(|lang| {
        if JSON_SCHEMA_FORMATS.contains(&lang.as_str()) {
            Some(lang.clone())
        } else {
            None
        }
    });

    // Convert default value if present
    let default = eure_meta
        .default
        .as_ref()
        .map(|doc| {
            let json_val = document_to_json(doc)?;
            match json_val {
                serde_json::Value::String(s) => Ok(s),
                other => Err(ConversionError::InvalidDefaultValue {
                    expected: "string",
                    actual: format!("{:?}", other),
                }),
            }
        })
        .transpose()?;

    Ok(JsonSchema::Typed(TypedSchema::String(StringSchema {
        min_length: eure.min_length,
        max_length: eure.max_length,
        pattern: eure.pattern.as_ref().map(|r| r.as_str().to_string()),
        format,
        default,
        metadata,
    })))
}

/// Convert Eure Integer schema to JSON Schema
fn convert_integer_schema(
    eure: &EureIntegerSchema,
    eure_meta: &EureMetadata,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
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

    let multiple_of = eure.multiple_of.as_ref().map(bigint_to_i64).transpose()?;

    // Convert default value if present
    let default = eure_meta
        .default
        .as_ref()
        .map(|doc| {
            let json_val = document_to_json(doc)?;
            match json_val {
                serde_json::Value::Number(n) if n.is_i64() => Ok(n.as_i64().unwrap()),
                other => Err(ConversionError::InvalidDefaultValue {
                    expected: "integer",
                    actual: format!("{:?}", other),
                }),
            }
        })
        .transpose()?;

    Ok(JsonSchema::Typed(TypedSchema::Integer(IntegerSchema {
        minimum,
        maximum,
        exclusive_minimum,
        exclusive_maximum,
        multiple_of,
        default,
        metadata,
    })))
}

/// Convert BigInt to i64, returning error if out of range
fn bigint_to_i64(val: &num_bigint::BigInt) -> Result<i64, ConversionError> {
    val.to_i64()
        .ok_or_else(|| ConversionError::BigIntOutOfRange(val.to_string()))
}

/// Convert Eure Float schema to JSON Schema
fn convert_float_schema(
    eure: &FloatSchema,
    eure_meta: &EureMetadata,
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

    let multiple_of = eure.multiple_of.map(validate_float).transpose()?;

    // Convert default value if present
    let default = eure_meta
        .default
        .as_ref()
        .map(|doc| {
            let json_val = document_to_json(doc)?;
            match json_val {
                serde_json::Value::Number(n) => n
                    .as_f64()
                    .ok_or_else(|| ConversionError::InvalidDefaultValue {
                        expected: "number",
                        actual: format!("{:?}", n),
                    })
                    .and_then(validate_float),
                other => Err(ConversionError::InvalidDefaultValue {
                    expected: "number",
                    actual: format!("{:?}", other),
                }),
            }
        })
        .transpose()?;

    Ok(JsonSchema::Typed(TypedSchema::Number(NumberSchema {
        minimum,
        maximum,
        exclusive_minimum,
        exclusive_maximum,
        multiple_of,
        default,
        metadata,
    })))
}

/// Convert Eure Boolean schema to JSON Schema
fn convert_boolean_schema(
    eure_meta: &EureMetadata,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Convert default value if present
    let default = eure_meta
        .default
        .as_ref()
        .map(|doc| {
            let json_val = document_to_json(doc)?;
            match json_val {
                serde_json::Value::Bool(b) => Ok(b),
                other => Err(ConversionError::InvalidDefaultValue {
                    expected: "boolean",
                    actual: format!("{:?}", other),
                }),
            }
        })
        .transpose()?;

    Ok(JsonSchema::Typed(TypedSchema::Boolean(BooleanSchema {
        default,
        metadata,
    })))
}

/// Convert Eure Array schema to JSON Schema
fn convert_array_schema(
    ctx: &mut ConversionContext,
    eure: &EureArraySchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let items = Some(Box::new(convert_node(ctx, eure.item)?));

    let contains = if let Some(contains_id) = &eure.contains {
        // Contains is now a schema node reference
        Some(Box::new(convert_node(ctx, *contains_id)?))
    } else {
        None
    };

    Ok(JsonSchema::Typed(TypedSchema::Array(ArraySchema {
        items,
        prefix_items: None, // Not used for regular arrays (only tuples use this)
        min_items: eure.min_length,
        max_items: eure.max_length,
        unique_items: if eure.unique { Some(true) } else { None },
        contains,
        metadata,
    })))
}

/// Convert Eure Map schema to JSON Schema
///
/// This is tricky because JSON Schema only supports string keys in objects.
/// If the key type is not Text, we return an error.
fn convert_map_schema(
    ctx: &mut ConversionContext,
    eure: &MapSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Check if key is text type (JSON Schema only supports string keys)
    let key_node = ctx.get_node(eure.key)?;
    if !matches!(key_node.content, SchemaNodeContent::Text(_)) {
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

/// Convert Eure Record schema to JSON Schema object
fn convert_record_schema(
    ctx: &mut ConversionContext,
    eure: &RecordSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let mut properties = IndexMap::new();
    let mut required = Vec::new();

    for (field_name, field) in &eure.properties {
        let is_optional = field.optional;
        let field_schema = convert_node(ctx, field.schema)?;

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

/// Convert Eure Tuple schema to JSON Schema
///
/// JSON Schema supports tuple validation via array with items as an array of schemas
/// However, this is less well-supported, so we note this as a potential limitation
fn convert_tuple_schema(
    ctx: &mut ConversionContext,
    eure: &TupleSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    // Convert each element schema to JSON Schema
    let prefix_items: Vec<JsonSchema> = eure
        .elements
        .iter()
        .map(|node_id| convert_node(ctx, *node_id))
        .collect::<Result<Vec<_>, _>>()?;

    // Use prefixItems (JSON Schema 2020-12) for tuple validation
    // Also set items: false to disallow additional elements
    Ok(JsonSchema::Typed(TypedSchema::Array(ArraySchema {
        items: Some(Box::new(JsonSchema::Boolean(false))),
        prefix_items: if prefix_items.is_empty() {
            None
        } else {
            Some(prefix_items)
        },
        min_items: Some(eure.elements.len() as u32),
        max_items: Some(eure.elements.len() as u32),
        unique_items: None,
        contains: None,
        metadata,
    })))
}

/// Convert Eure Union to JSON Schema
///
/// The conversion strategy depends on the variant representation:
/// - External: oneOf with object schemas (each with a single property)
/// - Internal: oneOf with allOf to merge tag and content
/// - Adjacent: oneOf with schemas having tag and content properties
/// - Untagged: oneOf with just the variant schemas (no tagging)
fn convert_union_schema(
    ctx: &mut ConversionContext,
    eure: &UnionSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    match &eure.repr {
        VariantRepr::External => convert_external_variant(ctx, eure, metadata),
        VariantRepr::Internal { tag } => convert_internal_variant(ctx, eure, tag, metadata),
        VariantRepr::Adjacent { tag, content } => {
            convert_adjacent_variant(ctx, eure, tag, content, metadata)
        }
        VariantRepr::Untagged => convert_untagged_variant(ctx, eure, metadata),
    }
}

/// Convert external variant representation
fn convert_external_variant(
    ctx: &mut ConversionContext,
    eure: &UnionSchema,
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
    eure: &UnionSchema,
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
    eure: &UnionSchema,
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

/// Convert untagged variant representation
fn convert_untagged_variant(
    ctx: &mut ConversionContext,
    eure: &UnionSchema,
    metadata: SchemaMetadata,
) -> Result<JsonSchema, ConversionError> {
    let mut schemas = Vec::new();

    for node_id in eure.variants.values() {
        let variant_schema = convert_node(ctx, *node_id)?;
        schemas.push(variant_schema);
    }

    Ok(JsonSchema::OneOf(OneOfSchema { schemas, metadata }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::data_model::VariantRepr;
    use eure_schema::{
        Bound, IntegerSchema as EureIntegerSchema, RecordFieldSchema, RecordSchema, SchemaDocument,
        SchemaNodeContent, UnknownFieldsPolicy,
    };
    use std::collections::{BTreeMap, HashMap, HashSet};

    #[test]
    fn test_convert_simple_text() {
        let mut doc = SchemaDocument::new();
        doc.root = doc.create_node(SchemaNodeContent::Text(TextSchema::default()));

        let result = eure_to_json_schema(&doc).unwrap();
        assert!(matches!(result, JsonSchema::Typed(TypedSchema::String(_))));
    }

    #[test]
    fn test_convert_text_with_language() {
        // Text with language (e.g., code) should still convert to JSON Schema string
        let mut doc = SchemaDocument::new();
        doc.root = doc.create_node(SchemaNodeContent::Text(TextSchema {
            language: Some("rust".to_string()),
            ..Default::default()
        }));

        let result = eure_to_json_schema(&doc).unwrap();
        assert!(matches!(result, JsonSchema::Typed(TypedSchema::String(_))));
    }

    #[test]
    fn test_convert_integer_with_bounds() {
        let mut doc = SchemaDocument::new();
        doc.root = doc.create_node(SchemaNodeContent::Integer(EureIntegerSchema {
            min: Bound::Inclusive(0.into()),
            max: Bound::Exclusive(100.into()),
            multiple_of: None,
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

        let text_id = doc.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = doc.create_node(SchemaNodeContent::Integer(EureIntegerSchema::default()));

        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            RecordFieldSchema {
                schema: text_id,
                optional: false,
                binding_style: None,
            },
        );
        properties.insert(
            "age".to_string(),
            RecordFieldSchema {
                schema: int_id,
                optional: false,
                binding_style: None,
            },
        );

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
    fn test_convert_untagged_union_to_oneof() {
        let mut doc = SchemaDocument::new();

        let text_id = doc.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = doc.create_node(SchemaNodeContent::Integer(EureIntegerSchema::default()));

        let mut variants = BTreeMap::new();
        variants.insert("TextVariant".to_string(), text_id);
        variants.insert("IntVariant".to_string(), int_id);

        doc.root = doc.create_node(SchemaNodeContent::Union(UnionSchema {
            variants,
            priority: None,
            repr: VariantRepr::Untagged,
            deny_untagged: HashSet::new(),
        }));

        let result = eure_to_json_schema(&doc).unwrap();
        match result {
            JsonSchema::OneOf(schema) => {
                assert_eq!(schema.schemas.len(), 2);
            }
            _ => panic!("Expected OneOf schema for untagged union"),
        }
    }

    #[test]
    fn test_convert_external_union_to_oneof() {
        let mut doc = SchemaDocument::new();

        let text_id = doc.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = doc.create_node(SchemaNodeContent::Integer(EureIntegerSchema::default()));

        let mut variants = BTreeMap::new();
        variants.insert("A".to_string(), text_id);
        variants.insert("B".to_string(), int_id);

        doc.root = doc.create_node(SchemaNodeContent::Union(UnionSchema {
            variants,
            priority: None,
            repr: VariantRepr::External,
            deny_untagged: HashSet::new(),
        }));

        let result = eure_to_json_schema(&doc).unwrap();
        match result {
            JsonSchema::OneOf(schema) => {
                assert_eq!(schema.schemas.len(), 2);
                // Each variant should be wrapped in an object with a single property
            }
            _ => panic!("Expected OneOf schema for external union"),
        }
    }
}
