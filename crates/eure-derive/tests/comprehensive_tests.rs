use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type, VariantRepr, RenameRule};
use serde::{Serialize, Deserialize};

#[test]
fn test_simple_struct() {
    #[derive(Eure, Serialize, Deserialize)]
    struct User {
        name: String,
        age: u32,
    }
    
    let schema = User::eure_schema();
    assert_eq!(User::type_name(), Some("User"));
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        assert_eq!(obj_schema.fields.len(), 2);
        assert!(obj_schema.fields.contains_key("name"));
        assert!(obj_schema.fields.contains_key("age"));
        
        let name_field = &obj_schema.fields["name"];
        assert_eq!(name_field.type_expr, Type::String);
        assert!(!name_field.optional);
        
        let age_field = &obj_schema.fields["age"];
        assert_eq!(age_field.type_expr, Type::Number);
        assert!(!age_field.optional);
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_optional_fields() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Profile {
        username: String,
        bio: Option<String>,
        age: Option<u32>,
    }
    
    let schema = Profile::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        let username_field = &obj_schema.fields["username"];
        assert!(!username_field.optional);
        
        let bio_field = &obj_schema.fields["bio"];
        assert!(bio_field.optional);
        assert_eq!(bio_field.type_expr, Type::String);
        
        let age_field = &obj_schema.fields["age"];
        assert!(age_field.optional);
        assert_eq!(age_field.type_expr, Type::Number);
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_newtype_struct() {
    #[derive(Eure, Serialize, Deserialize)]
    struct UserId(String);
    
    let schema = UserId::eure_schema();
    assert_eq!(schema.type_expr, Type::String);
}

#[test]
fn test_unit_struct() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Empty;
    
    let schema = Empty::eure_schema();
    assert_eq!(schema.type_expr, Type::Null);
}

#[test]
fn test_simple_enum() {
    #[derive(Eure, Serialize, Deserialize)]
    enum Status {
        Active,
        Inactive,
        Pending,
    }
    
    let schema = Status::eure_schema();
    
    if let Type::Variants(variant_schema) = &schema.type_expr {
        assert_eq!(variant_schema.variants.len(), 3);
        assert!(variant_schema.variants.contains_key("Active"));
        assert!(variant_schema.variants.contains_key("Inactive"));
        assert!(variant_schema.variants.contains_key("Pending"));
        assert_eq!(variant_schema.representation, VariantRepr::Tagged);
    } else {
        panic!("Expected variants schema");
    }
}

#[test]
fn test_enum_with_data() {
    #[derive(Eure, Serialize, Deserialize)]
    enum Message {
        Text(String),
        Number(i32),
        Struct { id: u64, content: String },
    }
    
    let schema = Message::eure_schema();
    
    if let Type::Variants(variant_schema) = &schema.type_expr {
        assert_eq!(variant_schema.variants.len(), 3);
        
        // Check Text variant
        let text_variant = &variant_schema.variants["Text"];
        assert!(text_variant.fields.contains_key("0"));
        assert_eq!(text_variant.fields["0"].type_expr, Type::String);
        
        // Check Struct variant
        let struct_variant = &variant_schema.variants["Struct"];
        assert!(struct_variant.fields.contains_key("id"));
        assert!(struct_variant.fields.contains_key("content"));
        assert_eq!(struct_variant.fields["id"].type_expr, Type::Number);
        assert_eq!(struct_variant.fields["content"].type_expr, Type::String);
    } else {
        panic!("Expected variants schema");
    }
}

#[test]
fn test_nested_types() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Address {
        street: String,
        city: String,
        zip: String,
    }
    
    #[derive(Eure, Serialize, Deserialize)]
    struct Person {
        name: String,
        addresses: Vec<Address>,
    }
    
    let schema = Person::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        let addresses_field = &obj_schema.fields["addresses"];
        
        if let Type::Array(inner) = &addresses_field.type_expr {
            // With recursive type detection, Address is now a TypeRef
            if let Type::TypeRef(type_name) = &**inner {
                assert_eq!(type_name, "Address");
            } else {
                panic!("Expected TypeRef for Address, got: {:?}", inner);
            }
        } else {
            panic!("Expected array type for addresses");
        }
        
        // Also verify that Address has a proper schema when called directly
        let address_schema = Address::eure_schema();
        if let Type::Object(addr_obj) = &address_schema.type_expr {
            assert_eq!(addr_obj.fields.len(), 3);
            assert!(addr_obj.fields.contains_key("street"));
            assert!(addr_obj.fields.contains_key("city"));
            assert!(addr_obj.fields.contains_key("zip"));
        } else {
            panic!("Expected object schema for Address::eure_schema()");
        }
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_generic_struct() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Container<T> {
        value: T,
        metadata: String,
    }
    
    let schema = Container::<String>::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        assert_eq!(obj_schema.fields["value"].type_expr, Type::String);
        assert_eq!(obj_schema.fields["metadata"].type_expr, Type::String);
    } else {
        panic!("Expected object schema");
    }
}

