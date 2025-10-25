use eure_schema::{extract_schema_from_value, validate_with_tree};

#[test]
fn test_section_field_validation() {
    // Schema with a section containing fields
    let schema_input = r#"
@ user
name.$type = .string
email.$type = .string
email.$optional = true
age.$type = .number
"#;

    let extracted = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    // Document with valid and invalid fields
    let doc_input = r#"
@ user
name = "Alice"
email = "alice@example.com"
age = 30
invalid_field = "should be reported"
"#;

    let tree = eure_parol::parse(doc_input).expect("Failed to parse document");

    let errors = validate_with_tree(&tree, doc_input, extracted.document_schema)
        .expect("Failed to validate");

    // Should only have one error for the invalid field
    assert_eq!(
        errors.len(),
        1,
        "Expected exactly one error for invalid_field"
    );

    if let eure_schema::ValidationErrorKind::UnexpectedField { field, path } = &errors[0].kind {
        assert_eq!(
            field,
            &eure_schema::KeyCmpValue::String("invalid_field".to_string())
        );
        assert_eq!(path.len(), 1); // Path should be [Ident("user")]
    } else {
        panic!("Expected UnexpectedField error, got {:?}", errors[0].kind);
    }

    // Test with only valid fields
    let valid_doc = r#"
@ user
name = "Bob"
age = 25
"#;

    let tree2 = eure_parol::parse(valid_doc).expect("Failed to parse document");

    // Need to extract schema again since it was moved
    let extracted2 = extract_schema_from_value(schema_input).expect("Failed to extract schema");

    let errors2 = validate_with_tree(&tree2, valid_doc, extracted2.document_schema)
        .expect("Failed to validate");

    assert_eq!(errors2.len(), 0, "Should have no errors for valid document");
}
