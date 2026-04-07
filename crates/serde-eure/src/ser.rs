use std::collections::{HashMap, HashSet};

use eure::document::EureDocument;
use eure::document::NodeId;
use eure::document::identifier::Identifier;
use eure::document::node::NodeValue;
use eure::document::parse::VariantPath;
use eure::value::ObjectKey;
use eure::value::PrimitiveValue;
use eure_schema::UnknownFieldsPolicy;
use eure_schema::interop::VariantRepr;
use eure_schema::{SchemaDocument, SchemaNodeContent, SchemaNodeId};
use num_bigint::BigInt;
use serde::Serialize;
use serde::ser::Error as _;
use serde::ser::{SerializeMap, SerializeSeq, SerializeTuple};

use crate::error::SerError;

pub fn to_serializer<S: serde::Serializer>(
    ser: S,
    doc: &EureDocument,
    schema: &SchemaDocument,
) -> Result<S::Ok, S::Error> {
    NodeWithSchema {
        doc,
        node_id: doc.get_root_id(),
        schema,
        schema_node_id: schema.root,
        variant_path: None,
    }
    .serialize(ser)
}

pub fn to_serializer_root<S: serde::Serializer>(
    ser: S,
    doc: &EureDocument,
    schema: &SchemaDocument,
) -> Result<S::Ok, S::Error> {
    to_serializer(ser, doc, schema)
}

#[derive(Clone)]
struct NodeWithSchema<'a> {
    doc: &'a EureDocument,
    node_id: NodeId,
    schema: &'a SchemaDocument,
    schema_node_id: SchemaNodeId,
    variant_path: Option<VariantPath>,
}

impl Serialize for NodeWithSchema<'_> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        serialize_node(
            ser,
            self.doc,
            self.node_id,
            self.schema,
            self.schema_node_id,
            self.variant_path.clone(),
        )
    }
}

struct UntypedNode<'a> {
    doc: &'a EureDocument,
    node_id: NodeId,
}

impl Serialize for UntypedNode<'_> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        serialize_untyped_node(ser, self.doc, self.node_id)
    }
}

struct ObjectKeyValue<'a> {
    key: &'a ObjectKey,
}

impl Serialize for ObjectKeyValue<'_> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        serialize_object_key(ser, self.key)
    }
}

struct ObjectKeyWithSchema<'a> {
    key: &'a ObjectKey,
    schema: &'a SchemaDocument,
    schema_node_id: SchemaNodeId,
}

impl Serialize for ObjectKeyWithSchema<'_> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        serialize_object_key_with_schema(ser, self.key, self.schema, self.schema_node_id)
    }
}

struct InternalTaggedVariant<'a> {
    tag: &'a str,
    variant_name: &'a str,
    variant_schema_id: SchemaNodeId,
    variant_path: Option<VariantPath>,
}

