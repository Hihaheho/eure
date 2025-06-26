use eure_parol::parse;
use eure_tree::prelude::*;
use eure_tree::value_visitor::{ValueVisitor, Values};
use eure_value::value::PathSegment;

#[test]
fn test_simple_bindings() {
    let input = r#"name = "Alice"
age = 30"#;

    // Parse the EURE text
    let tree = parse(input).expect("Failed to parse simple bindings");

    // Create storage for values
    let mut values = Values::default();

    // Create visitor
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Verify the extracted values
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings_view = eure_view.eure_bindings.get_view(&tree)
        .expect("Failed to get bindings")
        .expect("No bindings found");

    // Check first binding (name = "Alice")
    let first_binding = bindings_view.binding.get_view(&tree)
        .expect("Failed to get first binding");
    
    // Verify key
    let key_handles = values.get_keys(&first_binding.keys)
        .expect("Failed to get keys for first binding");
    assert_eq!(key_handles.len(), 1, "Expected exactly one key for 'name' binding");
    
    let path_seg = values.get_path_segment(&key_handles[0])
        .expect("Failed to get path segment");
    match path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "name"),
        _ => panic!("Expected Ident path segment for 'name'"),
    }

    // Verify value
    let binding_rhs = first_binding.binding_rhs.get_view(&tree)
        .expect("Failed to get binding rhs");
    match binding_rhs {
        BindingRhsView::ValueBinding(value_binding_handle) => {
            let value_binding = value_binding_handle.get_view(&tree)
                .expect("Failed to get value binding");
            let value = values.get_value(&value_binding.value)
                .expect("Failed to get value");
            match value {
                eure_value::value::Value::String(s) => assert_eq!(s, "Alice"),
                _ => panic!("Expected String value for 'name', got {:?}", value),
            }
        }
        _ => panic!("Expected ValueBinding for 'name'"),
    }

    // Check second binding (age = 30)
    let second_bindings = bindings_view.eure_bindings.get_view(&tree)
        .expect("Failed to get remaining bindings")
        .expect("No second binding found");
    
    let second_binding = second_bindings.binding.get_view(&tree)
        .expect("Failed to get second binding");
    
    // Verify key
    let key_handles = values.get_keys(&second_binding.keys)
        .expect("Failed to get keys for second binding");
    assert_eq!(key_handles.len(), 1, "Expected exactly one key for 'age' binding");
    
    let path_seg = values.get_path_segment(&key_handles[0])
        .expect("Failed to get path segment");
    match path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "age"),
        _ => panic!("Expected Ident path segment for 'age'"),
    }

    // Verify value
    let binding_rhs = second_binding.binding_rhs.get_view(&tree)
        .expect("Failed to get binding rhs");
    match binding_rhs {
        BindingRhsView::ValueBinding(value_binding_handle) => {
            let value_binding = value_binding_handle.get_view(&tree)
                .expect("Failed to get value binding");
            let value = values.get_value(&value_binding.value)
                .expect("Failed to get value");
            match value {
                eure_value::value::Value::I64(n) => assert_eq!(*n, 30),
                _ => panic!("Expected I64 value for 'age', got {:?}", value),
            }
        }
        _ => panic!("Expected ValueBinding for 'age'"),
    }
}

