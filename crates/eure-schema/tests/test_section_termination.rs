use eure_tree::value_visitor::{ValueVisitor, Values};
use eure_tree::visitor::CstVisitorSuper;
use eure_tree::prelude::*;
use eure_value::value::{Value, KeyCmpValue};

#[test]
fn test_section_termination_methods() {
    // Test 1: Sections are terminated by another section
    let input1 = r#"
@ section1
field1 = "value1"

@ section2
field2 = .$types.Test
"#;

    let tree1 = eure_parol::parse(input1).expect("Parse should succeed");
    let mut values1 = Values::default();
    let mut visitor1 = ValueVisitor::new(input1, &mut values1);
    tree1.visit_from_root(&mut visitor1).expect("Visit should succeed");
    
    let doc1 = if let Ok(root_view) = tree1.root_handle().get_view(&tree1) {
        values1.get_eure(&root_view.eure).expect("Should have eure value")
    } else {
        panic!("Invalid document structure");
    };
    
    println!("Test 1 - Section followed by section:");
    if let Value::Map(map) = doc1 {
        println!("  Top-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        if let Some(Value::Map(section2_map)) = map.0.get(&KeyCmpValue::String("section2".to_string())) {
            if let Some(field2_val) = section2_map.0.get(&KeyCmpValue::String("field2".to_string())) {
                println!("  section2.field2 type: {:?}", match field2_val {
                    Value::Path(_) => "Path",
                    Value::Map(_) => "Map",
                    _ => "Other",
                });
            }
        }
    }
    
    // Test 2: Root-level bindings after sections
    let input2 = r#"
@ section1
field1 = "value1"

root_field = .$types.Test
"#;

    let tree2 = eure_parol::parse(input2).expect("Parse should succeed");
    let mut values2 = Values::default();
    let mut visitor2 = ValueVisitor::new(input2, &mut values2);
    tree2.visit_from_root(&mut visitor2).expect("Visit should succeed");
    
    let doc2 = if let Ok(root_view) = tree2.root_handle().get_view(&tree2) {
        values2.get_eure(&root_view.eure).expect("Should have eure value")
    } else {
        panic!("Invalid document structure");
    };
    
    println!("\nTest 2 - Root binding after section (no @ terminator):");
    if let Value::Map(map) = doc2 {
        println!("  Top-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        // Check if root_field is at root or inside section1
        if map.0.contains_key(&KeyCmpValue::String("root_field".to_string())) {
            println!("  root_field is at ROOT level");
            if let Some(val) = map.0.get(&KeyCmpValue::String("root_field".to_string())) {
                println!("  root_field type: {:?}", match val {
                    Value::Path(_) => "Path", 
                    Value::Map(_) => "Map",
                    _ => "Other",
                });
            }
        } else if let Some(Value::Map(section1_map)) = map.0.get(&KeyCmpValue::String("section1".to_string())) {
            if section1_map.0.contains_key(&KeyCmpValue::String("root_field".to_string())) {
                println!("  root_field is INSIDE section1");
            }
        }
    }
}

#[test]
fn test_inline_block_termination() {
    // Test how inline blocks work
    let input = r#"
section1 {
  field1 = "value1"
}

root_field = .$types.Test
"#;

    let tree = eure_parol::parse(input).expect("Parse should succeed");
    let mut values = Values::default();
    let mut visitor = ValueVisitor::new(input, &mut values);
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    
    let doc = if let Ok(root_view) = tree.root_handle().get_view(&tree) {
        values.get_eure(&root_view.eure).expect("Should have eure value")
    } else {
        panic!("Invalid document structure");
    };
    
    println!("\nTest 3 - Inline block termination:");
    if let Value::Map(map) = doc {
        println!("  Top-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        if let Some(val) = map.0.get(&KeyCmpValue::String("root_field".to_string())) {
            println!("  root_field type: {:?}", match val {
                Value::Path(_) => "Path",
                Value::Map(_) => "Map", 
                _ => "Other",
            });
        }
    }
}