fn serialize_node<S: serde::Serializer>(
    ser: S,
    doc: &EureDocument,
    node_id: NodeId,
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
    variant_path: Option<VariantPath>,
) -> Result<S::Ok, S::Error> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)
        .map_err(|err| S::Error::custom(err.to_string()))?;
    let schema_content = &schema.node(schema_node_id).content;

    if let Some(path) = &variant_path
        && !path.is_empty()
        && !matches!(schema_content, SchemaNodeContent::Union(_))
    {
        return Err(S::Error::custom(
            SerError::Custom(format!("invalid variant name: {}", path)).to_string(),
        ));
    }

    match schema_content {
        SchemaNodeContent::Any | SchemaNodeContent::Literal(_) => {
            serialize_untyped_node(ser, doc, node_id)
        }
        SchemaNodeContent::Text(_) => match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Text(text)) => ser.serialize_str(text.as_str()),
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected text, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Integer(_) => match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Integer(value)) => serialize_bigint(ser, value),
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected integer, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Float(_) => match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::F32(value)) => {
                if value.is_finite() {
                    ser.serialize_f32(*value)
                } else {
                    Err(S::Error::custom(SerError::NonFiniteFloat.to_string()))
                }
            }
            NodeValue::Primitive(PrimitiveValue::F64(value)) => {
                if value.is_finite() {
                    ser.serialize_f64(*value)
                } else {
                    Err(S::Error::custom(SerError::NonFiniteFloat.to_string()))
                }
            }
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected float, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Boolean => match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Bool(value)) => ser.serialize_bool(*value),
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected boolean, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Null => match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Null) => ser.serialize_unit(),
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected null, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Array(array_schema) => match &doc.node(node_id).content {
            NodeValue::Array(values) => {
                let mut seq = ser.serialize_seq(Some(values.len()))?;
                for &child_id in values.iter() {
                    seq.serialize_element(&NodeWithSchema {
                        doc,
                        node_id: child_id,
                        schema,
                        schema_node_id: array_schema.item,
                        variant_path: None,
                    })?;
                }
                seq.end()
            }
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected array, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Tuple(tuple_schema) => match &doc.node(node_id).content {
            NodeValue::Tuple(values) => {
                if values.len() != tuple_schema.elements.len() {
                    return Err(S::Error::custom(
                        SerError::Custom(format!(
                            "tuple arity mismatch: schema has {} elements, document has {}",
                            tuple_schema.elements.len(),
                            values.len()
                        ))
                        .to_string(),
                    ));
                }
                let mut tuple = ser.serialize_tuple(values.len())?;
                for (index, &child_id) in values.iter().enumerate() {
                    let schema_node_id = tuple_schema.elements[index];
                    tuple.serialize_element(&NodeWithSchema {
                        doc,
                        node_id: child_id,
                        schema,
                        schema_node_id,
                        variant_path: None,
                    })?;
                }
                tuple.end()
            }
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected tuple, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Map(map_schema) => match &doc.node(node_id).content {
            NodeValue::Map(map) => {
                let mut map_ser = ser.serialize_map(Some(map.len()))?;
                for (key, &child_id) in map.iter() {
                    map_ser.serialize_entry(
                        &ObjectKeyWithSchema {
                            key,
                            schema,
                            schema_node_id: map_schema.key,
                        },
                        &NodeWithSchema {
                            doc,
                            node_id: child_id,
                            schema,
                            schema_node_id: map_schema.value,
                            variant_path: None,
                        },
                    )?;
                }
                map_ser.end()
            }
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected map, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Record(_) => match &doc.node(node_id).content {
            NodeValue::Map(map) => {
                let mut map_ser = ser.serialize_map(Some(map.len()))?;
                serialize_record_entries(&mut map_ser, doc, node_id, schema, schema_node_id)?;
                map_ser.end()
            }
            other => Err(S::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected record, got {}",
                    node_value_type(other)
                ))
                .to_string(),
            )),
        },
        SchemaNodeContent::Union(union_schema) => {
            let variant_path = match variant_path {
                Some(path) => path,
                None => extract_variant_path(doc, node_id)
                    .map_err(|err| S::Error::custom(err.to_string()))?,
            };
            let Some(variant_name) = variant_path.first() else {
                return Err(S::Error::custom(
                    SerError::Custom("no variant matched".to_string()).to_string(),
                ));
            };
            let Some(&variant_schema_id) = union_schema.variants.get(variant_name.as_ref()) else {
                return Err(S::Error::custom(
                    SerError::Custom(format!("invalid variant name: {}", variant_name)).to_string(),
                ));
            };
            let rest = variant_path.rest();
            let variant_name = variant_name.as_ref();
            let repr = union_schema
                .interop
                .variant_repr
                .as_ref()
                .unwrap_or(&VariantRepr::External);

            match repr {
                VariantRepr::External => {
                    let mut map = ser.serialize_map(Some(1))?;
                    map.serialize_entry(
                        variant_name,
                        &NodeWithSchema {
                            doc,
                            node_id,
                            schema,
                            schema_node_id: variant_schema_id,
                            variant_path: rest,
                        },
                    )?;
                    map.end()
                }
                VariantRepr::Internal { tag } => {
                    let mut map = ser.serialize_map(None)?;
                    serialize_internal_tagged_entries(
                        &mut map,
                        doc,
                        node_id,
                        schema,
                        InternalTaggedVariant {
                            tag,
                            variant_name,
                            variant_schema_id,
                            variant_path: rest,
                        },
                    )?;
                    map.end()
                }
                VariantRepr::Adjacent { tag, content } => {
                    if tag == content {
                        return Err(S::Error::custom(
                            SerError::Custom(format!(
                                "adjacent variant tag and content fields conflict: {tag}"
                            ))
                            .to_string(),
                        ));
                    }
                    let resolved_id = resolve_schema_node_id(schema, variant_schema_id)
                        .map_err(|e| S::Error::custom(e.to_string()))?;
                    let is_unit =
                        matches!(schema.node(resolved_id).content, SchemaNodeContent::Null);
                    if is_unit {
                        let mut map = ser.serialize_map(Some(1))?;
                        map.serialize_entry(tag, variant_name)?;
                        map.end()
                    } else {
                        let mut map = ser.serialize_map(Some(2))?;
                        map.serialize_entry(tag, variant_name)?;
                        map.serialize_entry(
                            content,
                            &NodeWithSchema {
                                doc,
                                node_id,
                                schema,
                                schema_node_id: variant_schema_id,
                                variant_path: rest,
                            },
                        )?;
                        map.end()
                    }
                }
                VariantRepr::Untagged => {
                    if union_schema.deny_untagged.contains(variant_name) {
                        return Err(S::Error::custom(
                            SerError::Custom(format!(
                                "variant {variant_name:?} requires explicit $variant tag and cannot be serialized as untagged"
                            ))
                            .to_string(),
                        ));
                    }
                    serialize_node(ser, doc, node_id, schema, variant_schema_id, rest)
                }
            }
        }
        SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
    }
}

