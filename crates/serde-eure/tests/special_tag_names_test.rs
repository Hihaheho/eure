use serde::{Deserialize, Serialize};
use serde_eure::{from_str, to_string, from_value};
use eure_value::value::{Value, Map, KeyCmpValue};
use ahash::AHashMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
enum UnderscoreTagEnum {
    Unit,
    Struct { field: String },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag-with-dash")]
enum DashTagEnum {
    Unit,
    Newtype(i32),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "@tag")]
enum AtTagEnum {
    Unit,
    Struct { value: u32 },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$tag")]
enum DollarTagEnum {
    Unit,
    Struct { data: String },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
enum AdjacentEnum {
    Unit,
    Newtype(String),
    Struct { field: i32 },
}

#[test]
#[ignore = "Underscore prefix in tag names is not currently supported by serde-eure deserializer"]
fn test_underscore_tag() {
    // Test unit variant
    let unit = UnderscoreTagEnum::Unit;
    let serialized = to_string(&unit).unwrap();
    assert!(serialized.contains("_tag"));
    assert!(serialized.contains("Unit"));
    
    let deserialized: UnderscoreTagEnum = from_str(&serialized).unwrap();
    assert_eq!(unit, deserialized);
    
    // Test struct variant
    let struct_var = UnderscoreTagEnum::Struct { field: "test".to_string() };
    let serialized = to_string(&struct_var).unwrap();
    let deserialized: UnderscoreTagEnum = from_str(&serialized).unwrap();
    assert_eq!(struct_var, deserialized);
    
    // Test direct value creation
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::String("_tag".to_string()), Value::String("Unit".to_string()));
    let value = Value::Map(Map(map));
    
    let from_value: UnderscoreTagEnum = from_value(value).unwrap();
    assert_eq!(from_value, UnderscoreTagEnum::Unit);
}

#[test]
fn test_dash_tag() {
    let unit = DashTagEnum::Unit;
    let serialized = to_string(&unit).unwrap();
    assert!(serialized.contains("tag-with-dash"));
    assert!(serialized.contains("Unit"));
    
    let deserialized: DashTagEnum = from_str(&serialized).unwrap();
    assert_eq!(unit, deserialized);
    
    // Test direct value creation
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::String("tag-with-dash".to_string()), Value::String("Unit".to_string()));
    let value = Value::Map(Map(map));
    
    let from_value: DashTagEnum = from_value(value).unwrap();
    assert_eq!(from_value, DashTagEnum::Unit);
}

#[test]
fn test_at_tag() {
    // Test unit variant
    let unit = AtTagEnum::Unit;
    let serialized = to_string(&unit).unwrap();
    assert!(serialized.contains("@tag"));
    assert!(serialized.contains("Unit"));
    
    let deserialized: AtTagEnum = from_str(&serialized).unwrap();
    assert_eq!(unit, deserialized);
    
    // Test struct variant
    let struct_var = AtTagEnum::Struct { value: 42 };
    let serialized = to_string(&struct_var).unwrap();
    let deserialized: AtTagEnum = from_str(&serialized).unwrap();
    assert_eq!(struct_var, deserialized);
    
    // Test direct value creation
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::String("@tag".to_string()), Value::String("Unit".to_string()));
    let value = Value::Map(Map(map));
    
    let from_value: AtTagEnum = from_value(value).unwrap();
    assert_eq!(from_value, AtTagEnum::Unit);
}

#[test]
#[ignore = "Dollar prefix in tag names is not currently supported by serde-eure deserializer"]
fn test_dollar_tag() {
    // Test unit variant
    let unit = DollarTagEnum::Unit;
    let serialized = to_string(&unit).unwrap();
    assert!(serialized.contains("$tag"));
    assert!(serialized.contains("Unit"));
    
    let deserialized: DollarTagEnum = from_str(&serialized).unwrap();
    assert_eq!(unit, deserialized);
    
    // Test struct variant
    let struct_var = DollarTagEnum::Struct { data: "test".to_string() };
    let serialized = to_string(&struct_var).unwrap();
    let deserialized: DollarTagEnum = from_str(&serialized).unwrap();
    assert_eq!(struct_var, deserialized);
    
    // Test direct value creation - this is where the issue might be
    let mut map = AHashMap::new();
    map.insert(KeyCmpValue::String("$tag".to_string()), Value::String("Unit".to_string()));
    let value = Value::Map(Map(map));
    
    let from_value: DollarTagEnum = from_value(value).unwrap();
    assert_eq!(from_value, DollarTagEnum::Unit);
}

#[test]
#[ignore = "Special characters in field names are not currently supported by serde-eure deserializer"]
fn test_mixed_special_chars_in_field_names() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct SpecialFields {
        #[serde(rename = "_id")]
        id: u32,
        #[serde(rename = "first-name")]
        first_name: String,
        #[serde(rename = "@type")]
        type_field: String,
        #[serde(rename = "$meta")]
        meta: String,
    }
    
    let value = SpecialFields {
        id: 123,
        first_name: "John".to_string(),
        type_field: "user".to_string(),
        meta: "metadata".to_string(),
    };
    
    let serialized = to_string(&value).unwrap();
    assert!(serialized.contains("_id"));
    assert!(serialized.contains("first-name"));
    assert!(serialized.contains("@type"));
    assert!(serialized.contains("$meta"));
    
    let deserialized: SpecialFields = from_str(&serialized).unwrap();
    assert_eq!(value, deserialized);
}

#[test]
fn test_adjacent_tagging() {
    // Test all variants with adjacent tagging
    let unit = AdjacentEnum::Unit;
    let serialized = to_string(&unit).unwrap();
    assert!(serialized.contains("type"));
    assert!(serialized.contains("Unit"));
    
    let deserialized: AdjacentEnum = from_str(&serialized).unwrap();
    assert_eq!(unit, deserialized);
    
    // Newtype variant
    let newtype = AdjacentEnum::Newtype("hello".to_string());
    let serialized = to_string(&newtype).unwrap();
    assert!(serialized.contains("type"));
    assert!(serialized.contains("Newtype"));
    assert!(serialized.contains("content"));
    
    let deserialized: AdjacentEnum = from_str(&serialized).unwrap();
    assert_eq!(newtype, deserialized);
    
    // Struct variant
    let struct_var = AdjacentEnum::Struct { field: 42 };
    let serialized = to_string(&struct_var).unwrap();
    assert!(serialized.contains("type"));
    assert!(serialized.contains("Struct"));
    assert!(serialized.contains("content"));
    
    let deserialized: AdjacentEnum = from_str(&serialized).unwrap();
    assert_eq!(struct_var, deserialized);
}

#[test]
#[ignore = "Dollar prefix in tag names is not currently supported by serde-eure deserializer in arrays"]
fn test_special_chars_in_array() {
    // Test that special tag names work correctly in arrays
    let array = vec![
        DollarTagEnum::Unit,
        DollarTagEnum::Struct { data: "test".to_string() },
    ];
    
    let serialized = to_string(&array).unwrap();
    assert!(serialized.contains("$tag"));
    
    let deserialized: Vec<DollarTagEnum> = from_str(&serialized).unwrap();
    assert_eq!(array, deserialized);
}