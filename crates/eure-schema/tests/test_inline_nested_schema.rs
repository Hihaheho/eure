use eure_schema::*;
use eure_value::value::KeyCmpValue;

#[test]
fn test_inline_nested_schema_extraction() {
    // Test that user.name.first.$type = .string creates proper nested structure
    let schema_text = r#"
user.$prefer.section = true
user.name.first.$type = .string
user.name.last.$type = .string
user.name.middle.$type = .string
user.name.middle.$optional = true
user.age.$type = .number
"#;
    
    let result = extract_schema_from_value(schema_text).expect("Failed to extract schema");
    
    eprintln!("Root fields: {:?}", result.document_schema.root.fields.keys().collect::<Vec<_>>());
    
    // Check user field
    let user_field = result.document_schema.root.fields
        .get(&KeyCmpValue::String("user".to_string()))
        .expect("Should have user field");
    
    eprintln!("User field type: {:?}", user_field.type_expr);
    
    // User should be an object
    match &user_field.type_expr {
        Type::Object(user_obj) => {
            eprintln!("User object has {} fields", user_obj.fields.len());
            
            // Check for name field
            let name_field = user_obj.fields
                .get(&KeyCmpValue::String("name".to_string()))
                .expect("User should have name field");
            
            eprintln!("Name field type: {:?}", name_field.type_expr);
            
            // Name should be an object
            match &name_field.type_expr {
                Type::Object(name_obj) => {
                    eprintln!("Name object has {} fields", name_obj.fields.len());
                    
                    // Check for first and last
                    assert!(name_obj.fields.contains_key(&KeyCmpValue::String("first".to_string())));
                    assert!(name_obj.fields.contains_key(&KeyCmpValue::String("last".to_string())));
                }
                _ => panic!("name field should be an object, got {:?}", name_field.type_expr)
            }
            
            // Check for age field
            assert!(user_obj.fields.contains_key(&KeyCmpValue::String("age".to_string())));
        }
        _ => panic!("user field should be an object, got {:?}", user_field.type_expr)
    }
}