use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Deserialize, Serialize};

#[test]
fn test_transparent_newtype() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(transparent)]
    struct UserId(String);

    let schema = UserId::eure_schema();

    // Should be String, not an object
    assert_eq!(schema.type_expr, Type::String);
}

#[test]
fn test_transparent_struct() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(transparent)]
    struct Wrapper {
        value: i32,
    }

    let schema = Wrapper::eure_schema();

    // Should be Number, not an object
    assert_eq!(schema.type_expr, Type::Number);
}

#[test]
fn test_transparent_with_option() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(transparent)]
    struct MaybeString {
        inner: Option<String>,
    }

    let schema = MaybeString::eure_schema();

    // Should be an optional String
    assert!(schema.optional);
    assert_eq!(schema.type_expr, Type::String);
}

#[test]
fn test_transparent_with_vec() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(transparent)]
    struct StringList(Vec<String>);

    let schema = StringList::eure_schema();

    // Should be an array of strings
    if let Type::Array(inner) = &schema.type_expr {
        assert_eq!(**inner, Type::String);
    } else {
        panic!("Expected array type");
    }
}

#[test]
fn test_transparent_preserves_constraints() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(transparent)]
    struct Email {
        #[eure(pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")]
        value: String,
    }

    // Constraints on fields inside transparent structs should now be preserved
    let schema = Email::eure_schema();
    assert_eq!(schema.type_expr, Type::String);
    // Check that the pattern constraint is preserved
    assert!(schema.constraints.pattern.is_some());
    assert_eq!(
        schema.constraints.pattern.as_ref().unwrap(),
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    );
}

#[test]
fn test_transparent_with_complex_type() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Inner {
        a: String,
        b: i32,
    }

    #[derive(Eure, Serialize, Deserialize)]
    #[serde(transparent)]
    struct Outer {
        inner: Inner,
    }

    let schema = Outer::eure_schema();

    // Should have the same schema as Inner
    if let Type::Object(obj_schema) = &schema.type_expr {
        assert_eq!(obj_schema.fields.len(), 2);
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("a"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("b"))
        );
    } else {
        panic!("Expected object schema");
    }
}
