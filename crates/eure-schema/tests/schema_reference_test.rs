//! Integration tests for $schema references and external schema validation
//!
//! These tests use the new value-based API

use eure_schema::{
    KeyCmpValue, ValidationErrorKind, extract_schema_from_value, validate_and_extract_schema,
    validate_self_describing, validate_with_schema_value,
};

#[test]
fn test_inline_schema_extraction() {
    let input = r#"
$types.Person {
  name = .string
  age = .number
  email = .string
  email.$optional = true
}

people.$array = .$types.Person

# Some actual data
@ people[] {
  name = "Alice"
  age = 30
  email = "alice@example.com"
}

@ people[] {
  name = "Bob"
  age = 25
}
"#;

    let result = extract_schema_from_value(input);
    assert!(result.is_ok(), "Should extract schema successfully");

    let schema = result.unwrap();
    assert!(
        !schema.is_pure_schema,
        "Document contains both schema and data"
    );

    // Verify the schema has the expected Person type
    let person_type_key = KeyCmpValue::String("Person".to_string());
    assert!(
        schema.document_schema.types.contains_key(&person_type_key),
        "Schema should contain Person type"
    );
}

#[test]
fn test_self_describing_validation() {
    let input = r#"
$types.Config {
  host = .string
  port = .number
  debug = .boolean
  debug.$optional = true
}

config.$type = .$types.Config

# Actual config data
config.host = "localhost"
config.port = 8080
config.debug = true
"#;

    let result = validate_self_describing(input);
    assert!(result.is_ok(), "Should validate self-describing document");

    let validation = result.unwrap();
    assert_eq!(
        validation.errors.len(),
        0,
        "Valid document should have no errors, but got: {:?}",
        validation.errors
    );
    assert!(!validation.schema.is_pure_schema, "Document contains data");
}

#[test]
fn test_self_describing_with_errors() {
    let input = r#"
$types.Config {
  host = .string
  port = .number
  debug = .boolean
}

config.$type = .$types.Config

# Invalid config data - missing required field and wrong type
config.host = 123  # Should be string
# Missing required 'port' field
"#;

    let result = validate_self_describing(input);
    assert!(result.is_ok(), "Should parse and validate even with errors");

    let validation = result.unwrap();
    assert!(validation.errors.len() > 0, "Should have validation errors");

    // Check for type mismatch error (actual is reported as "i64" not "number")
    let has_type_mismatch = validation.errors.iter().any(|e| {
        matches!(&e.kind,
            ValidationErrorKind::TypeMismatch { expected, actual }
            if expected == "string" && actual == "i64"
        )
    });
    assert!(
        has_type_mismatch,
        "Should have type mismatch error for host field"
    );

    // Check for missing required field
    let has_missing_field = validation.errors.iter().any(|e| {
        matches!(&e.kind,
            ValidationErrorKind::RequiredFieldMissing { field, .. }
            if matches!(field, KeyCmpValue::String(s) if s == "port")
        )
    });
    assert!(
        has_missing_field,
        "Should have error for missing port field"
    );
}

#[test]
fn test_validate_and_extract_schema() {
    let input = r#"
$types.Task {
  @ $variants.create-file {
    path = .string
    content = .string
  }
  @ $variants.run-command {
    command = .string
    args.$array = .string
    args.$optional = true
  }
}

tasks.$array = .$types.Task

# Test data with variants
@ tasks[] {
  $variant: create-file
  path = "/tmp/test.txt"
  content = "Hello, world!"
}

@ tasks[] {
  $variant: run-command
  command = "echo"
  args = ["Hello", "World"]
}
"#;

    let result = validate_and_extract_schema(input);
    assert!(result.is_ok(), "Should extract and validate successfully");

    let validation = result.unwrap();
    assert_eq!(
        validation.errors.len(),
        0,
        "Valid document should have no errors, but got: {:?}",
        validation.errors
    );

    // Verify schema contains Task type
    let task_type_key = KeyCmpValue::String("Task".to_string());
    assert!(
        validation
            .schema
            .document_schema
            .types
            .contains_key(&task_type_key),
        "Schema should contain Task type"
    );
}

