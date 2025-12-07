#![doc = include_str!("../README.md")]

mod config;
mod error;

pub use config::Config;
pub use error::{EureToJsonError, JsonToEureError};
use eure::data_model::VariantRepr;
use eure::document::node::NodeValue;
use eure::document::{EureDocument, NodeId};
use eure::value::{ObjectKey, PrimitiveValue};
use eure_document::identifier::Identifier;
use eure_document::text::Text;
use num_bigint::BigInt;
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

// ============================================================================
// JSON to EureDocument conversion
// ============================================================================

/// Convert a JSON value to an EureDocument.
///
/// # Variant Reconstruction
///
/// When `config.variant_repr` is set, the converter attempts to detect and reconstruct
/// variants with `$variant` extensions based on the JSON structure:
///
/// - **External** (`{"Tag": content}`): Single-key objects are interpreted as variants
///   where the key is the variant tag and the value is the content.
///
/// - **Internal** (`{"type": "Tag", ...fields}`): Objects containing the tag field
///   are interpreted as variants. The tag field is extracted and the remaining fields
///   become the variant content.
///
/// - **Adjacent** (`{"tag": "Tag", "content": ...}`): Objects with exactly the tag and
///   content fields are interpreted as variants.
///
/// - **Untagged**: Variant information is lost in this representation, so no variant
///   reconstruction is possible. Values are converted as plain data.
///
/// # Example
///
/// ```
/// use eure_json::{value_to_document, Config};
/// use serde_json::json;
///
/// let json = json!({"name": "Alice", "age": 30});
/// let doc = value_to_document(&json, &Config::default()).unwrap();
/// ```
pub fn value_to_document(
    value: &JsonValue,
    config: &Config,
) -> Result<EureDocument, JsonToEureError> {
    let mut doc = EureDocument::new();
    let root_id = doc.get_root_id();
    convert_json_to_node(&mut doc, root_id, value, config, true);
    Ok(doc)
}

/// Convert a JSON value and set it as the content of the given node.
///
/// `detect_variants` controls whether to attempt variant detection on objects.
/// This is set to `false` when converting variant content to prevent nested variant detection.
fn convert_json_to_node(
    doc: &mut EureDocument,
    node_id: NodeId,
    value: &JsonValue,
    config: &Config,
    detect_variants: bool,
) {
    match value {
        JsonValue::Null => {
            doc.node_mut(node_id).content = NodeValue::Primitive(PrimitiveValue::Null);
        }
        JsonValue::Bool(b) => {
            doc.node_mut(node_id).content = NodeValue::Primitive(PrimitiveValue::Bool(*b));
        }
        JsonValue::Number(n) => {
            // Try to convert to integer first, then fall back to float
            if let Some(i) = n.as_i64() {
                doc.node_mut(node_id).content =
                    NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(i)));
            } else if let Some(u) = n.as_u64() {
                doc.node_mut(node_id).content =
                    NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(u)));
            } else if let Some(f) = n.as_f64() {
                doc.node_mut(node_id).content = NodeValue::Primitive(PrimitiveValue::F64(f));
            }
        }
        JsonValue::String(s) => {
            doc.node_mut(node_id).content =
                NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(s.clone())));
        }
        JsonValue::Array(arr) => {
            doc.node_mut(node_id).content = NodeValue::empty_array();
            for item in arr {
                let child_id = doc.create_node(NodeValue::hole());
                convert_json_to_node(doc, child_id, item, config, detect_variants);
                if let NodeValue::Array(ref mut array) = doc.node_mut(node_id).content {
                    array.0.push(child_id);
                }
            }
        }
        JsonValue::Object(obj) => {
            // Try to detect variant based on config (only if detection is enabled)
            let detected = if detect_variants {
                detect_variant(obj, config)
            } else {
                None
            };

            if let Some((tag, content)) = detected {
                // This is a variant - set content and add $variant extension
                // Disable variant detection for the content to prevent nested detection
                convert_json_to_node(doc, node_id, &content, config, false);

                // Add $variant extension
                let variant_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
                    Text::plaintext(tag),
                )));
                let variant_ident: Identifier = "variant".parse().unwrap();
                doc.node_mut(node_id)
                    .extensions
                    .insert(variant_ident, variant_id);
            } else {
                // Regular object - convert as map
                doc.node_mut(node_id).content = NodeValue::empty_map();
                for (key, val) in obj {
                    let child_id = doc.create_node(NodeValue::hole());
                    convert_json_to_node(doc, child_id, val, config, detect_variants);
                    if let NodeValue::Map(ref mut map) = doc.node_mut(node_id).content {
                        map.0.insert(ObjectKey::String(key.clone()), child_id);
                    }
                }
            }
        }
    }
}

