use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Serialize, Deserialize};

#[test]
fn test_string_constraints() {
    #[derive(Eure, Serialize, Deserialize)]
    struct User {
        #[eure(length(min = 3, max = 20), pattern = "^[a-z0-9_]+$")]
        username: String,
        #[eure(pattern = r"^[^@]+@[^@]+\.[^@]+$")]
        email: String,
    }
    
    let schema = User::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        let username_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("username")];
        assert_eq!(username_field.constraints.length, Some((Some(3), Some(20))));
        assert_eq!(username_field.constraints.pattern, Some("^[a-z0-9_]+$".to_string()));
        
        let email_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("email")];
        assert_eq!(email_field.constraints.pattern, Some(r"^[^@]+@[^@]+\.[^@]+$".to_string()));
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_number_constraints() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Product {
        #[eure(range(min = 0.0, max = 1000000.0))]
        price: f64,
        #[eure(range(min = 1.0, max = 9999.0))]
        quantity: u32,
    }
    
    let schema = Product::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        let price_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("price")];
        assert_eq!(price_field.constraints.range, Some((Some(0.0), Some(1000000.0))));
        
        let quantity_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("quantity")];
        assert_eq!(quantity_field.constraints.range, Some((Some(1.0), Some(9999.0))));
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_array_constraints() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Config {
        #[eure(min_items = 1, max_items = 10, unique = true)]
        tags: Vec<String>,
        #[eure(min_items = 0, max_items = 100)]
        items: Vec<i32>,
    }
    
    let schema = Config::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        let tags_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("tags")];
        assert_eq!(tags_field.constraints.min_items, Some(1));
        assert_eq!(tags_field.constraints.max_items, Some(10));
        assert_eq!(tags_field.constraints.unique, Some(true));
        
        let items_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("items")];
        assert_eq!(items_field.constraints.min_items, Some(0));
        assert_eq!(items_field.constraints.max_items, Some(100));
        assert_eq!(items_field.constraints.unique, None);
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_preferences() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Document {
        #[eure(prefer_section = true)]
        metadata: Metadata,
        #[eure(prefer_section = false)]
        simple_data: String,
    }
    
    #[derive(Eure, Serialize, Deserialize)]
    struct Metadata {
        title: String,
        author: String,
    }
    
    let schema = Document::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        let metadata_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("metadata")];
        assert_eq!(metadata_field.preferences.section, Some(true));
        
        let simple_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("simple_data")];
        assert_eq!(simple_field.preferences.section, Some(false));
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_combined_attributes() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Account {
        #[serde(rename = "user_id")]
        #[eure(pattern = "^[A-Z0-9]{8}$")]
        id: String,
        
        #[eure(length(min = 8, max = 128), pattern = r"^(?=.*[A-Za-z])(?=.*\d).+$")]
        password: String,
        
        #[eure(range(min = 0.0, max = 1000000.0))]
        balance: Option<f64>,
    }
    
    let schema = Account::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        // Check id field (renamed)
        assert!(obj_schema.fields.contains_key(&eure_schema::KeyCmpValue::from("user_id")));
        let id_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("user_id")];
        assert_eq!(id_field.constraints.pattern, Some("^[A-Z0-9]{8}$".to_string()));
        assert_eq!(id_field.serde.rename, Some("user_id".to_string()));
        
        // Check password field
        let password_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("password")];
        assert_eq!(password_field.constraints.length, Some((Some(8), Some(128))));
        assert_eq!(password_field.constraints.pattern, Some(r"^(?=.*[A-Za-z])(?=.*\d).+$".to_string()));
        
        // Check balance field (optional)
        let balance_field = &obj_schema.fields[&eure_schema::KeyCmpValue::from("balance")];
        assert!(balance_field.optional);
        assert_eq!(balance_field.constraints.range, Some((Some(0.0), Some(1000000.0))));
    } else {
        panic!("Expected object schema");
    }
}