//! Test edge cases in field tracking to ensure proper distinction between field types

use eure_schema::{extract_schema_from_value, validate_with_tree};

#[test]
fn test_distinguish_extension_from_dollar_string() {
    // Test that Extension("variant") as a path segment is different from String("$variant") field name
    // Note: In EURE syntax, $variant: value and $variant = "value" are semantically identical
    // This test verifies that the internal representation can distinguish path types
    let schema = r#"
# Schema with a field that starts with $
$types.Config {
  "$variant" = .string      # This is a field named "$variant"
  name = .string
}

config = .$types.Config
"#;

    let document1 = r#"
# Document with extension field (this would be handled differently in real usage)
@ config
name = "test"
# Missing "$variant" field
"#;

    let document2 = r#"
# Document setting the field
@ config
"$variant" = "field-value"  # This is the field named "$variant"
name = "test"
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    // Document 1 should have error - missing required field "$variant"
    let tree1 = eure_parol::parse(document1).expect("Failed to parse document1");
    let errors1 = validate_with_tree(document1, extracted.document_schema.clone(), &tree1)
        .expect("Validation failed");

    assert!(
        !errors1.is_empty(),
        "Expected error for missing '$variant' field"
    );
    assert!(errors1.iter().any(|e|
        matches!(&e.kind, eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. }
                 if matches!(field, eure_schema::KeyCmpValue::String(s) if s == "$variant"))
    ), "Should report missing field '$variant'");

    // Document 2 should be valid
    let tree2 = eure_parol::parse(document2).expect("Failed to parse document2");
    let errors2 = validate_with_tree(document2, extracted.document_schema, &tree2)
        .expect("Validation failed");

    assert!(
        errors2.is_empty(),
        "Document 2 should be valid, got errors: {errors2:?}"
    );
}

#[test]
fn test_distinguish_dotted_field_from_nested_path() {
    // Test that "a.b.c" as a single field name is different from nested a.b.c
    let schema = r#"
# Schema with a field containing dots in its name
"a.b.c" = .string         # Single field with dots in name
nested = {
  a = {
    b = {
      c = .string         # Nested path a.b.c
    }
  }
}
"#;

    let document1 = r#"
# Document with dotted field name
"a.b.c" = "dotted-field"
@ nested.a.b
c = "nested-field"
"#;

    let document2 = r#"
# Document missing the dotted field
@ nested.a.b
c = "nested-field"
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    // Document 1 should be valid
    let tree1 = eure_parol::parse(document1).expect("Failed to parse document1");
    let errors1 = validate_with_tree(document1, extracted.document_schema.clone(), &tree1)
        .expect("Validation failed");

    assert!(
        errors1.is_empty(),
        "Document 1 should be valid, got errors: {errors1:?}"
    );

    // Document 2 should have error - missing required field "a.b.c"
    let tree2 = eure_parol::parse(document2).expect("Failed to parse document2");
    let errors2 = validate_with_tree(document2, extracted.document_schema, &tree2)
        .expect("Validation failed");

    assert!(
        !errors2.is_empty(),
        "Expected error for missing 'a.b.c' field"
    );
    assert!(errors2.iter().any(|e|
        matches!(&e.kind, eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. }
                 if matches!(field, eure_schema::KeyCmpValue::String(s) if s == "a.b.c"))
    ), "Should report missing field 'a.b.c'");
}

#[test]
fn test_array_field_vs_ident_with_brackets() {
    // Test that actions[] array is different from a field named "actions[]"
    let schema = r#"
# Schema with both array field and bracket-named field
actions.$array = .string          # Array field
"actions[]" = .string             # Field with brackets in name
"#;

    let document1 = r#"
# Document with array field
@ actions[]
# Array element content
"#;

    let document2 = r#"
# Document with bracket-named field
"actions[]" = "not-an-array"
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    // Document 1 should have the array field but missing the bracket-named field
    let tree1 = eure_parol::parse(document1).expect("Failed to parse document1");
    let errors1 = validate_with_tree(document1, extracted.document_schema.clone(), &tree1)
        .expect("Validation failed");

    assert!(
        !errors1.is_empty(),
        "Expected error for missing 'actions[]' field"
    );
    assert!(errors1.iter().any(|e|
        matches!(&e.kind, eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. }
                 if matches!(field, eure_schema::KeyCmpValue::String(s) if s == "actions[]"))
    ), "Should report missing field 'actions[]'");

    // Document 2 should have the bracket-named field but missing the array field
    let tree2 = eure_parol::parse(document2).expect("Failed to parse document2");
    let errors2 = validate_with_tree(document2, extracted.document_schema, &tree2)
        .expect("Validation failed");

    assert!(
        !errors2.is_empty(),
        "Expected error for missing 'actions' array field"
    );
    assert!(errors2.iter().any(|e|
        matches!(&e.kind, eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. }
                 if matches!(field, eure_schema::KeyCmpValue::String(s) if s == "actions"))
    ), "Should report missing field 'actions'");
}

#[test]
fn test_deep_nesting_field_tracking() {
    // Test that deeply nested fields are tracked correctly
    let schema = r#"
@ a.b.c.d.e.f
actions.$array = .string
"#;

    let document = r#"
@ a.b.c.d.e.f.actions[]
# Array element
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    let tree = eure_parol::parse(document).expect("Failed to parse document");
    let errors =
        validate_with_tree(document, extracted.document_schema, &tree).expect("Validation failed");

    // Should be valid - the deeply nested array field exists
    assert!(
        errors.is_empty(),
        "Document should be valid, got errors: {errors:?}"
    );
}
