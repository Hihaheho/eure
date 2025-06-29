use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Serialize, Deserialize};

#[test]
fn test_basic_struct() {
    #[derive(Eure, Serialize, Deserialize)]
    struct User {
        name: String,
        age: u32,
    }
    
    let schema = User::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        assert_eq!(obj_schema.fields.len(), 2);
        assert!(obj_schema.fields.contains_key("name"));
        assert!(obj_schema.fields.contains_key("age"));
    } else {
        panic!("Expected object schema");
    }
}