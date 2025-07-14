use eure_tree::value_visitor::ValueVisitor;
use eure_tree::prelude::CstFacade;
use eure_tree::prelude::*;
use eure_value::value::Value;

#[test]
fn test_section_boundaries() {
    // Test with explicit empty section to end the $types.User section
    let input = r#"
@ $types.User
name.$type = .string
age.$type = .number

@
users.$array = .$types.User
"#;

    // Parse to CST
    let tree = eure_parol::parse(input).expect("Parse should succeed");
    
    // Extract to Value
    let mut visitor = ValueVisitor::new(input);
    
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    
    // Get document value
    let document = visitor.into_document();
    let doc_value = document.to_value();
    
    // Check structure
    if let Value::Map(map) = doc_value {
        println!("Top-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        
        // Check if users is at root
        if map.0.contains_key(&eure_value::value::KeyCmpValue::String("users".to_string())) {
            println!("SUCCESS: 'users' is at root level");
        } else {
            println!("FAIL: 'users' is not at root level");
            
            // Check if it's inside $types
            if let Some(Value::Map(types_map)) = map.0.get(&eure_value::value::KeyCmpValue::String("$types".to_string()))
                && let Some(Value::Map(user_map)) = types_map.0.get(&eure_value::value::KeyCmpValue::String("User".to_string())) {
                    println!("User type keys: {:?}", user_map.0.keys().collect::<Vec<_>>());
                }
        }
    }
}

#[test]
fn test_inline_vs_section() {
    // Compare inline syntax behavior
    let input = r#"
$types.User {
  name.$type = .string
  age.$type = .number
}

users.$array = .$types.User
"#;

    // Parse to CST
    let tree = eure_parol::parse(input).expect("Parse should succeed");
    
    // Extract to Value
    let mut visitor = ValueVisitor::new(input);
    
    tree.visit_from_root(&mut visitor).expect("Visit should succeed");
    
    // Get document value
    let document = visitor.into_document();
    let doc_value = document.to_value();
    
    // Check structure
    if let Value::Map(map) = doc_value {
        println!("\nInline syntax - Top-level keys: {:?}", map.0.keys().collect::<Vec<_>>());
        assert!(map.0.contains_key(&eure_value::value::KeyCmpValue::String("users".to_string())),
                "With inline syntax, 'users' should be at root level");
    }
}