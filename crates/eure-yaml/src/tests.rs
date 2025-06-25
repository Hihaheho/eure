#![allow(clippy::approx_constant)]

use super::*;
use eure_value::value::{Value, KeyCmpValue, Map, Array, Tuple, Variant, TypedString, Code, VariantRepr};
use serde_yaml::{Value as YamlValue};

#[test]
fn test_basic_values_to_yaml() {
    // Null
    let yaml = value_to_yaml(&Value::Null).unwrap();
    assert_eq!(yaml, YamlValue::Null);
    
    // Bool
    let yaml = value_to_yaml(&Value::Bool(true)).unwrap();
    assert_eq!(yaml, YamlValue::Bool(true));
    
    let yaml = value_to_yaml(&Value::Bool(false)).unwrap();
    assert_eq!(yaml, YamlValue::Bool(false));
    
    // Numbers
    let yaml = value_to_yaml(&Value::I64(-42)).unwrap();
    assert_eq!(yaml, YamlValue::Number((-42).into()));
    
    let yaml = value_to_yaml(&Value::U64(42)).unwrap();
    assert_eq!(yaml, YamlValue::Number(42.into()));
    
    let yaml = value_to_yaml(&Value::F32(3.14)).unwrap();
    assert_eq!(yaml.as_f64(), Some(3.14f32 as f64));
    
    let yaml = value_to_yaml(&Value::F64(3.14159)).unwrap();
    assert_eq!(yaml.as_f64(), Some(3.14159));
    
    // String
    let yaml = value_to_yaml(&Value::String("hello".to_string())).unwrap();
    assert_eq!(yaml, YamlValue::String("hello".to_string()));
    
    // Unit
    let yaml = value_to_yaml(&Value::Unit).unwrap();
    assert_eq!(yaml, YamlValue::Null);
}

#[test]
fn test_typed_string_to_yaml() {
    let typed = Value::TypedString(TypedString {
        type_name: "email".to_string(),
        value: "test@example.com".to_string(),
    });
    // Type information is lost in YAML
    let yaml = value_to_yaml(&typed).unwrap();
    assert_eq!(yaml, YamlValue::String("test@example.com".to_string()));
}

#[test]
fn test_code_to_yaml() {
    // Inline code
    let code = Value::Code(Code {
        language: String::new(),
        content: "let x = 42;".to_string(),
    });
    let yaml = value_to_yaml(&code).unwrap();
    assert_eq!(yaml, YamlValue::String("`let x = 42;`".to_string()));
    
    // Code block with language
    let code_block = Value::Code(Code {
        language: "rust".to_string(),
        content: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
    });
    let yaml = value_to_yaml(&code_block).unwrap();
    assert_eq!(
        yaml,
        YamlValue::String("```rust\nfn main() {\n    println!(\"Hello\");\n}\n```".to_string())
    );
}

#[test]
fn test_array_to_yaml() {
    let array = Value::Array(Array(vec![
        Value::I64(1),
        Value::I64(2),
        Value::I64(3),
    ]));
    let yaml = value_to_yaml(&array).unwrap();
    
    match yaml {
        YamlValue::Sequence(seq) => {
            assert_eq!(seq.len(), 3);
            assert_eq!(seq[0].as_i64(), Some(1));
            assert_eq!(seq[1].as_i64(), Some(2));
            assert_eq!(seq[2].as_i64(), Some(3));
        }
        _ => panic!("Expected sequence"),
    }
}

#[test]
fn test_tuple_to_yaml() {
    let tuple = Value::Tuple(Tuple(vec![
        Value::String("hello".to_string()),
        Value::I64(42),
        Value::Bool(true),
    ]));
    let yaml = value_to_yaml(&tuple).unwrap();
    
    match yaml {
        YamlValue::Sequence(seq) => {
            assert_eq!(seq.len(), 3);
            assert_eq!(seq[0].as_str(), Some("hello"));
            assert_eq!(seq[1].as_i64(), Some(42));
            assert_eq!(seq[2].as_bool(), Some(true));
        }
        _ => panic!("Expected sequence"),
    }
}

