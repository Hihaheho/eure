use crate::{Config, Error};
use eure_value::value::{Value, Map, Array, Tuple, Variant, TypedString, Code, KeyCmpValue, VariantRepr};
use serde_yaml::{Value as YamlValue, Mapping};

/// Convert a Value to YAML with default configuration
pub fn value_to_yaml(value: &Value) -> Result<YamlValue, Error> {
    value_to_yaml_with_config(value, &Config::default())
}

/// Convert a Value to YAML with custom configuration
pub fn value_to_yaml_with_config(value: &Value, config: &Config) -> Result<YamlValue, Error> {
    match value {
        Value::Null => Ok(YamlValue::Null),
        Value::Bool(b) => Ok(YamlValue::Bool(*b)),
        Value::I64(i) => Ok(YamlValue::Number((*i).into())),
        Value::U64(u) => Ok(YamlValue::Number((*u).into())),
        Value::F32(f) => {
            if f.is_nan() || f.is_infinite() {
                // YAML supports special float values
                Ok(YamlValue::Number((*f as f64).into()))
            } else {
                Ok(YamlValue::Number((*f as f64).into()))
            }
        }
        Value::F64(f) => {
            if f.is_nan() || f.is_infinite() {
                // YAML supports special float values
                Ok(YamlValue::Number((*f).into()))
            } else {
                Ok(YamlValue::Number((*f).into()))
            }
        }
        Value::String(s) => Ok(YamlValue::String(s.clone())),
        Value::TypedString(TypedString { type_name: _, value }) => {
            // Type information is lost in YAML conversion
            Ok(YamlValue::String(value.clone()))
        }
        Value::Code(Code { language, content }) => {
            // Format as markdown code block
            let formatted = if language.is_empty() {
                format!("`{content}`")
            } else {
                format!("```{language}\n{content}\n```")
            };
            Ok(YamlValue::String(formatted))
        }
        Value::Array(Array(items)) => {
            let yaml_items: Result<Vec<_>, _> = items.iter()
                .map(|v| value_to_yaml_with_config(v, config))
                .collect();
            Ok(YamlValue::Sequence(yaml_items?))
        }
        Value::Tuple(Tuple(items)) => {
            // Tuples are represented as YAML sequences
            let yaml_items: Result<Vec<_>, _> = items.iter()
                .map(|v| value_to_yaml_with_config(v, config))
                .collect();
            Ok(YamlValue::Sequence(yaml_items?))
        }
        Value::Map(Map(map)) => {
            let mut yaml_map = Mapping::new();
            for (k, v) in map.iter() {
                let key = match k {
                    KeyCmpValue::String(s) => YamlValue::String(s.clone()),
                    KeyCmpValue::I64(i) => YamlValue::Number((*i).into()),
                    KeyCmpValue::U64(u) => YamlValue::Number((*u).into()),
                    KeyCmpValue::Bool(b) => YamlValue::Bool(*b),
                    KeyCmpValue::Null => YamlValue::Null,
                    _ => return Err(Error::ConversionError(
                        format!("Cannot use {k:?} as YAML map key")
                    )),
                };
                yaml_map.insert(key, value_to_yaml_with_config(v, config)?);
            }
            Ok(YamlValue::Mapping(yaml_map))
        }
        Value::Variant(variant) => {
            convert_variant_to_yaml(variant, config)
        }
        Value::Unit => {
            // Represent unit as null in YAML
            Ok(YamlValue::Null)
        }
    }
}

fn convert_variant_to_yaml(variant: &Variant, config: &Config) -> Result<YamlValue, Error> {
    match &config.variant_repr {
        VariantRepr::External => {
            // {"variant-name": content}
            let mut map = Mapping::new();
            map.insert(
                YamlValue::String(variant.tag.clone()),
                value_to_yaml_with_config(&variant.content, config)?
            );
            Ok(YamlValue::Mapping(map))
        }
        VariantRepr::Internal { tag } => {
            // {"tag": "variant-name", ...content fields...}
            match variant.content.as_ref() {
                Value::Map(Map(content_map)) => {
                    let mut map = Mapping::new();
                    map.insert(
                        YamlValue::String(tag.clone()),
                        YamlValue::String(variant.tag.clone())
                    );
                    
                    // Add all fields from content
                    for (k, v) in content_map.iter() {
                        let key = match k {
                            KeyCmpValue::String(s) => YamlValue::String(s.clone()),
                            _ => return Err(Error::ConversionError(
                                "Internal variant representation requires string keys".to_string()
                            )),
                        };
                        map.insert(key, value_to_yaml_with_config(v, config)?);
                    }
                    
                    Ok(YamlValue::Mapping(map))
                }
                _ => Err(Error::InvalidVariant(
                    "Internal variant representation requires map content".to_string()
                ))
            }
        }
        VariantRepr::Adjacent { tag, content } => {
            // {"tag": "variant-name", "content": content}
            let mut map = Mapping::new();
            map.insert(
                YamlValue::String(tag.clone()),
                YamlValue::String(variant.tag.clone())
            );
            map.insert(
                YamlValue::String(content.clone()),
                value_to_yaml_with_config(&variant.content, config)?
            );
            Ok(YamlValue::Mapping(map))
        }
        VariantRepr::Untagged => {
            // Just the content without any tag
            value_to_yaml_with_config(&variant.content, config)
        }
    }
}

