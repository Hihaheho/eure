#![allow(clippy::approx_constant)]
use crate::{json_to_value, json_to_value_with_config, value_to_json, value_to_json_with_config};
use eure_value::value::Value;

// Helper functions to simplify tests
fn to_json_string(value: &Value) -> Result<String, crate::Error> {
    value_to_json(value).map(|json| serde_json::to_string(&json).unwrap())
}

fn from_json_string(json_str: &str) -> Result<Value, crate::Error> {
    serde_json::from_str(json_str)
        .map_err(|e| crate::Error::ConversionError(e.to_string()))
        .and_then(|json| json_to_value(&json))
}

#[test]
fn test_dollar_tag_preservation() {
    use ahash::AHashMap;
    use eure_value::value::{KeyCmpValue, Map, Value};

    // Create a map with $tag
    let mut map = AHashMap::new();
    map.insert(
        KeyCmpValue::String("$tag".to_string()),
        Value::String("TestVariant".to_string()),
    );
    map.insert(
        KeyCmpValue::String("field".to_string()),
        Value::String("value".to_string()),
    );

    let value = Value::Map(Map(map));

    // Convert to JSON
    let json = value_to_json(&value).unwrap();
    let json_str = serde_json::to_string(&json).unwrap();
    println!("JSON: {json_str}");

    // Check that $tag is preserved
    assert!(json_str.contains("\"$tag\""));

    // Parse back
    let parsed = json_to_value(&json).unwrap();

    // Check the parsed value
    match parsed {
        Value::Map(Map(m)) => {
            assert!(m.contains_key(&KeyCmpValue::String("$tag".to_string())));
            assert!(m.contains_key(&KeyCmpValue::String("field".to_string())));
        }
        _ => panic!("Expected Map"),
    }
}

// ===== BASIC TYPE CONVERSIONS =====

#[test]
fn test_basic_type_conversions() {
    use eure_value::value::{Array, Value};

    // Null
    assert_eq!(to_json_string(&Value::Null).unwrap(), "null");
    assert_eq!(from_json_string("null").unwrap(), Value::Null);

    // Boolean
    assert_eq!(to_json_string(&Value::Bool(true)).unwrap(), "true");
    assert_eq!(to_json_string(&Value::Bool(false)).unwrap(), "false");
    assert_eq!(from_json_string("true").unwrap(), Value::Bool(true));
    assert_eq!(from_json_string("false").unwrap(), Value::Bool(false));

    // Numbers
    assert_eq!(to_json_string(&Value::F64(3.14)).unwrap(), "3.14");
    assert_eq!(from_json_string("42").unwrap(), Value::I64(42));
    assert_eq!(from_json_string("3.14").unwrap(), Value::F64(3.14));
    assert_eq!(from_json_string("-42").unwrap(), Value::I64(-42));

    // String
    assert_eq!(
        to_json_string(&Value::String("hello".to_string())).unwrap(),
        "\"hello\""
    );
    assert_eq!(
        from_json_string("\"world\"").unwrap(),
        Value::String("world".to_string())
    );

    // Empty collections
    assert_eq!(to_json_string(&Value::Array(Array(vec![]))).unwrap(), "[]");
    assert_eq!(from_json_string("[]").unwrap(), Value::Array(Array(vec![])));
}

#[test]
fn test_string_escaping() {
    use eure_value::value::Value;

    // Test escape sequences
    let test_cases = vec![
        ("hello\nworld", "\"hello\\nworld\""),
        ("tab\there", "\"tab\\there\""),
        ("quote\"test", "\"quote\\\"test\""),
        ("backslash\\test", "\"backslash\\\\test\""),
        ("unicode\u{1F600}", "\"unicode😀\""),
    ];

    for (input, expected) in test_cases {
        let value = Value::String(input.to_string());
        let json = to_json_string(&value).unwrap();
        assert_eq!(json, expected);

        // Round trip
        let parsed = from_json_string(&json).unwrap();
        assert_eq!(parsed, value);
    }
}

// ===== ERROR CASES =====

#[test]
fn test_json_parse_errors() {
    // Invalid JSON syntax
    assert!(from_json_string("").is_err());
    assert!(from_json_string("{").is_err());
    assert!(from_json_string("}").is_err());
    assert!(from_json_string("[1, 2,").is_err());
    assert!(from_json_string("\"unterminated").is_err());
    assert!(from_json_string("undefined").is_err());
    assert!(from_json_string("NaN").is_err());
    assert!(from_json_string("Infinity").is_err());
    assert!(from_json_string("{\"key\": }").is_err());
    assert!(from_json_string("[1, , 3]").is_err());

    // Invalid escape sequences
    assert!(from_json_string("\"\\x\"").is_err());
    assert!(from_json_string("\"\\u123\"").is_err()); // Too short
    assert!(from_json_string("\"\\uGHIJ\"").is_err()); // Invalid hex
}

