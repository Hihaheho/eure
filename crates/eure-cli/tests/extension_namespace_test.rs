use eure_parol::parse;
use eure_tree::value_visitor::{ValueVisitor, document_to_value};
use eure_value::value::{KeyCmpValue, Map, Value};

#[test]
fn test_top_level_extension_namespace() {
    let input = r#"$tag = "test-variant""#;

    // Parse
    let tree = parse(input).expect("Failed to parse input");

    // Create visitor
    let mut visitor = ValueVisitor::new(input);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Extract and verify result
    let doc = visitor.into_document();
    let value = document_to_value(doc);

    // Verify the result
    match value {
        Value::Map(map) => {
            // Extension namespace field should be present as a key
            let tag_value = map.0.get(&KeyCmpValue::String("$tag".to_string()))
                .expect("Expected $tag field");
            
            match tag_value {
                Value::String(s) => {
                    assert_eq!(s, "test-variant", "Expected value to be 'test-variant'")
                }
                _ => panic!("Expected string value, got {tag_value:?}"),
            }
        }
        _ => panic!("Expected map value"),
    }
}

#[test]
fn test_extension_namespace_in_object() {
    let input = r#"data = {$tag = "test-variant", field = "value"}"#;

    // Parse
    let tree = parse(input).expect("Failed to parse input");

    // Create visitor
    let mut visitor = ValueVisitor::new(input);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Extract result
    let doc = visitor.into_document();
    let value = document_to_value(doc);

    match value {
        Value::Map(root_map) => {
            // Get the data object
            let data_value = root_map.0.get(&KeyCmpValue::String("data".to_string()))
                .expect("Expected data field");
            
            if let Value::Map(Map(data_map)) = data_value {
                // In the new implementation, extension fields are included in the map
                assert!(data_map.len() >= 2, "Expected at least 2 fields in the map");

                // Check for $tag field
                let tag_value = data_map.get(&KeyCmpValue::String("$tag".to_string()))
                    .expect("Expected $tag field");
                assert!(matches!(tag_value, Value::String(s) if s == "test-variant"));

                // Check for regular field
                let field_value = data_map.get(&KeyCmpValue::String("field".to_string()))
                    .expect("Expected field");
                assert!(matches!(field_value, Value::String(s) if s == "value"));
            } else {
                panic!("Expected Map value for data, got {data_value:?}");
            }
        }
        _ => panic!("Expected root to be a map"),
    }
}

#[test]
fn test_multiple_extension_fields() {
    let input = r#"config = {$tag = "variant", $meta = "metadata", regular = "field"}"#;

    // Parse
    let tree = parse(input).expect("Failed to parse input");

    // Create visitor
    let mut visitor = ValueVisitor::new(input);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Extract the value
    let doc = visitor.into_document();
    let value = document_to_value(doc);

    match value {
        Value::Map(root_map) => {
            let config_value = root_map.0.get(&KeyCmpValue::String("config".to_string()))
                .expect("Expected config field");
            
            if let Value::Map(Map(config_map)) = config_value {
                // In the new implementation, all fields (including extension fields) are in the map
                assert!(config_map.len() >= 3, "Expected at least 3 fields in the map");

                // Verify extension fields
                let tag_value = config_map.get(&KeyCmpValue::String("$tag".to_string()))
                    .expect("Expected $tag field");
                assert!(matches!(tag_value, Value::String(s) if s == "variant"));

                let meta_value = config_map.get(&KeyCmpValue::String("$meta".to_string()))
                    .expect("Expected $meta field");
                assert!(matches!(meta_value, Value::String(s) if s == "metadata"));

                // Verify regular field
                let regular_value = config_map.get(&KeyCmpValue::String("regular".to_string()))
                    .expect("Expected regular field");
                assert!(matches!(regular_value, Value::String(s) if s == "field"));
            } else {
                panic!("Expected Map value for config");
            }
        }
        _ => panic!("Expected root to be a map"),
    }
}

#[test]
fn test_meta_extension_namespace() {
    let input = r#"$$meta = "meta-value""#;

    // Parse
    let tree = parse(input).expect("Failed to parse input");

    // Create visitor
    let mut visitor = ValueVisitor::new(input);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Extract and verify result
    let doc = visitor.into_document();
    let value = document_to_value(doc);

    match value {
        Value::Map(map) => {
            // Meta extension namespace field should be present
            let meta_value = map.0.get(&KeyCmpValue::String("$$meta".to_string()))
                .expect("Expected $$meta field");
            
            assert!(matches!(meta_value, Value::String(s) if s == "meta-value"));
        }
        _ => panic!("Expected map value"),
    }
}

// Note: The previous tests were checking that extension fields were stored
// separately as metadata. In the new implementation, extension fields are
// included directly in the map with their $ or $$ prefixes preserved.