use eure_tree::value_visitor::ValueVisitor;
use eure_tree::prelude::CstFacade;
use eure_tree::prelude::*;
use eure_value::value::{Value, KeyCmpValue};

#[test]
fn test_path_value_in_different_contexts() {
    // Test 1: Path value without any sections
    let input1 = r#"
test = .$types.User
"#;
    
    let tree1 = eure_parol::parse(input1).expect("Parse should succeed");
    let mut visitor1 = ValueVisitor::new(input1);
    tree1.visit_from_root(&mut visitor1).expect("Visit should succeed");
    let document1 = visitor1.into_document();
    let doc1 = document1.to_value();
    
    println!("Test 1 - No sections:");
    if let Value::Map(map) = doc1
        && let Some(val) = map.0.get(&KeyCmpValue::String("test".to_string())) {
            println!("  test value: {val:?}");
        }
    
    // Test 2: Path value with unrelated section
    let input2 = r#"
@ other
x = 1

@
test = .$types.User
"#;
    
    let tree2 = eure_parol::parse(input2).expect("Parse should succeed");
    let mut visitor2 = ValueVisitor::new(input2);
    tree2.visit_from_root(&mut visitor2).expect("Visit should succeed");
    let document2 = visitor2.into_document();
    let doc2 = document2.to_value();
    
    println!("\nTest 2 - With unrelated section:");
    if let Value::Map(map) = doc2
        && let Some(val) = map.0.get(&KeyCmpValue::String("test".to_string())) {
            println!("  test value: {val:?}");
        }
    
    // Test 3: Path value with $types section
    let input3 = r#"
@ $types
@ $types.User
name = .string

@
test = .$types.User
"#;
    
    let tree3 = eure_parol::parse(input3).expect("Parse should succeed");
    let mut visitor3 = ValueVisitor::new(input3);
    tree3.visit_from_root(&mut visitor3).expect("Visit should succeed");
    let document3 = visitor3.into_document();
    let doc3 = document3.to_value();
    
    println!("\nTest 3 - With $types section:");
    if let Value::Map(map) = doc3 {
        if let Some(val) = map.0.get(&KeyCmpValue::String("test".to_string())) {
            println!("  test value: {val:?}");
        }
        
        // Also check the $types structure
        if let Some(Value::Map(types_map)) = map.0.get(&KeyCmpValue::String("$types".to_string())) {
            println!("  $types keys: {:?}", types_map.0.keys().collect::<Vec<_>>());
        }
    }
}

#[test]
fn test_binding_vs_section_binding() {
    // Compare how bindings work inside vs outside sections
    let input = r#"
@ section1
inner = .path.value

@
outer = .path.value
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
    
    println!("\nBinding comparison:");
    if let Value::Map(map) = doc {
        // Check outer binding
        if let Some(val) = map.0.get(&KeyCmpValue::String("outer".to_string())) {
            println!("  outer value type: {:?}", match val {
                Value::Path(_) => "Path",
                Value::Map(_) => "Map",
                _ => "Other",
            });
        }
        
        // Check inner binding
        if let Some(Value::Map(section_map)) = map.0.get(&KeyCmpValue::String("section1".to_string()))
            && let Some(val) = section_map.0.get(&KeyCmpValue::String("inner".to_string())) {
                println!("  inner value type: {:?}", match val {
                    Value::Path(_) => "Path",
                    Value::Map(_) => "Map", 
                    _ => "Other",
                });
            }
    }
}