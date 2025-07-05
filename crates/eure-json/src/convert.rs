use eure_value::value::{Array, Code, KeyCmpValue, Map, Path, PathSegment, Tuple, TypedString, Value, Variant};
use serde_json::json;

use crate::{config::Config, error::Error};
use eure_value::value::VariantRepr;

/// Convert an eure Value to a JSON value using default configuration
pub fn value_to_json(value: &Value) -> Result<serde_json::Value, Error> {
    value_to_json_with_config(value, &Config::default())
}

/// Convert an eure Value to a JSON value with custom configuration
pub fn value_to_json_with_config(
    value: &Value,
    config: &Config,
) -> Result<serde_json::Value, Error> {
    match value {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(json!(*b)),
        Value::I64(i) => Ok(json!(*i)),
        Value::U64(u) => Ok(json!(*u)),
        Value::F32(f) => {
            if f.is_finite() {
                Ok(json!(*f))
            } else {
                Err(Error::InvalidNumber(format!("F32 value {f} is not finite")))
            }
        }
        Value::F64(f) => {
            if f.is_finite() {
                Ok(json!(*f))
            } else {
                Err(Error::InvalidNumber(format!("F64 value {f} is not finite")))
            }
        }
        Value::String(s) => Ok(json!(s)),
        Value::TypedString(TypedString { value, .. }) => {
            // In JSON, we lose the type information
            Ok(json!(value))
        }
        Value::Code(Code { language, content }) => {
            // Format as markdown code block
            if language.is_empty() {
                Ok(json!(format!("`{}`", content)))
            } else {
                Ok(json!(format!("```{}\n{}\n```", language, content)))
            }
        }
        Value::Array(Array(items)) => {
            let json_items: Result<Vec<_>, _> = items
                .iter()
                .map(|v| value_to_json_with_config(v, config))
                .collect();
            Ok(serde_json::Value::Array(json_items?))
        }
        Value::Tuple(Tuple(items)) => {
            let json_items: Result<Vec<_>, _> = items
                .iter()
                .map(|v| value_to_json_with_config(v, config))
                .collect();
            Ok(serde_json::Value::Array(json_items?))
        }
        Value::Map(Map(map)) => {
            let mut json_map = serde_json::Map::new();
            for (key, value) in map {
                let key_str = key_to_string(key)?;
                let json_value = value_to_json_with_config(value, config)?;
                json_map.insert(key_str, json_value);
            }
            Ok(serde_json::Value::Object(json_map))
        }
        Value::Variant(variant) => convert_variant_to_json(variant, config),
        Value::Unit => Ok(serde_json::Value::Null),
        Value::Hole => {
            // Holes cannot be meaningfully converted to JSON
            // Return an error or a special marker
            Err(Error::UnsupportedValue("Cannot convert hole value (!) to JSON - holes must be filled with actual values".to_string()))
        },
        Value::Path(Path(segments)) => {
            // Paths represented as dot-separated strings
            let path_str = segments.iter()
                .map(|seg| match seg {
                    PathSegment::Ident(id) => id.as_ref().to_string(),
                    PathSegment::Extension(id) => format!("${}", id.as_ref()),
                    PathSegment::MetaExt(id) => format!("$̄{}", id.as_ref()),
                    PathSegment::Value(v) => format!("[{v:?}]"),
                    PathSegment::TupleIndex(idx) => idx.to_string(),
                    PathSegment::Array { key, index } => {
                        if let Some(idx) = index {
                            format!("{key:?}[{idx:?}]")
                        } else {
                            format!("{key:?}[]")
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(".");
            Ok(serde_json::Value::String(format!(".{path_str}")))
        }
    }
}

/// Convert a Variant to JSON based on the representation strategy
fn convert_variant_to_json(variant: &Variant, config: &Config) -> Result<serde_json::Value, Error> {
    match &config.variant_repr {
        VariantRepr::External => {
            // {"variant-name": content}
            let content_json = value_to_json_with_config(&variant.content, config)?;
            let mut map = serde_json::Map::new();
            map.insert(variant.tag.clone(), content_json);
            Ok(serde_json::Value::Object(map))
        }
        VariantRepr::Internal { tag } => {
            // {"tag": "variant-name", ...content fields...}
            match &*variant.content {
                Value::Map(Map(content_map)) => {
                    let mut json_map = serde_json::Map::new();
                    json_map.insert(tag.clone(), json!(variant.tag));

                    // Add all fields from content
                    for (key, value) in content_map {
                        let key_str = key_to_string(key)?;
                        let json_value = value_to_json_with_config(value, config)?;
                        json_map.insert(key_str, json_value);
                    }
                    Ok(serde_json::Value::Object(json_map))
                }
                _ => Err(Error::InvalidVariant(
                    "Internal tagging requires variant content to be a Map".to_string(),
                )),
            }
        }
        VariantRepr::Adjacent { tag, content } => {
            // {"tag": "variant-name", "content": content}
            let content_json = value_to_json_with_config(&variant.content, config)?;
            let mut map = serde_json::Map::new();
            map.insert(tag.clone(), json!(variant.tag));
            map.insert(content.clone(), content_json);
            Ok(serde_json::Value::Object(map))
        }
        VariantRepr::Untagged => {
            // Just the content without any tag
            value_to_json_with_config(&variant.content, config)
        }
    }
}

/// Convert a KeyCmpValue to a string for use as JSON object key
fn key_to_string(key: &KeyCmpValue) -> Result<String, Error> {
    match key {
        KeyCmpValue::String(s) => Ok(s.clone()),
        KeyCmpValue::I64(i) => Ok(i.to_string()),
        KeyCmpValue::U64(u) => Ok(u.to_string()),
        KeyCmpValue::Null => Ok("null".to_string()),
        KeyCmpValue::Bool(b) => Ok(b.to_string()),
        KeyCmpValue::Unit => Ok("unit".to_string()),
        KeyCmpValue::Hole => Err(Error::UnsupportedValue(
            "Hole keys cannot be converted to JSON object keys".to_string(),
        )),
        KeyCmpValue::Tuple(_) => Err(Error::UnsupportedValue(
            "Tuple keys cannot be converted to JSON object keys".to_string(),
        )),
        KeyCmpValue::Extension(ext) => Ok(format!("${ext}")),
        KeyCmpValue::MetaExtension(meta) => Ok(format!("$${meta}")),
    }
}

/// Convert a JSON value to an eure Value using default configuration
pub fn json_to_value(json: &serde_json::Value) -> Result<Value, Error> {
    json_to_value_with_config(json, &Config::default())
}

/// Convert a JSON value to an eure Value with custom configuration
pub fn json_to_value_with_config(
    json: &serde_json::Value,
    config: &Config,
) -> Result<Value, Error> {
    match json {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::I64(i))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::U64(u))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::F64(f))
            } else {
                Err(Error::InvalidNumber(n.to_string()))
            }
        }
        serde_json::Value::String(s) => {
            // Check if it's a code block
            if s.starts_with("```") && s.ends_with("```") {
                let without_fences = &s[3..s.len() - 3];
                if let Some(newline_pos) = without_fences.find('\n') {
                    let language = without_fences[..newline_pos].to_string();
                    let mut content = without_fences[newline_pos + 1..].to_string();
                    // Remove trailing newline if present
                    if content.ends_with('\n') {
                        content.pop();
                    }
                    Ok(Value::Code(Code { language, content }))
                } else {
                    Ok(Value::String(s.clone()))
                }
            } else if s.starts_with('`') && s.ends_with('`') && s.len() > 2 {
                let content = s[1..s.len() - 1].to_string();
                Ok(Value::Code(Code {
                    language: String::new(),
                    content,
                }))
            } else {
                Ok(Value::String(s.clone()))
            }
        }
        serde_json::Value::Array(arr) => {
            let values: Result<Vec<_>, _> = arr
                .iter()
                .map(|v| json_to_value_with_config(v, config))
                .collect();
            Ok(Value::Array(Array(values?)))
        }
        serde_json::Value::Object(obj) => {
            // Try to detect if this is a variant based on config
            match &config.variant_repr {
                VariantRepr::External if obj.len() == 1 => {
                    // Single key might be variant name
                    let (tag, content) = obj.iter().next().unwrap();
                    let content_value = json_to_value_with_config(content, config)?;
                    Ok(Value::Variant(Variant {
                        tag: tag.clone(),
                        content: Box::new(content_value),
                    }))
                }
                VariantRepr::Internal { tag } => {
                    if let Some(variant_tag) = obj.get(tag).and_then(|v| v.as_str()) {
                        // This is a variant with internal tagging
                        let mut content_map = ahash::AHashMap::new();
                        for (k, v) in obj {
                            if k != tag {
                                let key = KeyCmpValue::String(k.clone());
                                let value = json_to_value_with_config(v, config)?;
                                content_map.insert(key, value);
                            }
                        }
                        Ok(Value::Variant(Variant {
                            tag: variant_tag.to_string(),
                            content: Box::new(Value::Map(Map(content_map))),
                        }))
                    } else {
                        // Regular map
                        convert_json_object_to_map(obj, config)
                    }
                }
                VariantRepr::Adjacent { tag, content } => {
                    if let (Some(variant_tag), Some(variant_content)) =
                        (obj.get(tag).and_then(|v| v.as_str()), obj.get(content))
                    {
                        // This is a variant with adjacent tagging
                        let content_value = json_to_value_with_config(variant_content, config)?;
                        Ok(Value::Variant(Variant {
                            tag: variant_tag.to_string(),
                            content: Box::new(content_value),
                        }))
                    } else {
                        // Regular map
                        convert_json_object_to_map(obj, config)
                    }
                }
                _ => {
                    // Regular map or untagged variant (can't distinguish)
                    convert_json_object_to_map(obj, config)
                }
            }
        }
    }
}

/// Convert a JSON object to an eure Map
fn convert_json_object_to_map(
    obj: &serde_json::Map<String, serde_json::Value>,
    config: &Config,
) -> Result<Value, Error> {
    let mut map = ahash::AHashMap::new();
    for (k, v) in obj {
        let key = KeyCmpValue::String(k.clone());
        let value = json_to_value_with_config(v, config)?;
        map.insert(key, value);
    }
    Ok(Value::Map(Map(map)))
}