// Tests with serde attributes

#[test]
fn test_serde_rename() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Config {
        #[serde(rename = "serverHost")]
        server_host: String,
        #[serde(rename = "serverPort")]
        server_port: u16,
    }
    
    let schema = Config::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        assert!(obj_schema.fields.contains_key("serverHost"));
        assert!(obj_schema.fields.contains_key("serverPort"));
        assert!(!obj_schema.fields.contains_key("server_host"));
        assert!(!obj_schema.fields.contains_key("server_port"));
        
        // Check that rename was captured in serde options
        let host_field = &obj_schema.fields["serverHost"];
        assert_eq!(host_field.serde.rename, Some("serverHost".to_string()));
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_serde_rename_all() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ApiResponse {
        status_code: u16,
        response_body: String,
        error_message: Option<String>,
    }
    
    let schema = ApiResponse::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        assert!(obj_schema.fields.contains_key("statusCode"));
        assert!(obj_schema.fields.contains_key("responseBody"));
        assert!(obj_schema.fields.contains_key("errorMessage"));
        
        // Check serde options were captured
        assert_eq!(schema.serde.rename_all, Some(RenameRule::CamelCase));
    } else {
        panic!("Expected object schema");
    }
}

#[test] 
fn test_untagged_enum() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Value {
        String(String),
        Number(f64),
        Boolean(bool),
    }
    
    let schema = Value::eure_schema();
    
    if let Type::Variants(variant_schema) = &schema.type_expr {
        assert_eq!(variant_schema.representation, VariantRepr::Untagged);
    } else {
        panic!("Expected variants schema");
    }
}

// Test internally tagged enum with different syntax
#[test]
fn test_internally_tagged_enum_simple() {
    #[derive(Eure, Serialize, Deserialize)]
    enum Action {
        Create { id: String },
        Update { id: String, data: String },
        Delete { id: String },
    }
    
    let schema = Action::eure_schema();
    
    if let Type::Variants(variant_schema) = &schema.type_expr {
        // Default is Tagged
        assert_eq!(variant_schema.representation, VariantRepr::Tagged);
        assert_eq!(variant_schema.variants.len(), 3);
    } else {
        panic!("Expected variants schema");
    }
}

#[test]
fn test_collections() {
    use std::collections::{HashMap, HashSet};
    
    #[derive(Eure, Serialize, Deserialize)]
    struct Collections {
        list: Vec<String>,
        set: HashSet<i32>,
        map: HashMap<String, bool>,
    }
    
    let schema = Collections::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        // Check Vec
        let list_field = &obj_schema.fields["list"];
        if let Type::Array(inner) = &list_field.type_expr {
            assert_eq!(**inner, Type::String);
        } else {
            panic!("Expected array type for list");
        }
        
        // Check HashSet
        let set_field = &obj_schema.fields["set"];
        if let Type::Array(inner) = &set_field.type_expr {
            assert_eq!(**inner, Type::Number);
            assert_eq!(set_field.constraints.unique, Some(true));
        } else {
            panic!("Expected array type for set");
        }
        
        // Check HashMap
        let map_field = &obj_schema.fields["map"];
        if let Type::Object(map_schema) = &map_field.type_expr {
            assert!(map_schema.fields.is_empty());
            assert!(map_schema.additional_properties.is_some());
            if let Some(additional) = &map_schema.additional_properties {
                assert_eq!(**additional, Type::Boolean);
            }
        } else {
            panic!("Expected object type for map");
        }
    } else {
        panic!("Expected object schema");
    }
}