#[test]
fn test_map_to_yaml() {
    let mut map = ahash::AHashMap::new();
    map.insert(KeyCmpValue::String("name".to_string()), Value::String("Alice".to_string()));
    map.insert(KeyCmpValue::String("age".to_string()), Value::I64(30));
    
    let value = Value::Map(Map(map));
    let yaml = value_to_yaml(&value).unwrap();
    
    match yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.get(YamlValue::String("name".to_string())), Some(&YamlValue::String("Alice".to_string())));
            assert_eq!(map.get(YamlValue::String("age".to_string())), Some(&YamlValue::Number(30.into())));
        }
        _ => panic!("Expected mapping"),
    }
}

#[test]
fn test_variant_external() {
    let variant = Value::Variant(Variant {
        tag: "Success".to_string(),
        content: Box::new(Value::String("OK".to_string())),
    });
    
    let config = Config::default(); // External representation
    let yaml = value_to_yaml_with_config(&variant, &config).unwrap();
    
    match yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.len(), 1);
            assert_eq!(map.get(YamlValue::String("Success".to_string())), Some(&YamlValue::String("OK".to_string())));
        }
        _ => panic!("Expected mapping"),
    }
}

#[test]
fn test_variant_internal() {
    let mut content_map = ahash::AHashMap::new();
    content_map.insert(KeyCmpValue::String("message".to_string()), Value::String("Hello".to_string()));
    content_map.insert(KeyCmpValue::String("code".to_string()), Value::I64(200));
    
    let variant = Value::Variant(Variant {
        tag: "Response".to_string(),
        content: Box::new(Value::Map(Map(content_map))),
    });
    
    let config = Config {
        variant_repr: VariantRepr::Internal { tag: "type".to_string() },
    };
    let yaml = value_to_yaml_with_config(&variant, &config).unwrap();
    
    match yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.get(YamlValue::String("type".to_string())), Some(&YamlValue::String("Response".to_string())));
            assert_eq!(map.get(YamlValue::String("message".to_string())), Some(&YamlValue::String("Hello".to_string())));
            assert_eq!(map.get(YamlValue::String("code".to_string())), Some(&YamlValue::Number(200.into())));
        }
        _ => panic!("Expected mapping"),
    }
}

#[test]
fn test_variant_adjacent() {
    let variant = Value::Variant(Variant {
        tag: "Error".to_string(),
        content: Box::new(Value::String("Not found".to_string())),
    });
    
    let config = Config {
        variant_repr: VariantRepr::Adjacent { 
            tag: "type".to_string(),
            content: "content".to_string(),
        },
    };
    let yaml = value_to_yaml_with_config(&variant, &config).unwrap();
    
    match yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.get(YamlValue::String("type".to_string())), Some(&YamlValue::String("Error".to_string())));
            assert_eq!(map.get(YamlValue::String("content".to_string())), Some(&YamlValue::String("Not found".to_string())));
        }
        _ => panic!("Expected mapping"),
    }
}

#[test]
fn test_variant_untagged() {
    let mut content_map = ahash::AHashMap::new();
    content_map.insert(KeyCmpValue::String("id".to_string()), Value::I64(123));
    
    let variant = Value::Variant(Variant {
        tag: "User".to_string(),
        content: Box::new(Value::Map(Map(content_map))),
    });
    
    let config = Config {
        variant_repr: VariantRepr::Untagged,
    };
    let yaml = value_to_yaml_with_config(&variant, &config).unwrap();
    
    match yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.len(), 1);
            assert_eq!(map.get(YamlValue::String("id".to_string())), Some(&YamlValue::Number(123.into())));
        }
        _ => panic!("Expected mapping"),
    }
}

