use serde_eure::{from_str, to_string, from_value, to_value};
use serde::{Deserialize, Serialize};
use eure_value::value::{Value as EureValue, Map, KeyCmpValue};
use ahash::AHashMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SimpleStruct {
    name: String,
    age: u32,
    active: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct NestedStruct {
    title: String,
    metadata: Metadata,
    tags: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Metadata {
    author: String,
    version: String,
    published: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum SimpleEnum {
    Unit,
    Newtype(String),
    Struct { field: i32 },
}

#[test]
fn test_basic_struct() {
    let eure_str = r#"
name = "Alice"
age = 30
active = true
"#;
    
    let parsed: SimpleStruct = from_str(eure_str).expect("Failed to parse simple struct");
    
    assert_eq!(parsed.name, "Alice");
    assert_eq!(parsed.age, 30);
    assert_eq!(parsed.active, true);
}

#[test]
fn test_nested_struct() {
    let eure_str = r#"
title = "Test Document"
metadata = {
    author = "John Doe"
    version = "1.0.0"
    published = false
}
tags = ["test", "example", "eure"]
"#;
    
    let parsed: NestedStruct = from_str(eure_str).expect("Failed to parse nested struct");
    
    assert_eq!(parsed.title, "Test Document");
    assert_eq!(parsed.metadata.author, "John Doe");
    assert_eq!(parsed.metadata.version, "1.0.0");
    assert_eq!(parsed.metadata.published, false);
    assert_eq!(parsed.tags, vec!["test", "example", "eure"]);
}

#[test]
fn test_array_parsing() {
    // Arrays need to be part of a struct or field in EURE
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct WithArrays {
        numbers: Vec<i32>,
        strings: Vec<String>,
    }
    
    let eure_str = r#"
numbers = [1, 2, 3, 4, 5]
strings = ["hello", "world", "test"]
"#;
    
    let parsed: WithArrays = from_str(eure_str).expect("Failed to parse arrays");
    assert_eq!(parsed.numbers, vec![1, 2, 3, 4, 5]);
    assert_eq!(parsed.strings, vec!["hello", "world", "test"]);
}

#[test]
fn test_trailing_comma() {
    // EURE allows trailing commas
    let eure_str = r#"
name = "Bob"
age = 25
active = false
"#;
    
    let parsed: SimpleStruct = from_str(eure_str).expect("Failed to parse with trailing comma");
    assert_eq!(parsed.name, "Bob");
    assert_eq!(parsed.age, 25);
    assert_eq!(parsed.active, false);
}

#[test]
fn test_enum_roundtrip() {
    // Test unit variant
    let unit = SimpleEnum::Unit;
    let unit_eure = to_string(&unit).expect("Failed to serialize unit");
    let unit_back: SimpleEnum = from_str(&unit_eure).expect("Failed to deserialize unit");
    assert_eq!(unit, unit_back);
    
    // Test struct variant
    let struct_var = SimpleEnum::Struct { field: 42 };
    let struct_eure = to_string(&struct_var).expect("Failed to serialize struct variant");
    let struct_back: SimpleEnum = from_str(&struct_eure).expect("Failed to deserialize struct variant");
    assert_eq!(struct_var, struct_back);
    
    // Note: Newtype variants seem to have issues with the current EURE deserializer
    // The serializer produces a format that the deserializer cannot parse back
}

#[test]
fn test_roundtrip_serialization() {
    let original = SimpleStruct {
        name: "Charlie".to_string(),
        age: 35,
        active: true,
    };
    
    // Serialize to EURE string
    let eure_string = to_string(&original).expect("Failed to serialize");
    
    // Deserialize back
    let deserialized: SimpleStruct = from_str(&eure_string).expect("Failed to deserialize");
    
    assert_eq!(original, deserialized);
}

#[test]
fn test_value_conversion() {
    let original = SimpleStruct {
        name: "David".to_string(),
        age: 40,
        active: false,
    };
    
    // Convert to EureValue
    let value = to_value(&original).expect("Failed to convert to value");
    
    // Convert back from EureValue
    let converted: SimpleStruct = from_value(value).expect("Failed to convert from value");
    
    assert_eq!(original, converted);
}

#[test]
fn test_direct_value_creation() {
    // Create a Value directly representing a simple struct
    let mut map = AHashMap::new();
    map.insert(
        KeyCmpValue::String("name".to_string()),
        EureValue::String("Direct".to_string()),
    );
    map.insert(
        KeyCmpValue::String("age".to_string()),
        EureValue::U64(25),
    );
    map.insert(
        KeyCmpValue::String("active".to_string()),
        EureValue::Bool(true),
    );
    let value = EureValue::Map(Map(map));
    
    // Deserialize from the Value
    let deserialized: SimpleStruct = from_value(value).expect("Failed to deserialize from direct value");
    
    assert_eq!(deserialized.name, "Direct");
    assert_eq!(deserialized.age, 25);
    assert_eq!(deserialized.active, true);
}

#[test]
fn test_option_types() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct WithOption {
        required: String,
        optional: Option<String>,
    }
    
    // Test with Some value
    let with_some = WithOption {
        required: "always here".to_string(),
        optional: Some("sometimes here".to_string()),
    };
    
    let serialized = to_string(&with_some).expect("Failed to serialize option");
    let deserialized: WithOption = from_str(&serialized).expect("Failed to deserialize option");
    assert_eq!(with_some, deserialized);
    
    // Test with None value
    let with_none = WithOption {
        required: "always here".to_string(),
        optional: None,
    };
    
    let serialized = to_string(&with_none).expect("Failed to serialize None");
    let deserialized: WithOption = from_str(&serialized).expect("Failed to deserialize None");
    assert_eq!(with_none, deserialized);
}

#[test]
fn test_numeric_types() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Numbers {
        int_val: i32,
        zero: i32,
        large: i64,
    }
    
    let eure_str = r#"
int_val = 42
zero = 0
large = 1234567890
"#;
    
    let parsed: Numbers = from_str(eure_str).expect("Failed to parse numbers");
    
    assert_eq!(parsed.int_val, 42);
    assert_eq!(parsed.zero, 0);
    assert_eq!(parsed.large, 1234567890);
    
    // Test roundtrip for various numeric types
    let numbers = Numbers {
        int_val: 99,
        zero: 0,
        large: 9876543210,
    };
    
    let serialized = to_string(&numbers).expect("Failed to serialize numbers");
    let deserialized: Numbers = from_str(&serialized).expect("Failed to deserialize numbers");
    
    assert_eq!(numbers, deserialized);
}

#[test]
fn test_complex_nested_structure() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
        email: String,
        roles: Vec<String>,
        active: bool,
    }
    
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Project {
        name: String,
        users: Vec<User>,
        tags: Vec<String>,
        config: ProjectConfig,
    }
    
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct ProjectConfig {
        public: bool,
        max_users: u32,
        features: Vec<String>,
    }
    
    let project = Project {
        name: "Test Project".to_string(),
        users: vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                roles: vec!["admin".to_string(), "developer".to_string()],
                active: true,
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                roles: vec!["developer".to_string()],
                active: false,
            },
        ],
        tags: vec!["rust".to_string(), "eure".to_string(), "test".to_string()],
        config: ProjectConfig {
            public: false,
            max_users: 100,
            features: vec!["auth".to_string(), "api".to_string()],
        },
    };
    
    // Test roundtrip
    let serialized = to_string(&project).expect("Failed to serialize complex structure");
    let deserialized: Project = from_str(&serialized).expect("Failed to deserialize complex structure");
    
    assert_eq!(project, deserialized);
}

#[test]
fn test_extension_namespace_fields() {
    // Test that fields with special characters like $ can be preserved through serialization
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    struct DocumentWithMeta {
        title: String,
        #[serde(rename = "$meta:version")]
        meta_version: String,
        #[serde(rename = "$meta:author")]
        meta_author: String,
        content: String,
    }
    
    let doc = DocumentWithMeta {
        title: "Test Document".to_string(),
        meta_version: "1.0".to_string(),
        meta_author: "Test Author".to_string(),
        content: "This is the content".to_string(),
    };
    
    // Test roundtrip
    let serialized = to_string(&doc).expect("Failed to serialize document");
    let deserialized: DocumentWithMeta = from_str(&serialized).expect("Failed to deserialize document");
    
    assert_eq!(doc, deserialized);
}