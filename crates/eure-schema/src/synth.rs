//! Type Synthesis for Eure Documents
//!
//! This module infers types from Eure document values without requiring a schema.
//! The synthesized types can be used for:
//! - Generating schema definitions from example data
//! - Type checking across multiple files
//! - Editor tooling (hover types, completions)
//!
//! # Example
//!
//! ```rust,ignore
//! use eure_document::document::{EureDocument, NodeId};
//! use eure_schema::synth::{synth, SynthType};
//!
//! let doc = eure!({ name = "Alice", age = 30 });
//! let ty = synth(&doc, doc.get_root_id());
//! // ty = Record { name: Text, age: Integer }
//! ```
//!
//! # Unification
//!
//! When synthesizing arrays, element types are unified:
//!
//! ```rust,ignore
//! // [ { a = 1 }, { a = "x", b = "y" } ]
//! // Result: Array<{ a: Integer } | { a: Text, b: Text }>
//! ```
//!
//! Holes are absorbed during unification:
//!
//! ```rust,ignore
//! // [1, !, 3]
//! // Result: Array<Integer>  (not Array<Integer | Hole>)
//! ```

mod types;
mod unify;

pub use types::*;
pub use unify::unify;

use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::text::Language;
use eure_document::value::{ObjectKey, PrimitiveValue};

/// Synthesize a type from a document node.
///
/// This function recursively traverses the document structure and infers
/// the most specific type for each value.
///
/// # Arguments
///
/// * `doc` - The Eure document containing the node
/// * `node_id` - The node to synthesize a type for
///
/// # Returns
///
/// The synthesized type for the node
pub fn synth(doc: &EureDocument, node_id: NodeId) -> SynthType {
    let node = doc.node(node_id);

    match &node.content {
        NodeValue::Hole(ident) => SynthType::Hole(ident.clone()),

        NodeValue::Primitive(prim) => synth_primitive(prim),

        NodeValue::Array(arr) => {
            if arr.is_empty() {
                SynthType::Array(Box::new(SynthType::Any))
            } else {
                let element_types: Vec<_> = arr.iter().map(|&id| synth(doc, id)).collect();
                let unified = element_types
                    .into_iter()
                    .reduce(unify)
                    .unwrap_or(SynthType::Any);
                SynthType::Array(Box::new(unified))
            }
        }

        NodeValue::Tuple(tuple) => {
            let element_types: Vec<_> = tuple.iter().map(|&id| synth(doc, id)).collect();
            SynthType::Tuple(element_types)
        }

        NodeValue::Map(map) => {
            if map.is_empty() {
                SynthType::Record(SynthRecord::empty())
            } else {
                let mut fields = Vec::with_capacity(map.len());
                for (key, &value_id) in map.iter() {
                    let field_name = object_key_to_field_name(key);
                    let field_type = synth(doc, value_id);
                    fields.push((field_name, SynthField::required(field_type)));
                }
                SynthType::Record(SynthRecord::new(fields))
            }
        }
    }
}

/// Synthesize type for a primitive value
fn synth_primitive(prim: &PrimitiveValue) -> SynthType {
    match prim {
        PrimitiveValue::Null => SynthType::Null,
        PrimitiveValue::Bool(_) => SynthType::Boolean,
        PrimitiveValue::Integer(_) => SynthType::Integer,
        PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => SynthType::Float,
        PrimitiveValue::Text(text) => SynthType::Text(synth_text_language(&text.language)),
    }
}

/// Extract language from Text value
fn synth_text_language(lang: &Language) -> Option<String> {
    match lang {
        Language::Implicit => None,
        Language::Plaintext => Some("plaintext".to_string()),
        Language::Other(lang) => Some(lang.to_string()),
    }
}

