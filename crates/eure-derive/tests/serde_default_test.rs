use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Deserialize, Serialize};

#[test]
fn test_field_level_default() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Config {
        host: String,
        #[serde(default)]
        port: u16,
        #[serde(default)]
        debug: bool,
        #[serde(default)]
        timeout: u64,
    }

    let schema = Config::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // host is required (not optional)
        assert!(!obj_schema.fields[&eure_schema::ObjectKey::from("host")].optional);

        // Fields with serde(default) should be optional
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("port")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("debug")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("timeout")].optional);
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_container_level_default() {
    #[derive(Default, Eure, Serialize, Deserialize)]
    #[serde(default)]
    struct Settings {
        name: String,
        value: i32,
        enabled: bool,
    }

    let schema = Settings::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // All fields should be optional when container has #[serde(default)]
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("name")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("value")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("enabled")].optional);
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_mixed_default_and_option() {
    #[derive(Eure, Serialize, Deserialize)]
    struct User {
        id: u64,
        #[serde(default)]
        name: String,
        email: Option<String>,
        #[serde(default)]
        age: Option<u32>,
    }

    let schema = User::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // id is required
        assert!(!obj_schema.fields[&eure_schema::ObjectKey::from("id")].optional);

        // name has default, so it's optional
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("name")].optional);
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("name")].type_expr,
            Type::String
        );

        // email is Option, so it's optional
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("email")].optional);
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("email")].type_expr,
            Type::String
        );

        // age is both Option and has default, still optional
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("age")].optional);
        assert_eq!(
            obj_schema.fields[&eure_schema::ObjectKey::from("age")].type_expr,
            Type::Number
        );
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_container_default_with_skip() {
    #[derive(Default, Eure, Serialize, Deserialize)]
    #[serde(default)]
    struct Data {
        value: String,
        count: u32,
        #[serde(skip)]
        internal: String,
    }

    let schema = Data::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // Container default makes all fields optional
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("value")].optional);
        assert!(obj_schema.fields[&eure_schema::ObjectKey::from("count")].optional);

        // Skipped field shouldn't appear
        assert!(
            !obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("internal"))
        );
    } else {
        panic!("Expected object schema");
    }
}
