use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Deserialize, Serialize};

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
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("name"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("age"))
        );
    } else {
        panic!("Expected object schema");
    }
}
