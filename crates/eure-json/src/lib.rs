#![doc = include_str!("../README.md")]

mod config;
mod error;

pub use config::Config;
pub use error::EureToJsonError;
use eure::data_model::VariantRepr;
use eure::document::node::NodeValue;
use eure::document::{EureDocument, NodeId};
use eure::value::{ObjectKey, PrimitiveValue};
use serde_json::Value as JsonValue;

pub fn document_to_value(
    doc: &EureDocument,
    config: &Config,
) -> Result<JsonValue, EureToJsonError> {
    let root_id = doc.get_root_id();
    convert_node(doc, root_id, config)
}

fn convert_node(
    doc: &EureDocument,
    node_id: NodeId,
    config: &Config,
) -> Result<JsonValue, EureToJsonError> {
    let node = doc.node(node_id);

    // Check for $variant extension
    let variant_ext: Option<&str> = node
        .extensions
        .iter()
        .find(|(k, _)| k.as_ref() == "variant")
        .and_then(|(_, &ext_id)| {
            if let NodeValue::Primitive(PrimitiveValue::Text(t)) = &doc.node(ext_id).content {
                Some(t.as_str())
            } else {
                None
            }
        });

    // If this node has a $variant extension, handle it as a variant
    if let Some(tag) = variant_ext {
        return convert_variant_node(doc, node_id, tag, config);
    }

    match &node.content {
        NodeValue::Hole(_) => Err(EureToJsonError::HoleNotSupported),
        NodeValue::Primitive(prim) => convert_primitive(prim),
        NodeValue::Array(arr) => {
            let mut result = Vec::new();
            for &child_id in &arr.0 {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Tuple(tuple) => {
            let mut result = Vec::new();
            for &child_id in &tuple.0 {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Map(map) => {
            let mut result = serde_json::Map::new();
            for (key, &child_id) in &map.0 {
                let key_string = convert_object_key(key)?;
                let value = convert_node(doc, child_id, config)?;
                result.insert(key_string, value);
            }
            Ok(JsonValue::Object(result))
        }
    }
}

fn convert_primitive(prim: &PrimitiveValue) -> Result<JsonValue, EureToJsonError> {
    match prim {
        PrimitiveValue::Null => Ok(JsonValue::Null),
        PrimitiveValue::Bool(b) => Ok(JsonValue::Bool(*b)),
        PrimitiveValue::Integer(bi) => {
            // Try to convert to i64 for JSON
            let i64_value = bi.to_string().parse::<i64>();
            if let Ok(i) = i64_value {
                return Ok(JsonValue::Number(i.into()));
            }

            // Try to convert to u64
            let u64_value = bi.to_string().parse::<u64>();
            if let Ok(u) = u64_value {
                return Ok(JsonValue::Number(u.into()));
            }

            Err(EureToJsonError::BigIntOutOfRange)
        }
        PrimitiveValue::F32(f) => {
            if let Some(num) = serde_json::Number::from_f64(*f as f64) {
                Ok(JsonValue::Number(num))
            } else {
                // NaN or infinity - not supported in JSON
                Err(EureToJsonError::NonFiniteFloat)
            }
        }
        PrimitiveValue::F64(f) => {
            if let Some(num) = serde_json::Number::from_f64(*f) {
                Ok(JsonValue::Number(num))
            } else {
                // NaN or infinity - not supported in JSON
                Err(EureToJsonError::NonFiniteFloat)
            }
        }
        PrimitiveValue::Text(text) => Ok(JsonValue::String(text.content.clone())),
    }
}

/// Convert a node that has a $variant extension
fn convert_variant_node(
    doc: &EureDocument,
    node_id: NodeId,
    tag: &str,
    config: &Config,
) -> Result<JsonValue, EureToJsonError> {
    // Convert the content (the node itself minus the $variant extension)
    let content_json = convert_node_content_only(doc, node_id, config)?;

    match &config.variant_repr {
        VariantRepr::External => {
            // {"variant-name": content}
            let mut map = serde_json::Map::new();
            map.insert(tag.to_string(), content_json);
            Ok(JsonValue::Object(map))
        }
        VariantRepr::Internal { tag: tag_field } => {
            // {"type": "variant-name", ...fields...}
            // Content must be an object to merge fields
            if let JsonValue::Object(mut content_map) = content_json {
                // Check if tag field already exists in content
                if content_map.contains_key(tag_field) {
                    return Err(EureToJsonError::VariantTagConflict {
                        tag: tag_field.clone(),
                    });
                }
                content_map.insert(tag_field.clone(), JsonValue::String(tag.to_string()));
                Ok(JsonValue::Object(content_map))
            } else {
                // If content is not an object, use External representation as fallback
                let mut map = serde_json::Map::new();
                map.insert(tag.to_string(), content_json);
                Ok(JsonValue::Object(map))
            }
        }
        VariantRepr::Adjacent {
            tag: tag_field,
            content: content_key,
        } => {
            // {"type": "variant-name", "content": {...}}
            // Check if tag and content keys are the same
            if tag_field == content_key {
                return Err(EureToJsonError::VariantAdjacentConflict {
                    field: tag_field.clone(),
                });
            }
            let mut map = serde_json::Map::new();
            map.insert(tag_field.clone(), JsonValue::String(tag.to_string()));
            map.insert(content_key.clone(), content_json);
            Ok(JsonValue::Object(map))
        }
        VariantRepr::Untagged => {
            // Just the content without variant information
            Ok(content_json)
        }
    }
}

/// Convert a node's content without checking for $variant extension (to avoid infinite recursion)
fn convert_node_content_only(
    doc: &EureDocument,
    node_id: NodeId,
    config: &Config,
) -> Result<JsonValue, EureToJsonError> {
    let node = doc.node(node_id);

    match &node.content {
        NodeValue::Hole(_) => Err(EureToJsonError::HoleNotSupported),
        NodeValue::Primitive(prim) => convert_primitive(prim),
        NodeValue::Array(arr) => {
            let mut result = Vec::new();
            for &child_id in &arr.0 {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Tuple(tuple) => {
            let mut result = Vec::new();
            for &child_id in &tuple.0 {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Map(map) => {
            let mut result = serde_json::Map::new();
            for (key, &child_id) in &map.0 {
                let key_string = convert_object_key(key)?;
                let value = convert_node(doc, child_id, config)?;
                result.insert(key_string, value);
            }
            Ok(JsonValue::Object(result))
        }
    }
}

fn convert_object_key(key: &ObjectKey) -> Result<String, EureToJsonError> {
    match key {
        ObjectKey::Bool(b) => Ok(b.to_string()),
        ObjectKey::Number(n) => Ok(n.to_string()),
        ObjectKey::String(s) => Ok(s.clone()),
        ObjectKey::Tuple(tuple) => {
            let mut parts = Vec::new();
            for item in &tuple.0 {
                parts.push(convert_object_key(item)?);
            }
            Ok(format!("({})", parts.join(", ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure::data_model::VariantRepr;
    use eure::document::node::NodeValue;
    use eure::value::{ObjectKey, PrimitiveValue};
    use eure_document::identifier::Identifier;
    use eure_document::text::Text;
    use serde_json::json;

    /// Helper to create a document with $variant extension
    fn create_variant_doc(tag: &str, content: NodeValue) -> EureDocument {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.node_mut(root).content = content;

        // Add $variant extension
        let variant_ident: Identifier = "variant".parse().unwrap();
        let ext_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext(tag.to_string()),
        )));
        doc.node_mut(root)
            .extensions
            .insert(variant_ident, ext_node);
        doc
    }

    // Test primitives
    #[test]
    fn test_null_conversion() {
        let doc = EureDocument::new_primitive(PrimitiveValue::Null);
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(null));
    }

    #[test]
    fn test_bool_true_conversion() {
        let doc = EureDocument::new_primitive(PrimitiveValue::Bool(true));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(true));
    }

    #[test]
    fn test_bool_false_conversion() {
        let doc = EureDocument::new_primitive(PrimitiveValue::Bool(false));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(false));
    }

    #[test]
    fn test_bigint_small_conversion() {
        use num_bigint::BigInt;
        let doc = EureDocument::new_primitive(PrimitiveValue::Integer(BigInt::from(42)));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn test_bigint_negative_conversion() {
        use num_bigint::BigInt;
        let doc = EureDocument::new_primitive(PrimitiveValue::Integer(BigInt::from(-42)));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(-42));
    }

    #[test]
    fn test_f32_conversion() {
        let doc = EureDocument::new_primitive(PrimitiveValue::F32(1.5));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(1.5));
    }

    #[test]
    fn test_f64_conversion() {
        let doc = EureDocument::new_primitive(PrimitiveValue::F64(2.5));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(2.5));
    }

    #[test]
    fn test_f32_nan_error() {
        let doc = EureDocument::new_primitive(PrimitiveValue::F32(f32::NAN));
        let config = Config::default();
        let result = document_to_value(&doc, &config);
        assert_eq!(result, Err(EureToJsonError::NonFiniteFloat));
    }

    #[test]
    fn test_f32_infinity_error() {
        let doc = EureDocument::new_primitive(PrimitiveValue::F32(f32::INFINITY));
        let config = Config::default();
        let result = document_to_value(&doc, &config);
        assert_eq!(result, Err(EureToJsonError::NonFiniteFloat));
    }

    #[test]
    fn test_f64_nan_error() {
        let doc = EureDocument::new_primitive(PrimitiveValue::F64(f64::NAN));
        let config = Config::default();
        let result = document_to_value(&doc, &config);
        assert_eq!(result, Err(EureToJsonError::NonFiniteFloat));
    }

    #[test]
    fn test_f64_infinity_error() {
        let doc = EureDocument::new_primitive(PrimitiveValue::F64(f64::INFINITY));
        let config = Config::default();
        let result = document_to_value(&doc, &config);
        assert_eq!(result, Err(EureToJsonError::NonFiniteFloat));
    }

    #[test]
    fn test_f64_neg_infinity_error() {
        let doc = EureDocument::new_primitive(PrimitiveValue::F64(f64::NEG_INFINITY));
        let config = Config::default();
        let result = document_to_value(&doc, &config);
        assert_eq!(result, Err(EureToJsonError::NonFiniteFloat));
    }

    #[test]
    fn test_text_plaintext_conversion() {
        let text = Text::plaintext("hello world".to_string());
        let doc = EureDocument::new_primitive(PrimitiveValue::Text(text));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!("hello world"));
    }

    #[test]
    fn test_text_with_language_conversion() {
        use eure_document::text::{Language, Text};
        let text = Text::new(
            "fn main() {}".to_string(),
            Language::Other("rust".to_string()),
        );
        let doc = EureDocument::new_primitive(PrimitiveValue::Text(text));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!("fn main() {}"));
    }

    #[test]
    fn test_text_implicit_conversion() {
        use eure_document::text::{Language, Text};
        let text = Text::new("print('hello')".to_string(), Language::Implicit);
        let doc = EureDocument::new_primitive(PrimitiveValue::Text(text));
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!("print('hello')"));
    }

    #[test]
    fn test_hole_error() {
        let mut doc = EureDocument::new();
        doc.node_mut(doc.get_root_id()).content = NodeValue::hole();
        let config = Config::default();
        let result = document_to_value(&doc, &config);
        assert_eq!(result, Err(EureToJsonError::HoleNotSupported));
    }

    #[test]
    fn test_uninitialized_error() {
        let doc = EureDocument::new();
        let config = Config::default();
        let result = document_to_value(&doc, &config);
        assert_eq!(result, Err(EureToJsonError::HoleNotSupported));
    }

    // Test variant conversions (using $variant extension)
    #[test]
    fn test_variant_external() {
        let doc = create_variant_doc("Success", NodeValue::Primitive(PrimitiveValue::Bool(true)));
        let config = Config {
            variant_repr: VariantRepr::External,
        };
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!({"Success": true}));
    }

    #[test]
    fn test_variant_untagged() {
        let doc = create_variant_doc("Success", NodeValue::Primitive(PrimitiveValue::Bool(true)));
        let config = Config {
            variant_repr: VariantRepr::Untagged,
        };
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!(true));
    }

    #[test]
    fn test_variant_internal_with_map_content() {
        use num_bigint::BigInt;

        // Create a document with a map content and $variant extension
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();

        // Add field to the map
        let field_node = doc
            .add_map_child(ObjectKey::String("field".to_string()), root)
            .unwrap()
            .node_id;
        doc.node_mut(field_node).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(42)));

        // Add $variant extension
        let variant_ident: Identifier = "variant".parse().unwrap();
        let ext_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext("Success".to_string()),
        )));
        doc.node_mut(root)
            .extensions
            .insert(variant_ident, ext_node);

        let config = Config {
            variant_repr: VariantRepr::Internal {
                tag: "type".to_string(),
            },
        };
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!({"type": "Success", "field": 42}));
    }

    #[test]
    fn test_variant_internal_tag_conflict() {
        // Create a document with a map that has a "type" field already
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();

        // Add "type" field to the map
        let type_node = doc
            .add_map_child(ObjectKey::String("type".to_string()), root)
            .unwrap()
            .node_id;
        doc.node_mut(type_node).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        // Add $variant extension
        let variant_ident: Identifier = "variant".parse().unwrap();
        let ext_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext("Success".to_string()),
        )));
        doc.node_mut(root)
            .extensions
            .insert(variant_ident, ext_node);

        let config = Config {
            variant_repr: VariantRepr::Internal {
                tag: "type".to_string(),
            },
        };
        let result = document_to_value(&doc, &config);
        assert_eq!(
            result,
            Err(EureToJsonError::VariantTagConflict {
                tag: "type".to_string()
            })
        );
    }

    #[test]
    fn test_variant_adjacent() {
        let doc = create_variant_doc("Success", NodeValue::Primitive(PrimitiveValue::Bool(true)));
        let config = Config {
            variant_repr: VariantRepr::Adjacent {
                tag: "tag".to_string(),
                content: "content".to_string(),
            },
        };
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!({"tag": "Success", "content": true}));
    }

    #[test]
    fn test_variant_adjacent_key_conflict() {
        let doc = create_variant_doc("Success", NodeValue::Primitive(PrimitiveValue::Bool(true)));
        let config = Config {
            variant_repr: VariantRepr::Adjacent {
                tag: "data".to_string(),
                content: "data".to_string(), // Same key!
            },
        };
        let result = document_to_value(&doc, &config);
        assert_eq!(
            result,
            Err(EureToJsonError::VariantAdjacentConflict {
                field: "data".to_string()
            })
        );
    }

    // Test arrays
    #[test]
    fn test_empty_array() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.node_mut(root).content = NodeValue::empty_array();
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_array_with_primitives() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.node_mut(root).content = NodeValue::empty_array();

        let child1 = doc.create_node(NodeValue::Primitive(PrimitiveValue::Bool(true)));
        let child2 = doc.create_node(NodeValue::Primitive(PrimitiveValue::Null));

        if let NodeValue::Array(ref mut arr) = doc.node_mut(root).content {
            arr.0.push(child1);
            arr.0.push(child2);
        }

        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!([true, null]));
    }

    // Test tuples
    #[test]
    fn test_empty_tuple() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.node_mut(root).content = NodeValue::empty_tuple();
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_tuple_with_primitives() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.node_mut(root).content = NodeValue::empty_tuple();

        let child1 = doc.create_node(NodeValue::Primitive(PrimitiveValue::Bool(true)));
        let child2 = doc.create_node(NodeValue::Primitive(PrimitiveValue::Null));

        if let NodeValue::Tuple(ref mut tuple) = doc.node_mut(root).content {
            tuple.0.push(child1);
            tuple.0.push(child2);
        }

        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!([true, null]));
    }

    // Test maps
    #[test]
    fn test_empty_map() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.node_mut(root).content = NodeValue::empty_map();
        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_map_with_string_keys() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();

        let child = doc
            .add_map_child(ObjectKey::String("key".to_string()), root)
            .unwrap()
            .node_id;
        doc.node_mut(child).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!({"key": true}));
    }

    #[test]
    fn test_map_with_number_key() {
        use num_bigint::BigInt;
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();

        let child = doc
            .add_map_child(ObjectKey::Number(BigInt::from(42)), root)
            .unwrap()
            .node_id;
        doc.node_mut(child).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!({"42": true}));
    }

    #[test]
    fn test_map_with_bool_key() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();

        let child = doc
            .add_map_child(ObjectKey::Bool(true), root)
            .unwrap()
            .node_id;
        doc.node_mut(child).content = NodeValue::Primitive(PrimitiveValue::Null);

        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!({"true": null}));
    }

    // Test nested structures
    #[test]
    fn test_nested_map_in_array() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();
        doc.node_mut(root).content = NodeValue::empty_array();

        let map_node = doc.create_node(NodeValue::empty_map());
        let value_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Bool(true)));

        if let NodeValue::Map(ref mut map) = doc.node_mut(map_node).content {
            map.0
                .insert(ObjectKey::String("nested".to_string()), value_node);
        }

        if let NodeValue::Array(ref mut arr) = doc.node_mut(root).content {
            arr.0.push(map_node);
        }

        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        assert_eq!(result, json!([{"nested": true}]));
    }

    // Test extensions are ignored
    #[test]
    fn test_extensions_ignored() {
        use eure_document::identifier::Identifier;
        let mut doc = EureDocument::new_primitive(PrimitiveValue::Bool(true));
        let root = doc.get_root_id();

        // Add an extension
        let ext_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Null));
        let ext_ident: Identifier = "ext".parse().unwrap();
        doc.node_mut(root).extensions.insert(ext_ident, ext_node);

        let config = Config::default();
        let result = document_to_value(&doc, &config).unwrap();
        // Extensions should be ignored, only the content should be converted
        assert_eq!(result, json!(true));
    }
}