#[test]
fn test_variant_conversion_errors() {
    use eure_value::value::{Value, Variant};

    // Test external variant representation
    let variant = Value::Variant(Variant {
        tag: "TestVariant".to_string(),
        content: Box::new(Value::String("content".to_string())),
    });

    let json = to_json_string(&variant).unwrap();
    assert!(json.contains("\"TestVariant\""));

    // Round trip - the JSON converter reconstructs Variants
    let json_value = value_to_json(&variant).unwrap();
    let parsed = json_to_value(&json_value).unwrap();

    // The JSON converter is smart enough to reconstruct the Variant
    match parsed {
        Value::Variant(Variant { tag, content }) => {
            assert_eq!(tag, "TestVariant");
            assert_eq!(*content, Value::String("content".to_string()));
        }
        _ => panic!("Expected Variant for external variant, got {parsed:?}"),
    }
}

#[test]
fn test_special_float_values() {
    use eure_value::value::Value;

    // NaN and Infinity are not valid JSON
    let nan = Value::F64(f64::NAN);
    let inf = Value::F64(f64::INFINITY);
    let neg_inf = Value::F64(f64::NEG_INFINITY);

    // These should error because JSON doesn't support NaN/Infinity
    let nan_json = to_json_string(&nan);
    let inf_json = to_json_string(&inf);
    let neg_inf_json = to_json_string(&neg_inf);

    // JSON doesn't support NaN/Infinity, so they should error
    assert!(nan_json.is_err());
    assert!(inf_json.is_err());
    assert!(neg_inf_json.is_err());
}

#[test]
fn test_deeply_nested_structures() {
    use ahash::AHashMap;
    use eure_value::value::{Array, KeyCmpValue, Map, Value};

    // Create deeply nested structure
    let mut innermost = AHashMap::new();
    innermost.insert(KeyCmpValue::String("leaf".to_string()), Value::Bool(true));

    let mut inner = AHashMap::new();
    inner.insert(
        KeyCmpValue::String("nested".to_string()),
        Value::Map(Map(innermost)),
    );

    let mut outer = AHashMap::new();
    outer.insert(
        KeyCmpValue::String("data".to_string()),
        Value::Map(Map(inner)),
    );
    outer.insert(
        KeyCmpValue::String("array".to_string()),
        Value::Array(Array(vec![
            Value::F64(1.0),
            Value::Array(Array(vec![Value::F64(2.0), Value::F64(3.0)])),
        ])),
    );

    let value = Value::Map(Map(outer));

    // Convert to JSON
    let json = to_json_string(&value).unwrap();

    // Parse back
    let parsed = from_json_string(&json).unwrap();

    // Check that the structure is preserved (maps remain maps, not variants)
    // The exact equality might not hold due to JSON conversion differences
    match (parsed, value) {
        (Value::Map(Map(parsed_map)), Value::Map(Map(orig_map))) => {
            // Check that both have the same keys
            assert_eq!(parsed_map.len(), orig_map.len());
            assert!(parsed_map.contains_key(&KeyCmpValue::String("data".to_string())));
            assert!(parsed_map.contains_key(&KeyCmpValue::String("array".to_string())));
        }
        _ => panic!("Expected both to be Maps"),
    }
}