fn serialize_internal_tagged_entries<M: SerializeMap>(
    map_ser: &mut M,
    doc: &EureDocument,
    node_id: NodeId,
    schema: &SchemaDocument,
    variant: InternalTaggedVariant<'_>,
) -> Result<(), M::Error> {
    let InternalTaggedVariant {
        tag,
        variant_name,
        variant_schema_id,
        variant_path,
    } = variant;
    let selected_variant_schema_id = resolve_schema_node_id(schema, variant_schema_id)
        .map_err(|err| M::Error::custom(err.to_string()))?;
    let is_unit_variant = matches!(
        schema.node(selected_variant_schema_id).content,
        SchemaNodeContent::Null
    );

    if is_unit_variant {
        let NodeValue::Primitive(PrimitiveValue::Null) = &doc.node(node_id).content else {
            return Err(M::Error::custom(
                SerError::Custom(format!(
                    "type mismatch: expected null, got {}",
                    node_value_type(&doc.node(node_id).content)
                ))
                .to_string(),
            ));
        };
    } else if let NodeValue::Map(content_map) = &doc.node(node_id).content
        && content_map.contains_key(&ObjectKey::String(tag.to_string()))
    {
        return Err(M::Error::custom(
            SerError::Custom(format!(
                "variant tag field conflicts with content field: {tag}"
            ))
            .to_string(),
        ));
    }

    map_ser.serialize_entry(tag, variant_name)?;

    if is_unit_variant {
        Ok(())
    } else {
        serialize_map_like_entries(
            map_ser,
            doc,
            node_id,
            schema,
            variant_schema_id,
            variant_path,
        )
    }
}

/// Recursively collect `(field_name → SchemaNodeId)` from flattened record schemas.
/// Only `Record` schemas contribute fields; other node types are silently ignored.
fn collect_flatten_fields(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    out: &mut HashMap<String, (SchemaNodeId, bool)>,
) {
    let Ok(resolved) = resolve_schema_node_id(schema, node_id) else {
        return;
    };

    match &schema.node(resolved).content {
        SchemaNodeContent::Record(rec) => {
            for (name, field) in &rec.properties {
                out.entry(name.clone())
                    .or_insert((field.schema, field.optional));
            }
            for &inner in &rec.flatten {
                collect_flatten_fields(schema, inner, out);
            }
        }
        SchemaNodeContent::Union(_)
        | SchemaNodeContent::Any
        | SchemaNodeContent::Text(_)
        | SchemaNodeContent::Integer(_)
        | SchemaNodeContent::Float(_)
        | SchemaNodeContent::Boolean
        | SchemaNodeContent::Null
        | SchemaNodeContent::Literal(_)
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Tuple(_)
        | SchemaNodeContent::Reference(_) => {}
    }
}

