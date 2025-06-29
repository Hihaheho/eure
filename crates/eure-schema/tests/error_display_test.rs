//! Tests for error message display formatting

use eure_schema::{ValidationError, ValidationErrorKind, Severity};
use eure_tree::tree::InputSpan;

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
        field: "name".to_string(),
        path: vec!["user".to_string()],
    };
    assert_eq!(
        error.to_string(),
        "Required field 'name' is missing at user"
    );
    
    let error_root = ValidationErrorKind::RequiredFieldMissing {
        field: "id".to_string(),
        path: vec![],
    };
    assert_eq!(
        error_root.to_string(),
        "Required field 'id' is missing"
    );
}

#[test]
fn test_string_length_violation_display() {
    let error = ValidationErrorKind::StringLengthViolation {
        min: Some(3),
        max: Some(10),
        actual: 15,
    };
    assert_eq!(
        error.to_string(),
        "String length must be between 3 and 10 characters, but got 15"
    );
    
    let error_min_only = ValidationErrorKind::StringLengthViolation {
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
    let error = ValidationErrorKind::NumberRangeViolation {
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
    let error = ValidationErrorKind::StringPatternViolation {
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
        span: InputSpan {
            start: 10,
            end: 20,
        },
        severity: Severity::Error,
    };
    assert_eq!(
        error.to_string(),
        "error: Type mismatch: expected boolean, but got string"
    );
    
    let warning = ValidationError {
        kind: ValidationErrorKind::PreferSection {
            path: vec!["config".to_string()],
        },
        span: InputSpan {
            start: 30,
            end: 40,
        },
        severity: Severity::Warning,
    };
    assert_eq!(
        warning.to_string(),
        "warning: Consider using binding syntax instead of section syntax for config"
    );
}

#[test]
fn test_array_violations_display() {
    let length_error = ValidationErrorKind::ArrayLengthViolation {
        min: Some(2),
        max: Some(5),
        actual: 7,
    };
    assert_eq!(
        length_error.to_string(),
        "Array must have between 2 and 5 items, but has 7"
    );
    
    let unique_error = ValidationErrorKind::ArrayUniqueViolation {
        duplicate: "item1".to_string(),
    };
    assert_eq!(
        unique_error.to_string(),
        "Array contains duplicate value: item1"
    );
}

#[test]
fn test_unexpected_field_display() {
    let error = ValidationErrorKind::UnexpectedField {
        field: "extra".to_string(),
        path: vec!["user".to_string(), "profile".to_string()],
    };
    assert_eq!(
        error.to_string(),
        "Unexpected field 'extra' at user.profile not defined in schema"
    );
}

#[test]
fn test_invalid_schema_pattern_display() {
    let error = ValidationErrorKind::InvalidSchemaPattern {
        pattern: "[invalid".to_string(),
        error: "unclosed character class".to_string(),
    };
    assert_eq!(
        error.to_string(),
        "Invalid pattern '/[invalid/': unclosed character class"
    );
}

#[test]
fn test_missing_variant_tag_display() {
    let error = ValidationErrorKind::MissingVariantTag;
    assert_eq!(
        error.to_string(),
        "Missing $variant tag for variant type"
    );
}

#[test]
fn test_prefer_warnings_display() {
    let section_warning = ValidationErrorKind::PreferSection {
        path: vec!["database".to_string(), "config".to_string()],
    };
    assert_eq!(
        section_warning.to_string(),
        "Consider using binding syntax instead of section syntax for database.config"
    );
    
    let array_warning = ValidationErrorKind::PreferArraySyntax {
        path: vec!["items".to_string()],
    };
    assert_eq!(
        array_warning.to_string(),
        "Consider using explicit array syntax instead of array append syntax for items"
    );
}