/// Convert ObjectKey to a field name string
///
/// For string keys, returns the raw string.
/// For other key types, uses the Display representation.
fn object_key_to_field_name(key: &ObjectKey) -> String {
    match key {
        ObjectKey::String(s) => s.clone(),
        ObjectKey::Number(n) => n.to_string(),
        ObjectKey::Tuple(t) => format!("{:?}", t), // Fallback for tuple keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::document::node::{NodeArray, NodeMap};
    use eure_document::eure;
    use eure_document::text::Text;
    use eure_document::value::ObjectKey;
    use num_bigint::BigInt;

    #[test]
    fn test_synth_primitives() {
        let doc = EureDocument::new_primitive(PrimitiveValue::Null);
        assert_eq!(synth(&doc, doc.get_root_id()), SynthType::Null);

        let doc = EureDocument::new_primitive(PrimitiveValue::Bool(true));
        assert_eq!(synth(&doc, doc.get_root_id()), SynthType::Boolean);

        let doc = EureDocument::new_primitive(PrimitiveValue::Integer(BigInt::from(42)));
        assert_eq!(synth(&doc, doc.get_root_id()), SynthType::Integer);

        let doc = EureDocument::new_primitive(PrimitiveValue::F64(2.5));
        assert_eq!(synth(&doc, doc.get_root_id()), SynthType::Float);

        let doc = EureDocument::new_primitive(PrimitiveValue::Text(Text::plaintext("hello")));
        assert_eq!(
            synth(&doc, doc.get_root_id()),
            SynthType::Text(Some("plaintext".to_string()))
        );
    }

    #[test]
    fn test_synth_empty_array() {
        let doc = eure!({ arr = [] });
        let root = doc.node(doc.get_root_id());
        let arr_id = root
            .as_map()
            .unwrap()
            .get_node_id(&ObjectKey::String("arr".into()))
            .unwrap();
        assert_eq!(
            synth(&doc, arr_id),
            SynthType::Array(Box::new(SynthType::Any))
        );
    }

    #[test]
    fn test_synth_homogeneous_array() {
        let doc = eure!({ arr = [1, 2, 3] });
        let root = doc.node(doc.get_root_id());
        let arr_id = root
            .as_map()
            .unwrap()
            .get_node_id(&ObjectKey::String("arr".into()))
            .unwrap();
        assert_eq!(
            synth(&doc, arr_id),
            SynthType::Array(Box::new(SynthType::Integer))
        );
    }

    #[test]
    fn test_synth_heterogeneous_array() {
        let doc = eure!({ arr = [1, "hello"] });
        let root = doc.node(doc.get_root_id());
        let arr_id = root
            .as_map()
            .unwrap()
            .get_node_id(&ObjectKey::String("arr".into()))
            .unwrap();
        assert_eq!(
            synth(&doc, arr_id),
            SynthType::Array(Box::new(SynthType::Union(SynthUnion {
                variants: vec![
                    SynthType::Integer,
                    SynthType::Text(Some("plaintext".to_string()))
                ]
            })))
        );
    }

    #[test]
    fn test_synth_tuple() {
        let doc = eure!({ tup = (1, "hello", true) });
        let root = doc.node(doc.get_root_id());
        let tup_id = root
            .as_map()
            .unwrap()
            .get_node_id(&ObjectKey::String("tup".into()))
            .unwrap();
        assert_eq!(
            synth(&doc, tup_id),
            SynthType::Tuple(vec![
                SynthType::Integer,
                SynthType::Text(Some("plaintext".to_string())),
                SynthType::Boolean,
            ])
        );
    }

    #[test]
    fn test_synth_record() {
        let doc = eure!({
            name = "Alice"
            age = 30
        });
        let ty = synth(&doc, doc.get_root_id());
        let expected = SynthType::Record(SynthRecord::new([
            (
                "name".to_string(),
                SynthField::required(SynthType::Text(Some("plaintext".to_string()))),
            ),
            ("age".to_string(), SynthField::required(SynthType::Integer)),
        ]));
        assert_eq!(ty, expected);
    }

    #[test]
    fn test_synth_array_of_records_union() {
        // Build [ { a = 1 }, { a = "x", b = "y" } ] programmatically
        let mut doc = EureDocument::new_empty();

        // Create first record: { a = 1 }
        let a1_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(
            1,
        ))));
        let rec1_id = doc.create_node(NodeValue::Map(NodeMap::from_iter([(
            ObjectKey::String("a".into()),
            a1_id,
        )])));

        // Create second record: { a = "x", b = "y" }
        let a2_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(
            "x",
        ))));
        let b2_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(
            "y",
        ))));
        let rec2_id = doc.create_node(NodeValue::Map(NodeMap::from_iter([
            (ObjectKey::String("a".into()), a2_id),
            (ObjectKey::String("b".into()), b2_id),
        ])));

        // Create array
        let arr_id = doc.create_node(NodeValue::Array(NodeArray::from_vec(vec![
            rec1_id, rec2_id,
        ])));

        let ty = synth(&doc, arr_id);

        // Different shapes form a union of records
        let expected = SynthType::Array(Box::new(SynthType::Union(SynthUnion {
            variants: vec![
                SynthType::Record(SynthRecord::new([(
                    "a".to_string(),
                    SynthField::required(SynthType::Integer),
                )])),
                SynthType::Record(SynthRecord::new([
                    (
                        "a".to_string(),
                        SynthField::required(SynthType::Text(Some("plaintext".to_string()))),
                    ),
                    (
                        "b".to_string(),
                        SynthField::required(SynthType::Text(Some("plaintext".to_string()))),
                    ),
                ])),
            ],
        })));
        assert_eq!(ty, expected);
    }

    #[test]
    fn test_synth_nested() {
        let doc = eure!({
            items = [1, 2]
            meta {
                count = 2
            }
        });
        let ty = synth(&doc, doc.get_root_id());
        let expected = SynthType::Record(SynthRecord::new([
            (
                "items".to_string(),
                SynthField::required(SynthType::Array(Box::new(SynthType::Integer))),
            ),
            (
                "meta".to_string(),
                SynthField::required(SynthType::Record(SynthRecord::new([(
                    "count".to_string(),
                    SynthField::required(SynthType::Integer),
                )]))),
            ),
        ]));
        assert_eq!(ty, expected);
    }

    #[test]
    fn test_synth_hole_absorbed() {
        // Build [1, !, 3] programmatically (hole should be absorbed)
        let mut doc = EureDocument::new_empty();
        let i1 = doc.create_node(NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(
            1,
        ))));
        let hole = doc.create_node(NodeValue::Hole(None));
        let i3 = doc.create_node(NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(
            3,
        ))));
        let arr_id = doc.create_node(NodeValue::Array(NodeArray::from_vec(vec![i1, hole, i3])));

        assert_eq!(
            synth(&doc, arr_id),
            SynthType::Array(Box::new(SynthType::Integer))
        );
    }

    #[test]
    fn test_synth_same_shape_records_merge() {
        // Build [ { a = 1 }, { a = "x" } ] - same shape, different field types
        let mut doc = EureDocument::new_empty();

        let a1_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(
            1,
        ))));
        let rec1_id = doc.create_node(NodeValue::Map(NodeMap::from_iter([(
            ObjectKey::String("a".into()),
            a1_id,
        )])));

        let a2_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(
            "x",
        ))));
        let rec2_id = doc.create_node(NodeValue::Map(NodeMap::from_iter([(
            ObjectKey::String("a".into()),
            a2_id,
        )])));

        let arr_id = doc.create_node(NodeValue::Array(NodeArray::from_vec(vec![
            rec1_id, rec2_id,
        ])));

        // Same shape records should merge, resulting in Record { a: Integer | Text }
        let expected = SynthType::Array(Box::new(SynthType::Record(SynthRecord::new([(
            "a".to_string(),
            SynthField::required(SynthType::Union(SynthUnion {
                variants: vec![
                    SynthType::Integer,
                    SynthType::Text(Some("plaintext".to_string())),
                ],
            })),
        )]))));
        assert_eq!(synth(&doc, arr_id), expected);
    }
}