fn serialize_record_entries<M: SerializeMap>(
    map_ser: &mut M,
    doc: &EureDocument,
    node_id: NodeId,
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
) -> Result<(), M::Error> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)
        .map_err(|err| M::Error::custom(err.to_string()))?;
    let SchemaNodeContent::Record(record_schema) = &schema.node(schema_node_id).content else {
        return Err(M::Error::custom("record schema expected"));
    };
    let NodeValue::Map(map) = &doc.node(node_id).content else {
        return Err(M::Error::custom("record node must be a map"));
    };

    let mut written = HashSet::new();
    let mut flatten_fields = HashMap::new();
    for &fid in &record_schema.flatten {
        collect_flatten_fields(schema, fid, &mut flatten_fields);
    }

    for (field_name, field_schema) in &record_schema.properties {
        let key = ObjectKey::String(field_name.clone());
        if let Some(&child_id) = map.get(&key) {
            map_ser.serialize_entry(
                field_name,
                &NodeWithSchema {
                    doc,
                    node_id: child_id,
                    schema,
                    schema_node_id: field_schema.schema,
                    variant_path: None,
                },
            )?;
            written.insert(key);
        }
    }

    for (field_name, &(fschema_id, _optional)) in &flatten_fields {
        let key = ObjectKey::String(field_name.clone());
        if written.contains(&key) {
            continue;
        }
        if let Some(&child_id) = map.get(&key) {
            map_ser.serialize_entry(
                field_name,
                &NodeWithSchema {
                    doc,
                    node_id: child_id,
                    schema,
                    schema_node_id: fschema_id,
                    variant_path: None,
                },
            )?;
            written.insert(key);
        }
    }

    for (field_name, field_schema) in &record_schema.properties {
        if !field_schema.optional && !map.contains_key(&ObjectKey::String(field_name.clone())) {
            return Err(M::Error::custom(
                SerError::MissingField(field_name.clone()).to_string(),
            ));
        }
    }
    for (field_name, &(_schema_id, optional)) in &flatten_fields {
        if !optional && !map.contains_key(&ObjectKey::String(field_name.clone())) {
            return Err(M::Error::custom(
                SerError::MissingField(field_name.clone()).to_string(),
            ));
        }
    }

    for (key, &child_id) in map.iter() {
        if written.contains(key) {
            continue;
        }

        match &record_schema.unknown_fields {
            UnknownFieldsPolicy::Schema(unknown_schema_id) => map_ser.serialize_entry(
                &ObjectKeyValue { key },
                &NodeWithSchema {
                    doc,
                    node_id: child_id,
                    schema,
                    schema_node_id: *unknown_schema_id,
                    variant_path: None,
                },
            )?,
            UnknownFieldsPolicy::Allow => map_ser.serialize_entry(
                &ObjectKeyValue { key },
                &UntypedNode {
                    doc,
                    node_id: child_id,
                },
            )?,
            UnknownFieldsPolicy::Deny => {
                return Err(M::Error::custom(
                    SerError::Custom(format!(
                        "document contains field {:?} not allowed by Deny-policy record schema",
                        key
                    ))
                    .to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn serialize_map_like_entries<M: SerializeMap>(
    map_ser: &mut M,
    doc: &EureDocument,
    node_id: NodeId,
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
    variant_path: Option<VariantPath>,
) -> Result<(), M::Error> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)
        .map_err(|err| M::Error::custom(err.to_string()))?;
    let schema_content = &schema.node(schema_node_id).content;

    if let Some(path) = &variant_path
        && !path.is_empty()
        && !matches!(schema_content, SchemaNodeContent::Union(_))
    {
        return Err(M::Error::custom(
            SerError::Custom(
                "variant path has remaining components but schema is not a union".to_string(),
            )
            .to_string(),
        ));
    }

    match schema_content {
        SchemaNodeContent::Record(_) => {
            serialize_record_entries(map_ser, doc, node_id, schema, schema_node_id)
        }
        SchemaNodeContent::Union(union_schema) => serialize_union_map_like_entries(
            map_ser,
            doc,
            node_id,
            schema,
            union_schema,
            variant_path,
        ),
        SchemaNodeContent::Map(map_schema) => {
            let NodeValue::Map(map) = &doc.node(node_id).content else {
                return Err(M::Error::custom("map node must be a map"));
            };
            for (key, &child_id) in map.iter() {
                map_ser.serialize_entry(
                    &ObjectKeyWithSchema {
                        key,
                        schema,
                        schema_node_id: map_schema.key,
                    },
                    &NodeWithSchema {
                        doc,
                        node_id: child_id,
                        schema,
                        schema_node_id: map_schema.value,
                        variant_path: None,
                    },
                )?;
            }
            Ok(())
        }
        SchemaNodeContent::Any => {
            // Schema-free: emit whatever the document has untyped.
            let NodeValue::Map(map) = &doc.node(node_id).content else {
                return Err(M::Error::custom("map-like node must be a map"));
            };
            for (key, &child_id) in map.iter() {
                map_ser.serialize_entry(
                    &ObjectKeyValue { key },
                    &UntypedNode {
                        doc,
                        node_id: child_id,
                    },
                )?;
            }
            Ok(())
        }
        other => Err(M::Error::custom(
            SerError::Custom(format!(
                "schema type {} cannot be serialized as a map-like internal-tagged variant",
                schema_content_type_name(other)
            ))
            .to_string(),
        )),
    }
}

fn schema_content_type_name(content: &SchemaNodeContent) -> &'static str {
    match content {
        SchemaNodeContent::Any => "any",
        SchemaNodeContent::Text(_) => "text",
        SchemaNodeContent::Integer(_) => "integer",
        SchemaNodeContent::Float(_) => "float",
        SchemaNodeContent::Boolean => "boolean",
        SchemaNodeContent::Null => "null",
        SchemaNodeContent::Literal(_) => "literal",
        SchemaNodeContent::Array(_) => "array",
        SchemaNodeContent::Tuple(_) => "tuple",
        SchemaNodeContent::Map(_) => "map",
        SchemaNodeContent::Record(_) => "record",
        SchemaNodeContent::Union(_) => "union",
        SchemaNodeContent::Reference(_) => "reference",
    }
}

fn serialize_union_map_like_entries<M: SerializeMap>(
    map_ser: &mut M,
    doc: &EureDocument,
    node_id: NodeId,
    schema: &SchemaDocument,
    union_schema: &eure_schema::UnionSchema,
    variant_path: Option<VariantPath>,
) -> Result<(), M::Error> {
    let variant_path = match variant_path {
        Some(path) => path,
        None => {
            extract_variant_path(doc, node_id).map_err(|err| M::Error::custom(err.to_string()))?
        }
    };
    let Some(variant_name) = variant_path.first() else {
        return Err(M::Error::custom(
            SerError::Custom("no variant matched".to_string()).to_string(),
        ));
    };
    let Some(&variant_schema_id) = union_schema.variants.get(variant_name.as_ref()) else {
        return Err(M::Error::custom(
            SerError::Custom(format!("invalid variant name: {variant_name}")).to_string(),
        ));
    };

    let rest = variant_path.rest();
    let variant_name = variant_name.as_ref();
    let repr = union_schema
        .interop
        .variant_repr
        .as_ref()
        .unwrap_or(&VariantRepr::External);

    match repr {
        VariantRepr::External => map_ser.serialize_entry(
            variant_name,
            &NodeWithSchema {
                doc,
                node_id,
                schema,
                schema_node_id: variant_schema_id,
                variant_path: rest,
            },
        ),
        VariantRepr::Internal { tag } => serialize_internal_tagged_entries(
            map_ser,
            doc,
            node_id,
            schema,
            InternalTaggedVariant {
                tag,
                variant_name,
                variant_schema_id,
                variant_path: rest,
            },
        ),
        VariantRepr::Adjacent { tag, content } => {
            if tag == content {
                return Err(M::Error::custom(
                    SerError::Custom(format!(
                        "adjacent variant tag and content fields conflict: {tag}"
                    ))
                    .to_string(),
                ));
            }
            let resolved_id = resolve_schema_node_id(schema, variant_schema_id)
                .map_err(|e| M::Error::custom(e.to_string()))?;
            let is_unit = matches!(schema.node(resolved_id).content, SchemaNodeContent::Null);
            map_ser.serialize_entry(tag, variant_name)?;
            if !is_unit {
                map_ser.serialize_entry(
                    content,
                    &NodeWithSchema {
                        doc,
                        node_id,
                        schema,
                        schema_node_id: variant_schema_id,
                        variant_path: rest,
                    },
                )?;
            }
            Ok(())
        }
        VariantRepr::Untagged => {
            if union_schema.deny_untagged.contains(variant_name) {
                return Err(M::Error::custom(
                    SerError::Custom(format!(
                        "variant {variant_name:?} requires explicit $variant tag and cannot be serialized as untagged"
                    ))
                    .to_string(),
                ));
            }
            serialize_map_like_entries(map_ser, doc, node_id, schema, variant_schema_id, rest)
        }
    }
}

fn serialize_untyped_node<S: serde::Serializer>(
    ser: S,
    doc: &EureDocument,
    node_id: NodeId,
) -> Result<S::Ok, S::Error> {
    match &doc.node(node_id).content {
        NodeValue::Hole(_) => Err(S::Error::custom(SerError::UnexpectedHole.to_string())),
        NodeValue::PartialMap(_) => Err(S::Error::custom(
            SerError::PartialMapUnsupported.to_string(),
        )),
        NodeValue::Primitive(primitive) => serialize_primitive(ser, primitive),
        NodeValue::Array(values) => {
            let mut seq = ser.serialize_seq(Some(values.len()))?;
            for &child_id in values.iter() {
                seq.serialize_element(&UntypedNode {
                    doc,
                    node_id: child_id,
                })?;
            }
            seq.end()
        }
        NodeValue::Tuple(values) => {
            let mut tuple = ser.serialize_tuple(values.len())?;
            for &child_id in values.iter() {
                tuple.serialize_element(&UntypedNode {
                    doc,
                    node_id: child_id,
                })?;
            }
            tuple.end()
        }
        NodeValue::Map(values) => {
            let mut map = ser.serialize_map(Some(values.len()))?;
            for (key, &child_id) in values.iter() {
                map.serialize_entry(
                    &ObjectKeyValue { key },
                    &UntypedNode {
                        doc,
                        node_id: child_id,
                    },
                )?;
            }
            map.end()
        }
    }
}

fn serialize_primitive<S: serde::Serializer>(
    ser: S,
    primitive: &PrimitiveValue,
) -> Result<S::Ok, S::Error> {
    match primitive {
        PrimitiveValue::Null => ser.serialize_unit(),
        PrimitiveValue::Bool(value) => ser.serialize_bool(*value),
        PrimitiveValue::Integer(value) => serialize_bigint(ser, value),
        PrimitiveValue::F32(value) => {
            if value.is_finite() {
                ser.serialize_f32(*value)
            } else {
                Err(S::Error::custom(SerError::NonFiniteFloat.to_string()))
            }
        }
        PrimitiveValue::F64(value) => {
            if value.is_finite() {
                ser.serialize_f64(*value)
            } else {
                Err(S::Error::custom(SerError::NonFiniteFloat.to_string()))
            }
        }
        PrimitiveValue::Text(text) => ser.serialize_str(text.as_str()),
    }
}

fn serialize_object_key<S: serde::Serializer>(ser: S, key: &ObjectKey) -> Result<S::Ok, S::Error> {
    match key {
        ObjectKey::String(value) => ser.serialize_str(value),
        ObjectKey::Number(value) => serialize_bigint(ser, value),
        ObjectKey::Tuple(values) => {
            let mut tuple = ser.serialize_tuple(values.len())?;
            for key in &values.0 {
                tuple.serialize_element(&ObjectKeyValue { key })?;
            }
            tuple.end()
        }
    }
}

fn serialize_object_key_with_schema<S: serde::Serializer>(
    ser: S,
    key: &ObjectKey,
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
) -> Result<S::Ok, S::Error> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)
        .map_err(|err| S::Error::custom(err.to_string()))?;

    match &schema.node(schema_node_id).content {
        SchemaNodeContent::Any => serialize_object_key(ser, key),
        SchemaNodeContent::Text(_) => serialize_text_object_key(ser, key),
        SchemaNodeContent::Integer(_) => serialize_integer_object_key(ser, key),
        SchemaNodeContent::Boolean => serialize_bool_object_key(ser, key),
        SchemaNodeContent::Literal(expected) => serialize_literal_object_key(ser, key, expected),
        SchemaNodeContent::Tuple(tuple_schema) => {
            serialize_tuple_object_key(ser, key, schema, &tuple_schema.elements)
        }
        SchemaNodeContent::Union(union_schema) => {
            serialize_union_object_key(ser, key, schema, union_schema)
        }
        SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
        SchemaNodeContent::Float(_)
        | SchemaNodeContent::Null
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Record(_) => {
            Err(S::Error::custom(unsupported_complex_map_keys().to_string()))
        }
    }
}

