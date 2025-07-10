use eure_tree::value_visitor::ValueVisitor;
use eure_tree::prelude::CstFacade;
use eure_tree::prelude::*;
use eure_value::value::{Value, KeyCmpValue};

#[test]
fn test_section_with_value_assignment() {
    // Test how @ section = value is parsed
    let input = r#"
@ section1 = "direct value"
@ section2.nested = 123
@ section3.$extension = true
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
    
    println!("Document structure: {doc:#?}");
    
    if let Value::Map(map) = doc {
        println!("\nTop-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        // These should create sections, but the value assignment might not work
        // as the grammar shows SectionBody can be "Bind Value" but the visitor
        // might not handle it properly
    }
}

#[test]
fn test_actual_failing_case() {
    // This is what's in the failing test
    let input = r#"
@ $types.User
name.$type = .string
age.$type = .number

@
users.$array = .$types.User
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
    
    println!("\nActual structure from failing test: {doc:#?}");
    
    if let Value::Map(map) = doc {
        println!("\nTop-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        // Check if there's a section named "users.$array"
        for (key, value) in &map.0 {
            match key {
                KeyCmpValue::String(s) if s.contains("users") => {
                    println!("Found key containing 'users': {s:?} -> {value:?}");
                }
                _ => {}
            }
        }
    }
}