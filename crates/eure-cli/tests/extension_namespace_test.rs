use eure_parol::parse;
use eure_tree::prelude::*;
use eure_tree::value_visitor::{ValueVisitor, Values};
use eure_value::value::{KeyCmpValue, Map, Value};

#[test]
fn test_top_level_extension_namespace() {
    let input = r#"$tag = "test-variant""#;

    // Parse
    let tree = parse(input).expect("Failed to parse input");

    // Create visitor and values storage
    let mut values = Values::default();
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Extract and verify result
    let root_view = tree
        .root_handle()
        .get_view(&tree)
        .expect("Failed to get root view");
    let eure_view = root_view
        .eure
        .get_view(&tree)
        .expect("Failed to get eure view");
    let bindings_view = eure_view
        .eure_bindings
        .get_view(&tree)
        .expect("Failed to get bindings view")
        .expect("Expected bindings to exist");
    let binding = bindings_view
        .binding
        .get_view(&tree)
        .expect("Failed to get binding view");

    // Verify the key is preserved with extension namespace
    if let Some(key_handles) = values.get_keys(&binding.keys) {
        assert!(!key_handles.is_empty(), "Expected at least one key");

        for key_handle in key_handles {
            if let Some(path_seg) = values.get_path_segment(key_handle) {
                match path_seg {
                    eure_value::value::PathSegment::Extension(ident) => {
                        assert_eq!(
                            ident.to_string(),
                            "tag",
                            "Expected extension identifier to be 'tag'"
                        );
                    }
                    _ => panic!("Expected extension namespace, got regular identifier"),
                }
            }
        }
    } else {
        panic!("No keys found in binding");
    }

    // Verify the value
    let binding_rhs_view = binding
        .binding_rhs
        .get_view(&tree)
        .expect("Failed to get binding RHS");
    if let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view {
        let value_binding_view = value_binding_handle
            .get_view(&tree)
            .expect("Failed to get value binding view");
        let value = values
            .get_value(&value_binding_view.value)
            .expect("Failed to get value");

        match value {
            Value::String(s) => {
                assert_eq!(s, "test-variant", "Expected value to be 'test-variant'")
            }
            _ => panic!("Expected string value, got {value:?}"),
        }
    } else {
        panic!("Expected value binding");
    }
}

#[test]
fn test_extension_namespace_in_object() {
    let input = r#"data = {$tag = "test-variant", field = "value"}"#;

    // Parse
    let tree = parse(input).expect("Failed to parse input");

    // Create visitor and values storage
    let mut values = Values::default();
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Extract result
    let root_view = tree
        .root_handle()
        .get_view(&tree)
        .expect("Failed to get root view");
    let eure_view = root_view
        .eure
        .get_view(&tree)
        .expect("Failed to get eure view");
    let bindings_view = eure_view
        .eure_bindings
        .get_view(&tree)
        .expect("Failed to get bindings view")
        .expect("Expected bindings to exist");
    let binding = bindings_view
        .binding
        .get_view(&tree)
        .expect("Failed to get binding view");

    // Verify the top-level key is "data"
    if let Some(key_handles) = values.get_keys(&binding.keys) {
        assert_eq!(key_handles.len(), 1, "Expected exactly one key");

        if let Some(path_seg) = values.get_path_segment(&key_handles[0]) {
            match path_seg {
                eure_value::value::PathSegment::Ident(ident) => {
                    assert_eq!(
                        ident.to_string(),
                        "data",
                        "Expected top-level key to be 'data'"
                    );
                }
                _ => panic!("Expected regular identifier for top-level key"),
            }
        }
    }

    // Get the object value
    let binding_rhs_view = binding
        .binding_rhs
        .get_view(&tree)
        .expect("Failed to get binding RHS");
    if let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view {
        let value_binding_view = value_binding_handle
            .get_view(&tree)
            .expect("Failed to get value binding view");
        let value = values
            .get_value(&value_binding_view.value)
            .expect("Failed to get value");

        if let Value::Map(Map(map)) = value {
            // Extension namespace fields should be metadata, not data
            // So the map should only contain the regular field
            assert_eq!(map.len(), 1, "Expected map to contain 1 entry (extension fields are metadata)");

            // Check for field key
            let has_field = map.iter().any(|(key, val)| {
                matches!(key, KeyCmpValue::String(s) if s == "field")
                    && matches!(val, Value::String(s) if s == "value")
            });
            assert!(has_field, "Expected to find 'field' key with value 'value'");

            // Ensure no extension fields are in the data map
            let has_extension_fields = map
                .iter()
                .any(|(key, _)| matches!(key, KeyCmpValue::String(s) if s.starts_with('$')));
            assert!(
                !has_extension_fields,
                "Found extension fields in data map - they should be metadata!"
            );
        } else {
            panic!("Expected Map value, got {value:?}");
        }
    } else {
        panic!("Expected value binding");
    }
}

#[test]
fn test_multiple_extension_fields() {
    let input = r#"config = {$tag = "variant", $meta = "metadata", regular = "field"}"#;

    // Parse
    let tree = parse(input).expect("Failed to parse input");

    // Create visitor and values storage
    let mut values = Values::default();
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Extract the object value
    let root_view = tree
        .root_handle()
        .get_view(&tree)
        .expect("Failed to get root view");
    let eure_view = root_view
        .eure
        .get_view(&tree)
        .expect("Failed to get eure view");
    let bindings_view = eure_view
        .eure_bindings
        .get_view(&tree)
        .expect("Failed to get bindings view")
        .expect("Expected bindings to exist");
    let binding = bindings_view
        .binding
        .get_view(&tree)
        .expect("Failed to get binding view");

    let binding_rhs_view = binding
        .binding_rhs
        .get_view(&tree)
        .expect("Failed to get binding RHS");
    if let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view {
        let value_binding_view = value_binding_handle
            .get_view(&tree)
            .expect("Failed to get value binding view");
        let value = values
            .get_value(&value_binding_view.value)
            .expect("Failed to get value");

        if let Value::Map(Map(map)) = value {
            // Extension namespace fields ($tag, $meta) should be metadata, not data
            // So the map should only contain the regular field
            assert_eq!(map.len(), 1, "Expected map to contain 1 entry (extension fields are metadata)");

            // Verify no extension namespace fields are in the data
            let extension_fields = map
                .iter()
                .filter(|(key, _)| matches!(key, KeyCmpValue::String(s) if s.starts_with('$')))
                .count();
            assert_eq!(extension_fields, 0, "Extension namespace fields should not be in data map");

            // Verify regular field
            assert!(
                map.iter().any(|(k, v)| {
                    matches!(k, KeyCmpValue::String(s) if s == "regular")
                        && matches!(v, Value::String(s) if s == "field")
                }),
                "Expected regular field"
            );
        } else {
            panic!("Expected Map value");
        }
    }
}
