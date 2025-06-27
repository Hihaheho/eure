use serde::{Deserialize, Serialize};
use serde_eure::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum KebabEnum {
    TestVariant { some_field: String },
    AnotherVariant { long_field_name: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(non_snake_case)]
enum SnakeEnum {
    TestVariant { someField: String },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CamelStruct {
    first_name: String,
    last_name: String,
    age_in_years: u32,
}

#[test]
fn test_kebab_case_enum() {
    let value = KebabEnum::TestVariant {
        some_field: "hello".to_string(),
    };

    let serialized = to_string(&value).unwrap();
    // Serde correctly applies rename_all
    assert!(serialized.contains("test-variant"));
    assert!(serialized.contains("some_field")); // Field names are already snake_case

    // Test roundtrip
    let deserialized: KebabEnum = from_str(&serialized).unwrap();
    assert_eq!(value, deserialized);
}

#[test]
fn test_kebab_case_enum_in_array() {
    let array = vec![
        KebabEnum::TestVariant {
            some_field: "first".to_string(),
        },
        KebabEnum::AnotherVariant {
            long_field_name: 42,
        },
    ];

    let serialized = to_string(&array).unwrap();
    assert!(serialized.contains("test-variant"));
    assert!(serialized.contains("another-variant"));
    assert!(serialized.contains("some_field")); // Already snake_case
    assert!(serialized.contains("long_field_name")); // Already snake_case

    // Test roundtrip
    let deserialized: Vec<KebabEnum> = from_str(&serialized).unwrap();
    assert_eq!(array, deserialized);
}

#[test]
fn test_snake_case_enum() {
    let value = SnakeEnum::TestVariant {
        someField: "test".to_string(),
    };

    let serialized = to_string(&value).unwrap();
    assert!(serialized.contains("test_variant"));
    assert!(serialized.contains("someField")); // Original field name is someField

    // Test roundtrip
    let deserialized: SnakeEnum = from_str(&serialized).unwrap();
    assert_eq!(value, deserialized);
}

#[test]
fn test_camel_case_struct() {
    let value = CamelStruct {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        age_in_years: 30,
    };

    let serialized = to_string(&value).unwrap();
    assert!(serialized.contains("firstName"));
    assert!(serialized.contains("lastName"));
    assert!(serialized.contains("ageInYears"));

    // Test roundtrip
    let deserialized: CamelStruct = from_str(&serialized).unwrap();
    assert_eq!(value, deserialized);
}

#[test]
fn test_rename_field_attribute() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct RenameFields {
        #[serde(rename = "custom_name")]
        original_name: String,
        #[serde(rename = "@id")]
        id: u32,
        #[serde(rename = "_internal")]
        internal_field: bool,
    }

    let value = RenameFields {
        original_name: "test".to_string(),
        id: 123,
        internal_field: true,
    };

    let serialized = to_string(&value).unwrap();
    assert!(serialized.contains("custom_name"));
    assert!(serialized.contains("@id"));
    assert!(serialized.contains("_internal"));
    assert!(!serialized.contains("original_name"));
    assert!(!serialized.contains("internal_field"));

    // Test roundtrip
    let deserialized: RenameFields = from_str(&serialized).unwrap();
    assert_eq!(value, deserialized);
}

#[test]
fn test_skip_serializing_if() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct SkipFields {
        always: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        optional: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        items: Vec<String>,
    }

    // Test with all fields present
    let value_full = SkipFields {
        always: "present".to_string(),
        optional: Some("included".to_string()),
        items: vec!["item1".to_string()],
    };

    let serialized_full = to_string(&value_full).unwrap();
    assert!(serialized_full.contains("always"));
    assert!(serialized_full.contains("optional"));
    assert!(serialized_full.contains("items"));

    // Test with skipped fields
    let value_skip = SkipFields {
        always: "present".to_string(),
        optional: None,
        items: vec![],
    };

    let serialized_skip = to_string(&value_skip).unwrap();
    assert!(serialized_skip.contains("always"));
    assert!(!serialized_skip.contains("optional"));
    assert!(!serialized_skip.contains("items"));
}

#[test]
fn test_flatten_attribute() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Inner {
        inner_field: String,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Outer {
        outer_field: String,
        #[serde(flatten)]
        inner: Inner,
    }

    let value = Outer {
        outer_field: "outer".to_string(),
        inner: Inner {
            inner_field: "inner".to_string(),
        },
    };

    let serialized = to_string(&value).unwrap();
    assert!(serialized.contains("outer_field"));
    assert!(serialized.contains("inner_field"));

    // The serialized form should have both fields at the same level
    let lines: Vec<&str> = serialized.lines().collect();
    let outer_line = lines.iter().find(|l| l.contains("outer_field")).unwrap();
    let inner_line = lines.iter().find(|l| l.contains("inner_field")).unwrap();

    // Both should be at the same indentation level
    let outer_indent = outer_line.chars().take_while(|c| c.is_whitespace()).count();
    let inner_indent = inner_line.chars().take_while(|c| c.is_whitespace()).count();
    assert_eq!(outer_indent, inner_indent);
}
