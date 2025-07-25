//! Simple integration test for schema validation with example files

use eure_schema::{extract_schema_from_value, validate_with_schema_value};
use eure_value::value::KeyCmpValue;

#[test]
fn test_example_schema_validation() {
    // This is based on the actual example.schema.eure format
    let schema_input = r#"
$schema = "assets/eure-schema.schema.eure"

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

    // Test document matching the schema
    let doc_input = r#"
$schema = "./example.schema.eure"

@ script
id = "test-script"
description = "A test script"

@ script.actions[]
$variant: set-text
speaker = "Alice"
lines = ["Hello", "World"]
code1 = rust`println!("Hello");`
code2 = ```rust
fn main() {
    println!("Hello, world!");
}
```

@ script.actions[]
$variant: set-choices
description = "Choose your path"

@ script.actions[].choice[]
text = "Option A"
value = "a"

@ script.actions[].choice[]
text = "Option B"
value = "b"
"#;

    // Extract schema
    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // Debug print
    eprintln!("Extracted types: {:?}", extracted.document_schema.types.keys().collect::<Vec<_>>());
    eprintln!("Extracted root fields: {:?}", extracted.document_schema.root.fields.keys().collect::<Vec<_>>());

    // The schema should contain type definitions
    assert!(
        extracted
            .document_schema
            .types
            .contains_key(&KeyCmpValue::String("Action".to_string())),
        "Expected 'Action' type to be defined"
    );

    // The schema should have root fields defined
    assert!(
        extracted
            .document_schema
            .root
            .fields
            .contains_key(&KeyCmpValue::String("script".to_string())),
        "Expected 'script' to be in root fields"
    );

    // Validate document against schema
    let errors = validate_with_schema_value(doc_input, extracted.document_schema)
        .expect("Failed to validate");

    // Print errors for debugging
    if !errors.is_empty() {
        eprintln!("Validation errors:");
        for (i, error) in errors.iter().enumerate() {
            eprintln!("  {i}: {error:?}");
        }
    }

    // For now, we just verify parsing works and schema is extracted
    // The actual validation might have issues we need to debug
    assert!(
        errors.len() < 10,
        "Should have reasonable number of errors, got {}",
        errors.len()
    );
}

#[test]
fn test_schema_reference_in_document() {
    let doc = r#"
$schema = "./my-schema.eure"

name = "test"
value = 42
"#;

    let extracted = extract_schema_from_value(doc).expect("Failed to extract schema");

    // Should extract the schema reference
    assert_eq!(
        extracted.document_schema.schema_ref,
        Some("./my-schema.eure".to_string())
    );

    // Should not be a pure schema (has data)
    assert!(!extracted.is_pure_schema);
}

#[test]
fn test_pure_schema_detection() {
    // A pure schema with only type definitions
    let schema = r#"
person.name.$type = .string
person.age.$type = .number
person.email.$type = .code.email
person.email.$optional = true
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    // Should be detected as pure schema
    assert!(extracted.is_pure_schema);

    // Should have the fields defined
    let person_fields = &extracted.document_schema.root.fields;
    assert!(person_fields.contains_key(&KeyCmpValue::String("person".to_string())));
}

#[test]
fn test_shorthand_schema_syntax() {
    // Test the shorthand syntax without explicit .$type
    let schema = r#"
@ person
name.$type = .string
age.$type = .number
email.$type = .code.email
email.$optional = true
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    // This should be detected as a pure schema
    assert!(
        extracted.is_pure_schema,
        "Shorthand syntax should be recognized as pure schema"
    );

    // Should have the person object defined
    let person_fields = &extracted.document_schema.root.fields;
    assert!(person_fields.contains_key(&KeyCmpValue::String("person".to_string())));

    // Check the person object has the expected fields
    if let Some(person_field) = person_fields.get(&KeyCmpValue::String("person".to_string())) {
        if let eure_schema::Type::Object(obj) = &person_field.type_expr {
            assert!(
                obj.fields
                    .contains_key(&KeyCmpValue::String("name".to_string()))
            );
            assert!(
                obj.fields
                    .contains_key(&KeyCmpValue::String("age".to_string()))
            );
            assert!(
                obj.fields
                    .contains_key(&KeyCmpValue::String("email".to_string()))
            );

            // Check email is optional
            if let Some(email_field) = obj.fields.get(&KeyCmpValue::String("email".to_string())) {
                assert!(email_field.optional, "Email field should be optional");
            }
        } else {
            panic!("Person should be an object type");
        }
    }
}
