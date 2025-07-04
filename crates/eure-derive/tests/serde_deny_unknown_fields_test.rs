use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Serialize, Deserialize};

#[test]
fn test_deny_unknown_fields() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct StrictConfig {
        host: String,
        port: u16,
    }
    
    let schema = StrictConfig::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        // With deny_unknown_fields, additional_properties should be None
        assert_eq!(obj_schema.additional_properties, None);
        assert_eq!(obj_schema.fields.len(), 2);
    } else {
        panic!("Expected object schema");
    }
}

#[test]
fn test_without_deny_unknown_fields() {
    #[derive(Eure, Serialize, Deserialize)]
    struct FlexibleConfig {
        host: String,
        port: u16,
    }
    
    let schema = FlexibleConfig::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        // Without deny_unknown_fields, we still set additional_properties to None
        // This is the current default behavior
        assert_eq!(obj_schema.additional_properties, None);
        assert_eq!(obj_schema.fields.len(), 2);
    } else {
        panic!("Expected object schema");
    }
}

#[test] 
fn test_deny_unknown_fields_with_flatten() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Base {
        id: u64,
    }
    
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Extended {
        name: String,
        #[serde(flatten)]
        base: Base,
    }
    
    let schema = Extended::eure_schema();
    
    if let Type::Object(obj_schema) = &schema.type_expr {
        // Note: serde doesn't support deny_unknown_fields with flatten,
        // but our schema generation still works
        assert_eq!(obj_schema.additional_properties, None);
        assert_eq!(obj_schema.fields.len(), 2); // name + id from base
        assert!(obj_schema.fields.contains_key(&eure_schema::KeyCmpValue::from("name")));
        assert!(obj_schema.fields.contains_key(&eure_schema::KeyCmpValue::from("id")));
    } else {
        panic!("Expected object schema");
    }
}