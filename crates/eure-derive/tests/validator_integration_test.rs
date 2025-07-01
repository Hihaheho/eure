//! Integration tests that validate EURE documents against generated schemas

use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type, DocumentSchema, ObjectSchema, FieldSchema, validate_with_schema, has_errors};
use serde::{Serialize, Deserialize};

// Type alias to simplify complex type signature
type TypeDefinition = (&'static str, fn() -> FieldSchema);

/// Helper function to validate a EURE document string against a schema
fn validate_document<T: ToEureSchema>(document: &str) -> Result<(), String> {
    validate_document_with_types::<T>(document, &[])
}

/// Helper function to validate a EURE document string against a schema with type definitions
fn validate_document_with_types<T: ToEureSchema>(
    document: &str, 
    type_definitions: &[TypeDefinition]
) -> Result<(), String> {
    // Parse the EURE document using the high-level parse function
    let parsed = match eure_parol::parse(document) {
        Ok(cst) => cst,
        Err(e) => return Err(format!("Parse error: {e:?}")),
    };
    
    // Get the schema
    let schema = T::eure_schema();
    
    // Create a document schema with the type schema as root
    let mut doc_schema = DocumentSchema::default();
    
    // Register type definitions
    for (name, schema_fn) in type_definitions {
        doc_schema.types.insert(name.to_string(), schema_fn());
    }
    
    // Add the generated schema to the document schema  
    match &schema.type_expr {
        Type::Object(obj_schema) => {
            doc_schema.root = obj_schema.clone();
        }
        Type::Variants(variant_schema) => {
            // For variant types at the root, we use cascade type
            // The validator will handle variant validation when it sees $variant field
            
            // Empty root schema - all fields come from cascade type
            doc_schema.root = ObjectSchema {
                fields: indexmap::IndexMap::new(),
                additional_properties: None,
            };
            
            // Set cascade type to the variant schema
            doc_schema.cascade_type = Some(Type::Variants(variant_schema.clone()));
        }
        _ => {
            // For other types, wrap in a single field
            let mut root = ObjectSchema::default();
            root.fields.insert("value".to_string(), schema);
            doc_schema.root = root;
        }
    }
    
    
    // Validate the document
    let errors = validate_with_schema(document, &parsed, doc_schema.clone());
    
    if !has_errors(&errors) {
        Ok(())
    } else {
        let error_messages: Vec<String> = errors.iter()
            .filter(|e| e.severity == eure_schema::Severity::Error)
            .map(|e| format!("{:?}", e.kind))
            .collect();
        eprintln!("Document schema root fields: {:?}", doc_schema.root.fields.keys().collect::<Vec<_>>());
        eprintln!("Document schema types: {:?}", doc_schema.types.keys().collect::<Vec<_>>());
        eprintln!("Document schema cascade type: {:?}", doc_schema.cascade_type);
        eprintln!("Validation errors: {:?}", error_messages);
        eprintln!("Full errors: {:#?}", errors);
        Err(format!("Validation errors: {}", error_messages.join(", ")))
    }
}

#[test]
fn test_simple_struct_validation() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Person {
        name: String,
        age: u32,
        email: Option<String>,
    }
    
    // Valid document
    let valid_doc = r#"
name = "John Doe"
age = 30
email = "john@example.com"
"#;
    
    assert!(validate_document::<Person>(valid_doc).is_ok());
    
    // Missing required field
    let missing_field = r#"
name = "John Doe"
# age is missing
email = "john@example.com"
"#;
    
    assert!(validate_document::<Person>(missing_field).is_err());
    
    // Wrong type
    let wrong_type = r#"
name = "John Doe"
age = "thirty"  # Should be a number
email = "john@example.com"
"#;
    
    assert!(validate_document::<Person>(wrong_type).is_err());
}

#[test]
fn test_optional_fields_validation() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Config {
        host: String,
        port: u16,
        debug: Option<bool>,
        timeout: Option<u32>,
    }
    
    // All fields present
    let all_fields = r#"
host = "localhost"
port = 8080
debug = true
timeout = 30
"#;
    
    assert!(validate_document::<Config>(all_fields).is_ok());
    
    // Optional fields missing
    let minimal = r#"
