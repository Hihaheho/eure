use eure_parol::parse;
use eure_tree::value_visitor::{ValueVisitor, document_to_value};
use eure_value::value::{ObjectKey, Map, Value};

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
            // Extensions are metadata and should NOT appear in the Value
            // A top-level extension creates an empty root map
            assert!(
                map.0.is_empty(),
                "Expected empty map since $tag is an extension"
            );
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
            let data_value = root_map
                .0
                .get(&ObjectKey::String("data".to_string()))
                .expect("Expected data field");

            if let Value::Map(Map(data_map)) = data_value {
                // Extensions are metadata and should NOT appear in the Value
                assert_eq!(
                    data_map.len(),
                    1,
                    "Expected only 1 field (extensions excluded)"
                );

                // Check for regular field
                let field_value = data_map
                    .get(&ObjectKey::String("field".to_string()))
                    .expect("Expected field");
                assert!(matches!(field_value, Value::String(s) if s == "value"));

                // $tag should NOT be in the value
                assert!(
                    !data_map.contains_key(&ObjectKey::String("$tag".to_string())),
                    "$tag extension should not appear in Value"
                );
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
            let config_value = root_map
                .0
                .get(&ObjectKey::String("config".to_string()))
                .expect("Expected config field");

            if let Value::Map(Map(config_map)) = config_value {
                // Extensions are metadata and should NOT appear in the Value
                assert_eq!(
                    config_map.len(),
                    1,
                    "Expected only 1 field (extensions excluded)"
                );

                // Verify regular field
                let regular_value = config_map
                    .get(&ObjectKey::String("regular".to_string()))
                    .expect("Expected regular field");
                assert!(matches!(regular_value, Value::String(s) if s == "field"));

                // Extension fields should NOT be in the value
                assert!(
                    !config_map.contains_key(&ObjectKey::String("$tag".to_string())),
                    "$tag extension should not appear in Value"
                );
                assert!(
                    !config_map.contains_key(&ObjectKey::String("$meta".to_string())),
                    "$meta extension should not appear in Value"
                );
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
            // Meta extensions should appear in the Value as KeyCmpValue::MetaExtension
            use eure_value::identifier::Identifier;
            use std::str::FromStr;

            let meta_value = map
                .0
                .get(&ObjectKey::MetaExtension(
                    Identifier::from_str("meta").unwrap(),
                ))
                .expect("Expected $$meta field as MetaExtension key");

            assert!(matches!(meta_value, Value::String(s) if s == "meta-value"));
        }
        _ => panic!("Expected map value"),
    }
}

// Note: The previous tests were checking that extension fields were stored
// separately as metadata. In the new implementation, extension fields are
// included directly in the map with their $ or $$ prefixes preserved.