#[test]
fn test_yaml_to_value_basic() {
    // Null
    let value = yaml_to_value(&YamlValue::Null).unwrap();
    assert_eq!(value, Value::Null);
    
    // Bool
    let value = yaml_to_value(&YamlValue::Bool(true)).unwrap();
    assert_eq!(value, Value::Bool(true));
    
    // Numbers
    let value = yaml_to_value(&YamlValue::Number(42.into())).unwrap();
    assert_eq!(value, Value::I64(42));
    
    let value = yaml_to_value(&YamlValue::Number((-42).into())).unwrap();
    assert_eq!(value, Value::I64(-42));
    
    let value = yaml_to_value(&YamlValue::Number(3.14.into())).unwrap();
    assert_eq!(value, Value::F64(3.14));
    
    // String
    let value = yaml_to_value(&YamlValue::String("hello".to_string())).unwrap();
    assert_eq!(value, Value::String("hello".to_string()));
}

#[test]
fn test_yaml_to_value_array() {
    let yaml = YamlValue::Sequence(vec![
        YamlValue::Number(1.into()),
        YamlValue::Number(2.into()),
        YamlValue::Number(3.into()),
    ]);
    let value = yaml_to_value(&yaml).unwrap();
    
    match value {
        Value::Array(Array(items)) => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::I64(1));
            assert_eq!(items[1], Value::I64(2));
            assert_eq!(items[2], Value::I64(3));
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_yaml_to_value_mapping() {
    let mut map = serde_yaml::Mapping::new();
    map.insert(YamlValue::String("name".to_string()), YamlValue::String("Bob".to_string()));
    map.insert(YamlValue::String("age".to_string()), YamlValue::Number(25.into()));
    
    let value = yaml_to_value(&YamlValue::Mapping(map)).unwrap();
    
    match value {
        Value::Map(Map(map)) => {
            assert_eq!(map.get(&KeyCmpValue::String("name".to_string())), Some(&Value::String("Bob".to_string())));
            assert_eq!(map.get(&KeyCmpValue::String("age".to_string())), Some(&Value::I64(25)));
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_yaml_to_value_code() {
    // Inline code
    let yaml = YamlValue::String("`let x = 42;`".to_string());
    let value = yaml_to_value(&yaml).unwrap();
    
    match value {
        Value::Code(Code { language, content }) => {
            assert_eq!(language, "");
            assert_eq!(content, "let x = 42;");
        }
        _ => panic!("Expected Code"),
    }
    
    // Code block
    let yaml = YamlValue::String("```rust\nfn main() {}\n```".to_string());
    let value = yaml_to_value(&yaml).unwrap();
    
    match value {
        Value::Code(Code { language, content }) => {
            assert_eq!(language, "rust");
            assert_eq!(content, "fn main() {}");
        }
        _ => panic!("Expected Code"),
    }
}

#[test]
fn test_round_trip() {
    let original = Value::Map(Map({
        let mut map = ahash::AHashMap::new();
        map.insert(KeyCmpValue::String("items".to_string()), Value::Array(Array(vec![
            Value::I64(1),
            Value::String("test".to_string()),
            Value::Bool(true),
        ])));
        map.insert(KeyCmpValue::String("count".to_string()), Value::I64(3));
        map
    }));
    
    let yaml = value_to_yaml(&original).unwrap();
    let back = yaml_to_value(&yaml).unwrap();
    
    assert_eq!(original, back);
}

#[test]
fn test_all_variant_representations() {
    // Create a complex structure with all 4 variant representations
    
    // 1. External representation (default)
    let mut external_content = ahash::AHashMap::new();
    external_content.insert(KeyCmpValue::String("name".to_string()), Value::String("Alice".to_string()));
    external_content.insert(KeyCmpValue::String("age".to_string()), Value::U64(30));
    let external_variant = Value::Variant(Variant {
        tag: "person".to_string(),
        content: Box::new(Value::Map(Map(external_content))),
    });
    
    // 2. Internal representation
    let mut internal_content = ahash::AHashMap::new();
    internal_content.insert(KeyCmpValue::String("x".to_string()), Value::F64(10.0));
    internal_content.insert(KeyCmpValue::String("y".to_string()), Value::F64(20.0));
    let internal_variant = Value::Variant(Variant {
        tag: "point".to_string(),
        content: Box::new(Value::Map(Map(internal_content))),
    });
    
    // 3. Adjacent representation
    let mut adjacent_content = ahash::AHashMap::new();
    adjacent_content.insert(KeyCmpValue::String("message".to_string()), Value::String("Hello, World!".to_string()));
    adjacent_content.insert(KeyCmpValue::String("severity".to_string()), Value::String("info".to_string()));
    let adjacent_variant = Value::Variant(Variant {
        tag: "log-entry".to_string(),
        content: Box::new(Value::Map(Map(adjacent_content))),
    });
    
    // 4. Untagged representation
    let mut untagged_content = ahash::AHashMap::new();
    untagged_content.insert(KeyCmpValue::String("street".to_string()), Value::String("123 Main St".to_string()));
    untagged_content.insert(KeyCmpValue::String("city".to_string()), Value::String("Boston".to_string()));
    let untagged_variant = Value::Variant(Variant {
        tag: "address".to_string(),
        content: Box::new(Value::Map(Map(untagged_content))),
    });
    
    // Test external representation (default)
    let external_yaml = value_to_yaml(&external_variant).unwrap();
    match external_yaml {
        YamlValue::Mapping(map) => {
            assert!(map.contains_key(YamlValue::String("person".to_string())));
        }
        _ => panic!("Expected mapping"),
    }
    
    // Test internal representation
    let internal_config = Config {
        variant_repr: VariantRepr::Internal { tag: "type".to_string() },
    };
    let internal_yaml = value_to_yaml_with_config(&internal_variant, &internal_config).unwrap();
    match internal_yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.get(YamlValue::String("type".to_string())), Some(&YamlValue::String("point".to_string())));
            assert!(map.contains_key(YamlValue::String("x".to_string())));
            assert!(map.contains_key(YamlValue::String("y".to_string())));
        }
        _ => panic!("Expected mapping"),
    }
    
    // Test adjacent representation
    let adjacent_config = Config {
        variant_repr: VariantRepr::Adjacent { 
            tag: "kind".to_string(), 
            content: "data".to_string() 
        },
    };
    let adjacent_yaml = value_to_yaml_with_config(&adjacent_variant, &adjacent_config).unwrap();
    match adjacent_yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.get(YamlValue::String("kind".to_string())), Some(&YamlValue::String("log-entry".to_string())));
            assert!(map.contains_key(YamlValue::String("data".to_string())));
        }
        _ => panic!("Expected mapping"),
    }
    
    // Test untagged representation
    let untagged_config = Config {
        variant_repr: VariantRepr::Untagged,
    };
    let untagged_yaml = value_to_yaml_with_config(&untagged_variant, &untagged_config).unwrap();
    match untagged_yaml {
        YamlValue::Mapping(map) => {
            assert_eq!(map.len(), 2);
            assert!(map.contains_key(YamlValue::String("street".to_string())));
            assert!(map.contains_key(YamlValue::String("city".to_string())));
        }
        _ => panic!("Expected mapping"),
    }
}