host = "localhost"
port = 8080
"#;
    
    assert!(validate_document::<Config>(minimal).is_ok());
}

#[test]
fn test_constraints_validation() {
    #[derive(Eure, Serialize, Deserialize)]
    struct User {
        #[eure(length(min = 3, max = 20), pattern = "^[a-zA-Z0-9_]+$")]
        username: String,
        #[eure(range(min = 18.0, max = 120.0))]
        age: u8,
        #[eure(min_items = 1, max_items = 5)]
        tags: Vec<String>,
    }
    
    // Valid document
    let valid = r#"
username = "john_doe"
age = 25
@ tags[0] = "developer"
@ tags[1] = "rust"
"#;
    
    match validate_document::<User>(valid) {
        Ok(_) => {},
        Err(e) => panic!("Validation failed: {e}"),
    }
    
    // Username too short
    let short_username = r#"
username = "jo"
age = 25
@ tags[0] = "developer"
"#;
    
    let result = validate_document::<User>(short_username);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    println!("Short username error: {err_msg}");
    assert!(err_msg.contains("length") || err_msg.contains("StringLengthViolation"));
    
    // Invalid pattern
    let invalid_pattern = r#"
username = "john-doe!"
age = 25
@ tags[0] = "developer"
"#;
    
    let result = validate_document::<User>(invalid_pattern);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("pattern") || err_msg.contains("StringPatternViolation"));
    
    // Age out of range
    let invalid_age = r#"
username = "john_doe"
age = 150
@ tags[0] = "developer"
"#;
    
    let result = validate_document::<User>(invalid_age);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("range") || err_msg.contains("NumberRangeViolation"));
    
    // Too many tags
    let too_many_tags = r#"
username = "john_doe"
age = 25
@ tags[0] = "one"
@ tags[1] = "two"
@ tags[2] = "three"
@ tags[3] = "four"
@ tags[4] = "five"
@ tags[5] = "six"
"#;
    
    let result = validate_document::<User>(too_many_tags);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("max_items") || err_msg.contains("ArrayLengthViolation"));
}

#[test]
fn test_nested_struct_validation() {
    #[derive(Eure, Serialize, Deserialize)]
    struct Address {
        street: String,
        city: String,
        zip: String,
    }
    
    #[derive(Eure, Serialize, Deserialize)]
    struct Company {
        name: String,
        address: Address,
        employees: Vec<Person>,
    }
    
    #[derive(Eure, Serialize, Deserialize)]
    struct Person {
        name: String,
        role: String,
    }
    
    // Valid nested document
    let valid_nested = r#"
name = "Acme Corp"

@ address {
    street = "123 Main St"
    city = "Springfield"
    zip = "12345"
}

@ employees[0] {
    name = "John Doe"
    role = "CEO"
}

@ employees[1] {
    name = "Jane Smith"
    role = "CTO"
}
"#;
    
    // Register the type definitions that Company references
    let type_defs: &[TypeDefinition] = &[
        ("Address", || Address::eure_schema()),
        ("Person", || Person::eure_schema()),
    ];
    
    
    match validate_document_with_types::<Company>(valid_nested, type_defs) {
        Ok(_) => {},
        Err(e) => panic!("Nested validation failed: {e}"),
    }
    
    // Missing nested field
    let missing_nested = r#"
name = "Acme Corp"

@ address {
    street = "123 Main St"
    # city is missing
    zip = "12345"
}

@ employees[0] {
    name = "John Doe"
    role = "CEO"
}
"#;
    
    assert!(validate_document_with_types::<Company>(missing_nested, type_defs).is_err());
}

#[test]
fn test_enum_validation() {
    #[derive(Eure, Serialize, Deserialize)]
    enum Status {
        Success { message: String },
        Error { code: u32, message: String },
        Pending,
    }
    
    // Valid success variant
    let success = r#"
$variant = "Success"
message = "Operation completed"
"#;
    
    assert!(validate_document::<Status>(success).is_ok());
    
    // Valid error variant
    let error = r#"
$variant = "Error"
code = 404
message = "Not found"
"#;
    
    assert!(validate_document::<Status>(error).is_ok());
    
    // Valid pending variant
    let pending = r#"
$variant = "Pending"
"#;
    
    assert!(validate_document::<Status>(pending).is_ok());
    
    // Invalid variant
    let invalid_variant = r#"
$variant = "Unknown"
message = "This variant doesn't exist"
"#;
    
    assert!(validate_document::<Status>(invalid_variant).is_err());
    
    // Missing field in variant
    let missing_field = r#"
$variant = "Error"
# code is missing
message = "Not found"
"#;
    
    assert!(validate_document::<Status>(missing_field).is_err());
}

