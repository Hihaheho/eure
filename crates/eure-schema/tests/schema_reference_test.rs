//! Integration tests for $schema references and external schema validation

use eure_schema::{extract_schema, validate_self_describing, validate_with_schema};
use eure_parol::parse;
use std::fs;

#[test]
fn test_schema_reference_extraction() {
    let input = r#"
$schema = "./example.schema.eure"

$eure {
  data-model: eure
}

@ script
id = "test-id"
description = "A test script"
"#;

    let tree = parse(input).expect("Failed to parse");
    let extracted = extract_schema(input, &tree);

    // Should extract the $schema reference
    assert_eq!(
        extracted.document_schema.schema_ref,
        Some("./example.schema.eure".to_string())
    );

    // Should not be a pure schema (has data content)
    assert!(!extracted.is_pure_schema);
}

#[test]
fn test_pure_schema_validation() {
    let schema_input = r#"
# Schema definitions for scripts

# Root fields
$eure.$type = .object
$eure.data-model.$type = .path

script.$type = .object
script.id.$type = .string
script.description.$type = .string
script.actions.$type = .array
script.actions.$array = .$variants.action

# Define variant types for actions
@ $variants.action.set-text {
  speaker.$type = .string
  lines.$type = .array
  lines.$array = .string
  code1.$type = .code.rust
  code1.$optional = true
  code2.$type = .code.rust
  code2.$optional = true
}

@ $variants.action.set-choices {
  description.$type = .string
  choice.$type = .array
  choice.$array = .object
}

# Type for choice items
@ $variants.action.set-choices.choice[] {
  text.$type = .string
  value.$type = .string
}
"#;

    let tree = parse(schema_input).expect("Failed to parse schema");
    let extracted = extract_schema(schema_input, &tree);

    // Should be identified as a pure schema
    assert!(extracted.is_pure_schema, "Schema should be pure");
    
    // Should have no inline validation errors
    let validation_result = validate_self_describing(schema_input, &tree);
    assert_eq!(validation_result.errors.len(), 0, "Pure schema should have no validation errors");
}

#[test]
fn test_document_with_variant_validation() {
    // First, create the schema
    let schema_input = r#"
script.$type = .object
script.actions.$type = .array
script.actions.$array = .$variants.action

@ $variants.action.set-text {
  speaker.$type = .string
  lines.$type = .array
  lines.$array = .string
}

@ $variants.action.set-choices {
  description.$type = .string
  choice.$type = .array
  choice.$array = .object
}

@ $variants.action.set-choices.choice[] {
  text.$type = .string
  value.$type = .string
}
"#;

    let schema_tree = parse(schema_input).expect("Failed to parse schema");
    let schema = extract_schema(schema_input, &schema_tree).document_schema;

    // Now create a document that uses the schema
    let doc_input = r#"
@ script
@ script.actions[]
$variant: set-text
speaker = "Alice"
lines = ["Hello", "World"]

@ script.actions[]
$variant: set-choices
description = "Choose an option"

@ script.actions[].choice[]
text = "Option A"
value = "a"

@ script.actions[].choice[]
text = "Option B"
value = "b"
"#;

    let doc_tree = parse(doc_input).expect("Failed to parse document");
    let errors = validate_with_schema(doc_input, &doc_tree, schema);

    // Should validate successfully
    assert_eq!(errors.len(), 0, "Document should validate against schema");
}

#[test]
fn test_document_with_missing_required_field() {
    let schema_input = r#"
item.$type = .object
item.id.$type = .string
item.name.$type = .string
item.description.$type = .string
item.description.$optional = true
"#;

    let schema_tree = parse(schema_input).expect("Failed to parse schema");
    let schema = extract_schema(schema_input, &schema_tree).document_schema;

    let doc_input = r#"
@ item
id = "123"
# Missing required field 'name'
description = "An item without a name"
"#;

    let doc_tree = parse(doc_input).expect("Failed to parse document");
    let errors = validate_with_schema(doc_input, &doc_tree, schema);

    // Should have validation error for missing field
    assert_eq!(errors.len(), 1, "Should have one validation error");
    
    match &errors[0].kind {
        eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. } => {
            assert_eq!(field, "name");
        }
        _ => panic!("Expected RequiredFieldMissing error"),
    }
}

