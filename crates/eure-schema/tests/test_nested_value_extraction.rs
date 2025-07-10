use eure_tree::value_visitor::ValueVisitor;
use eure_tree::prelude::CstFacade;
use eure_tree::prelude::*;
use eure_value::value::{Value, KeyCmpValue};

#[test]
fn test_nested_section_extraction() {
    // Test with sections that have nested content
    let input = r#"
@ $types
@ $types.User
name.$type = .string

@
users.$array = .$types.User
"#;

    // Parse to CST
    let tree = eure_parol::parse(input).expect("Parse should succeed");
    
    // Extract to Value
    let mut values = Values::default();
    let mut visitor = ValueVisitor::new(input, &mut values);
    
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    
    // Get document value
    let doc_value = if let Ok(root_view) = tree.root_handle().get_view(&tree) {
        values.get_eure(&root_view.eure).expect("Should have eure value")
    } else {
        panic!("Invalid document structure");
    };
    
    println!("Full document: {doc_value:#?}");
    
    // Check if users.$array has the correct Path value
    if let Value::Map(map) = doc_value
        && let Some(Value::Map(users_map)) = map.0.get(&KeyCmpValue::String("users".to_string()))
            && let Some(array_value) = users_map.0.get(&KeyCmpValue::String("$array".to_string())) {
                println!("\nusers.$array value: {array_value:?}");
                match array_value {
                    Value::Path(_) => println!("SUCCESS: $array has Path value"),
                    Value::Map(_) => println!("FAIL: $array has Map value instead of Path"),
                    _ => println!("FAIL: $array has unexpected value type"),
                }
            }
}