#[test]
fn test_serde_rename_validation() {
    #[derive(Eure, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ApiResponse {
        response_code: u32,
        response_message: String,
        #[serde(rename = "data")]
        response_data: Option<String>,
    }
    
    // Valid with renamed fields
    let valid = r#"
responseCode = 200
responseMessage = "OK"
data = "Some data"
"#;
    
    assert!(validate_document::<ApiResponse>(valid).is_ok());
    
    // Using original field names should fail
    let original_names = r#"
response_code = 200
response_message = "OK"
response_data = "Some data"
"#;
    
    assert!(validate_document::<ApiResponse>(original_names).is_err());
}

#[test]
fn test_array_validation() {
    #[derive(Eure, Serialize, Deserialize)]
    struct TodoList {
        title: String,
        #[eure(min_items = 1, unique = true)]
        items: Vec<String>,
    }
    
    // Valid array
    let valid = r#"
title = "Shopping List"
@ items[0] = "Milk"
@ items[1] = "Bread"
@ items[2] = "Eggs"
"#;
    
    assert!(validate_document::<TodoList>(valid).is_ok());
    
    // Empty array (violates min_items)
    let empty = r#"
title = "Shopping List"
items = []
"#;
    
    assert!(validate_document::<TodoList>(empty).is_err());
    
}

#[test]
fn test_complex_validation_scenario() {
    #[derive(Eure, Serialize, Deserialize)]
    struct DatabaseConfig {
        #[eure(pattern = "^[a-zA-Z][a-zA-Z0-9_]*$")]
        name: String,
        #[eure(range(min = 1024.0, max = 65535.0))]
        port: u16,
        credentials: Credentials,
        #[eure(min_items = 1)]
        replicas: Vec<ReplicaConfig>,
    }
    
    #[derive(Eure, Serialize, Deserialize)]
    struct Credentials {
        #[eure(length(min = 8, max = 128))]
        username: String,
        #[eure(length(min = 12, max = 128), pattern = "^[A-Za-z0-9]+$")]
        password: String,
    }
    
    #[derive(Eure, Serialize, Deserialize)]
    struct ReplicaConfig {
        host: String,
        port: u16,
        priority: u8,
    }
    
    // Register the type definitions
    let type_defs: &[TypeDefinition] = &[
        ("Credentials", || Credentials::eure_schema()),
        ("ReplicaConfig", || ReplicaConfig::eure_schema()),
    ];
    
    // Valid complex configuration
    let valid = r#"
name = "production_db"
port = 5432

@ credentials {
    username = "db_admin_user"
    password = "SecurePass123"
}

@ replicas[0] {
    host = "replica1.example.com"
    port = 5433
    priority = 1
}

@ replicas[1] {
    host = "replica2.example.com"
    port = 5434
    priority = 2
}
"#;
    
    assert!(validate_document_with_types::<DatabaseConfig>(valid, type_defs).is_ok());
    
    // Invalid database name
    let invalid_name = r#"
name = "123_invalid"  # Starts with number
port = 5432

@ credentials {
    username = "db_admin_user"
    password = "SecurePass123"
}

@ replicas[0] {
    host = "replica1.example.com"
    port = 5433
    priority = 1
}
"#;
    
    assert!(validate_document_with_types::<DatabaseConfig>(invalid_name, type_defs).is_err());
    
    // Weak password
    let weak_password = r#"
name = "production_db"
port = 5432

@ credentials {
    username = "db_admin_user"
    password = "weak-password!"  # Contains special characters
}

@ replicas[0] {
    host = "replica1.example.com"
    port = 5433
    priority = 1
}
"#;
    
    assert!(validate_document_with_types::<DatabaseConfig>(weak_password, type_defs).is_err());
}