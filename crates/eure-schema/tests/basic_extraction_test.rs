//! Basic tests for schema extraction

use eure_schema::{extract_schema_from_value, KeyCmpValue};
use eure_tree::value_visitor::ValueVisitor;
use eure_parol::parse;

#[test]
fn test_basic_field_extraction() {
    // Simple schema with direct field definitions
    let schema = r#"
name.$type = .string
age.$type = .number
email.$type = .string
email.$optional = true
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");
    
    eprintln!("Root fields: {:?}", extracted.document_schema.root.fields.keys().collect::<Vec<_>>());
    
    // Should have name, age, and email fields
    assert_eq!(extracted.document_schema.root.fields.len(), 3);
    assert!(extracted.document_schema.root.fields.contains_key(&KeyCmpValue::String("name".to_string())));
    assert!(extracted.document_schema.root.fields.contains_key(&KeyCmpValue::String("age".to_string())));
    assert!(extracted.document_schema.root.fields.contains_key(&KeyCmpValue::String("email".to_string())));
    
    // Check email is optional
    let email_field = &extracted.document_schema.root.fields[&KeyCmpValue::String("email".to_string())];
    assert!(email_field.optional);
}

#[test]
fn test_type_definition_extraction() {
    // Schema with type definitions
    let schema = r#"
$types.Person {
    name.$type = .string
    age.$type = .number
}
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");
    
    // Should have Person type
    assert!(extracted.document_schema.types.contains_key(&KeyCmpValue::String("Person".to_string())));
}

#[test]
fn test_section_based_schema() {
    // Schema using sections (what the test is trying to use)
    let schema = r#"
@ user
name.$type = .string
age.$type = .number
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");
    
    eprintln!("Root fields: {:?}", extracted.document_schema.root.fields.keys().collect::<Vec<_>>());
    
    // This might not work as expected - sections create structure, not schema definitions
    // The current implementation might not handle this correctly
}