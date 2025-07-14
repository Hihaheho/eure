use eure_tree::value_visitor::ValueVisitor;
use eure_tree::prelude::CstFacade;
use eure_tree::prelude::*;
use eure_value::value::{Value, KeyCmpValue};

#[test]
fn test_bindings_before_sections() {
    // According to grammar: Eure: { Binding } { Section }
    // Root bindings come BEFORE sections
    let input = r#"
root1 = "value1"
root2 = .$types.Test

@ section1
field1 = "in_section"
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    let document = visitor.into_document();
    let doc = document.to_value();
    
    println!("Document structure: {doc:#?}");
    
    if let Value::Map(map) = doc {
        println!("\nTop-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        // Check root bindings
        assert!(map.0.contains_key(&KeyCmpValue::String("root1".to_string())), "root1 should be at root");
        assert!(map.0.contains_key(&KeyCmpValue::String("root2".to_string())), "root2 should be at root");
        assert!(map.0.contains_key(&KeyCmpValue::String("section1".to_string())), "section1 should be at root");
        
        // Check root2 is a Path
        if let Some(val) = map.0.get(&KeyCmpValue::String("root2".to_string())) {
            match val {
                Value::Path(_) => println!("root2 is correctly a Path"),
                _ => panic!("root2 should be a Path, but got: {val:?}"),
            }
        }
    }
}

#[test]
fn test_sections_only() {
    // No root bindings, only sections
    let input = r#"
@ section1
field1 = "value1"

@ section2  
field2 = .$types.Test
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    let document = visitor.into_document();
    let doc = document.to_value();
    
    println!("\nSections-only document: {doc:#?}");
    
    if let Value::Map(map) = doc {
        println!("Top-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        // Both sections should be at root
        assert!(map.0.contains_key(&KeyCmpValue::String("section1".to_string())));
        assert!(map.0.contains_key(&KeyCmpValue::String("section2".to_string())));
        
        // Check section2.field2 is a Path
        if let Some(Value::Map(section2_map)) = map.0.get(&KeyCmpValue::String("section2".to_string()))
            && let Some(val) = section2_map.0.get(&KeyCmpValue::String("field2".to_string())) {
                match val {
                    Value::Path(_) => println!("section2.field2 is correctly a Path"),
                    _ => panic!("section2.field2 should be a Path, but got: {val:?}"),
                }
            }
    }
}

#[test]
fn test_empty_section() {
    // Empty sections should create empty maps
    let input = r#"
@ empty_section

@ section_with_content
field = "value"
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    let document = visitor.into_document();
    let doc = document.to_value();
    
    if let Value::Map(map) = doc {
        // Check empty_section exists and is empty
        if let Some(Value::Map(empty_map)) = map.0.get(&KeyCmpValue::String("empty_section".to_string())) {
            assert!(empty_map.0.is_empty(), "empty_section should be an empty map");
            println!("empty_section is correctly an empty map");
        } else {
            panic!("empty_section should exist and be a Map");
        }
    }
}