#[test]
fn test_complex_variant_with_nested_arrays() {
    // This tests the exact structure from example.eure using the concise syntax
    let schema_input = r#"
@ $types.Action {
  @ $variants.set-text {
    speaker = .string
    lines.$array = .string
    code1 = .code.rust
    code1.$optional = true
    code2 = .code.rust
    code2.$optional = true
  }
  @ $variants.set-choices {
    description = .string
    choice.$array = .object
  }
  @ $variants.set-choices.choice[] {
    text = .string
    value = .string
  }
}

@ script
id = .string
description = .string
description.$optional = true
actions.$array = .Action
"#;

    let schema_tree = parse(schema_input).expect("Failed to parse schema");
    let schema = extract_schema(schema_input, &schema_tree).document_schema;

    let doc_input = r#"
@ script
id = "aaa"
description = "aaa"

@ script.actions[]
$variant: set-text
speaker = "ryo"
lines = ["aaa", "bbb"]
code1 = rust`let a = 1;`
code2 = ```rust
fn main() {
  println!("Hello, world!");
}
```

@ script.actions[]
$variant: set-choices
description = "aaa"

@ script.actions[].choice[]
text = "aaa"
value = "aaa"

@ script.actions[].choice[]
text = "bbb"
value = "bbb"

# Valid multiple sections by using a explicit block
@ script.actions[] {
  $variant: set-choices

  @ choice[]
  text = "aaa"
  value = "aaa"

  @ choice[]
  text = "bbb"
  value = "bbb"
}

@ script.actions[]
$variant: set-text
speaker = "ryo"
lines = [
  "long string item in array",
  "with trailing comma",
]
"#;

    let doc_tree = parse(doc_input).expect("Failed to parse document");
    let errors = validate_with_schema(doc_input, &doc_tree, schema);

    // Document should validate successfully
    assert_eq!(
        errors.len(), 
        0, 
        "Document should validate against schema. Errors: {:?}", 
        errors
    );
}

#[test]
fn test_variant_type_mismatch() {
    let schema_input = r#"
items.$type = .array
items.$array = .$variants.item

@ $variants.item.type-a {
  name.$type = .string
  count.$type = .number
}

@ $variants.item.type-b {
  label.$type = .string
  enabled.$type = .boolean
}
"#;

    let schema_tree = parse(schema_input).expect("Failed to parse schema");
    let schema = extract_schema(schema_input, &schema_tree).document_schema;

    let doc_input = r#"
@ items[]
$variant: type-a
name = "Item A"
count = "not a number"  # Type error: string instead of number

@ items[]
$variant: type-b
label = "Item B"
enabled = "yes"  # Type error: string instead of boolean
"#;

    let doc_tree = parse(doc_input).expect("Failed to parse document");
    let errors = validate_with_schema(doc_input, &doc_tree, schema);

    // Should have type mismatch errors
    assert_eq!(errors.len(), 2, "Should have two type mismatch errors");
    
    for error in &errors {
        match &error.kind {
            eure_schema::ValidationErrorKind::TypeMismatch { expected, actual } => {
                assert!(
                    (expected == "number" && actual == "string") ||
                    (expected == "boolean" && actual == "string"),
                    "Unexpected type mismatch: expected={}, actual={}",
                    expected, actual
                );
            }
            _ => panic!("Expected TypeMismatch error, got: {:?}", error.kind),
        }
    }
}

#[test]
fn test_self_describing_with_schema_ref() {
    // Test a self-describing document that also has a $schema reference
    let input = r#"
$schema = "./my-schema.eure"

# Inline schema constraints
name.$type = .string
age.$type = .number
age.$range = [0, 150]

# Data
name = "John"
age = 30
"#;

    let tree = parse(input).expect("Failed to parse");
    let validation_result = validate_self_describing(input, &tree);

    // Should extract schema reference
    assert_eq!(
        validation_result.schema.document_schema.schema_ref,
        Some("./my-schema.eure".to_string())
    );

    // Should validate successfully with inline constraints
    assert_eq!(validation_result.errors.len(), 0);

    // Should have extracted the inline schema
    assert!(validation_result.schema.document_schema.root.fields.contains_key("name"));
    assert!(validation_result.schema.document_schema.root.fields.contains_key("age"));
}

#[test]
fn test_schema_file_temporary_integration() {
    // Create temporary files to test file-based schema loading
    let temp_dir = std::env::temp_dir().join("eure_schema_test");
    let _ = fs::create_dir_all(&temp_dir);
    
    let schema_path = temp_dir.join("test.schema.eure");
    let schema_content = r#"
person.$type = .object
person.name.$type = .string
person.age.$type = .number
person.email.$type = .typed-string.email
person.email.$optional = true
"#;
    
    fs::write(&schema_path, schema_content).expect("Failed to write schema file");
    
    // Parse and extract schema
    let schema_tree = parse(schema_content).expect("Failed to parse schema");
    let extracted = extract_schema(schema_content, &schema_tree);
    
    assert!(extracted.is_pure_schema, "Should be a pure schema");
    assert!(extracted.document_schema.root.fields.contains_key("person"));
    
    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}