#[test]
fn test_object_value() {
    let input = r#"user = {
    "id" = 123,
    "active" = true
}
items = ["apple", "banana"]"#;

    // Parse the EURE text
    let tree = parse(input).expect("Failed to parse object value");

    // Create storage for values
    let mut values = Values::default();

    // Create visitor
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Verify the extracted values
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings_view = eure_view.eure_bindings.get_view(&tree)
        .expect("Failed to get bindings")
        .expect("No bindings found");

    // Check first binding (user object)
    let first_binding = bindings_view.binding.get_view(&tree)
        .expect("Failed to get first binding");
    
    // Verify key
    let key_handles = values.get_keys(&first_binding.keys)
        .expect("Failed to get keys for first binding");
    assert_eq!(key_handles.len(), 1, "Expected exactly one key for 'user' binding");
    
    let path_seg = values.get_path_segment(&key_handles[0])
        .expect("Failed to get path segment");
    match path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "user"),
        _ => panic!("Expected Ident path segment for 'user'"),
    }

    // Verify it's a value binding with a map (object)
    let binding_rhs = first_binding.binding_rhs.get_view(&tree)
        .expect("Failed to get binding rhs");
    match binding_rhs {
        BindingRhsView::ValueBinding(value_binding_handle) => {
            let value_binding = value_binding_handle.get_view(&tree)
                .expect("Failed to get value binding");
            let value = values.get_value(&value_binding.value)
                .expect("Failed to get value");
            match value {
                eure_value::value::Value::Map(map) => {
                    assert_eq!(map.0.len(), 2, "Expected map with 2 entries");
                    // Verify the map contains expected keys
                    let has_id = map.0.iter().any(|(k, v)| {
                        matches!(k, eure_value::value::KeyCmpValue::String(s) if s == "id") &&
                        matches!(v, eure_value::value::Value::I64(123))
                    });
                    let has_active = map.0.iter().any(|(k, v)| {
                        matches!(k, eure_value::value::KeyCmpValue::String(s) if s == "active") &&
                        matches!(v, eure_value::value::Value::Bool(true))
                    });
                    assert!(has_id, "Expected map to contain 'id' = 123");
                    assert!(has_active, "Expected map to contain 'active' = true");
                }
                _ => panic!("Expected Map value for 'user', got {:?}", value),
            }
        }
        _ => panic!("Expected ValueBinding for 'user' object"),
    }

    // Check second binding (items array)
    let second_bindings = bindings_view.eure_bindings.get_view(&tree)
        .expect("Failed to get remaining bindings")
        .expect("No second binding found");
    
    let second_binding = second_bindings.binding.get_view(&tree)
        .expect("Failed to get second binding");
    
    // Verify key
    let key_handles = values.get_keys(&second_binding.keys)
        .expect("Failed to get keys for second binding");
    assert_eq!(key_handles.len(), 1, "Expected exactly one key for 'items' binding");
    
    let path_seg = values.get_path_segment(&key_handles[0])
        .expect("Failed to get path segment");
    match path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "items"),
        _ => panic!("Expected Ident path segment for 'items'"),
    }
    
    // Verify it's a value binding with an array
    let binding_rhs = second_binding.binding_rhs.get_view(&tree)
        .expect("Failed to get binding rhs");
    match binding_rhs {
        BindingRhsView::ValueBinding(value_binding_handle) => {
            let value_binding = value_binding_handle.get_view(&tree)
                .expect("Failed to get value binding");
            let value = values.get_value(&value_binding.value)
                .expect("Failed to get value");
            match value {
                eure_value::value::Value::Array(arr) => {
                    assert_eq!(arr.0.len(), 2, "Expected array with 2 elements");
                    match &arr.0[0] {
                        eure_value::value::Value::String(s) => assert_eq!(s, "apple"),
                        _ => panic!("Expected first element to be string 'apple'"),
                    }
                    match &arr.0[1] {
                        eure_value::value::Value::String(s) => assert_eq!(s, "banana"),
                        _ => panic!("Expected second element to be string 'banana'"),
                    }
                }
                _ => panic!("Expected Array value for 'items', got {:?}", value),
            }
        }
        _ => panic!("Expected ValueBinding for 'items' array"),
    }
}

#[test]
fn test_with_sections() {
    let input = r#"name = "root value"

@ user
id = 456
email = "test@example.com""#;

    // Parse the EURE text
    let tree = parse(input).expect("Failed to parse sections");

    // Create storage for values
    let mut values = Values::default();

    // Create visitor
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Verify the extracted values
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    
    // Check root binding
    let bindings_view = eure_view.eure_bindings.get_view(&tree)
        .expect("Failed to get bindings")
        .expect("No bindings found");

    let root_binding = bindings_view.binding.get_view(&tree)
        .expect("Failed to get root binding");
    
    // Verify root key
    let key_handles = values.get_keys(&root_binding.keys)
        .expect("Failed to get keys for root binding");
    assert_eq!(key_handles.len(), 1, "Expected exactly one key for 'name' binding");
    
    let path_seg = values.get_path_segment(&key_handles[0])
        .expect("Failed to get path segment");
    match path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "name"),
        _ => panic!("Expected Ident path segment for 'name'"),
    }

    // Verify root value
    let binding_rhs = root_binding.binding_rhs.get_view(&tree)
        .expect("Failed to get binding rhs");
    match binding_rhs {
        BindingRhsView::ValueBinding(value_binding_handle) => {
            let value_binding = value_binding_handle.get_view(&tree)
                .expect("Failed to get value binding");
            let value = values.get_value(&value_binding.value)
                .expect("Failed to get value");
            match value {
                eure_value::value::Value::String(s) => assert_eq!(s, "root value"),
                _ => panic!("Expected String value for 'name', got {:?}", value),
            }
        }
        _ => panic!("Expected ValueBinding for 'name'"),
    }

    // Check sections
    let sections_view = eure_view.eure_sections.get_view(&tree)
        .expect("Failed to get sections")
        .expect("No sections found");

    let section = sections_view.section.get_view(&tree)
        .expect("Failed to get section");
    
    // Verify section key
    let section_keys = values.get_keys(&section.keys)
        .expect("Failed to get section keys");
    assert_eq!(section_keys.len(), 1, "Expected exactly one key for section");
    
    let section_path_seg = values.get_path_segment(&section_keys[0])
        .expect("Failed to get section path segment");
    match section_path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "user"),
        _ => panic!("Expected Ident path segment for 'user' section"),
    }

    // Verify section has body with bindings
    let section_body = section.section_body.get_view(&tree)
        .expect("Failed to get section body");
    match section_body {
        SectionBodyView::SectionBodyList(section_body_list_handle) => {
            let section_body_list = section_body_list_handle.get_view(&tree)
                .expect("Failed to get section body list")
                .expect("No section body list found");
            
            // First binding in section (id = 456)
            let first_section_binding = section_body_list.binding.get_view(&tree)
                .expect("Failed to get first section binding");
            
            let id_keys = values.get_keys(&first_section_binding.keys)
                .expect("Failed to get keys for id binding");
            assert_eq!(id_keys.len(), 1, "Expected exactly one key for 'id' binding");
            
            let id_path_seg = values.get_path_segment(&id_keys[0])
                .expect("Failed to get id path segment");
            match id_path_seg {
                PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "id"),
                _ => panic!("Expected Ident path segment for 'id'"),
            }
        }
        _ => panic!("Expected SectionBodyList for user section"),
    }
}

