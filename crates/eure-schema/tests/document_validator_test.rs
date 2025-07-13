//! Tests for document-based validation

use eure_schema::{
    document_to_schema, validate_document, 
    ValidationError, ValidationErrorKind, Severity, DocumentSchema,
    ObjectSchema, FieldSchema, Type, VariantSchema, VariantRepr, Constraints
};
use eure_tree::document::EureDocument;
use eure_tree::value_visitor::ValueVisitor;
use eure_value::value::KeyCmpValue;
use indexmap::IndexMap;

/// Helper function to parse a document and extract EureDocument
fn parse_to_document(input: &str) -> EureDocument {
    let tree = eure_parol::parse(input).expect("Failed to parse");
    
    let mut visitor = ValueVisitor::new(input);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    
    visitor.into_document()
}

#[test]
fn test_simple_validation() {
    // Create a simple schema
    let mut schema = DocumentSchema::default();
    schema.root.fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    schema.root.fields.insert(
        KeyCmpValue::String("age".to_string()),
        FieldSchema {
            type_expr: Type::Number,
            optional: true,
            ..Default::default()
        }
    );
    
    // Valid document
    let valid_doc = r#"
name = "Alice"
age = 30
"#;
    
    let document = parse_to_document(valid_doc);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid document should have no errors");
}

#[test]
fn test_missing_required_field() {
    // Create schema with required field
    let mut schema = DocumentSchema::default();
    schema.root.fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    schema.root.fields.insert(
        KeyCmpValue::String("age".to_string()),
        FieldSchema {
            type_expr: Type::Number,
            optional: true,
            ..Default::default()
        }
    );
    
    // Document missing required field
    let invalid_doc = r#"
age = 30
"#;
    
    let document = parse_to_document(invalid_doc);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 1, "Should have one error");
    assert!(matches!(
        &errors[0].kind,
        ValidationErrorKind::RequiredFieldMissing { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "name")
    ));
}

#[test]
fn test_type_mismatch() {
    // Create schema expecting string
    let mut schema = DocumentSchema::default();
    schema.root.fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    
    // Document with wrong type
    let invalid_doc = r#"
name = 123
"#;
    
    let document = parse_to_document(invalid_doc);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 1, "Should have one error");
    assert!(matches!(
        &errors[0].kind,
        ValidationErrorKind::TypeMismatch { expected, actual }
        if expected == "string" && (actual == "i64" || actual == "u64")
    ));
}

#[test]
fn test_array_validation() {
    // Create schema with array field
    let mut schema = DocumentSchema::default();
    schema.root.fields.insert(
        KeyCmpValue::String("items".to_string()),
        FieldSchema {
            type_expr: Type::Array(Box::new(Type::String)),
            optional: false,
            constraints: Constraints {
                ..Default::default()
            },
            ..Default::default()
        }
    );
    
    // Valid array
    let valid_doc = r#"
items = ["a", "b"]
"#;
    
    let document = parse_to_document(valid_doc);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid array should have no errors");
    
    // Array with wrong element type
    let invalid_doc = r#"
items = ["a", 123, "c"]
"#;
    
    let document = parse_to_document(invalid_doc);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 1, "Should have one error for type mismatch");
    assert!(matches!(
        &errors[0].kind,
        ValidationErrorKind::TypeMismatch { .. }
    ));
}

#[test]
fn test_nested_object_validation() {
    // Create schema with nested object
    let mut schema = DocumentSchema::default();
    
    let mut user_fields = IndexMap::new();
    user_fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    user_fields.insert(
        KeyCmpValue::String("email".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: true,
            ..Default::default()
        }
    );
    
    schema.root.fields.insert(
        KeyCmpValue::String("user".to_string()),
        FieldSchema {
            type_expr: Type::Object(ObjectSchema {
                fields: user_fields,
                additional_properties: None,
            }),
            optional: false,
            ..Default::default()
        }
    );
    
    // Valid nested object using sections
    let valid_doc = r#"
@ user
name = "Alice"
email = "alice@example.com"
"#;
    
    let document = parse_to_document(valid_doc);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid nested object should have no errors");
}

#[test]
fn test_variant_validation() {
    // Create schema with variants
    let mut variants = IndexMap::new();
    
    // Create variant
    let mut create_fields = IndexMap::new();
    create_fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    variants.insert(
        KeyCmpValue::String("create".to_string()),
        ObjectSchema {
            fields: create_fields,
            additional_properties: None,
        }
    );
    
    // Update variant
    let mut update_fields = IndexMap::new();
    update_fields.insert(
        KeyCmpValue::String("id".to_string()),
        FieldSchema {
            type_expr: Type::Number,
            optional: false,
            ..Default::default()
        }
    );
    update_fields.insert(
        KeyCmpValue::String("name".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: true,
            ..Default::default()
        }
    );
    variants.insert(
        KeyCmpValue::String("update".to_string()),
        ObjectSchema {
            fields: update_fields,
            additional_properties: None,
        }
    );
    
    let mut schema = DocumentSchema::default();
    schema.root.fields.insert(
        KeyCmpValue::String("action".to_string()),
        FieldSchema {
            type_expr: Type::Variants(VariantSchema {
                variants,
                representation: VariantRepr::Tagged,
            }),
            optional: false,
            ..Default::default()
        }
    );
    
    // Valid variant document
    let valid_doc = r#"
@ action
$variant = "create"
name = "New Item"
"#;
    
    let document = parse_to_document(valid_doc);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid variant should have no errors");
    
    // Invalid variant - missing required field
    let invalid_doc = r#"
@ action
$variant = "update"
name = "Updated"
"#;
    
    let document = parse_to_document(invalid_doc);
    let errors = validate_document(&document, &schema);
    
    assert!(errors.len() > 0, "Should have errors for missing required field 'id'");
}

#[test]
fn test_hole_detection() {
    // Create simple schema
    let mut schema = DocumentSchema::default();
    schema.root.fields.insert(
        KeyCmpValue::String("value".to_string()),
        FieldSchema {
            type_expr: Type::String,
            optional: false,
            ..Default::default()
        }
    );
    
    // Document with hole
    let doc_with_hole = r#"
value = !
"#;
    
    let document = parse_to_document(doc_with_hole);
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 1, "Should have one error");
    assert!(matches!(
        &errors[0].kind,
        ValidationErrorKind::HoleExists { .. }
    ));
}

#[test]
fn test_schema_extraction_and_validation() {
    // Test schema extraction from document
    let schema_doc = r#"
@ $types.User {
    name.$type = .string
    age.$type = .number
    age.$optional = true
}

@ users
$array = .$types.User
"#;
    
    let schema_document = parse_to_document(schema_doc);
    let schema = document_to_schema(&schema_document).expect("Failed to extract schema");
    
    // Validate a document against extracted schema
    let doc = r#"
@ users[]
name = "Alice"
age = 30

@ users[]
name = "Bob"
"#;
    
    let document = parse_to_document(doc);
    let errors = validate_document(&document, &schema);
    
    // Debug: print the errors
    if !errors.is_empty() {
        eprintln!("Schema extraction test - found {} errors:", errors.len());
        for (i, error) in errors.iter().enumerate() {
            eprintln!("  Error {}: {:?}", i + 1, error.kind);
        }
    }
    
    assert_eq!(errors.len(), 0, "Document should be valid against extracted schema");
}