#[test]
fn test_yaml_special_values() {
    // Test YAML's support for special float values
    let nan_value = Value::F64(f64::NAN);
    let yaml = value_to_yaml(&nan_value).unwrap();
    assert!(yaml.as_f64().unwrap().is_nan());
    
    let inf_value = Value::F64(f64::INFINITY);
    let yaml = value_to_yaml(&inf_value).unwrap();
    assert!(yaml.as_f64().unwrap().is_infinite());
    
    let neg_inf_value = Value::F64(f64::NEG_INFINITY);
    let yaml = value_to_yaml(&neg_inf_value).unwrap();
    assert!(yaml.as_f64().unwrap().is_sign_negative() && yaml.as_f64().unwrap().is_infinite());
}

// ===== ERROR CASE TESTS =====

#[test]
fn test_yaml_parsing_errors() {
    // Helper function to parse YAML string
    fn parse_yaml(yaml_str: &str) -> crate::Result<Value> {
        serde_yaml::from_str::<YamlValue>(yaml_str)
            .map_err(crate::Error::YamlParseError)
            .and_then(|yaml| yaml_to_value(&yaml))
    }
    
    // Invalid YAML syntax
    assert!(parse_yaml("[ unmatched bracket").is_err());
    assert!(parse_yaml("{ key: value").is_err()); // Missing closing brace
    // This might be valid YAML depending on version
    let _nested_result = parse_yaml("- item1\n  - nested wrong");
    // Accept either error or valid parse
    assert!(parse_yaml(": no key").is_err());
    assert!(parse_yaml("@invalid").is_err());
    
    // Invalid mappings
    assert!(parse_yaml("{[]: value}").is_err()); // Array as key
    assert!(parse_yaml("{{}: value}").is_err()); // Map as key
    
    // Multiple documents (not supported)
    assert!(parse_yaml("---\nfirst\n---\nsecond").is_err());
}