#[test]
fn test_parse_error() {
    let invalid_input = r#"name = 
age = "#;  // Invalid: missing values

    // Parse should fail
    let result = parse(invalid_input);
    assert!(result.is_err(), "Expected parse error for invalid input");
}

#[test]
fn test_empty_input() {
    let empty_input = "";

    // Parse the empty EURE text
    let tree = parse(empty_input).expect("Failed to parse empty input");

    // Create storage for values
    let mut values = Values::default();

    // Create visitor
    let mut visitor = ValueVisitor::new(empty_input, &mut values);

    // Visit the tree should succeed
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit empty tree");

    // Verify no bindings
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings = eure_view.eure_bindings.get_view(&tree)
        .expect("Failed to get bindings option");
    
    assert!(bindings.is_none(), "Expected no bindings in empty input");
}

#[test]
fn test_nested_sections() {
    let input = r#"@ config.database
host = "localhost"
port = 5432"#;

    // Parse the EURE text
    let tree = parse(input).expect("Failed to parse nested sections");

    // Create storage for values
    let mut values = Values::default();

    // Create visitor
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Verify the section structure
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    
    let sections_view = eure_view.eure_sections.get_view(&tree)
        .expect("Failed to get sections")
        .expect("No sections found");

    let section = sections_view.section.get_view(&tree)
        .expect("Failed to get section");
    
    // Verify section has two keys (config.database)
    let section_keys = values.get_keys(&section.keys)
        .expect("Failed to get section keys");
    assert_eq!(section_keys.len(), 2, "Expected two keys for nested section");
    
    // First key should be "config"
    let first_path_seg = values.get_path_segment(&section_keys[0])
        .expect("Failed to get first path segment");
    match first_path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "config"),
        _ => panic!("Expected Ident path segment for 'config'"),
    }
    
    // Second key should be "database"
    let second_path_seg = values.get_path_segment(&section_keys[1])
        .expect("Failed to get second path segment");
    match second_path_seg {
        PathSegment::Ident(ident) => assert_eq!(ident.to_string(), "database"),
        _ => panic!("Expected Ident path segment for 'database'"),
    }
}