/// Detected variant information: (tag_name, content_value)
type DetectedVariant = (String, JsonValue);

/// Try to detect a variant in a JSON object based on the configured representation.
///
/// Returns `Some((tag, content))` if a variant is detected, `None` otherwise.
fn detect_variant(
    obj: &serde_json::Map<String, JsonValue>,
    config: &Config,
) -> Option<DetectedVariant> {
    match &config.variant_repr {
        VariantRepr::External => {
            // External: {"Tag": content} - single-key object
            if obj.len() == 1 {
                let (key, value) = obj.iter().next()?;
                // Heuristic: variant tags typically start with uppercase
                // But we accept any single-key object as a variant for round-trip fidelity
                Some((key.clone(), value.clone()))
            } else {
                None
            }
        }
        VariantRepr::Internal { tag } => {
            // Internal: {"type": "Tag", ...fields}
            let tag_value = obj.get(tag)?;
            let tag_str = tag_value.as_str()?;

            // Create content object without the tag field
            let mut content_obj = serde_json::Map::new();
            for (k, v) in obj {
                if k != tag {
                    content_obj.insert(k.clone(), v.clone());
                }
            }

            Some((tag_str.to_string(), JsonValue::Object(content_obj)))
        }
        VariantRepr::Adjacent { tag, content } => {
            // Adjacent: {"tag": "Tag", "content": {...}}
            // Must have exactly these two fields
            if obj.len() != 2 {
                return None;
            }

            let tag_value = obj.get(tag)?;
            let tag_str = tag_value.as_str()?;
            let content_value = obj.get(content)?;

            Some((tag_str.to_string(), content_value.clone()))
        }
        VariantRepr::Untagged => {
            // Untagged: variant information is lost, cannot reconstruct
            None
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

    // ========================================================================
    // JSON to Eure conversion tests
    // ========================================================================

    #[test]
    fn test_json_to_eure_null() {
        let json = json!(null);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();
        let expected = EureDocument::new_primitive(PrimitiveValue::Null);
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_bool_true() {
        let json = json!(true);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();
        let expected = EureDocument::new_primitive(PrimitiveValue::Bool(true));
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_bool_false() {
        let json = json!(false);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();
        let expected = EureDocument::new_primitive(PrimitiveValue::Bool(false));
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_integer() {
        use num_bigint::BigInt;
        let json = json!(42);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();
        let expected = EureDocument::new_primitive(PrimitiveValue::Integer(BigInt::from(42)));
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_negative_integer() {
        use num_bigint::BigInt;
        let json = json!(-42);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();
        let expected = EureDocument::new_primitive(PrimitiveValue::Integer(BigInt::from(-42)));
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_float() {
        let json = json!(3.14);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();
        let expected = EureDocument::new_primitive(PrimitiveValue::F64(3.14));
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_string() {
        let json = json!("hello world");
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();
        let expected =
            EureDocument::new_primitive(PrimitiveValue::Text(Text::plaintext("hello world")));
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_empty_array() {
        let json = json!([]);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();

        let mut expected = EureDocument::new();
        expected.node_mut(expected.get_root_id()).content = NodeValue::empty_array();
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_array_with_values() {
        let json = json!([1, true, "hello"]);
        let config = Config::default();
        let doc = value_to_document(&json, &config).unwrap();

        // Verify root is an array with 3 elements
        let root = doc.node(doc.get_root_id());
        let array = root.as_array().expect("Expected array");
        assert_eq!(array.0.len(), 3);

        // Verify element contents
        assert_eq!(
            doc.node(array.0[0]).content,
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(1)))
        );
        assert_eq!(
            doc.node(array.0[1]).content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        );
        assert_eq!(
            doc.node(array.0[2]).content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }

    #[test]
    fn test_json_to_eure_empty_object() {
        let json = json!({});
        let config = Config {
            variant_repr: VariantRepr::Untagged, // Disable variant detection
        };
        let doc = value_to_document(&json, &config).unwrap();

        let mut expected = EureDocument::new();
        expected.node_mut(expected.get_root_id()).content = NodeValue::empty_map();
        assert_eq!(doc, expected);
    }

    #[test]
    fn test_json_to_eure_object_with_fields() {
        let json = json!({"name": "Alice", "age": 30});
        let config = Config {
            variant_repr: VariantRepr::Untagged, // Disable variant detection
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify root is a map with 2 fields
        let root = doc.node(doc.get_root_id());
        let map = root.as_map().expect("Expected map");
        assert_eq!(map.0.len(), 2);

        // Verify field contents
        let name_id = map.get(&ObjectKey::String("name".to_string())).unwrap();
        assert_eq!(
            doc.node(name_id).content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("Alice")))
        );

        let age_id = map.get(&ObjectKey::String("age".to_string())).unwrap();
        assert_eq!(
            doc.node(age_id).content,
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(30)))
        );
    }

    #[test]
    fn test_json_to_eure_nested_structure() {
        let json = json!({
            "users": [
                {"name": "Alice"},
                {"name": "Bob"}
            ]
        });
        let config = Config {
            variant_repr: VariantRepr::Untagged, // Disable variant detection
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify structure: root -> users (array) -> [obj, obj]
        let root = doc.node(doc.get_root_id());
        let map = root.as_map().expect("Expected map");
        let users_id = map.get(&ObjectKey::String("users".to_string())).unwrap();
        let users = doc.node(users_id).as_array().expect("Expected array");
        assert_eq!(users.0.len(), 2);
    }

    // Variant detection tests

    #[test]
    fn test_json_to_eure_variant_external() {
        let json = json!({"Success": true});
        let config = Config {
            variant_repr: VariantRepr::External,
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify the document has $variant extension
        let root = doc.node(doc.get_root_id());
        let variant_ident: Identifier = "variant".parse().unwrap();
        let variant_id = root
            .extensions
            .get(&variant_ident)
            .expect("Expected $variant");
        assert_eq!(
            doc.node(*variant_id).content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("Success")))
        );

        // Verify content is bool true
        assert_eq!(
            root.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        );
    }

    #[test]
    fn test_json_to_eure_variant_internal() {
        let json = json!({"type": "Success", "value": 42});
        let config = Config {
            variant_repr: VariantRepr::Internal {
                tag: "type".to_string(),
            },
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify the document has $variant extension
        let root = doc.node(doc.get_root_id());
        let variant_ident: Identifier = "variant".parse().unwrap();
        let variant_id = root
            .extensions
            .get(&variant_ident)
            .expect("Expected $variant");
        assert_eq!(
            doc.node(*variant_id).content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("Success")))
        );

        // Verify content is a map with only "value" field (type field removed)
        let map = root.as_map().expect("Expected map");
        assert_eq!(map.0.len(), 1);
        assert!(map.get(&ObjectKey::String("value".to_string())).is_some());
        assert!(map.get(&ObjectKey::String("type".to_string())).is_none());
    }

    #[test]
    fn test_json_to_eure_variant_adjacent() {
        let json = json!({"tag": "Success", "content": {"value": 42}});
        let config = Config {
            variant_repr: VariantRepr::Adjacent {
                tag: "tag".to_string(),
                content: "content".to_string(),
            },
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify the document has $variant extension
        let root = doc.node(doc.get_root_id());
        let variant_ident: Identifier = "variant".parse().unwrap();
        let variant_id = root
            .extensions
            .get(&variant_ident)
            .expect("Expected $variant");
        assert_eq!(
            doc.node(*variant_id).content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("Success")))
        );

        // Verify content is a map with "value" field
        let map = root.as_map().expect("Expected map");
        assert_eq!(map.0.len(), 1);
        assert!(map.get(&ObjectKey::String("value".to_string())).is_some());
    }

    #[test]
    fn test_json_to_eure_variant_untagged_no_detection() {
        let json = json!({"Success": true});
        let config = Config {
            variant_repr: VariantRepr::Untagged,
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify no $variant extension (untagged doesn't detect variants)
        let root = doc.node(doc.get_root_id());
        let variant_ident: Identifier = "variant".parse().unwrap();
        assert!(root.extensions.get(&variant_ident).is_none());

        // Verify it's just a regular map
        let map = root.as_map().expect("Expected map");
        assert_eq!(map.0.len(), 1);
    }

    #[test]
    fn test_json_to_eure_roundtrip_primitives() {
        let config = Config::default();

        // Test roundtrip for various primitives
        let test_values = vec![
            json!(null),
            json!(true),
            json!(false),
            json!(42),
            json!(-123),
            json!(3.14),
            json!("hello"),
        ];

        for json_val in test_values {
            let doc = value_to_document(&json_val, &config).unwrap();
            let roundtrip = document_to_value(&doc, &config).unwrap();
            assert_eq!(json_val, roundtrip, "Roundtrip failed for {:?}", json_val);
        }
    }

    #[test]
    fn test_json_to_eure_roundtrip_array() {
        let config = Config {
            variant_repr: VariantRepr::Untagged, // Disable variant detection for clean roundtrip
        };
        let json_val = json!([1, 2, 3, "hello", true, null]);

        let doc = value_to_document(&json_val, &config).unwrap();
        let roundtrip = document_to_value(&doc, &config).unwrap();
        assert_eq!(json_val, roundtrip);
    }

    #[test]
    fn test_json_to_eure_roundtrip_nested_object() {
        let config = Config {
            variant_repr: VariantRepr::Untagged, // Disable variant detection for clean roundtrip
        };
        let json_val = json!({
            "name": "test",
            "nested": {
                "a": 1,
                "b": [true, false]
            }
        });

        let doc = value_to_document(&json_val, &config).unwrap();
        let roundtrip = document_to_value(&doc, &config).unwrap();
        assert_eq!(json_val, roundtrip);
    }

    #[test]
    fn test_json_to_eure_roundtrip_variant_external() {
        let config = Config {
            variant_repr: VariantRepr::External,
        };
        let json_val = json!({"Success": {"value": 42}});

        let doc = value_to_document(&json_val, &config).unwrap();
        let roundtrip = document_to_value(&doc, &config).unwrap();
        assert_eq!(json_val, roundtrip);
    }

    #[test]
    fn test_json_to_eure_roundtrip_variant_internal() {
        let config = Config {
            variant_repr: VariantRepr::Internal {
                tag: "type".to_string(),
            },
        };
        let json_val = json!({"type": "Success", "value": 42});

        let doc = value_to_document(&json_val, &config).unwrap();
        let roundtrip = document_to_value(&doc, &config).unwrap();
        assert_eq!(json_val, roundtrip);
    }

    #[test]
    fn test_json_to_eure_roundtrip_variant_adjacent() {
        let config = Config {
            variant_repr: VariantRepr::Adjacent {
                tag: "tag".to_string(),
                content: "content".to_string(),
            },
        };
        let json_val = json!({"tag": "Success", "content": true});

        let doc = value_to_document(&json_val, &config).unwrap();
        let roundtrip = document_to_value(&doc, &config).unwrap();
        assert_eq!(json_val, roundtrip);
    }

    #[test]
    fn test_json_to_eure_multi_key_object_not_variant() {
        // Multi-key objects should not be detected as External variants
        let json = json!({"a": 1, "b": 2});
        let config = Config {
            variant_repr: VariantRepr::External,
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify no $variant extension
        let root = doc.node(doc.get_root_id());
        let variant_ident: Identifier = "variant".parse().unwrap();
        assert!(root.extensions.get(&variant_ident).is_none());

        // Verify it's a regular map with 2 fields
        let map = root.as_map().expect("Expected map");
        assert_eq!(map.0.len(), 2);
    }

    #[test]
    fn test_json_to_eure_internal_non_string_tag_not_variant() {
        // If the tag field is not a string, don't detect as variant
        let json = json!({"type": 123, "value": "test"});
        let config = Config {
            variant_repr: VariantRepr::Internal {
                tag: "type".to_string(),
            },
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify no $variant extension (type is not a string)
        let root = doc.node(doc.get_root_id());
        let variant_ident: Identifier = "variant".parse().unwrap();
        assert!(root.extensions.get(&variant_ident).is_none());
    }

    #[test]
    fn test_json_to_eure_adjacent_extra_fields_not_variant() {
        // Adjacent requires exactly 2 fields
        let json = json!({"tag": "Success", "content": true, "extra": 1});
        let config = Config {
            variant_repr: VariantRepr::Adjacent {
                tag: "tag".to_string(),
                content: "content".to_string(),
            },
        };
        let doc = value_to_document(&json, &config).unwrap();

        // Verify no $variant extension (has extra field)
        let root = doc.node(doc.get_root_id());
        let variant_ident: Identifier = "variant".parse().unwrap();
        assert!(root.extensions.get(&variant_ident).is_none());

        // Verify it's a regular map with 3 fields
        let map = root.as_map().expect("Expected map");
        assert_eq!(map.0.len(), 3);
    }
}