fn serialize_text_object_key<S: serde::Serializer>(
    ser: S,
    key: &ObjectKey,
) -> Result<S::Ok, S::Error> {
    match key {
        ObjectKey::String(value) => ser.serialize_str(value),
        ObjectKey::Number(value) => ser.serialize_str(&value.to_string()),
        ObjectKey::Tuple(_) => Err(S::Error::custom(
            object_key_type_mismatch("text", key).to_string(),
        )),
    }
}

fn serialize_integer_object_key<S: serde::Serializer>(
    ser: S,
    key: &ObjectKey,
) -> Result<S::Ok, S::Error> {
    match key {
        ObjectKey::Number(value) => serialize_bigint(ser, value),
        ObjectKey::String(value) => {
            let parsed = parse_bigint(value).map_err(|err| S::Error::custom(err.to_string()))?;
            serialize_bigint(ser, &parsed)
        }
        ObjectKey::Tuple(_) => Err(S::Error::custom(
            object_key_type_mismatch("integer", key).to_string(),
        )),
    }
}

fn serialize_bool_object_key<S: serde::Serializer>(
    ser: S,
    key: &ObjectKey,
) -> Result<S::Ok, S::Error> {
    match key {
        ObjectKey::String(value) => match value.as_str() {
            "true" => ser.serialize_bool(true),
            "false" => ser.serialize_bool(false),
            _ => Err(S::Error::custom(
                object_key_type_mismatch("boolean", key).to_string(),
            )),
        },
        ObjectKey::Number(_) | ObjectKey::Tuple(_) => Err(S::Error::custom(
            object_key_type_mismatch("boolean", key).to_string(),
        )),
    }
}

