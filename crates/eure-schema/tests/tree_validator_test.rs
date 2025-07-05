//! Integration tests for tree-based validation

use eure_schema::{extract_schema_from_value, validate_with_tree};

#[test]
fn test_example_eure_validation() {
    // Load the schema from example.schema.eure
    let schema_input = include_str!("../../../example.schema.eure");
    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema from example.schema.eure");
    
    // Load the example document
    let doc_input = include_str!("../../../example.eure");
    
    // Parse the document
    let tree = eure_parol::parse(doc_input).expect("Failed to parse example.eure");
    
    // Validate the document against the schema
    let errors = validate_with_tree(doc_input, extracted.document_schema, &tree)
        .expect("Failed to validate");
    
    // Print all errors for debugging
    println!("Found {} validation errors:", errors.len());
    for (i, error) in errors.iter().enumerate() {
        println!("  {}: {:?}", i + 1, error);
    }
    
    // Check for false positive "Required field 'script' is missing" errors
    let false_positive_count = errors.iter()
        .filter(|e| matches!(&e.kind, 
            eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. } 
            if matches!(field, eure_schema::KeyCmpValue::String(s) if s == "script")
        ))
        .count();
    
    // The document has a valid 'script' section, so there should be NO such errors
    assert_eq!(false_positive_count, 0, 
        "Found {false_positive_count} false positive 'Required field script is missing' errors");
    
    // The document should have some valid content
    assert!(doc_input.contains("@ script"), "Document should contain script section");
    
    // There might be other legitimate errors, but not related to missing 'script' field
    println!("Test passed: No false positive 'script missing' errors");
}

#[test]
fn test_nested_section_validation() {
    // Test schema with nested objects
    let schema_input = r#"
@ user {
    @ name {
        @ first
        $type = .string
        
        @ last
        $type = .string
    }
    
    @ age
    $type = .number
}
"#;

    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");
    
    // Document with nested sections
    let doc_input = r#"
@ user
age = 30

@ user.name
first = "John"
last = "Doe"
"#;

    let tree = eure_parol::parse(doc_input).expect("Failed to parse document");
    
    let errors = validate_with_tree(doc_input, extracted.document_schema, &tree)
        .expect("Failed to validate");
    
    // Should not have any "Required field 'user' is missing" errors
    let user_missing_errors = errors.iter()
        .filter(|e| matches!(&e.kind,
            eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. }
            if matches!(field, eure_schema::KeyCmpValue::String(s) if s == "user")
        ))
        .count();
    
    assert_eq!(user_missing_errors, 0, 
        "Should not report 'user' as missing when it exists");
}

#[test]
fn test_actually_missing_required_field() {
    // Test that we still catch genuinely missing required fields
    let schema_input = r#"
@ name
$type = .string

@ age
$type = .number

@ email
$type = .string
$optional = true
"#;

    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");
    
    // Document missing required 'age' field
    let doc_input = r#"
name = "Alice"
email = "alice@example.com"
"#;

    let tree = eure_parol::parse(doc_input).expect("Failed to parse document");
    
    let errors = validate_with_tree(doc_input, extracted.document_schema, &tree)
        .expect("Failed to validate");
    
    // Should have exactly one error about missing 'age' field
    let age_missing_errors = errors.iter()
        .filter(|e| matches!(&e.kind,
            eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. }
            if matches!(field, eure_schema::KeyCmpValue::String(s) if s == "age")
        ))
        .count();
    
    assert_eq!(age_missing_errors, 1, 
        "Should report 'age' as missing exactly once");
}