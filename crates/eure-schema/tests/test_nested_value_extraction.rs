use eure_tree::value_visitor::ValueVisitor;
use eure_tree::document::{NodeValue, DocumentKey};
use eure_value::value::KeyCmpValue;
use eure_value::identifier::Identifier;
use std::str::FromStr;

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
    
    // Use the new API
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    
    // Get document
    let document = visitor.into_document();
    
    println!("Document root has {} entries", match &document.get_root().content {
        NodeValue::Map { entries, .. } => entries.len(),
        _ => 0
    });
    
    // Check if users.$array has the correct Path value
    let root = document.get_root();
    if let NodeValue::Map { entries, .. } = &root.content {
        // Find "users" entry
        let users_key = DocumentKey::Ident(Identifier::from_str("users").unwrap());
        if let Some(users_node_id) = entries.iter().find(|(k, _)| k == &users_key).map(|(_, id)| id) {
            let users_node = document.get_node(*users_node_id);
            println!("\nFound users node");
            
            if let NodeValue::Map { entries: users_entries, .. } = &users_node.content {
                // Check for $array extension
                let array_ext = Identifier::from_str("array").unwrap();
                if let Some(array_node_id) = users_node.extensions.get(&array_ext) {
                    let array_node = document.get_node(*array_node_id);
                    println!("users.$array value: {:?}", array_node.content);
                    match &array_node.content {
                        NodeValue::Path { .. } => println!("SUCCESS: $array has Path value"),
                        NodeValue::Map { .. } => println!("FAIL: $array has Map value instead of Path"),
                        _ => println!("FAIL: $array has unexpected value type"),
                    }
                } else {
                    println!("No $array extension found on users node");
                }
            }
        } else {
            println!("No users field found in root");
        }
    }
}