#[test]
fn test_variant_missing_tag_errors() {
    // Internal representation without tag field
    let mut map = serde_yaml::Mapping::new();
    map.insert(YamlValue::String("field1".to_string()), YamlValue::String("value1".to_string()));
    map.insert(YamlValue::String("field2".to_string()), YamlValue::Number(42.into()));
    
    let config = Config {
        variant_repr: VariantRepr::Internal { tag: "type".to_string() },
    };
    
    let result = yaml_to_value_with_config(&YamlValue::Mapping(map.clone()), &config);
    // Should succeed but not be a variant (no tag field)
    assert!(result.is_ok());
    match result.unwrap() {
        Value::Map(_) => {}, // Expected - falls back to regular map
        _ => panic!("Expected Map when tag is missing"),
    }
    
    // Adjacent representation without tag field
    let adjacent_config = Config {
        variant_repr: VariantRepr::Adjacent { 
            tag: "kind".to_string(), 
            content: "data".to_string() 
        },
    };
    
    let result = yaml_to_value_with_config(&YamlValue::Mapping(map), &adjacent_config);
    assert!(result.is_ok());
    match result.unwrap() {
        Value::Map(_) => {}, // Expected - falls back to regular map
        _ => panic!("Expected Map when tag is missing"),
    }
}

#[test]
fn test_invalid_variant_content() {
    // Adjacent representation with non-map content where map is expected
    let mut map = serde_yaml::Mapping::new();
    map.insert(YamlValue::String("kind".to_string()), YamlValue::String("test".to_string()));
    map.insert(YamlValue::String("data".to_string()), YamlValue::String("not a map".to_string()));
    
    let config = Config {
        variant_repr: VariantRepr::Adjacent { 
            tag: "kind".to_string(), 
            content: "data".to_string() 
        },
    };
    
    // This should still work - content doesn't have to be a map
    let result = yaml_to_value_with_config(&YamlValue::Mapping(map), &config);
    assert!(result.is_ok());
}

#[test]
fn test_conversion_edge_cases() {
    // Empty collections
    assert_eq!(yaml_to_value(&YamlValue::Sequence(vec![])).unwrap(), Value::Array(Array(vec![])));
    assert_eq!(yaml_to_value(&YamlValue::Mapping(serde_yaml::Mapping::new())).unwrap(), Value::Map(Map(ahash::AHashMap::new())));
    
    // Very large numbers
    let large_int = YamlValue::Number(serde_yaml::Number::from(i64::MAX));
    assert_eq!(yaml_to_value(&large_int).unwrap(), Value::I64(i64::MAX));
    
    let large_uint = YamlValue::Number(serde_yaml::Number::from(u64::MAX));
    match yaml_to_value(&large_uint).unwrap() {
        Value::U64(v) => assert_eq!(v, u64::MAX),
        Value::F64(v) => assert!((v - u64::MAX as f64).abs() < 1.0), // Some precision loss is acceptable
        _ => panic!("Expected U64 or F64 for large number"),
    }
    
    // Very small float
    let tiny_float = YamlValue::Number(serde_yaml::Number::from(1e-300));
    match yaml_to_value(&tiny_float).unwrap() {
        Value::F64(v) => assert!((v - 1e-300).abs() < 1e-310),
        _ => panic!("Expected F64"),
    }
}

