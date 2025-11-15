use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Deserialize, Serialize};

#[test]
fn test_serde_skip() {
    #[derive(Eure, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
        #[serde(skip)]
        _internal_state: String,
        email: Option<String>,
    }

    let schema = User::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // Should have 3 fields (internal_state is skipped)
        assert_eq!(obj_schema.fields.len(), 3);
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("id"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("name"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("email"))
        );
        assert!(
            !obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("internal_state"))
        );
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_serde_flatten() {
    #[derive(Eure, Serialize, Deserialize)]
    struct BaseInfo {
        id: u64,
        created_at: String,
    }

    #[derive(Eure, Serialize, Deserialize)]
    struct User {
        name: String,
        #[serde(flatten)]
        base: BaseInfo,
        email: String,
    }

    let schema = User::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // Should have 4 fields (name, email, and 2 from BaseInfo)
        assert_eq!(obj_schema.fields.len(), 4);
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("name"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("email"))
        );
        // Fields from BaseInfo should be flattened in
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("id"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("created_at"))
        );
        // The base field itself should not appear
        assert!(
            !obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("base"))
        );
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_multiple_flatten() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Timestamps {
        created_at: String,
        updated_at: String,
    }

    #[derive(Eure, Serialize, Deserialize)]
    struct Metadata {
        tags: Vec<String>,
        category: String,
    }

    #[derive(Eure, Serialize, Deserialize)]
    struct Document {
        title: String,
        #[serde(flatten)]
        timestamps: Timestamps,
        #[serde(flatten)]
        metadata: Metadata,
        content: String,
    }

    let schema = Document::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // Should have 6 fields total
        assert_eq!(obj_schema.fields.len(), 6);
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("title"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("content"))
        );
        // From Timestamps
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("created_at"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("updated_at"))
        );
        // From Metadata
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("tags"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("category"))
        );
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_skip_and_flatten_combined() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Base {
        id: u64,
        #[serde(skip)]
        _secret: String,
    }

    #[derive(Eure, Serialize, Deserialize)]
    struct Extended {
        name: String,
        #[serde(flatten)]
        base: Base,
        #[serde(skip)]
        _cache: Option<String>,
    }

    let schema = Extended::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        // Should have 2 fields (name and id from Base, secret and cache are skipped)
        assert_eq!(obj_schema.fields.len(), 2);
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("name"))
        );
        assert!(
            obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("id"))
        );
        assert!(
            !obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("secret"))
        );
        assert!(
            !obj_schema
                .fields
                .contains_key(&eure_schema::ObjectKey::from("cache"))
        );
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_description_attribute() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Config {
        #[eure(description = "The server hostname or IP address")]
        host: String,
        #[eure(
            description = "The port number to listen on",
            range(min = 1.0, max = 65535.0)
        )]
        port: u16,
        #[eure(description = "Enable debug mode")]
        debug: bool,
    }

    let schema = Config::eure_schema();

    if let Type::Object(obj_schema) = &schema.type_expr {
        let host_field = &obj_schema.fields[&eure_schema::ObjectKey::from("host")];
        assert_eq!(
            host_field.description,
            Some("The server hostname or IP address".to_string())
        );

        let port_field = &obj_schema.fields[&eure_schema::ObjectKey::from("port")];
        assert_eq!(
            port_field.description,
            Some("The port number to listen on".to_string())
        );
        assert_eq!(
            port_field.constraints.range,
            Some((Some(1.0), Some(65535.0)))
        );

        let debug_field = &obj_schema.fields[&eure_schema::ObjectKey::from("debug")];
        assert_eq!(
            debug_field.description,
            Some("Enable debug mode".to_string())
        );
    } else {
        panic!("Expected object schema");
    }
}