#[test]
fn test_special_values() {
    let input = r#"enabled = true
disabled = false
nothing = null"#;

    // Parse the EURE text
    let tree = parse(input).expect("Failed to parse special values");

    // Create storage for values
    let mut values = Values::default();

    // Create visitor
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Helper function to check a binding's value
    fn check_binding_value(
        binding: BindingView,
        expected_key: &str,
        expected_value: &str,
        values: &Values,
        tree: &impl CstFacade,
    ) {
        // Check key
        let key_handles = values.get_keys(&binding.keys)
            .expect("Failed to get keys");
        assert_eq!(key_handles.len(), 1);
        
        let path_seg = values.get_path_segment(&key_handles[0])
            .expect("Failed to get path segment");
        match path_seg {
            PathSegment::Ident(ident) => assert_eq!(ident.to_string(), expected_key),
            _ => panic!("Expected Ident path segment"),
        }

        // Check value
        let binding_rhs = binding.binding_rhs.get_view(tree)
            .expect("Failed to get binding rhs");
        match binding_rhs {
            BindingRhsView::ValueBinding(value_binding_handle) => {
                let value_binding = value_binding_handle.get_view(tree)
                    .expect("Failed to get value binding");
                let value = values.get_value(&value_binding.value)
                    .expect("Failed to get value");
                match (expected_value, value) {
                    ("true", eure_value::value::Value::Bool(b)) => assert!(b),
                    ("false", eure_value::value::Value::Bool(b)) => assert!(!b),
                    ("null", eure_value::value::Value::Null) => {},
                    _ => panic!("Unexpected value type or value: expected {}, got {:?}", expected_value, value),
                }
            }
            _ => panic!("Expected ValueBinding"),
        }
    }

    // Get bindings
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings_view = eure_view.eure_bindings.get_view(&tree)
        .expect("Failed to get bindings")
        .expect("No bindings found");

    // Check enabled = true
    let first_binding = bindings_view.binding.get_view(&tree)
        .expect("Failed to get first binding");
    check_binding_value(first_binding, "enabled", "true", &values, &tree);

    // Check disabled = false
    let second_bindings = bindings_view.eure_bindings.get_view(&tree)
        .expect("Failed to get remaining bindings")
        .expect("No second binding found");
    let second_binding = second_bindings.binding.get_view(&tree)
        .expect("Failed to get second binding");
    check_binding_value(second_binding, "disabled", "false", &values, &tree);

    // Check nothing = null
    let third_bindings = second_bindings.eure_bindings.get_view(&tree)
        .expect("Failed to get remaining bindings")
        .expect("No third binding found");
    let third_binding = third_bindings.binding.get_view(&tree)
        .expect("Failed to get third binding");
    check_binding_value(third_binding, "nothing", "null", &values, &tree);
}

#[test]
fn test_numeric_values() {
    let input = r#"integer = 42
large_number = 1000000
small_number = 7"#;

    // Parse the EURE text
    let tree = parse(input).expect("Failed to parse numeric values");

    // Create storage for values
    let mut values = Values::default();

    // Create visitor
    let mut visitor = ValueVisitor::new(input, &mut values);

    // Visit the tree
    tree.visit_from_root(&mut visitor)
        .expect("Failed to visit tree");

    // Helper to navigate through bindings
    fn get_nth_binding(
        initial_bindings: EureBindingsView,
        n: usize,
        tree: &impl CstFacade,
    ) -> BindingView {
        let mut current_bindings = Some(initial_bindings);
        let mut index = 0;
        
        while let Some(bindings) = current_bindings {
            if index == n {
                return bindings.binding.get_view(tree)
                    .expect("Failed to get binding");
            }
            
            current_bindings = bindings.eure_bindings.get_view(tree)
                .expect("Failed to get next bindings");
            index += 1;
        }
        
        panic!("Could not find binding at index {}", n);
    }

    // Get root bindings
    let root_view = tree.root_handle().get_view(&tree).expect("Failed to get root view");
    let eure_view = root_view.eure.get_view(&tree).expect("Failed to get eure view");
    let bindings_view = eure_view.eure_bindings.get_view(&tree)
        .expect("Failed to get bindings")
        .expect("No bindings found");

    // Test each numeric value
    let test_cases = [
        ("integer", "42"),
        ("large_number", "1000000"),
        ("small_number", "7"),
    ];

    for (i, (key, expected_value)) in test_cases.iter().enumerate() {
        let binding = get_nth_binding(bindings_view, i, &tree);
        
        // Check key
        let key_handles = values.get_keys(&binding.keys)
            .expect("Failed to get keys");
        assert_eq!(key_handles.len(), 1);
        
        let path_seg = values.get_path_segment(&key_handles[0])
            .expect("Failed to get path segment");
        match path_seg {
            PathSegment::Ident(ident) => assert_eq!(ident.to_string(), *key),
            _ => panic!("Expected Ident path segment for '{}'", key),
        }

        // Check value
        let binding_rhs = binding.binding_rhs.get_view(&tree)
            .expect("Failed to get binding rhs");
        match binding_rhs {
            BindingRhsView::ValueBinding(value_binding_handle) => {
                let value_binding = value_binding_handle.get_view(&tree)
                    .expect("Failed to get value binding");
                let value = values.get_value(&value_binding.value)
                    .expect("Failed to get value");
                match (*expected_value, value) {
                    ("42", eure_value::value::Value::I64(n)) => assert_eq!(*n, 42),
                    ("1000000", eure_value::value::Value::I64(n)) => assert_eq!(*n, 1000000),
                    ("7", eure_value::value::Value::I64(n)) => assert_eq!(*n, 7),
                    _ => panic!("Unexpected value type or value: expected {}, got {:?}", expected_value, value),
                }
            }
            _ => panic!("Expected ValueBinding for '{}'", key),
        }
    }
}