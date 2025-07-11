//! Test array element sections work correctly

use eure_tree::value_visitor::ValueVisitor;
use eure_tree::document::NodeValue;

#[test]
fn test_array_element_sections() {
    let document = r#"
name = "Test Corp"

@ items[0] {
    id = 1
    name = "First"
}

@ items[1] {
    id = 2
    name = "Second"
}
"#;

    // Parse the EURE document
    let parsed = eure_parol::parse(document).expect("Failed to parse");

    // Convert CST to EureDocument
    let mut visitor = ValueVisitor::new(document);
    parsed.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    let eure_document = visitor.into_document();
    
    // Verify the structure
    let root = eure_document.get_root();
    match &root.content {
        NodeValue::Map { entries, .. } => {
            assert_eq!(entries.len(), 2, "Expected 2 entries at root");
            
            // Find the items entry
            let items_entry = entries.iter()
                .find(|(k, _)| matches!(k, eure_tree::document::DocumentKey::Ident(id) if id.to_string() == "items"))
                .expect("Should have 'items' key");
            
            let items_node = eure_document.get_node(items_entry.1);
            match &items_node.content {
                NodeValue::Array { children, .. } => {
                    assert_eq!(children.len(), 2, "Expected 2 array elements");
                    
                    // Check first element
                    let elem0 = eure_document.get_node(children[0]);
                    match &elem0.content {
                        NodeValue::Map { entries, .. } => {
                            assert_eq!(entries.len(), 2, "Expected 2 fields in first element");
                        }
                        _ => panic!("Expected first array element to be a map"),
                    }
                    
                    // Check second element
                    let elem1 = eure_document.get_node(children[1]);
                    match &elem1.content {
                        NodeValue::Map { entries, .. } => {
                            assert_eq!(entries.len(), 2, "Expected 2 fields in second element");
                        }
                        _ => panic!("Expected second array element to be a map"),
                    }
                }
                _ => panic!("Expected 'items' to be an array"),
            }
        }
        _ => panic!("Expected root to be a map"),
    }
}

#[test]
fn test_mixed_array_syntax() {
    let document = r#"
# Array with inline elements
colors = ["red", "green", "blue"]

# Array with section elements
@ users[0] {
    name = "Alice"
    admin = true
}

@ users[1] {
    name = "Bob"
    admin = false
}
"#;

    let parsed = eure_parol::parse(document).expect("Failed to parse");
    let mut visitor = ValueVisitor::new(document);
    parsed.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    let eure_document = visitor.into_document();
    let root = eure_document.get_root();
    
    match &root.content {
        NodeValue::Map { entries, .. } => {
            assert_eq!(entries.len(), 2, "Expected 2 entries at root");
            
            // Check colors array
            let colors_entry = entries.iter()
                .find(|(k, _)| matches!(k, eure_tree::document::DocumentKey::Ident(id) if id.to_string() == "colors"))
                .expect("Should have 'colors' key");
            
            let colors_node = eure_document.get_node(colors_entry.1);
            match &colors_node.content {
                NodeValue::Array { children, .. } => {
                    assert_eq!(children.len(), 3, "Expected 3 color elements");
                }
                _ => panic!("Expected 'colors' to be an array"),
            }
            
            // Check users array
            let users_entry = entries.iter()
                .find(|(k, _)| matches!(k, eure_tree::document::DocumentKey::Ident(id) if id.to_string() == "users"))
                .expect("Should have 'users' key");
            
            let users_node = eure_document.get_node(users_entry.1);
            match &users_node.content {
                NodeValue::Array { children, .. } => {
                    assert_eq!(children.len(), 2, "Expected 2 user elements");
                    
                    // Both elements should be maps
                    for (i, &child_id) in children.iter().enumerate() {
                        let child = eure_document.get_node(child_id);
                        match &child.content {
                            NodeValue::Map { entries, .. } => {
                                assert_eq!(entries.len(), 2, "Expected 2 fields in user {}", i);
                            }
                            _ => panic!("Expected user {} to be a map", i),
                        }
                    }
                }
                _ => panic!("Expected 'users' to be an array"),
            }
        }
        _ => panic!("Expected root to be a map"),
    }
}