//! Integration tests for tree-based validation

use eure_schema::{extract_schema_from_value, validate_with_tree};

#[test]
fn test_example_eure_validation() {
    // Use inline schema instead of external file
    let schema_input = r#"
$types.Action {
  @ $variants.set-text {
    speaker = .string
    lines.$array = .string
    code1 = .code.rust
    code2 = .code.rust
  }
  @ $variants.set-choices {
    description = .string
  }
  @ $variants.set-choices.choice.$array {
    text = .string
    value = .string
  }
}

@ script
id.$type = .string
description.$type = .string
description.$optional = true
actions.$array = .$types.Action
"#;
    let extracted = extract_schema_from_value(schema_input)
        .expect("Failed to extract schema");
    
    // Use inline document instead of external file
    let doc_input = r#"
@ script
id = "test-id"
description = "test description"

@ script.actions[]
$variant: set-text
speaker = "alice"
lines = ["hello", "world"]
code1 = rust`let x = 1;`
code2 = ```rust
fn main() {
    println!("Hello!");
}
```

@ script.actions[]
$variant: set-choices
description = "Choose an option"

@ script.actions[].choice[]
text = "Option A"
value = "a"

@ script.actions[].choice[]
text = "Option B"
value = "b"

# Test block syntax too
@ script.actions[] {
  $variant: set-choices
  description = "Another choice"

  @ choice[]
  text = "Option C"
  value = "c"

  @ choice[]
  text = "Option D"
  value = "d"
}
"#;
    
    // Parse the document
    let tree = eure_parol::parse(doc_input).expect("Failed to parse example.eure");
    
    // Validate the document against the schema
    let errors = validate_with_tree(&tree, doc_input, extracted.document_schema)
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
    
    let errors = validate_with_tree(&tree, doc_input, extracted.document_schema)
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
    
    let errors = validate_with_tree(&tree, doc_input, extracted.document_schema)
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