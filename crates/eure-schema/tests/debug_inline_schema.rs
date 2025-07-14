use eure_schema::*;
use eure_value::value::KeyCmpValue;

#[test]
fn test_inline_schema_extraction() {
    let schema_text = r#"user.$type = .object
user.$prefer.section = true  
user.age.$type = .number"#;
    
    let result = extract_schema_from_value(schema_text).expect("Failed to extract schema");
    
    eprintln!("Root fields: {:?}", result.document_schema.root.fields.keys().collect::<Vec<_>>());
    
    if let Some(user_field) = result.document_schema.root.fields.get(&KeyCmpValue::String("user".to_string())) {
        eprintln!("User field type: {:?}", user_field.type_expr);
        
        if let Type::Object(user_obj) = &user_field.type_expr {
            eprintln!("User object fields: {:?}", user_obj.fields.keys().collect::<Vec<_>>());
            
            // This should contain "age"
            assert!(user_obj.fields.contains_key(&KeyCmpValue::String("age".to_string())), 
                    "user object should contain 'age' field");
        } else {
            panic!("user field should be an object");
        }
    } else {
        panic!("root should contain 'user' field");
    }
}