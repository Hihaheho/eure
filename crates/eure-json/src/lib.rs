#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

mod config;
mod error;

pub use config::Config;
pub use error::{EureToJsonError, JsonToEureError};
use eure::data_model::VariantRepr;
use eure::document::node::NodeValue;
use eure::document::{EureDocument, NodeId};
use eure::query::{ParseDocument, TextFile};
use eure::value::{ObjectKey, PrimitiveValue};
use eure_document::text::Text;
use num_bigint::BigInt;
use query_flow::{Db, QueryError, query};
use serde_json::Value as JsonValue;

#[query]
pub fn eure_to_json(
    db: &impl Db,
    text_file: TextFile,
    config: Config,
) -> Result<JsonValue, QueryError> {
    let parsed = db.query(ParseDocument::new(text_file.clone()))?;
    Ok(document_to_value(&parsed.doc, &config)?)
}

/// Convert a JSON file to an Eure document.
///
/// This query reads the JSON file, parses it, and converts it to an EureDocument.
#[query]
pub fn json_to_eure(
    db: &impl Db,
    json_file: TextFile,
    config: Config,
) -> Result<EureDocument, QueryError> {
    let content = db.asset(json_file.clone())?.suspend()?;
    let json: JsonValue = serde_json::from_str(content.get())?;
    Ok(value_to_document(&json, &config)?)
}

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
            for &child_id in arr.iter() {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Tuple(tuple) => {
            let mut result = Vec::new();
            for &child_id in tuple.iter() {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Map(map) => {
            let mut result = serde_json::Map::new();
            for (key, &child_id) in map.iter() {
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
            for &child_id in arr.iter() {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Tuple(tuple) => {
            let mut result = Vec::new();
            for &child_id in tuple.iter() {
                result.push(convert_node(doc, child_id, config)?);
            }
            Ok(JsonValue::Array(result))
        }
        NodeValue::Map(map) => {
            let mut result = serde_json::Map::new();
            for (key, &child_id) in map.iter() {
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
/// This conversion produces plain data structures without any variant detection.
/// JSON objects become Eure maps, arrays become arrays, and primitives are converted
/// directly. Variant reconstruction is not possible without schema information.
///
/// The `config` parameter is accepted for API consistency but is not used for
/// JSON to Eure conversion (variant detection requires schema information).
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
    _config: &Config,
) -> Result<EureDocument, JsonToEureError> {
    let mut doc = EureDocument::new();
    let root_id = doc.get_root_id();
    convert_json_to_node(&mut doc, root_id, value);
    Ok(doc)
}

/// Convert a JSON value and set it as the content of the given node.
fn convert_json_to_node(doc: &mut EureDocument, node_id: NodeId, value: &JsonValue) {
    match value {
        JsonValue::Null => {
            doc.node_mut(node_id).content = NodeValue::Primitive(PrimitiveValue::Null);
        }
        JsonValue::Bool(b) => {
            doc.node_mut(node_id).content = NodeValue::Primitive(PrimitiveValue::Bool(*b));
        }
        JsonValue::Number(n) => {
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
                convert_json_to_node(doc, child_id, item);
                if let NodeValue::Array(ref mut array) = doc.node_mut(node_id).content {
                    let _ = array.push(child_id);
                }
            }
        }
        JsonValue::Object(obj) => {
            doc.node_mut(node_id).content = NodeValue::empty_map();
            for (key, val) in obj {
                let child_id = doc.create_node(NodeValue::hole());
                convert_json_to_node(doc, child_id, val);
                if let NodeValue::Map(ref mut map) = doc.node_mut(node_id).content {
                    map.insert(ObjectKey::String(key.clone()), child_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure::data_model::VariantRepr;
    use eure_document::eure;
    use serde_json::json;

    // ========================================================================
    // Eure to JSON conversion tests
    // ========================================================================

    #[test]
    fn test_null() {
        let eure = eure!({ = null });
        let json = json!(null);
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_bool_true() {
        let eure = eure!({ = true });
        let json = json!(true);
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_bool_false() {
        let eure = eure!({ = false });
        let json = json!(false);
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_integer() {
        let eure = eure!({ = 42 });
        let json = json!(42);
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_negative_integer() {
        let eure = eure!({ = -42 });
        let json = json!(-42);
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_float() {
        let eure = eure!({ = 1.5f64 });
        let json = json!(1.5);
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_string() {
        let eure = eure!({ = "hello world" });
        let json = json!("hello world");
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_array() {
        let eure = eure!({
            items[] = 1,
            items[] = 2,
            items[] = 3,
        });
        let json = json!({"items": [1, 2, 3]});
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_tuple() {
        let eure = eure!({
            point.#0 = 1.5f64,
            point.#1 = 2.5f64,
        });
        let json = json!({"point": [1.5, 2.5]});
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_empty_map() {
        let eure = eure!({});
        let json = json!({});
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_map_with_fields() {
        let eure = eure!({ key = true });
        let json = json!({"key": true});
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_nested_map() {
        let eure = eure!({
            user.name = "Alice",
            user.age = 30,
        });
        let json = json!({"user": {"name": "Alice", "age": 30}});
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    #[test]
    fn test_array_of_maps() {
        let eure = eure!({
            users[].name = "Alice",
            users[].name = "Bob",
        });
        let json = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
        assert_eq!(document_to_value(&eure, &Config::default()).unwrap(), json);
    }

    // Variant tests (Eure with $variant -> JSON)
    #[test]
    fn test_variant_external() {
        let eure = eure!({
            = true,
            %variant = "Success",
        });
        let config = Config {
            variant_repr: VariantRepr::External,
        };
        let json = json!({"Success": true});
        assert_eq!(document_to_value(&eure, &config).unwrap(), json);
    }

    #[test]
    fn test_variant_untagged() {
        let eure = eure!({
            = true,
            %variant = "Success",
        });
        let config = Config {
            variant_repr: VariantRepr::Untagged,
        };
        let json = json!(true);
        assert_eq!(document_to_value(&eure, &config).unwrap(), json);
    }

    #[test]
    fn test_variant_internal() {
        let eure = eure!({
            field = 42,
            %variant = "Success",
        });
        let config = Config {
            variant_repr: VariantRepr::Internal {
                tag: "type".to_string(),
            },
        };
        let json = json!({"type": "Success", "field": 42});
        assert_eq!(document_to_value(&eure, &config).unwrap(), json);
    }

    #[test]
    fn test_variant_adjacent() {
        let eure = eure!({
            = true,
            %variant = "Success",
        });
        let config = Config {
            variant_repr: VariantRepr::Adjacent {
                tag: "tag".to_string(),
                content: "content".to_string(),
            },
        };
        let json = json!({"tag": "Success", "content": true});
        assert_eq!(document_to_value(&eure, &config).unwrap(), json);
    }

    // Error tests
    #[test]
    fn test_hole_error() {
        let eure = eure!({ placeholder = ! });
        let result = document_to_value(&eure, &Config::default());
        assert_eq!(result, Err(EureToJsonError::HoleNotSupported));
    }

    #[test]
    fn test_f64_nan_error() {
        let nan_value = f64::NAN;
        let eure = eure!({ = nan_value });
        let result = document_to_value(&eure, &Config::default());
        assert_eq!(result, Err(EureToJsonError::NonFiniteFloat));
    }

    #[test]
    fn test_f64_infinity_error() {
        let inf_value = f64::INFINITY;
        let eure = eure!({ = inf_value });
        let result = document_to_value(&eure, &Config::default());
        assert_eq!(result, Err(EureToJsonError::NonFiniteFloat));
    }

    // ========================================================================
    // JSON to Eure conversion tests
    // ========================================================================

    #[test]
    fn test_json_to_eure_null() {
        let json = json!(null);
        let expected = eure!({ = null });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_bool() {
        let json = json!(true);
        let expected = eure!({ = true });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_integer() {
        let json = json!(42);
        let expected = eure!({ = 42 });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_float() {
        let json = json!(1.5);
        let expected = eure!({ = 1.5f64 });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_string() {
        let json = json!("hello");
        let expected = eure!({ = "hello" });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_array() {
        // Test array conversion via roundtrip (eure! macro doesn't support root arrays)
        let json = json!([1, 2, 3]);
        let doc = value_to_document(&json, &Config::default()).unwrap();
        let roundtrip = document_to_value(&doc, &Config::default()).unwrap();
        assert_eq!(json, roundtrip);
    }

    #[test]
    fn test_json_to_eure_empty_object() {
        let json = json!({});
        let expected = eure!({});
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_object() {
        let json = json!({"name": "Alice", "age": 30});
        let expected = eure!({
            name = "Alice",
            age = 30,
        });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_nested() {
        let json = json!({"user": {"name": "Alice"}});
        let expected = eure!({ user.name = "Alice" });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_json_to_eure_array_of_objects() {
        let json = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
        let expected = eure!({
            users[].name = "Alice",
            users[].name = "Bob",
        });
        assert_eq!(
            value_to_document(&json, &Config::default()).unwrap(),
            expected
        );
    }

    // ========================================================================
    // Roundtrip tests
    // ========================================================================

    #[test]
    fn test_roundtrip_primitives() {
        for json in [
            json!(null),
            json!(true),
            json!(42),
            json!(1.5),
            json!("hello"),
        ] {
            let doc = value_to_document(&json, &Config::default()).unwrap();
            let roundtrip = document_to_value(&doc, &Config::default()).unwrap();
            assert_eq!(json, roundtrip);
        }
    }

    #[test]
    fn test_roundtrip_array() {
        let json = json!([1, 2, 3, "hello", true, null]);
        let doc = value_to_document(&json, &Config::default()).unwrap();
        let roundtrip = document_to_value(&doc, &Config::default()).unwrap();
        assert_eq!(json, roundtrip);
    }

    #[test]
    fn test_roundtrip_nested() {
        let json = json!({"name": "test", "nested": {"a": 1, "b": [true, false]}});
        let doc = value_to_document(&json, &Config::default()).unwrap();
        let roundtrip = document_to_value(&doc, &Config::default()).unwrap();
        assert_eq!(json, roundtrip);
    }

    #[test]
    fn test_roundtrip_deeply_nested() {
        let json = json!({"Ok": {"Some": {"method": "add"}}});
        let doc = value_to_document(&json, &Config::default()).unwrap();
        let roundtrip = document_to_value(&doc, &Config::default()).unwrap();
        assert_eq!(json, roundtrip);
    }
}