#[test]
fn test_pure_schema_document() {
    let input = r#"
$types.User {
  username = .string
  password = .string
  roles.$array = .string
  roles.$optional = true
}

$types.Post {
  title = .string
  content = .string
  author = .$types.User
  tags.$array = .string
  tags.$optional = true
}

posts.$array = .$types.Post
"#;

    let result = extract_schema_from_value(input);
    assert!(result.is_ok(), "Should extract schema successfully");

    let schema = result.unwrap();
    assert!(
        schema.is_pure_schema,
        "Document should be identified as pure schema"
    );

    // Verify both types are present
    let user_type_key = KeyCmpValue::String("User".to_string());
    let post_type_key = KeyCmpValue::String("Post".to_string());
    assert!(
        schema.document_schema.types.contains_key(&user_type_key),
        "Schema should contain User type"
    );
    assert!(
        schema.document_schema.types.contains_key(&post_type_key),
        "Schema should contain Post type"
    );
}

// Note: Using integer cents instead of float dollars since EURE parser
// doesn't support floating point literals yet
#[test]
fn test_external_schema_validation() {
    // First, extract a schema from a pure schema document
    let schema_doc = r#"
$types.Product {
  name = .string
  price = .number
  description = .string
  description.$optional = true
  in_stock = .boolean
}

products.$array = .$types.Product
"#;

    let schema_result = extract_schema_from_value(schema_doc);
    assert!(schema_result.is_ok(), "Should extract schema");
    let schema = schema_result.unwrap().document_schema;

    // Now validate a separate document against this schema
    let data_doc = r#"
@ products[] {
  name = "Widget"
  price = 1999  # Using cents instead of dollars due to lack of float support
  in_stock = true
}

@ products[] {
  name = "Gadget"
  price = 2999  # Using cents instead of dollars due to lack of float support
  description = "A useful gadget"
  in_stock = false
}
"#;

    let validation_result = validate_with_schema_value(data_doc, schema);
    assert!(validation_result.is_ok(), "Should validate document");

    let errors = validation_result.unwrap();
    assert_eq!(
        errors.len(),
        0,
        "Valid document should have no errors, but got: {:?}",
        errors
    );
}

#[test]
fn test_cascade_type_schema() {
    let input = r#"
# Define a cascade type for all fields at root level
$cascade-type = .string

# These fields will all be strings due to cascade
field1 = "value1"
field2 = "value2"
field3 = "value3"

# Override for specific field with explicit type
field4.$type = .number
field4 = 42
"#;

    let result = validate_self_describing(input);
    assert!(result.is_ok(), "Should validate cascade type document");

    let validation = result.unwrap();
    assert_eq!(
        validation.errors.len(),
        0,
        "Valid document should have no errors, but got: {:?}",
        validation.errors
    );
}

// Note: Using integer milliseconds instead of float seconds since EURE parser
// doesn't support floating point literals yet
#[test]
fn test_array_of_variants() {
    let input = r#"
$types.Action {
  @ $variants.click {
    x = .number
    y = .number
    button = .string
    button.$optional = true
  }
  @ $variants.type {
    text = .string
    field = .string
  }
  @ $variants.wait {
    seconds = .number
  }
}

actions.$array = .$types.Action

# Test data with section syntax
@ actions[] {
  $variant: click
  x = 100
  y = 200
}

@ actions[] {
  $variant: type
  text = "Hello"
  field = "input-field"
}

@ actions[] {
  $variant: wait
  seconds = 2500  # Using milliseconds instead of seconds due to lack of float support
}
"#;

    let result = validate_and_extract_schema(input);
    assert!(result.is_ok(), "Should validate array of variants");

    let validation = result.unwrap();
    assert_eq!(
        validation.errors.len(),
        0,
        "Valid document should have no errors, but got: {:?}",
        validation.errors
    );
}