#[test]
fn test_empty_and_whitespace_keys() {
    use ahash::AHashMap;
    use eure_value::value::{KeyCmpValue, Map, Value};

    // Empty key
    let mut map = AHashMap::new();
    map.insert(
        KeyCmpValue::String("".to_string()),
        Value::String("empty key".to_string()),
    );
    map.insert(
        KeyCmpValue::String(" ".to_string()),
        Value::String("space key".to_string()),
    );
    map.insert(
        KeyCmpValue::String("\t".to_string()),
        Value::String("tab key".to_string()),
    );

    let value = Value::Map(Map(map));

    // Convert to JSON
    let json = to_json_string(&value).unwrap();

    // Should contain the empty keys
    assert!(json.contains("\"\""));
    assert!(json.contains("\" \""));
    assert!(json.contains("\"\\t\""));

    // Parse back
    let parsed = from_json_string(&json).unwrap();
    match parsed {
        Value::Map(Map(m)) => {
            assert!(m.contains_key(&KeyCmpValue::String("".to_string())));
            assert!(m.contains_key(&KeyCmpValue::String(" ".to_string())));
            assert!(m.contains_key(&KeyCmpValue::String("\t".to_string())));
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_large_numbers() {
    use eure_value::value::Value;

    // Test very large and small numbers
    let large = Value::F64(1e308);
    let small = Value::F64(1e-308);
    let negative_large = Value::F64(-1e308);

    // Convert to JSON
    let large_json = to_json_string(&large).unwrap();
    let small_json = to_json_string(&small).unwrap();
    let neg_json = to_json_string(&negative_large).unwrap();

    // Parse back
    assert_eq!(from_json_string(&large_json).unwrap(), large);
    assert_eq!(from_json_string(&small_json).unwrap(), small);
    assert_eq!(from_json_string(&neg_json).unwrap(), negative_large);

    // Test precision limits
    let precise = Value::F64(1.234_567_890_123_456_7);
    let json = to_json_string(&precise).unwrap();
    let parsed = from_json_string(&json).unwrap();

    // Some precision may be lost
    match parsed {
        Value::F64(v) => assert!((v - 1.234_567_890_123_456_7).abs() < 1e-10),
        _ => panic!("Expected F64"),
    }
}

#[test]
fn test_variant_representations() {
    use crate::Config;
    use ahash::AHashMap;
    use eure_value::value::{KeyCmpValue, Map, Value, Variant};

    let content_map = {
        let mut m = AHashMap::new();
        m.insert(KeyCmpValue::String("x".to_string()), Value::F64(1.0));
        m.insert(KeyCmpValue::String("y".to_string()), Value::F64(2.0));
        Value::Map(Map(m))
    };

    let variant = Value::Variant(Variant {
        tag: "Point".to_string(),
        content: Box::new(content_map.clone()),
    });

    // Test different representations
    let configs = vec![
        Config {
            variant_repr: crate::VariantRepr::External,
        },
        Config {
            variant_repr: crate::VariantRepr::Internal {
                tag: "type".to_string(),
            },
        },
        Config {
            variant_repr: crate::VariantRepr::Adjacent {
                tag: "t".to_string(),
                content: "c".to_string(),
            },
        },
        Config {
            variant_repr: crate::VariantRepr::Untagged,
        },
    ];

    for config in configs {
        let json = value_to_json_with_config(&variant, &config).unwrap();
        let json_str = serde_json::to_string(&json).unwrap();

        // Verify structure based on representation
        match &config.variant_repr {
            crate::VariantRepr::External => {
                assert!(json_str.contains("\"Point\""));
            }
            crate::VariantRepr::Internal { tag } => {
                assert!(json_str.contains(&format!("\"{tag}\"")));
                assert!(json_str.contains("\"Point\""));
            }
            crate::VariantRepr::Adjacent { tag, content } => {
                assert!(json_str.contains(&format!("\"{tag}\"")));
                assert!(json_str.contains(&format!("\"{content}\"")));
                assert!(json_str.contains("\"Point\""));
            }
            crate::VariantRepr::Untagged => {
                assert!(!json_str.contains("\"Point\""));
            }
        }

        // Round trip with same config
        let parsed = json_to_value_with_config(&json, &config).unwrap();

        // For untagged, we lose the variant info
        if let crate::VariantRepr::Untagged = config.variant_repr {
            assert_eq!(parsed, content_map);
        }
    }
}

#[test]
fn test_unicode_handling() {
    use eure_value::value::Value;

    // Various unicode strings
    let test_cases = vec![
        "Hello, 世界",
        "Emoji: 😀🎉🚀",
        "Math: ∑∏∫∂",
        "Mixed: café ☕",
        "\u{0000}null\u{0001}char",
        "Right-to-left: العربية",
    ];

    for text in test_cases {
        let value = Value::String(text.to_string());
        let json = to_json_string(&value).unwrap();
        let parsed = from_json_string(&json).unwrap();
        assert_eq!(parsed, value);
    }
}

#[test]
fn test_tuple_conversions() {
    use eure_value::value::{Tuple, Value};

    // Empty tuple
    let empty = Value::Tuple(Tuple(vec![]));
    assert_eq!(to_json_string(&empty).unwrap(), "[]");

    // Single element tuple
    let single = Value::Tuple(Tuple(vec![Value::I64(42)]));
    assert_eq!(to_json_string(&single).unwrap(), "[42]");

    // Multiple elements
    let multi = Value::Tuple(Tuple(vec![
        Value::String("hello".to_string()),
        Value::I64(123),
        Value::Bool(true),
    ]));
    let json = to_json_string(&multi).unwrap();
    assert_eq!(json, "[\"hello\",123,true]");

    // Note: When parsing back from JSON, tuples become arrays
    let parsed = from_json_string(&json).unwrap();
    match parsed {
        Value::Array(_) => {} // Expected: JSON doesn't distinguish tuples from arrays
        _ => panic!("Expected Array"),
    }
}
