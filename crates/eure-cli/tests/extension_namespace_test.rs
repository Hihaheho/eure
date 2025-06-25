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
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    // Extract and verify result
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings_view = eure_view.eure_bindings.get_view(&tree).expect("Failed to get bindings view")
        .expect("Expected bindings to exist");
    let binding = bindings_view.binding.get_view(&tree).expect("Failed to get binding view");
    
    // Verify the key is preserved with extension namespace
    if let Some(key_handles) = values.get_keys(&binding.keys) {
        assert!(!key_handles.is_empty(), "Expected at least one key");
        
        for key_handle in key_handles {
            if let Some(path_seg) = values.get_path_segment(key_handle) {
                match path_seg {
                    eure_value::value::PathSegment::Extension(ident) => {
                        assert_eq!(ident.to_string(), "tag", "Expected extension identifier to be 'tag'");
                    }
                    _ => panic!("Expected extension namespace, got regular identifier"),
                }
            }
        }
    } else {
        panic!("No keys found in binding");
    }
    
    // Verify the value
    let binding_rhs_view = binding.binding_rhs.get_view(&tree).expect("Failed to get binding RHS");
    if let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view {
        let value_binding_view = value_binding_handle.get_view(&tree).expect("Failed to get value binding view");
        let value = values.get_value(&value_binding_view.value).expect("Failed to get value");
        
        match value {
            Value::String(s) => assert_eq!(s, "test-variant", "Expected value to be 'test-variant'"),
            _ => panic!("Expected string value, got {:?}", value),
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
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    // Extract result
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings_view = eure_view.eure_bindings.get_view(&tree).expect("Failed to get bindings view")
        .expect("Expected bindings to exist");
    let binding = bindings_view.binding.get_view(&tree).expect("Failed to get binding view");
    
    // Verify the top-level key is "data"
    if let Some(key_handles) = values.get_keys(&binding.keys) {
        assert_eq!(key_handles.len(), 1, "Expected exactly one key");
        
        if let Some(path_seg) = values.get_path_segment(&key_handles[0]) {
            match path_seg {
                eure_value::value::PathSegment::Ident(ident) => {
                    assert_eq!(ident.to_string(), "data", "Expected top-level key to be 'data'");
                }
                _ => panic!("Expected regular identifier for top-level key"),
            }
        }
    }
    
    // Get the object value
    let binding_rhs_view = binding.binding_rhs.get_view(&tree).expect("Failed to get binding RHS");
    if let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view {
        let value_binding_view = value_binding_handle.get_view(&tree).expect("Failed to get value binding view");
        let value = values.get_value(&value_binding_view.value).expect("Failed to get value");
        
        if let Value::Map(Map(map)) = value {
            // Verify the map contains both keys
            assert_eq!(map.len(), 2, "Expected map to contain 2 entries");
            
            // Check for $tag key with preserved prefix
            let has_extension_tag = map.iter().any(|(key, val)| {
                matches!(key, KeyCmpValue::String(s) if s == "$tag") &&
                matches!(val, Value::String(s) if s == "test-variant")
            });
            assert!(has_extension_tag, "Expected to find $tag key with preserved $ prefix");
            
            // Ensure $tag prefix wasn't lost (would appear as just "tag")
            let has_plain_tag = map.iter().any(|(key, _)| {
                matches!(key, KeyCmpValue::String(s) if s == "tag")
            });
            assert!(!has_plain_tag, "Found 'tag' without $ prefix - extension namespace was lost!");
            
            // Check for field key
            let has_field = map.iter().any(|(key, val)| {
                matches!(key, KeyCmpValue::String(s) if s == "field") &&
                matches!(val, Value::String(s) if s == "value")
            });
            assert!(has_field, "Expected to find 'field' key with value 'value'");
        } else {
            panic!("Expected Map value, got {:?}", value);
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
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    // Extract the object value
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings_view = eure_view.eure_bindings.get_view(&tree).expect("Failed to get bindings view")
        .expect("Expected bindings to exist");
    let binding = bindings_view.binding.get_view(&tree).expect("Failed to get binding view");
    
    let binding_rhs_view = binding.binding_rhs.get_view(&tree).expect("Failed to get binding RHS");
    if let BindingRhsView::ValueBinding(value_binding_handle) = binding_rhs_view {
        let value_binding_view = value_binding_handle.get_view(&tree).expect("Failed to get value binding view");
        let value = values.get_value(&value_binding_view.value).expect("Failed to get value");
        
        if let Value::Map(Map(map)) = value {
            assert_eq!(map.len(), 3, "Expected map to contain 3 entries");
            
            // Verify all extension namespace fields are preserved
            let extension_fields = map.iter().filter(|(key, _)| {
                matches!(key, KeyCmpValue::String(s) if s.starts_with('$'))
            }).count();
            assert_eq!(extension_fields, 2, "Expected 2 extension namespace fields");
            
            // Verify specific extension fields
            assert!(map.iter().any(|(k, v)| {
                matches!(k, KeyCmpValue::String(s) if s == "$tag") &&
                matches!(v, Value::String(s) if s == "variant")
            }), "Expected $tag field");
            
            assert!(map.iter().any(|(k, v)| {
                matches!(k, KeyCmpValue::String(s) if s == "$meta") &&
                matches!(v, Value::String(s) if s == "metadata")
            }), "Expected $meta field");
            
            // Verify regular field
            assert!(map.iter().any(|(k, v)| {
                matches!(k, KeyCmpValue::String(s) if s == "regular") &&
                matches!(v, Value::String(s) if s == "field")
            }), "Expected regular field");
        } else {
            panic!("Expected Map value");
        }
    }
}