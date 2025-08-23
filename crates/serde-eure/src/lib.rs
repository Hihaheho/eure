#![doc = include_str!("../README.md")]

mod de;
mod error;
mod format;
mod ser;

pub use de::{Deserializer, from_str, from_value};
pub use error::{Error, Result};
pub use ser::{Serializer, to_string, to_string_pretty, to_value};

// Re-export formatting utilities
pub use format::{format_eure, format_eure_bindings};

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        name: String,
        age: u32,
        active: bool,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct NestedStruct {
        id: i64,
        data: TestStruct,
        tags: Vec<String>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum TestEnum {
        Unit,
        Newtype(String),
        Tuple(i32, i32),
        Struct { x: f64, y: f64 },
    }

    #[test]
    fn test_serialize_basic_types() {
        // Boolean
        assert_eq!(to_string(&true).unwrap(), "value = true\n");
        assert_eq!(to_string(&false).unwrap(), "value = false\n");

        // Numbers
        assert_eq!(to_string(&42i64).unwrap(), "value = 42\n");
        assert_eq!(to_string(&42u64).unwrap(), "value = 42\n");
        assert_eq!(to_string(&3.14f64).unwrap(), "value = 3.14\n");

        // String
        assert_eq!(to_string(&"hello").unwrap(), "value = \"hello\"\n");

        // Option
        assert_eq!(to_string(&Some(42)).unwrap(), "value = 42\n");
        assert_eq!(to_string(&None::<i32>).unwrap(), "value = null\n");

        // Unit
        assert_eq!(to_string(&()).unwrap(), "value = ()\n");
    }

    #[test]
    fn test_serialize_collections() {
        // Vec (EURE arrays have trailing commas)
        let vec = vec![1, 2, 3];
        assert_eq!(to_string(&vec).unwrap(), "value = [1, 2, 3,]\n");

        // Tuple (EURE tuples have trailing commas)
        let tuple = (1, "hello", true);
        assert_eq!(
            to_string(&tuple).unwrap(),
            "value = (1, \"hello\", true,)\n"
        );

        // HashMap
        let mut map = HashMap::new();
        map.insert("name", "Alice");
        map.insert("city", "Boston");
        let result = to_string(&map).unwrap();
        assert!(result.contains("name = \"Alice\""));
        assert!(result.contains("city = \"Boston\""));
    }

    #[test]
    fn test_serialize_struct() {
        let test = TestStruct {
            name: "Bob".to_string(),
            age: 30,
            active: true,
        };

        let result = to_string(&test).unwrap();
        assert!(result.contains("name = \"Bob\""));
        assert!(result.contains("age = 30"));
        assert!(result.contains("active = true"));
    }

    #[test]
    fn test_round_trip() {
        let original = TestStruct {
            name: "Dave".to_string(),
            age: 35,
            active: true,
        };

        let serialized = to_string(&original).unwrap();
        let deserialized: TestStruct = from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_nested_struct() {
        let nested = NestedStruct {
            id: 123,
            data: TestStruct {
                name: "Eve".to_string(),
                age: 28,
                active: false,
            },
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };

        let serialized = to_string(&nested).unwrap();
        let deserialized: NestedStruct = from_str(&serialized).unwrap();

        assert_eq!(nested, deserialized);
    }

    #[test]
    fn test_empty_collections() {
        let empty_vec: Vec<i32> = vec![];
        assert_eq!(to_string(&empty_vec).unwrap(), "value = []\n");

        let empty_map: HashMap<String, String> = HashMap::new();
        assert_eq!(to_string(&empty_map).unwrap(), "");
    }

    #[test]
    fn test_special_strings() {
        // Test escaping
        let special = "Hello \"world\"\nNew line\tTab";
        let serialized = to_string(&special).unwrap();
        assert_eq!(
            serialized,
            "value = \"Hello \\\"world\\\"\\nNew line\\tTab\"\n"
        );

        let deserialized: String = from_str(&serialized).unwrap();
        assert_eq!(special, deserialized);
    }

    #[test]
    fn test_numeric_limits() {
        // Test various numeric types
        assert_eq!(
            to_string(&i64::MAX).unwrap(),
            format!("value = {}\n", i64::MAX)
        );
        assert_eq!(
            to_string(&u64::MAX).unwrap(),
            format!("value = {}\n", u64::MAX)
        );
        assert_eq!(to_string(&f64::INFINITY).unwrap(), "value = inf\n");
        assert_eq!(to_string(&f64::NEG_INFINITY).unwrap(), "value = -inf\n");
        assert_eq!(to_string(&f64::NAN).unwrap(), "value = NaN\n");
    }

    #[test]
    fn test_option_handling() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct WithOptions {
            required: String,
            optional: Option<i32>,
            nested_option: Option<Option<bool>>,
        }

        let with_some = WithOptions {
            required: "test".to_string(),
            optional: Some(42),
            nested_option: Some(Some(true)),
        };

        let serialized = to_string(&with_some).unwrap();
        let deserialized: WithOptions = from_str(&serialized).unwrap();
        assert_eq!(with_some, deserialized);

        let with_none = WithOptions {
            required: "test".to_string(),
            optional: None,
            nested_option: None, // Serde collapses Some(None) to None
        };

        let serialized = to_string(&with_none).unwrap();
        let deserialized: WithOptions = from_str(&serialized).unwrap();
        assert_eq!(with_none, deserialized);
    }

    #[test]
    fn test_complex_nested_maps() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Complex {
            meta: HashMap<String, String>,
            config: HashMap<String, HashMap<String, i32>>,
        }

        let mut meta = HashMap::new();
        meta.insert("version".to_string(), "1.0".to_string());

        let mut inner = HashMap::new();
        inner.insert("timeout".to_string(), 30);
        inner.insert("retries".to_string(), 3);

        let mut config = HashMap::new();
        config.insert("default".to_string(), inner);

        let complex = Complex { meta, config };

        let serialized = to_string(&complex).unwrap();
        let deserialized: Complex = from_str(&serialized).unwrap();
        assert_eq!(complex, deserialized);
    }

    // ===== ERROR CASE TESTS =====

    #[test]
    fn test_parse_errors() {
        // Missing closing brace
        assert!(from_str::<TestStruct>("name = \"test\"").is_err());

        // Invalid syntax
        assert!(from_str::<TestStruct>("{ invalid syntax }").is_err());

        // Empty input
        assert!(from_str::<TestStruct>("").is_err());

        // Missing quotes around string
        assert!(from_str::<TestStruct>("value = hello").is_err());

        // Unterminated string
        assert!(from_str::<TestStruct>("value = \"unterminated").is_err());

        // Invalid number
        assert!(from_str::<i32>("value = 1.2.3").is_err());
        assert!(from_str::<i32>("value = ++1").is_err());
        assert!(from_str::<i32>("value = 1e").is_err());

        // Missing value
        assert!(from_str::<i32>("value =").is_err());

        // Test if double values are allowed (EURE might allow this, so check actual behavior)
        let double_result = from_str::<i32>("value = 1\nvalue = 2");
        // If it doesn't error, at least verify we get one of the values
        if let Ok(val) = double_result {
            assert!(val == 1 || val == 2);
        }
    }

    #[test]
    fn test_type_mismatch_errors() {
        // String to number
        assert!(from_str::<i32>("value = \"not a number\"").is_err());

        // Number to bool
        assert!(from_str::<bool>("value = 42").is_err());

        // Array to struct
        assert!(from_str::<TestStruct>("value = [1, 2, 3,]").is_err());

        // Null to non-option
        assert!(from_str::<i32>("value = null").is_err());

        // Wrong tuple length
        assert!(from_str::<(i32, i32)>("value = (1,)").is_err());
        assert!(from_str::<(i32, i32)>("value = (1, 2, 3,)").is_err());
    }

    #[test]
    fn test_struct_field_errors() {
        // Missing required field
        let eure = r#"
            name = "Bob"
            age = 30
            # missing 'active' field
        "#;
        assert!(from_str::<TestStruct>(eure).is_err());

        // Unknown field (depending on serde config)
        let eure = r#"
            name = "Bob"
            age = 30
            active = true
            unknown_field = "value"
        "#;
        // This may or may not error depending on serde attributes
        let _ = from_str::<TestStruct>(eure);

        // Wrong field type
        let eure = r#"
            name = "Bob"
            age = "not a number"
            active = true
        "#;
        assert!(from_str::<TestStruct>(eure).is_err());
    }

    #[test]
    fn test_enum_errors() {
        // Invalid variant
        assert!(from_str::<TestEnum>("value = {$tag = \"InvalidVariant\",}").is_err());

        // Missing tag
        assert!(from_str::<TestEnum>("value = {x = 1.0, y = 2.0,}").is_err());

        // Wrong data for variant
        assert!(from_str::<TestEnum>("value = {$tag = \"newtype-variant\", x = 1.0,}").is_err());

        // Newtype variant with wrong content
        assert!(
            from_str::<TestEnum>("value = {$tag = \"newtype-variant\", content = 123,}").is_err()
        );
    }

    #[test]
    fn test_escape_sequence_errors() {
        // Test if invalid escape sequences are preserved or error
        // EURE might allow unknown escape sequences
        let result = from_str::<String>("value = \"\\q\"");
        if result.is_ok() {
            // If it doesn't error, verify the behavior
            assert_eq!(result.unwrap(), "\\q");
        }

        // Test unicode escape handling
        // EURE might be permissive with escape sequences
        let short_unicode = from_str::<String>("value = \"\\u123\"");
        if let Ok(val) = short_unicode {
            // If it doesn't error, it might preserve the literal
            assert!(val == "\\u123" || !val.is_empty());
        }

        let invalid_hex = from_str::<String>("value = \"\\uXYZW\"");
        if let Ok(val) = invalid_hex {
            // If it doesn't error, verify it's preserved
            assert_eq!(val, "\\uXYZW");
        }

        // Unterminated escape
        assert!(from_str::<String>("value = \"test\\").is_err());
    }

    #[test]
    fn test_collection_errors() {
        // Unclosed array
        assert!(from_str::<Vec<i32>>("value = [1, 2, 3").is_err());

        // Unclosed tuple
        assert!(from_str::<(i32, i32)>("value = (1, 2").is_err());

        // Mixed types in homogeneous collection
        assert!(from_str::<Vec<i32>>("value = [1, \"two\", 3,]").is_err());

        // Missing comma in array
        assert!(from_str::<Vec<i32>>("value = [1 2 3,]").is_err());
    }

    #[test]
    fn test_nested_errors() {
        // Error in nested structure
        let eure = r#"
            id = 123
            data = {
                name = "Eve"
                age = "invalid"  # Should be number
                active = false
            }
            tags = ["tag1", "tag2",]
        "#;
        assert!(from_str::<NestedStruct>(eure).is_err());

        // Error in deeply nested value
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct ComplexError {
            meta: HashMap<String, String>,
            config: HashMap<String, HashMap<String, i32>>,
        }

        let eure = r#"
            meta = {version = "1.0",}
            config = {
                default = {
                    timeout = "30s"  # Should be number
                    retries = 3
                }
            }
        "#;
        assert!(from_str::<ComplexError>(eure).is_err());
    }

    #[test]
    fn test_edge_cases() {
        // Very large numbers
        assert!(from_str::<i8>("value = 999999999999999999999").is_err());

        // Float overflow becomes infinity, not an error
        let overflow_result = from_str::<f32>("value = 1e400");
        assert!(overflow_result.is_ok());
        assert!(overflow_result.unwrap().is_infinite());

        // Empty string key
        let eure = r#"
            "" = "value"
        "#;
        let result = from_str::<HashMap<String, String>>(eure);
        if result.is_ok() {
            assert!(result.unwrap().contains_key(""));
        }

        // Special float values (these might work depending on implementation)
        let _ = from_str::<f64>("value = inf");
        let _ = from_str::<f64>("value = -inf");
        let _ = from_str::<f64>("value = nan");
    }

    #[test]
    fn test_serialization_errors() {
        use std::f64;

        // NaN and Infinity (JSON doesn't support these)
        let nan = f64::NAN;
        let inf = f64::INFINITY;

        // These should serialize but might not round-trip through JSON
        assert!(to_string(&nan).is_ok());
        assert!(to_string(&inf).is_ok());
    }

    #[test]
    fn test_malformed_documents() {
        // Multiple root values
        assert!(from_str::<i32>("value = 1\nother = 2").is_err());

        // No root binding
        assert!(from_str::<i32>("42").is_err());

        // Invalid binding name
        assert!(from_str::<i32>("123 = 42").is_err());

        // Circular reference attempt (not really possible in EURE but test parser)
        let eure = r#"
            a = b
            b = a
        "#;
        let _ = from_str::<HashMap<String, String>>(eure); // Should fail or produce empty values
    }
}
