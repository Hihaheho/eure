use ahash::AHashMap;
use eure_value::value::{KeyCmpValue, Map, Value};
use serde::{Deserialize, Serialize};
use serde_eure::{from_str, from_value, to_string, to_value};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$tag")]
enum DollarTagEnum {
    Unit,
    Struct { field: String },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct DollarFields {
    #[serde(rename = "$id")]
    id: u32,
    #[serde(rename = "$type")]
    type_field: String,
    #[serde(rename = "$meta")]
    metadata: Metadata,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Metadata {
    version: String,
    author: String,
}

#[test]
fn test_dollar_in_tag_name() {
    // Test serialization
    let unit = DollarTagEnum::Unit;
    let value = to_value(&unit).unwrap();

    // Verify the serialized value has the correct structure
    if let Value::Map(Map(map)) = &value {
        assert!(map.contains_key(&KeyCmpValue::String("$tag".to_string())));
        assert_eq!(
            map.get(&KeyCmpValue::String("$tag".to_string())),
            Some(&Value::String("Unit".to_string()))
        );
    } else {
        panic!("Expected Map value");
    }

    // Test deserialization from Value
    let deserialized: DollarTagEnum = from_value(value).unwrap();
    assert_eq!(unit, deserialized);

    // Test string roundtrip
    let serialized = to_string(&unit).unwrap();
    assert!(serialized.contains("$tag"));
    let from_string: DollarTagEnum = from_str(&serialized).unwrap();
    assert_eq!(unit, from_string);
}

#[test]
fn test_dollar_in_field_names() {
    let data = DollarFields {
        id: 123,
        type_field: "user".to_string(),
        metadata: Metadata {
            version: "1.0".to_string(),
            author: "test".to_string(),
        },
    };

    let serialized = to_string(&data).unwrap();
    assert!(serialized.contains("$id"));
    assert!(serialized.contains("$type"));
    assert!(serialized.contains("$meta"));

    let deserialized: DollarFields = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_dollar_tag_direct_value_creation() {
    // Test creating Value directly with $ in key
    let mut map = AHashMap::new();
    map.insert(
        KeyCmpValue::String("$tag".to_string()),
        Value::String("Unit".to_string()),
    );
    let value = Value::Map(Map(map));

    // This should deserialize successfully
    let result: DollarTagEnum = from_value(value).unwrap();
    assert_eq!(result, DollarTagEnum::Unit);

    // Test struct variant
    let mut struct_map = AHashMap::new();
    struct_map.insert(
        KeyCmpValue::String("$tag".to_string()),
        Value::String("Struct".to_string()),
    );
    struct_map.insert(
        KeyCmpValue::String("field".to_string()),
        Value::String("test".to_string()),
    );
    let struct_value = Value::Map(Map(struct_map));

    let struct_result: DollarTagEnum = from_value(struct_value).unwrap();
    assert_eq!(
        struct_result,
        DollarTagEnum::Struct {
            field: "test".to_string()
        }
    );
}

#[test]
fn test_dollar_fields_in_arrays() {
    let array = vec![
        DollarTagEnum::Unit,
        DollarTagEnum::Struct {
            field: "test".to_string(),
        },
    ];

    let serialized = to_string(&array).unwrap();
    assert!(serialized.contains("$tag"));

    let deserialized: Vec<DollarTagEnum> = from_str(&serialized).unwrap();
    assert_eq!(array, deserialized);
}

#[test]
fn test_multiple_dollar_prefixed_fields() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct MultiDollar {
        #[serde(rename = "$first")]
        first: String,
        #[serde(rename = "$second")]
        second: i32,
        #[serde(rename = "$third")]
        third: bool,
        regular: String,
    }

    let data = MultiDollar {
        first: "one".to_string(),
        second: 2,
        third: true,
        regular: "normal".to_string(),
    };

    let serialized = to_string(&data).unwrap();
    assert!(serialized.contains("$first"));
    assert!(serialized.contains("$second"));
    assert!(serialized.contains("$third"));
    assert!(serialized.contains("regular"));

    let deserialized: MultiDollar = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_nested_dollar_fields() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Outer {
        #[serde(rename = "$outer")]
        outer_field: String,
        inner: Inner,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Inner {
        #[serde(rename = "$inner")]
        inner_field: String,
    }

    let data = Outer {
        outer_field: "outer".to_string(),
        inner: Inner {
            inner_field: "inner".to_string(),
        },
    };

    let serialized = to_string(&data).unwrap();
    assert!(serialized.contains("$outer"));
    assert!(serialized.contains("$inner"));

    let deserialized: Outer = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_dollar_variant_compatibility() {
    // Test that $variant fields (used internally by EURE) don't conflict
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct WithDollarVariant {
        #[serde(rename = "$variant")]
        variant: String,
        other: String,
    }

    let data = WithDollarVariant {
        variant: "test_variant".to_string(),
        other: "data".to_string(),
    };

    let serialized = to_string(&data).unwrap();
    assert!(serialized.contains("$variant"));

    let deserialized: WithDollarVariant = from_str(&serialized).unwrap();
    assert_eq!(data, deserialized);
}