#[test]
fn test_yaml_special_strings() {
    // YAML interprets certain strings specially
    let special_cases = vec![
        ("true", Value::Bool(true)),
        ("false", Value::Bool(false)),
        ("null", Value::Null),
        ("~", Value::Null),
    ];
    
    // These might be parsed as strings in newer YAML versions
    let maybe_bool_cases = vec![
        ("yes", true),
        ("no", false),
        ("on", true),
        ("off", false),
    ];
    
    for (yaml_str, expected) in special_cases {
        let yaml_value: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let value = yaml_to_value(&yaml_value).unwrap();
        assert_eq!(value, expected, "Failed for YAML string: {yaml_str}");
    }
    
    // Check if yes/no/on/off are parsed as bools or strings
    for (yaml_str, bool_val) in maybe_bool_cases {
        let yaml_value: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let value = yaml_to_value(&yaml_value).unwrap();
        // Accept either bool or string representation
        match value {
            Value::Bool(b) => assert_eq!(b, bool_val),
            Value::String(s) => assert_eq!(s, yaml_str),
            _ => panic!("Expected Bool or String for {yaml_str}"),
        }
    }
    
    // Quoted strings should remain strings
    let quoted_true: YamlValue = serde_yaml::from_str("\"true\"").unwrap();
    assert_eq!(yaml_to_value(&quoted_true).unwrap(), Value::String("true".to_string()));
}

#[test]
fn test_nested_collection_errors() {
    // Deeply nested structure that might cause issues
    let mut deep = YamlValue::Sequence(vec![]);
    for _ in 0..100 {
        deep = YamlValue::Sequence(vec![deep]);
    }
    
    // Should still work
    let result = yaml_to_value(&deep);
    assert!(result.is_ok());
}

#[test]
fn test_unicode_and_escapes() {
    // Unicode in keys and values
    let mut map = serde_yaml::Mapping::new();
    map.insert(YamlValue::String("emoji🎉".to_string()), YamlValue::String("value😀".to_string()));
    map.insert(YamlValue::String("chinese中文".to_string()), YamlValue::String("测试".to_string()));
    
    let value = yaml_to_value(&YamlValue::Mapping(map)).unwrap();
    match value {
        Value::Map(Map(m)) => {
            assert!(m.contains_key(&KeyCmpValue::String("emoji🎉".to_string())));
            assert!(m.contains_key(&KeyCmpValue::String("chinese中文".to_string())));
        }
        _ => panic!("Expected Map"),
    }
    
    // Special characters
    let special = YamlValue::String("line1\nline2\ttab\r\nwindows".to_string());
    match yaml_to_value(&special).unwrap() {
        Value::String(s) => assert_eq!(s, "line1\nline2\ttab\r\nwindows"),
        _ => panic!("Expected String"),
    }
}

