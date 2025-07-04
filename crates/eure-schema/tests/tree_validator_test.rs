//! Tests for tree-based validation with span tracking

use eure_schema::{validate_with_tree, DocumentSchema, FieldSchema, Type, ObjectSchema, Severity};
use eure_value::value::KeyCmpValue;
use indexmap::IndexMap;

#[test]
fn test_tree_validation_with_spans() {
    let input = r#"
# Test document
name = 123  # Wrong type - should be string
age = "not a number"  # Wrong type - should be number
"#;

    // Parse the document
    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    // Create a simple schema
    let mut fields = IndexMap::new();
    fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    fields.insert(
        KeyCmpValue::String("age".to_string()),
        FieldSchema {
            type_expr: Type::Number,
            optional: false,
            ..Default::default()
        }
    );
    
    let schema = DocumentSchema {
        root: ObjectSchema {
            fields,
            additional_properties: None,
        },
        ..Default::default()
    };
    
    // Validate using tree-based validator
    let errors = validate_with_tree(input, schema, &tree)
        .expect("Validation should not fail");
    
    // Should have 2 type mismatch errors
    assert_eq!(errors.len(), 2, "Expected 2 validation errors");
    
    // Check that errors have spans
    for error in &errors {
        assert!(error.span.is_some(), "Error should have span information");
        assert_eq!(error.severity, Severity::Error);
    }
    
    // Check error types
    let name_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(&e.kind, 
            eure_schema::ValidationErrorKind::TypeMismatch { expected, actual } 
            if expected == "string" && actual == "number"
        ))
        .collect();
    assert_eq!(name_errors.len(), 1, "Should have one string/number mismatch");
    
    let age_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(&e.kind,
            eure_schema::ValidationErrorKind::TypeMismatch { expected, actual }
            if expected == "number" && actual == "string"
        ))
        .collect();
    assert_eq!(age_errors.len(), 1, "Should have one number/string mismatch");
}

#[test]
fn test_tree_validation_missing_fields() {
    let input = r#"
name = "John"
# age field is missing
"#;

    // Parse the document
    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    // Create schema with required fields
    let mut fields = IndexMap::new();
    fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    fields.insert(
        KeyCmpValue::String("age".to_string()),
        FieldSchema {
            type_expr: Type::Number,
            optional: false, // Required field
            ..Default::default()
        }
    );
    
    let schema = DocumentSchema {
        root: ObjectSchema {
            fields,
            additional_properties: None,
        },
        ..Default::default()
    };
    
    // Validate
    let errors = validate_with_tree(input, schema, &tree)
        .expect("Validation should not fail");
    
    // Should have 1 missing field error
    assert_eq!(errors.len(), 1, "Expected 1 validation error");
    
    match &errors[0].kind {
        eure_schema::ValidationErrorKind::RequiredFieldMissing { field, .. } => {
            assert_eq!(field, &KeyCmpValue::String("age".to_string()));
        }
        _ => panic!("Expected RequiredFieldMissing error"),
    }
}

#[test]
fn test_tree_validation_unexpected_fields() {
    let input = r#"
name = "John"
age = 30
extra = "unexpected"  # This field is not in schema
"#;

    // Parse the document
    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    // Create schema without extra field
    let mut fields = IndexMap::new();
    fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    fields.insert(
        KeyCmpValue::String("age".to_string()),
        FieldSchema {
            type_expr: Type::Number,
            optional: false,
            ..Default::default()
        }
    );
    
    let schema = DocumentSchema {
        root: ObjectSchema {
            fields,
            additional_properties: None, // No additional properties allowed
        },
        ..Default::default()
    };
    
    // Validate
    let errors = validate_with_tree(input, schema, &tree)
        .expect("Validation should not fail");
    
    // Should have 1 unexpected field error
    assert_eq!(errors.len(), 1, "Expected 1 validation error");
    
    match &errors[0].kind {
        eure_schema::ValidationErrorKind::UnexpectedField { field, .. } => {
            assert_eq!(field, &KeyCmpValue::String("extra".to_string()));
        }
        _ => panic!("Expected UnexpectedField error"),
    }
}

#[test]
fn test_self_describing_with_tree() {
    let input = r#"
# Self-describing document
name.$type = .string
age.$type = .number

# Data
name = "Alice"
age = 25
"#;

    // Parse the document
    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    // Validate self-describing document
    let result = eure_schema::validate_self_describing_with_tree(input, &tree)
        .expect("Should extract and validate");
    
    // Should have no errors
    assert_eq!(result.errors.len(), 0, "Expected no validation errors");
    
    // Should have extracted the schema
    assert!(!result.schema.is_pure_schema);
    assert_eq!(result.schema.document_schema.root.fields.len(), 2);
}