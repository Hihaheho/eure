//! End-to-end tests for variant array field validation

use eure_schema::{
    KeyCmpValue, ValidationErrorKind, extract_schema_from_value, validate_with_tree,
};

#[test]
fn test_variant_with_simple_array_validates() {
    let schema = r#"
$types.Action {
  @ $variants.set-text {
    speaker = .string
    lines.$array = .string
  }
}

actions.$array = .$types.Action
"#;

    let document = r#"
@ actions[]
$variant: set-text
speaker = "Alice"
lines = ["Hello", "World"]
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    let tree = eure_parol::parse(document).expect("Failed to parse document");

    let errors =
        validate_with_tree(&tree, document, extracted.document_schema).expect("Validation failed");

    assert!(
        errors.is_empty(),
        "Expected no validation errors, got: {errors:?}"
    );
}

#[test]
fn test_variant_missing_array_field() {
    let schema = r#"
$types.Action {
  @ $variants.set-text {
    speaker = .string
    lines.$array = .string
  }
}

actions.$array = .$types.Action
"#;

    let document = r#"
@ actions[]
$variant: set-text
speaker = "Alice"
# Missing lines field!
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    let tree = eure_parol::parse(document).expect("Failed to parse document");

    let errors =
        validate_with_tree(&tree, document, extracted.document_schema).expect("Validation failed");

    // Should have error for missing required field
    assert!(
        !errors.is_empty(),
        "Expected validation errors for missing lines field"
    );

    let has_missing_field_error = errors.iter().any(|e|
        matches!(&e.kind, ValidationErrorKind::RequiredFieldMissing { field, .. } if matches!(field, KeyCmpValue::String(s) if s == "lines"))
    );
    assert!(
        has_missing_field_error,
        "Expected missing field error for 'lines'"
    );
}

#[test]
fn test_variant_array_wrong_type() {
    let schema = r#"
$types.Action {
  @ $variants.set-text {
    speaker = .string
    lines.$array = .string
  }
}

actions.$array = .$types.Action
"#;

    let document = r#"
@ actions[]
$variant: set-text
speaker = "Alice"
lines = [123, 456]  # Wrong type - numbers instead of strings
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    let tree = eure_parol::parse(document).expect("Failed to parse document");

    let errors =
        validate_with_tree(&tree, document, extracted.document_schema).expect("Validation failed");

    // Should have type errors
    assert!(
        !errors.is_empty(),
        "Expected validation errors for wrong array element types"
    );

    let has_type_error = errors
        .iter()
        .any(|e| matches!(&e.kind, ValidationErrorKind::TypeMismatch { .. }));
    assert!(has_type_error, "Expected type mismatch error");
}

#[test]
fn test_complex_variant_array_validates() {
    let schema = r#"
$types.Choice {
  text = .string
  value = .string
}

$types.Action {
  @ $variants.set-choices {
    description = .string
    choices.$array = .$types.Choice
  }
}

@ root
actions.$array = .$types.Action
"#;

    let document = r#"
@ root

@ root.actions[]
$variant: set-choices
description = "Pick one"
choices = [
  { text = "Option A", value = "a" },
  { text = "Option B", value = "b" }
]
"#;

    let extracted = extract_schema_from_value(schema).expect("Failed to extract schema");

    let tree = eure_parol::parse(document).expect("Failed to parse document");

    let errors =
        validate_with_tree(&tree, document, extracted.document_schema).expect("Validation failed");

    assert!(
        errors.is_empty(),
        "Expected no validation errors, got: {errors:?}"
    );
}