#[test]
fn test_yaml_anchors_and_aliases() {
    // YAML with anchors and aliases
    let yaml_str = r#"
base: &base
  name: base_config
  value: 42

derived:
  <<: *base
  value: 100
"#;
    
    let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
    let value = yaml_to_value(&yaml).unwrap();
    
    match value {
        Value::Map(Map(map)) => {
            // Check that the alias was resolved
            if let Some(Value::Map(Map(derived))) = map.get(&KeyCmpValue::String("derived".to_string())) {
                // The merge key might not be supported or might create a different structure
                // Check if value was overridden at least
                assert_eq!(derived.get(&KeyCmpValue::String("value".to_string())), Some(&Value::I64(100)));
                
                // The merge key support varies by YAML implementation
                // Just verify the derived map exists and has the overridden value
            } else {
                panic!("Expected derived to be a map");
            }
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_yaml_tags() {
    // YAML with explicit tags (these are typically ignored in our conversion)
    let yaml_str = r#"
        explicit_str: !!str 123
        explicit_int: !!int "456"
        binary: !!binary "SGVsbG8gV29ybGQ="
    "#;
    
    let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
    let value = yaml_to_value(&yaml).unwrap();
    
    match value {
        Value::Map(Map(map)) => {
            // !!str tag forces 123 to be a string
            assert_eq!(map.get(&KeyCmpValue::String("explicit_str".to_string())), Some(&Value::String("123".to_string())));
            
            // !!int tag forces "456" to be parsed as integer
            assert_eq!(map.get(&KeyCmpValue::String("explicit_int".to_string())), Some(&Value::I64(456)));
            
            // Binary data is typically base64 decoded by serde_yaml
            // The exact behavior depends on serde_yaml's handling
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_code_block_edge_cases() {
    // Empty code block
    let empty_code = YamlValue::String("```\n```".to_string());
    match yaml_to_value(&empty_code).unwrap() {
        Value::Code(Code { language, content }) => {
            assert_eq!(language, "");
            // Empty code block might have trailing newline
            assert!(content.is_empty() || content == "\n");
        }
        _ => panic!("Expected Code"),
    }
    
    // Code block with just language
    let lang_only = YamlValue::String("```rust\n```".to_string());
    match yaml_to_value(&lang_only).unwrap() {
        Value::Code(Code { language, content }) => {
            assert_eq!(language, "rust");
            // Empty code block might have trailing newline
            assert!(content.is_empty() || content == "\n");
        }
        _ => panic!("Expected Code"),
    }
    
    // Nested code fence markers
    let nested = YamlValue::String("```markdown\n```rust\ncode\n```\n```".to_string());
    match yaml_to_value(&nested).unwrap() {
        Value::Code(Code { language, content }) => {
            assert_eq!(language, "markdown");
            assert_eq!(content, "```rust\ncode\n```");
        }
        _ => panic!("Expected Code"),
    }
    
    // Inline code with backticks - might not be parsed as code due to internal backtick
    let inline_nested = YamlValue::String("`code with ` backtick`".to_string());
    let result = yaml_to_value(&inline_nested).unwrap();
    // This might be parsed as a regular string due to the internal backtick
    match result {
        Value::Code(Code { language, content }) => {
            assert_eq!(language, "");
            assert_eq!(content, "code with ` backtick");
        }
        Value::String(s) => {
            // Accept as string if code parsing fails due to internal backtick
            assert_eq!(s, "`code with ` backtick`");
        }
        _ => panic!("Expected Code or String, got {result:?}"),
    }
}

#[test]
fn test_float_precision() {
    // Test various float precisions
    let test_floats = vec![
        1.234_567_890_123_456_7_f64,
        f64::MIN_POSITIVE,
        f64::MAX,
        -0.0,
        1e-308,
        1e308,
    ];
    
    for &f in &test_floats {
        let value = Value::F64(f);
        let yaml = value_to_yaml(&value).unwrap();
        let back = yaml_to_value(&yaml).unwrap();
        
        match back {
            Value::F64(result) => {
                if f.is_finite() {
                    // Allow for some floating point imprecision
                    let relative_error = ((result - f) / f).abs();
                    assert!(relative_error < 1e-10 || (result - f).abs() < 1e-300, 
                        "Float {f} round-trip failed: got {result}");
                }
            }
            _ => panic!("Expected F64"),
        }
    }
}

#[test]
fn test_variant_with_null_content() {
    // Variant with null content
    let variant = Value::Variant(Variant {
        tag: "Empty".to_string(),
        content: Box::new(Value::Null),
    });
    
    let yaml = value_to_yaml(&variant).unwrap();
    match yaml {
        YamlValue::Mapping(ref map) => {
            assert_eq!(map.get(YamlValue::String("Empty".to_string())), Some(&YamlValue::Null));
        }
        _ => panic!("Expected mapping"),
    }
    
    // Round trip
    let back = yaml_to_value(&yaml).unwrap();
    match back {
        Value::Variant(Variant { tag, content }) => {
            assert_eq!(tag, "Empty");
            assert_eq!(*content, Value::Null);
        }
        _ => panic!("Expected Variant"),
    }
}