fn serialize_literal_object_key<S: serde::Serializer>(
    ser: S,
    key: &ObjectKey,
    expected: &EureDocument,
) -> Result<S::Ok, S::Error> {
    match expected_primitive(expected).map_err(|err| S::Error::custom(err.to_string()))? {
        PrimitiveValue::Text(expected_value) => match key {
            ObjectKey::String(value) if value == expected_value.as_str() => {
                ser.serialize_str(value)
            }
            ObjectKey::String(_) => Err(S::Error::custom(literal_mismatch().to_string())),
            ObjectKey::Number(_) | ObjectKey::Tuple(_) => Err(S::Error::custom(
                object_key_type_mismatch("text", key).to_string(),
            )),
        },
        PrimitiveValue::Integer(expected_value) => match key {
            ObjectKey::Number(value) if value == expected_value => serialize_bigint(ser, value),
            ObjectKey::Number(_) => Err(S::Error::custom(literal_mismatch().to_string())),
            ObjectKey::String(value) => {
                let parsed =
                    parse_bigint(value).map_err(|err| S::Error::custom(err.to_string()))?;
                if &parsed == expected_value {
                    serialize_bigint(ser, &parsed)
                } else {
                    Err(S::Error::custom(literal_mismatch().to_string()))
                }
            }
            ObjectKey::Tuple(_) => Err(S::Error::custom(
                object_key_type_mismatch("integer", key).to_string(),
            )),
        },
        PrimitiveValue::Bool(expected_value) => match key {
            ObjectKey::String(value)
                if (*expected_value && value == "true")
                    || (!*expected_value && value == "false") =>
            {
                ser.serialize_bool(*expected_value)
            }
            ObjectKey::String(_) => Err(S::Error::custom(literal_mismatch().to_string())),
            ObjectKey::Number(_) | ObjectKey::Tuple(_) => Err(S::Error::custom(
                object_key_type_mismatch("boolean", key).to_string(),
            )),
        },
        PrimitiveValue::Null | PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => Err(
            S::Error::custom(unsupported_complex_literal_key().to_string()),
        ),
    }
}