/// Convert YAML to Value with default configuration
pub fn yaml_to_value(yaml: &YamlValue) -> Result<Value, Error> {
    yaml_to_value_with_config(yaml, &Config::default())
}

/// Convert YAML to Value with custom configuration
pub fn yaml_to_value_with_config(yaml: &YamlValue, config: &Config) -> Result<Value, Error> {
    match yaml {
        YamlValue::Null => Ok(Value::Null),
        YamlValue::Bool(b) => Ok(Value::Bool(*b)),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::I64(i))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::U64(u))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::F64(f))
            } else {
                Err(Error::ConversionError("Invalid number value".to_string()))
            }
        }
        YamlValue::String(s) => {
            // Check if it's a code block
            if let Some(code_value) = parse_code_block(s) {
                Ok(code_value)
            } else {
                Ok(Value::String(s.clone()))
            }
        }
        YamlValue::Sequence(seq) => {
            let values: Result<Vec<_>, _> = seq.iter()
                .map(|v| yaml_to_value_with_config(v, config))
                .collect();
            Ok(Value::Array(Array(values?)))
        }
        YamlValue::Mapping(map) => {
            // Try to parse as variant based on configuration
            if let Some(variant) = try_parse_variant(map, config)? {
                Ok(Value::Variant(variant))
            } else {
                let mut result_map = ahash::AHashMap::new();
                for (k, v) in map.iter() {
                    let key = yaml_key_to_key_cmp_value(k)?;
                    let value = yaml_to_value_with_config(v, config)?;
                    result_map.insert(key, value);
                }
                Ok(Value::Map(Map(result_map)))
            }
        }
        YamlValue::Tagged(tagged) => {
            // Handle YAML tags if needed
            yaml_to_value_with_config(&tagged.value, config)
        }
    }
}

fn yaml_key_to_key_cmp_value(yaml: &YamlValue) -> Result<KeyCmpValue, Error> {
    match yaml {
        YamlValue::String(s) => Ok(KeyCmpValue::String(s.clone())),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(KeyCmpValue::I64(i))
            } else if let Some(u) = n.as_u64() {
                Ok(KeyCmpValue::U64(u))
            } else {
                Err(Error::ConversionError("Float keys are not supported".to_string()))
            }
        }
        YamlValue::Bool(b) => Ok(KeyCmpValue::Bool(*b)),
        YamlValue::Null => Ok(KeyCmpValue::Null),
        _ => Err(Error::ConversionError(
            format!("Cannot use {yaml:?} as map key")
        )),
    }
}

fn parse_code_block(s: &str) -> Option<Value> {
    // Check for inline code
    if s.starts_with('`') && s.ends_with('`') && s.len() > 2 && !s[1..s.len()-1].contains('`') {
        let content = &s[1..s.len()-1];
        return Some(Value::Code(Code {
            language: String::new(),
            content: content.to_string(),
        }));
    }
    
    // Check for code block
    if s.starts_with("```") && s.ends_with("```") {
        let content = &s[3..s.len()-3];
        let lines: Vec<&str> = content.lines().collect();
        
        if let Some(first_line) = lines.first() {
            // Check if first line is a language identifier
            if !first_line.is_empty() && !first_line.contains(' ') {
                let language = first_line.to_string();
                let code_content = lines[1..].join("\n");
                return Some(Value::Code(Code {
                    language,
                    content: code_content,
                }));
            }
        }
        
        // No language specified
        return Some(Value::Code(Code {
            language: String::new(),
            content: content.to_string(),
        }));
    }
    
    None
}

fn try_parse_variant(map: &Mapping, config: &Config) -> Result<Option<Variant>, Error> {
    match &config.variant_repr {
        VariantRepr::External => {
            // External: single key-value pair where key is the variant name
            if map.len() == 1
                && let Some((k, v)) = map.iter().next()
                    && let YamlValue::String(tag) = k {
                        let content = yaml_to_value_with_config(v, config)?;
                        return Ok(Some(Variant {
                            tag: tag.clone(),
                            content: Box::new(content),
                        }));
                    }
            Ok(None)
        }
        VariantRepr::Internal { tag } => {
            // Internal: map with tag field containing variant name
            if let Some(YamlValue::String(variant_tag)) = map.get(YamlValue::String(tag.clone())) {
                let mut content_map = ahash::AHashMap::new();
                
                for (k, v) in map.iter() {
                    if let YamlValue::String(key_str) = k
                        && key_str != tag {
                            let value = yaml_to_value_with_config(v, config)?;
                            content_map.insert(KeyCmpValue::String(key_str.clone()), value);
                        }
                }
                
                return Ok(Some(Variant {
                    tag: variant_tag.clone(),
                    content: Box::new(Value::Map(Map(content_map))),
                }));
            }
            Ok(None)
        }
        VariantRepr::Adjacent { tag, content } => {
            // Adjacent: map with tag and content fields
            let tag_value = map.get(YamlValue::String(tag.clone()));
            let content_value = map.get(YamlValue::String(content.clone()));
            
            if let (Some(YamlValue::String(variant_tag)), Some(variant_content)) = (tag_value, content_value) {
                let content = yaml_to_value_with_config(variant_content, config)?;
                return Ok(Some(Variant {
                    tag: variant_tag.clone(),
                    content: Box::new(content),
                }));
            }
            Ok(None)
        }
        VariantRepr::Untagged => {
            // Untagged: cannot determine if it's a variant from YAML alone
            Ok(None)
        }
    }
}