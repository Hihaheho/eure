//! Tests for error message display formatting

use eure_value::value::KeyCmpValue;

use eure_schema::{ValidationError, ValidationErrorKind, Severity};
use eure_value::value::PathSegment;
use eure_value::identifier::Identifier;
use eure_tree::document::NodeId;
use std::str::FromStr;

#[test]
fn test_type_mismatch_display() {
    let error = ValidationErrorKind::TypeMismatch {
        expected: "string".to_string(),
        actual: "number".to_string(),
    };
    assert_eq!(
        error.to_string(),
        "Type mismatch: expected string, but got number"
    );
}

#[test]
fn test_required_field_missing_display() {
    let error = ValidationErrorKind::RequiredFieldMissing {
        field: KeyCmpValue::String("name".to_string()),
        path: vec![PathSegment::Ident(Identifier::from_str("user").unwrap())],
    };
    assert_eq!(
        error.to_string(),
        "Required field String(\"name\") is missing at user"
    );
    
    let error_root = ValidationErrorKind::RequiredFieldMissing {
        field: KeyCmpValue::String("id".to_string()),
        path: vec![],
    };
    assert_eq!(
        error_root.to_string(),
        "Required field String(\"id\") is missing"
    );
}

#[test]
fn test_string_length_violation_display() {
    let error = ValidationErrorKind::LengthViolation {
        min: Some(3),
        max: Some(10),
        actual: 15,
    };
    assert_eq!(
        error.to_string(),
        "String length must be between 3 and 10 characters, but got 15"
    );
    
    let error_min_only = ValidationErrorKind::LengthViolation {
        min: Some(5),
        max: None,
        actual: 2,
    };
    assert_eq!(
        error_min_only.to_string(),
        "String must be at least 5 characters long, but got 2"
    );
}

#[test]
fn test_number_range_violation_display() {
    let error = ValidationErrorKind::RangeViolation {
        min: Some(0.0),
        max: Some(100.0),
        actual: 150.0,
    };
    assert_eq!(
        error.to_string(),
        "Number must be between 0 and 100, but got 150"
    );
}

#[test]
fn test_string_pattern_violation_display() {
    let error = ValidationErrorKind::PatternMismatch {
        pattern: r"^\d{3}-\d{4}$".to_string(),
        value: "not-a-phone".to_string(),
    };
    assert_eq!(
        error.to_string(),
        r"String 'not-a-phone' does not match pattern /^\d{3}-\d{4}$/"
    );
}

#[test]
fn test_unknown_variant_display() {
    let error = ValidationErrorKind::UnknownVariant {
        variant: "foo".to_string(),
        available: vec!["create".to_string(), "update".to_string(), "delete".to_string()],
    };
    assert_eq!(
        error.to_string(),
        "Unknown variant 'foo'. Available variants: create, update, delete"
    );
    
    let error_no_available = ValidationErrorKind::UnknownVariant {
        variant: "bar".to_string(),
        available: vec![],
    };
    assert_eq!(
        error_no_available.to_string(),
        "Unknown variant 'bar'"
    );
}

#[test]
fn test_validation_error_display() {
    let error = ValidationError {
        kind: ValidationErrorKind::TypeMismatch {
            expected: "boolean".to_string(),
            actual: "string".to_string(),
        },
        severity: Severity::Error,
        node_id: NodeId::from(0),
    };
    assert_eq!(
        error.to_string(),
        "error: Type mismatch: expected boolean, but got string"
    );
}

#[test]
fn test_array_violations_display() {
    let length_error = ValidationErrorKind::ArrayLengthViolation {
        min: Some(2),
        max: Some(5),
        length: 7,
    };
    assert_eq!(
        length_error.to_string(),
        "Array must have between 2 and 5 items, but has 7"
    );
}

#[test]
fn test_unexpected_field_display() {
    let error = ValidationErrorKind::UnexpectedField {
        field: KeyCmpValue::String("extra".to_string()),
        path: vec![PathSegment::Ident(Identifier::from_str("user").unwrap()), PathSegment::Ident(Identifier::from_str("profile").unwrap())],
    };
    assert_eq!(
        error.to_string(),
        "Unexpected field String(\"extra\") at user.profile not defined in schema"
    );
}

// InvalidSchemaPattern is not in the new implementation

// TODO: Add VariantDiscriminatorMissing variant if needed
// #[test]
// fn test_variant_discriminator_missing_display() {
//     let error = ValidationErrorKind::VariantDiscriminatorMissing;
//     assert_eq!(
//         error.to_string(),
//         "Variant discriminator field '$variant' is missing"
//     );
// }

// Preference warnings are not implemented in the new value-based validator