fn serialize_tuple_object_key<S: serde::Serializer>(
    ser: S,
    key: &ObjectKey,
    schema: &SchemaDocument,
    elements: &[SchemaNodeId],
) -> Result<S::Ok, S::Error> {
    let ObjectKey::Tuple(values) = key else {
        return Err(S::Error::custom(
            object_key_type_mismatch("tuple", key).to_string(),
        ));
    };
    if values.len() != elements.len() {
        return Err(S::Error::custom(
            SerError::Custom(format!(
                "tuple length mismatch: expected {}, got {}",
                elements.len(),
                values.len()
            ))
            .to_string(),
        ));
    }

    let mut tuple = ser.serialize_tuple(values.len())?;
    for (key, &schema_node_id) in values.0.iter().zip(elements) {
        tuple.serialize_element(&ObjectKeyWithSchema {
            key,
            schema,
            schema_node_id,
        })?;
    }
    tuple.end()
}

fn serialize_union_object_key<S: serde::Serializer>(
    ser: S,
    key: &ObjectKey,
    schema: &SchemaDocument,
    union_schema: &eure_schema::UnionSchema,
) -> Result<S::Ok, S::Error> {
    if let ObjectKey::String(value) = key
        && union_schema.variants.contains_key(value)
    {
        return ser.serialize_str(value);
    }

    for &variant_schema_id in union_schema.variants.values() {
        if object_key_matches_schema(schema, variant_schema_id, key)
            .map_err(|err| S::Error::custom(err.to_string()))?
        {
            return serialize_object_key_with_schema(ser, key, schema, variant_schema_id);
        }
    }

    Err(S::Error::custom(
        object_key_type_mismatch("union-compatible", key).to_string(),
    ))
}

