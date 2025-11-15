use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Deserialize, Serialize};
use std::option;

// Type alias for Option
type MaybeString = Option<String>;
type MaybeNumber = std::option::Option<u32>;

#[test]
fn test_option_type_aliases() {
    #[derive(Eure, Serialize, Deserialize)]
    struct WithAliases {
        regular_option: Option<String>,
        aliased_option: MaybeString,
        fully_qualified: std::option::Option<i32>,
        module_qualified: option::Option<bool>,
        nested_alias: MaybeNumber,
    }

    let schema = WithAliases::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // All fields should be optional
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("regular_option")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("aliased_option")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("fully_qualified")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("module_qualified")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("nested_alias")].optional);

        // Check inner types
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("regular_option")].type_expr,
            Type::String
        );
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("aliased_option")].type_expr,
            Type::String
        );
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("fully_qualified")].type_expr,
            Type::Number
        );
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("module_qualified")].type_expr,
            Type::Boolean
        );
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("nested_alias")].type_expr,
            Type::Number
        );
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_nested_options() {
    #[derive(Eure, Serialize, Deserialize)]
    struct NestedOptions {
        maybe_maybe_string: Option<Option<String>>,
        vec_of_options: Vec<Option<i32>>,
        option_vec: Option<Vec<String>>,
    }

    let schema = NestedOptions::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // Check maybe_maybe_string - should be optional Option<String>
        let maybe_maybe = &obj_schema.fields[&eure_schema::ObjectKey::from("maybe_maybe_string")];
        assert!(maybe_maybe.optional);
        // The inner type is Option<String> which itself has optional=true

        // Check vec_of_options - Vec<Option<i32>>
        let vec_opts = &obj_schema.fields[&eure_schema::ObjectKey::from("vec_of_options")];
        assert!(!vec_opts.optional); // Vec itself is not optional
        if let Type::Array(_inner) = &vec_opts.type_expr {
            // Inner should be Option<i32> but we can't easily test this
            // without exposing more internals
        } else {
            panic!("Expected array type");
        }

        // Check option_vec - Option<Vec<String>>
        let opt_vec = &obj_schema.fields[&eure_schema::ObjectKey::from("option_vec")];
        assert!(opt_vec.optional);
        if let Type::Array(inner) = &opt_vec.type_expr {
            assert_eq!(**inner, Type::String);
        } else {
            panic!("Expected array type");
        }
    } else {
        panic!("Expected object schema");
    }
}
