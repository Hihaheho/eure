use eure_parol::parse;
use eure_tree::value_visitor::{ValueVisitor, document_to_value};
use eure_value::identifier::Identifier;
use eure_value::value::{KeyCmpValue, Map, Value};
use std::str::FromStr;

#[test]
fn test_simple_bindings() {
    let input = r#"name = "Alice"
age = 30"#;

    // Parse the EURE text
    let tree = parse(input).expect("Failed to parse simple bindings");

    // Create visitor
    let mut visitor = ValueVisitor::new(input);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Get the extracted value
    let doc = visitor.into_document();
    let value = document_to_value(doc);

    // Verify the extracted values
    match value {
        Value::Map(map) => {
            // Check name
            let name_value = map
                .0
                .get(&KeyCmpValue::String("name".to_string()))
                .expect("name field not found");
            assert!(matches!(name_value, Value::String(s) if s == "Alice"));

            // Check age
            let age_value = map
                .0
                .get(&KeyCmpValue::String("age".to_string()))
                .expect("age field not found");
            assert!(matches!(age_value, Value::I64(30)));
        }
        _ => panic!("Expected a map value"),
    }
}

#[test]
fn test_nested_sections() {
    let input = r#"
@ database
host = "localhost"
port = 5432

@ database.credentials
username = "admin"
password = "secret"
"#;

    let tree = parse(input).expect("Failed to parse nested sections");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    let doc = visitor.into_document();
    let value = document_to_value(doc);

    match value {
        Value::Map(root) => {
            // Get database section
            let db_value = root
                .0
                .get(&KeyCmpValue::String("database".to_string()))
                .expect("database section not found");

            match db_value {
                Value::Map(db_map) => {
                    // Check host
                    let host = db_map
                        .0
                        .get(&KeyCmpValue::String("host".to_string()))
                        .expect("host not found");
                    assert!(matches!(host, Value::String(s) if s == "localhost"));

                    // Check port
                    let port = db_map
                        .0
                        .get(&KeyCmpValue::String("port".to_string()))
                        .expect("port not found");
                    assert!(matches!(port, Value::I64(5432)));

                    // Check credentials subsection
                    let creds = db_map
                        .0
                        .get(&KeyCmpValue::String("credentials".to_string()))
                        .expect("credentials not found");

                    match creds {
                        Value::Map(creds_map) => {
                            let username = creds_map
                                .0
                                .get(&KeyCmpValue::String("username".to_string()))
                                .expect("username not found");
                            assert!(matches!(username, Value::String(s) if s == "admin"));

                            let password = creds_map
                                .0
                                .get(&KeyCmpValue::String("password".to_string()))
                                .expect("password not found");
                            assert!(matches!(password, Value::String(s) if s == "secret"));
                        }
                        _ => panic!("Expected credentials to be a map"),
                    }
                }
                _ => panic!("Expected database to be a map"),
            }
        }
        _ => panic!("Expected root to be a map"),
    }
}

#[test]
fn test_arrays() {
    let input = r#"
numbers = [1, 2, 3]
mixed = ["text", 42, true]
"#;

    let tree = parse(input).expect("Failed to parse arrays");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    let doc = visitor.into_document();
    let value = document_to_value(doc);

    match value {
        Value::Map(map) => {
            // Check numbers array
            let numbers = map
                .0
                .get(&KeyCmpValue::String("numbers".to_string()))
                .expect("numbers not found");
            match numbers {
                Value::Array(arr) => {
                    assert_eq!(arr.0.len(), 3);
                    assert!(matches!(&arr.0[0], Value::I64(1)));
                    assert!(matches!(&arr.0[1], Value::I64(2)));
                    assert!(matches!(&arr.0[2], Value::I64(3)));
                }
                _ => panic!("Expected numbers to be an array"),
            }

            // Check mixed array
            let mixed = map
                .0
                .get(&KeyCmpValue::String("mixed".to_string()))
                .expect("mixed not found");
            match mixed {
                Value::Array(arr) => {
                    assert_eq!(arr.0.len(), 3);
                    assert!(matches!(&arr.0[0], Value::String(s) if s == "text"));
                    assert!(matches!(&arr.0[1], Value::I64(42)));
                    assert!(matches!(&arr.0[2], Value::Bool(true)));
                }
                _ => panic!("Expected mixed to be an array"),
            }
        }
        _ => panic!("Expected root to be a map"),
    }
}

#[test]
fn test_extension_fields() {
    // Test that extensions can be added to any node type
    let input = r#"
name = "test"
name.$type = .string
"#;

    let tree = parse(input).expect("Failed to parse extension fields");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    let doc = visitor.into_document();
    let value = document_to_value(doc);

    match value {
        Value::Map(map) => {
            // Extensions are metadata and should NOT appear in the Value
            assert!(map.0.contains_key(&KeyCmpValue::String("name".to_string())));
            // Extensions should not be in the value
            assert!(
                !map.0
                    .contains_key(&KeyCmpValue::String("name.$type".to_string()))
            );

            // The value should just be the string
            let name_value = map.0.get(&KeyCmpValue::String("name".to_string())).unwrap();
            assert!(matches!(name_value, Value::String(s) if s == "test"));
        }
        _ => panic!("Expected root to be a map"),
    }
}

#[test]
fn test_meta_extension_on_map() {
    // Test that meta-extensions work on map nodes
    let input = r#"
config = {}
config.$$meta = "value"
"#;

    let tree = parse(input).expect("Failed to parse meta extension");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    let doc = visitor.into_document();
    let value = document_to_value(doc);

    match value {
        Value::Map(root_map) => {
            let config_value = root_map
                .0
                .get(&KeyCmpValue::String("config".to_string()))
                .unwrap();
            match config_value {
                Value::Map(config_map) => {
                    // Meta-extensions should appear in the Value as KeyCmpValue::MetaExtension
                    assert!(config_map.0.contains_key(&KeyCmpValue::MetaExtension(
                        Identifier::from_str("meta").unwrap()
                    )));
                }
                _ => panic!("Expected config to be a map"),
            }
        }
        _ => panic!("Expected root to be a map"),
    }
}

// Note: The previous tests were extensively testing internal implementation details
// of the old Values struct that no longer exists. The new implementation uses
// EureDocument internally and only exposes the final Value through document_to_value.
// These tests demonstrate the new API usage for common scenarios.
