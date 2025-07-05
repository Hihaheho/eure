use eure_tree::value_visitor::{ValueVisitor, Values};
use eure_tree::prelude::*;
use eure_value::value::Value;

#[test]
fn trace_value_extraction() {
    let input = r#"
@ $types.User
name.$type = .string
age.$type = .number

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
    
    // Print the structure
    println!("Document value: {doc_value:#?}");
    
    // Check what's in the map
    if let Value::Map(map) = doc_value {
        println!("\nTop-level keys:");
        for (key, _) in &map.0 {
            println!("  {key:?}");
        }
        
        // Check if users key exists
        if let Some(users_value) = map.0.get(&eure_value::value::KeyCmpValue::String("users".to_string())) {
            println!("\nFound 'users' key with value: {users_value:?}");
        } else {
            println!("\n'users' key not found!");
        }
    }
}