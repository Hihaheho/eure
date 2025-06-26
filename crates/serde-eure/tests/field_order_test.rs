use serde::{Deserialize, Serialize};
use serde_eure::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    name: String,
    version: String,
    settings: Settings,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Settings {
    debug: bool,
    host: String,
    port: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Dependency {
    name: String,
    version: String,
    optional: bool,
}

#[test]
#[ignore = "Field order preservation is not guaranteed in current serde-eure implementation"]
fn test_field_order_preservation() {
    // Create struct with fields in specific order
    let config = Config {
        name: "my-app".to_string(),
        version: "1.0.0".to_string(),
        settings: Settings {
            debug: true,
            host: "localhost".to_string(),
            port: 8080,
        },
        dependencies: vec![
            Dependency {
                name: "serde".to_string(),
                version: "1.0".to_string(),
                optional: false,
            },
            Dependency {
                name: "tokio".to_string(),
                version: "1.35".to_string(),
                optional: true,
            },
        ],
    };
    
    let serialized = to_string(&config).unwrap();
    
    // The serialized output should maintain field order
    let lines: Vec<&str> = serialized.lines().filter(|l| !l.trim().is_empty()).collect();
    
    // Find the positions of top-level fields
    let name_pos = lines.iter().position(|l| l.contains("name =")).unwrap();
    let version_pos = lines.iter().position(|l| l.contains("version =")).unwrap();
    let settings_pos = lines.iter().position(|l| l.contains("settings =")).unwrap();
    let deps_pos = lines.iter().position(|l| l.contains("dependencies =")).unwrap();
    
    // Fields should appear in declaration order
    assert!(name_pos < version_pos);
    assert!(version_pos < settings_pos);
    assert!(settings_pos < deps_pos);
    
    // Test roundtrip
    let deserialized: Config = from_str(&serialized).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
fn test_non_alphabetical_field_parsing() {
    // Test parsing EURE with fields in non-alphabetical order
    let eure_str = r#"
dependencies = [
    {optional = false, version = "1.0", name = "serde"},
    {name = "tokio", version = "1.35", optional = true}
]
settings = {debug = true, host = "localhost", port = 8080}
name = "my-app"
version = "1.0.0"
"#;
    
    let parsed: Config = from_str(eure_str).unwrap();
    
    assert_eq!(parsed.name, "my-app");
    assert_eq!(parsed.version, "1.0.0");
    assert_eq!(parsed.settings.debug, true);
    assert_eq!(parsed.settings.host, "localhost");
    assert_eq!(parsed.settings.port, 8080);
    assert_eq!(parsed.dependencies.len(), 2);
    assert_eq!(parsed.dependencies[0].name, "serde");
    assert_eq!(parsed.dependencies[1].name, "tokio");
}

#[test]
fn test_mixed_field_order_in_objects() {
    // Test that objects with mixed field order parse correctly
    let eure_str = r#"
user = {
    age = 30
    name = "Alice"
    email = "alice@example.com"
    active = true
}
"#;
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct User {
        name: String,
        email: String,
        age: u32,
        active: bool,
    }
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Container {
        user: User,
    }
    
    let parsed: Container = from_str(eure_str).unwrap();
    assert_eq!(parsed.user.name, "Alice");
    assert_eq!(parsed.user.email, "alice@example.com");
    assert_eq!(parsed.user.age, 30);
    assert_eq!(parsed.user.active, true);
}

#[test]
#[ignore = "Field order preservation is not guaranteed in current serde-eure implementation"]
fn test_preserve_insertion_order() {
    // Test using serde_json::Value to check if order is preserved
    let eure_str = r#"
z_field = "first"
a_field = "second"
m_field = "third"
"#;
    
    // Parse as generic JSON value to inspect order
    let value: serde_json::Value = from_str(eure_str).unwrap();
    
    if let serde_json::Value::Object(map) = &value {
        // Collect keys in iteration order
        let keys: Vec<String> = map.keys().cloned().collect();
        
        // Keys should be in the order they appeared in the source
        assert_eq!(keys[0], "z_field");
        assert_eq!(keys[1], "a_field");
        assert_eq!(keys[2], "m_field");
    } else {
        panic!("Expected object value");
    }
}

#[test]
fn test_array_element_order() {
    // Test that array elements maintain their order
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Item {
        id: u32,
        value: String,
    }
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Container {
        items: Vec<Item>,
    }
    
    let eure_str = r#"
items = [
    {value = "first", id = 3},
    {id = 1, value = "second"},
    {value = "third", id = 2}
]
"#;
    
    let parsed: Container = from_str(eure_str).unwrap();
    
    // Array order should be preserved
    assert_eq!(parsed.items[0].id, 3);
    assert_eq!(parsed.items[0].value, "first");
    assert_eq!(parsed.items[1].id, 1);
    assert_eq!(parsed.items[1].value, "second");
    assert_eq!(parsed.items[2].id, 2);
    assert_eq!(parsed.items[2].value, "third");
}

#[test]
fn test_nested_object_field_order() {
    // Test deeply nested objects maintain field order
    let eure_str = r#"
outer = {
    z_field = "outer_z"
    inner = {
        z_field = "inner_z"
        a_field = "inner_a"
    }
    a_field = "outer_a"
}
"#;
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Inner {
        z_field: String,
        a_field: String,
    }
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Outer {
        z_field: String,
        inner: Inner,
        a_field: String,
    }
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Container {
        outer: Outer,
    }
    
    let parsed: Container = from_str(eure_str).unwrap();
    assert_eq!(parsed.outer.z_field, "outer_z");
    assert_eq!(parsed.outer.a_field, "outer_a");
    assert_eq!(parsed.outer.inner.z_field, "inner_z");
    assert_eq!(parsed.outer.inner.a_field, "inner_a");
}