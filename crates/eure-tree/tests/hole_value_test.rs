use eure_tree::prelude::*;
use eure_value::value::Value;

#[test]
fn test_parse_hole_value() {
    let input = r#"
        name = "John"
        age = !
        address = {
            street = !
            city = "New York"
        }
        items = [!, "second", !]
    "#;

    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");

    // Get the document
    let document = visitor.into_document();
    let doc_value = eure_tree::value_visitor::document_to_value(document);
    
    // Verify holes are present
    match doc_value {
        Value::Map(map) => {
            // Check age field
            assert!(matches!(
                map.0.get(&eure_value::value::KeyCmpValue::String("age".to_string())), 
                Some(Value::Hole)
            ), "Expected age to be a hole");
            
            // Check address.street
            if let Some(Value::Map(address_map)) = map.0.get(&eure_value::value::KeyCmpValue::String("address".to_string())) {
                assert!(matches!(
                    address_map.0.get(&eure_value::value::KeyCmpValue::String("street".to_string())), 
                    Some(Value::Hole)
                ), "Expected street to be a hole");
            } else {
                panic!("Address field not found or not a map");
            }
            
            // Check items array
            if let Some(Value::Array(items)) = map.0.get(&eure_value::value::KeyCmpValue::String("items".to_string())) {
                assert_eq!(items.0.len(), 3);
                assert!(matches!(&items.0[0], Value::Hole), "Expected first item to be a hole");
                assert!(matches!(&items.0[1], Value::String(s) if s == "second"), "Expected second item to be 'second'");
                assert!(matches!(&items.0[2], Value::Hole), "Expected third item to be a hole");
            } else {
                panic!("Items field not found or not an array");
            }
        }
        _ => panic!("Expected document to be a map"),
    }
}

#[test]
fn test_hole_in_different_contexts() {
    // Test holes in various positions
    let test_cases = vec![
        ("value = !", "simple hole"),
        ("tuple = (1, !, 3)", "hole in tuple"),
        ("nested = {a = {b = !}}", "hole in nested object"),
        ("@section = !", "hole in section"),
        ("mixed = [!, null, !, true]", "multiple holes in array"),
    ];

    for (input, description) in test_cases {
        let tree = eure_parol::parse(input).unwrap_or_else(|_| panic!("Failed to parse: {description}"));
        
        let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input);
        tree.visit_from_root(&mut visitor).unwrap_or_else(|_| panic!("Failed to visit tree: {description}"));
        
        // Just verify parsing succeeds - the important thing is that holes are recognized and don't cause parse errors
    }
}

#[test]
fn test_hole_value_type() {
    let input = "test = !";
    
    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    let document = visitor.into_document();
    let doc_value = eure_tree::value_visitor::document_to_value(document);
    
    match doc_value {
        Value::Map(map) => {
            let test_value = map.0.get(&eure_value::value::KeyCmpValue::String("test".to_string()))
                .expect("test field not found");
            assert!(matches!(test_value, Value::Hole), "Expected test value to be a hole, got {test_value:?}");
        }
        _ => panic!("Expected document to be a map"),
    }
}