fn object_key_matches_schema(
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
    key: &ObjectKey,
) -> Result<bool, SerError> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)?;

    match &schema.node(schema_node_id).content {
        SchemaNodeContent::Any => Ok(true),
        SchemaNodeContent::Text(_) => Ok(!matches!(key, ObjectKey::Tuple(_))),
        SchemaNodeContent::Integer(_) => Ok(match key {
            ObjectKey::Number(_) => true,
            ObjectKey::String(value) => value.parse::<BigInt>().is_ok(),
            ObjectKey::Tuple(_) => false,
        }),
        SchemaNodeContent::Boolean => Ok(matches!(
            key,
            ObjectKey::String(value) if value == "true" || value == "false"
        )),
        SchemaNodeContent::Literal(expected) => literal_object_key_matches(expected, key),
        SchemaNodeContent::Tuple(tuple_schema) => match key {
            ObjectKey::Tuple(values) if values.len() == tuple_schema.elements.len() => {
                for (key, &element_schema_id) in values.0.iter().zip(&tuple_schema.elements) {
                    if !object_key_matches_schema(schema, element_schema_id, key)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        },
        SchemaNodeContent::Union(union_schema) => {
            if let ObjectKey::String(value) = key
                && union_schema.variants.contains_key(value)
            {
                return Ok(true);
            }

            for &variant_schema_id in union_schema.variants.values() {
                if object_key_matches_schema(schema, variant_schema_id, key)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
        SchemaNodeContent::Float(_)
        | SchemaNodeContent::Null
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Record(_) => Ok(false),
    }
}

fn literal_object_key_matches(expected: &EureDocument, key: &ObjectKey) -> Result<bool, SerError> {
    match expected_primitive(expected)? {
        PrimitiveValue::Text(expected_value) => Ok(matches!(
            key,
            ObjectKey::String(value) if value == expected_value.as_str()
        )),
        PrimitiveValue::Integer(expected_value) => Ok(match key {
            ObjectKey::Number(value) => value == expected_value,
            ObjectKey::String(value) => value
                .parse::<BigInt>()
                .is_ok_and(|parsed| &parsed == expected_value),
            ObjectKey::Tuple(_) => false,
        }),
        PrimitiveValue::Bool(expected_value) => Ok(matches!(
            key,
            ObjectKey::String(value)
                if (*expected_value && value == "true") || (!*expected_value && value == "false")
        )),
        PrimitiveValue::Null | PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => Ok(false),
    }
}

fn expected_primitive(expected: &EureDocument) -> Result<&PrimitiveValue, SerError> {
    match &expected.node(expected.get_root_id()).content {
        NodeValue::Primitive(primitive) => Ok(primitive),
        _ => Err(unsupported_complex_literal_key()),
    }
}

fn parse_bigint(value: &str) -> Result<BigInt, SerError> {
    value
        .parse()
        .map_err(|_| SerError::Custom(format!("invalid integer map key: {value}")))
}

fn object_key_type_mismatch(expected: &str, key: &ObjectKey) -> SerError {
    SerError::Custom(format!(
        "type mismatch: expected {expected} map key, got {}",
        object_key_type(key)
    ))
}

fn object_key_type(key: &ObjectKey) -> &'static str {
    match key {
        ObjectKey::String(_) => "text",
        ObjectKey::Number(_) => "integer",
        ObjectKey::Tuple(_) => "tuple",
    }
}

fn literal_mismatch() -> SerError {
    SerError::Custom("literal mismatch".to_string())
}

fn unsupported_complex_map_keys() -> SerError {
    SerError::Custom("complex map keys are unsupported in serde-eure v1".to_string())
}

fn unsupported_complex_literal_key() -> SerError {
    SerError::Custom("complex literal map keys are unsupported in serde-eure v1".to_string())
}

fn serialize_bigint<S: serde::Serializer>(ser: S, value: &BigInt) -> Result<S::Ok, S::Error> {
    if let Ok(value) = i64::try_from(value) {
        ser.serialize_i64(value)
    } else if let Ok(value) = u64::try_from(value) {
        ser.serialize_u64(value)
    } else if let Ok(value) = i128::try_from(value) {
        ser.serialize_i128(value)
    } else if let Ok(value) = u128::try_from(value) {
        ser.serialize_u128(value)
    } else {
        Err(S::Error::custom(SerError::BigIntOutOfRange.to_string()))
    }
}

fn extract_variant_path(doc: &EureDocument, node_id: NodeId) -> Result<VariantPath, SerError> {
    let node = doc.node(node_id);
    let Some(variant_id) = node.extensions.get(&Identifier::VARIANT).copied() else {
        return Err(SerError::Custom("no variant matched".to_string()));
    };
    let variant_node = doc.node(variant_id);
    let value = match &variant_node.content {
        NodeValue::Primitive(PrimitiveValue::Text(text)) => text.as_str(),
        other => {
            return Err(SerError::Custom(format!(
                "type mismatch: expected text, got {}",
                node_value_type(other)
            )));
        }
    };
    VariantPath::parse(value)
        .map_err(|_| SerError::Custom(format!("invalid variant name: {value}")))
}

fn resolve_schema_node_id(
    schema: &SchemaDocument,
    mut schema_node_id: SchemaNodeId,
) -> Result<SchemaNodeId, SerError> {
    for _ in 0..schema.nodes.len() {
        match &schema.node(schema_node_id).content {
            SchemaNodeContent::Reference(type_ref) => {
                if let Some(namespace) = &type_ref.namespace {
                    return Err(SerError::Custom(format!(
                        "cross-schema references are unsupported: {namespace}.{}",
                        type_ref.name
                    )));
                }
                schema_node_id = schema.types.get(&type_ref.name).copied().ok_or_else(|| {
                    SerError::Custom(format!("undefined type reference: {}", type_ref.name))
                })?;
            }
            _ => return Ok(schema_node_id),
        }
    }

    Err(SerError::Custom(
        "schema reference cycle detected".to_string(),
    ))
}

fn node_value_type(value: &NodeValue) -> &'static str {
    match value {
        NodeValue::Hole(_) => "hole",
        NodeValue::Primitive(PrimitiveValue::Null) => "null",
        NodeValue::Primitive(PrimitiveValue::Bool(_)) => "boolean",
        NodeValue::Primitive(PrimitiveValue::Integer(_)) => "integer",
        NodeValue::Primitive(PrimitiveValue::F32(_))
        | NodeValue::Primitive(PrimitiveValue::F64(_)) => "float",
        NodeValue::Primitive(PrimitiveValue::Text(_)) => "text",
        NodeValue::Array(_) => "array",
        NodeValue::Map(_) => "map",
        NodeValue::Tuple(_) => "tuple",
        NodeValue::PartialMap(_) => "partial-map",
    }
}

#[cfg(test)]
mod tests {
    use eure::eure;
    use eure_schema::TextSchema;
    use eure_schema::interop::UnionInterop;
    use eure_schema::{
        IntegerSchema, RecordFieldSchema, RecordSchema, SchemaDocument, SchemaNodeContent,
    };
    use serde_json::json;

    use super::to_serializer;

    fn make_record_schema() -> SchemaDocument {
        let mut schema = SchemaDocument::new();
        let text_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let record_id = schema.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [
                (
                    "name".to_string(),
                    RecordFieldSchema {
                        schema: text_id,
                        optional: false,
                        binding_style: None,
                        field_codegen: Default::default(),
                    },
                ),
                (
                    "age".to_string(),
                    RecordFieldSchema {
                        schema: int_id,
                        optional: false,
                        binding_style: None,
                        field_codegen: Default::default(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            flatten: Vec::new(),
            unknown_fields: eure_schema::UnknownFieldsPolicy::Deny,
        }));
        schema.root = record_id;
        let _ = UnionInterop::default();
        schema
    }

    #[test]
    fn serializes_simple_record_to_json() {
        let schema = make_record_schema();
        let doc = eure!({
            name = "Alice",
            age = 30,
        });

        let actual = to_serializer(serde_json::value::Serializer, &doc, &schema).unwrap();
        assert_eq!(actual, json!({ "name": "Alice", "age": 30 }));
    }
}
