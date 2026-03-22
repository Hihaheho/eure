//! Helpers for validating map/object keys against schema nodes.

use eure_document::document::EureDocument;
use eure_document::document::node::NodeValue;
use eure_document::value::{ObjectKey, PrimitiveValue};
use num_bigint::BigInt;

use crate::{Bound, IntegerSchema, SchemaNodeContent, SchemaNodeId, TextSchema, TypeReference};

use super::context::ValidationContext;

const KEY_SCHEMA_RECURSION_LIMIT: usize = 100;

pub(super) fn key_matches_schema(
    ctx: &ValidationContext<'_>,
    key: &ObjectKey,
    schema_id: SchemaNodeId,
) -> bool {
    key_matches_schema_inner(ctx, key, schema_id, 0)
}

fn key_matches_schema_inner(
    ctx: &ValidationContext<'_>,
    key: &ObjectKey,
    schema_id: SchemaNodeId,
    depth: usize,
) -> bool {
    if depth >= KEY_SCHEMA_RECURSION_LIMIT {
        return false;
    }

    match ctx.resolve_schema_content(schema_id) {
        SchemaNodeContent::Any => true,
        SchemaNodeContent::Text(schema) => matches_text_key(key, schema),
        SchemaNodeContent::Integer(schema) => matches_integer_key(key, schema),
        SchemaNodeContent::Boolean => matches_boolean_key(key),
        SchemaNodeContent::Tuple(schema) => matches_tuple_key(ctx, key, schema, depth + 1),
        SchemaNodeContent::Literal(expected) => key_matches_literal(key, expected),
        SchemaNodeContent::Union(schema) => {
            // A Union-typed map key is valid if the key string matches a variant name,
            // or if the key matches any variant's schema recursively.
            let name_match =
                matches!(key, ObjectKey::String(s) if schema.variants.contains_key(s.as_str()));
            name_match
                || schema
                    .variants
                    .values()
                    .copied()
                    .any(|variant_id| key_matches_schema_inner(ctx, key, variant_id, depth + 1))
        }
        SchemaNodeContent::Reference(type_ref) => resolve_local_reference(ctx, type_ref)
            .is_some_and(|resolved_id| key_matches_schema_inner(ctx, key, resolved_id, depth + 1)),
        SchemaNodeContent::Float(_)
        | SchemaNodeContent::Null
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Record(_) => false,
    }
}

fn resolve_local_reference(
    ctx: &ValidationContext<'_>,
    type_ref: &TypeReference,
) -> Option<SchemaNodeId> {
    if type_ref.namespace.is_some() {
        return None;
    }

    ctx.schema.types.get(&type_ref.name).copied()
}

fn matches_text_key(key: &ObjectKey, schema: &TextSchema) -> bool {
    let ObjectKey::String(value) = key else {
        return false;
    };

    let len = value.chars().count();
    if let Some(min) = schema.min_length
        && len < min as usize
    {
        return false;
    }
    if let Some(max) = schema.max_length
        && len > max as usize
    {
        return false;
    }
    if let Some(regex) = &schema.pattern
        && !regex.is_match(value)
    {
        return false;
    }

    true
}

fn matches_integer_key(key: &ObjectKey, schema: &IntegerSchema) -> bool {
    let ObjectKey::Number(value) = key else {
        return false;
    };

    if !integer_in_range(value, schema) {
        return false;
    }

    if let Some(divisor) = &schema.multiple_of
        && value % divisor != BigInt::from(0)
    {
        return false;
    }

    true
}

fn integer_in_range(value: &BigInt, schema: &IntegerSchema) -> bool {
    match (&schema.min, &schema.max) {
        (Bound::Unbounded, Bound::Unbounded) => true,
        (Bound::Inclusive(min), Bound::Unbounded) => value >= min,
        (Bound::Exclusive(min), Bound::Unbounded) => value > min,
        (Bound::Unbounded, Bound::Inclusive(max)) => value <= max,
        (Bound::Unbounded, Bound::Exclusive(max)) => value < max,
        (Bound::Inclusive(min), Bound::Inclusive(max)) => value >= min && value <= max,
        (Bound::Inclusive(min), Bound::Exclusive(max)) => value >= min && value < max,
        (Bound::Exclusive(min), Bound::Inclusive(max)) => value > min && value <= max,
        (Bound::Exclusive(min), Bound::Exclusive(max)) => value > min && value < max,
    }
}

fn matches_boolean_key(key: &ObjectKey) -> bool {
    matches!(
        key,
        ObjectKey::String(value) if value == "true" || value == "false"
    )
}

fn matches_tuple_key(
    ctx: &ValidationContext<'_>,
    key: &ObjectKey,
    schema: &crate::TupleSchema,
    depth: usize,
) -> bool {
    let ObjectKey::Tuple(items) = key else {
        return false;
    };

    if items.len() != schema.elements.len() {
        return false;
    }

    items
        .0
        .iter()
        .zip(schema.elements.iter().copied())
        .all(|(item, schema_id)| key_matches_schema_inner(ctx, item, schema_id, depth))
}

fn key_matches_literal(key: &ObjectKey, expected: &EureDocument) -> bool {
    key_matches_literal_node(key, expected, expected.get_root_id())
}

fn key_matches_literal_node(
    key: &ObjectKey,
    expected: &EureDocument,
    node_id: eure_document::document::NodeId,
) -> bool {
    match &expected.node(node_id).content {
        NodeValue::Primitive(PrimitiveValue::Integer(value)) => {
            matches!(key, ObjectKey::Number(actual) if actual == value)
        }
        NodeValue::Primitive(PrimitiveValue::Text(text)) => {
            matches!(key, ObjectKey::String(actual) if actual == text.as_str())
        }
        NodeValue::Primitive(PrimitiveValue::Bool(value)) => {
            matches!(key, ObjectKey::String(actual) if actual == if *value { "true" } else { "false" })
        }
        NodeValue::Tuple(tuple) => {
            let ObjectKey::Tuple(items) = key else {
                return false;
            };

            if items.len() != tuple.len() {
                return false;
            }

            items
                .0
                .iter()
                .zip(tuple.iter())
                .all(|(item, child_id)| key_matches_literal_node(item, expected, *child_id))
        }
        NodeValue::Primitive(PrimitiveValue::Null)
        | NodeValue::Primitive(PrimitiveValue::F32(_))
        | NodeValue::Primitive(PrimitiveValue::F64(_))
        | NodeValue::Array(_)
        | NodeValue::Map(_)
        | NodeValue::Hole(_) => false,
    }
}
