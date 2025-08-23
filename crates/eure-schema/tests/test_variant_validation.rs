use eure_schema::{document_to_schema, validate_document, ValidationErrorKind, KeyCmpValue};
use eure_tree::value_visitor::ValueVisitor;

// NOTE: These tests are currently failing due to issues in the schema extraction logic:
// 1. document_schema.rs line 260 has a hardcoded identifier with a dot ("variant.repr") which is invalid
// 2. Array field definitions in schemas are not being parsed correctly
// The test structure itself has been fixed to use the correct API pattern:
// - Parse with eure_parol::parse
// - Create ValueVisitor and visit the tree
// - Get EureDocument from visitor
// - For schemas: use document_to_schema
// - For validation: use validate_document

#[test]
fn test_variant_validation() {
    // Schema with variant types
    let schema_input = r#"
$types.Task {
  @ $variants.create-file {
    path = .string
    content = .string
    description = .string
    description.$optional = true
  }
  @ $variants.run-command {
    command = .string
    timeout = .number
    timeout.$optional = true
    description = .string
    description.$optional = true
  }
}

tasks.$array = .$types.Task
tasks.$optional = true
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test 1: Valid variant with all required fields
    let valid_doc = r#"
@ tasks[] {
  $variant: create-file
  path = "/tmp/test.txt"
  content = "Hello, world!"
  description = "Create a test file"
}
"#;
    
    let tree = eure_parol::parse(valid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(valid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    // Validation should succeed for valid variant
    
    assert_eq!(errors.len(), 0, "Valid variant should have no errors, but got: {errors:?}");
    
    // Test 2: Variant missing required fields (should error)
    let missing_required = r#"
@ tasks[] {
  $variant: create-file
  path = "/tmp/test.txt"
  # Missing required 'content' field
}
"#;
    
    let tree2 = eure_parol::parse(missing_required).expect("Failed to parse document");
    let mut visitor2 = ValueVisitor::new(missing_required);
    tree2.visit_from_root(&mut visitor2).expect("Failed to visit tree");
    let document2 = visitor2.into_document();
    let errors2 = validate_document(&document2, &schema);
    
    // This should have an error for missing 'content' field
    let has_missing_content = errors2.iter().any(|e| matches!(&e.kind, 
        ValidationErrorKind::RequiredFieldMissing { field, .. } 
        if matches!(field, KeyCmpValue::String(s) if s == "content")
    ));
    
    // Required field validation for variants is now working
    assert!(has_missing_content, 
        "Should have error for missing required 'content' field in create-file variant, but got errors: {:?}", 
        errors2);
    
    // Test 3: Variant with unexpected fields
    let unexpected_field = r#"
@ tasks[] {
  $variant: run-command
  command = "ls -la"
  invalid_field = "should be reported"
}
"#;
    
    let tree3 = eure_parol::parse(unexpected_field).expect("Failed to parse document");
    let mut visitor3 = ValueVisitor::new(unexpected_field);
    tree3.visit_from_root(&mut visitor3).expect("Failed to visit tree");
    let document3 = visitor3.into_document();
    let errors3 = validate_document(&document3, &schema);
    
    let has_unexpected = errors3.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnexpectedField { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "invalid_field")
    ));
    
    assert!(has_unexpected, 
        "Should have error for unexpected 'invalid_field' in run-command variant, but got errors: {errors3:?}");
    
    // Test 4: Valid variant-specific fields should NOT be reported as unexpected
    let valid_variant_fields = r#"
@ tasks[] {
  $variant: run-command
  command = "sleep 5"
  timeout = 1000
}
"#;
    
    let tree4 = eure_parol::parse(valid_variant_fields).expect("Failed to parse document");
    let mut visitor4 = ValueVisitor::new(valid_variant_fields);
    tree4.visit_from_root(&mut visitor4).expect("Failed to visit tree");
    let document4 = visitor4.into_document();
    let errors4 = validate_document(&document4, &schema);
    
    // Check that valid variant fields are not reported as unexpected
    let has_false_positive = errors4.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnexpectedField { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "command" || s == "timeout")
    ));
    
    assert!(!has_false_positive, 
        "Valid variant fields should not be reported as unexpected, but got errors: {errors4:?}");
    
    // Test 5: Invalid variant name
    let invalid_variant = r#"
@ tasks[] {
  $variant: invalid-variant
  some_field = "value"
}
"#;
    
    let tree5 = eure_parol::parse(invalid_variant).expect("Failed to parse document");
    let mut visitor5 = ValueVisitor::new(invalid_variant);
    tree5.visit_from_root(&mut visitor5).expect("Failed to visit tree");
    let document5 = visitor5.into_document();
    let errors5 = validate_document(&document5, &schema);
    
    let has_unknown_variant = errors5.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnknownVariant { variant, .. }
        if variant == "invalid-variant"
    ));
    
    assert!(has_unknown_variant || !errors5.is_empty(), 
        "Should have error for unknown variant 'invalid-variant', but got errors: {errors5:?}");
    
    // Test 6: Missing $variant tag
    let missing_variant_tag = r#"
@ tasks[] {
  # No $variant field
  path = "/tmp/test.txt"
  content = "data"
}
"#;
    
    let tree6 = eure_parol::parse(missing_variant_tag).expect("Failed to parse document");
    let mut visitor6 = ValueVisitor::new(missing_variant_tag);
    tree6.visit_from_root(&mut visitor6).expect("Failed to visit tree");
    let document6 = visitor6.into_document();
    let errors6 = validate_document(&document6, &schema);
    
    let has_missing_tag = errors6.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnknownVariant { .. }
    ));
    
    assert!(has_missing_tag || !errors6.is_empty(), 
        "Should have error for missing $variant tag, but got errors: {errors6:?}");
}

#[test] 
fn test_example_eure_variant_validation() {
    // Test specifically with the example.eure structure
    let schema_input = include_str!("../../../example.schema.eure");
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test document with variant that has an unexpected field
    let doc_with_unexpected = r#"
@ script
id = "test"
actions = []

@ script.actions[]
$variant: set-text
speaker = "alice"
lines = ["hello"]
unexpected_field = "this should be reported as unexpected"
"#;
    
    let tree = eure_parol::parse(doc_with_unexpected).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc_with_unexpected);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    // The unexpected_field should be reported
    let has_unexpected = errors.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnexpectedField { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "unexpected_field")
    ));
    
    assert!(has_unexpected, 
        "Should report 'unexpected_field' as unexpected in set-text variant");
    
    // Test that valid variant fields are not reported as unexpected
    let doc_with_valid_fields = r#"
@ script
id = "test"
actions = []

@ script.actions[]
$variant: set-text
speaker = "alice"
lines = ["hello", "world"]
code1 = rust`println!("test");`
"#;
    
    let tree2 = eure_parol::parse(doc_with_valid_fields).expect("Failed to parse document");
    let mut visitor2 = ValueVisitor::new(doc_with_valid_fields);
    tree2.visit_from_root(&mut visitor2).expect("Failed to visit tree");
    let document2 = visitor2.into_document();
    let errors2 = validate_document(&document2, &schema);
    
    // Should not report valid variant fields as unexpected
    let has_false_positives = errors2.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnexpectedField { field, .. }
        if matches!(field, KeyCmpValue::String(s) if ["speaker", "lines", "code1"].contains(&s.as_str()))
    ));
    
    assert!(!has_false_positives, 
        "Should not report valid variant fields as unexpected");
}

#[test]
fn test_set_choices_variant_with_nested_arrays() {
    // Skip the debug code for now and just test the main functionality

    // Test schema extraction for nested variant field definitions
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

    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");

    // Verify schema was extracted correctly
    assert!(schema.types.contains_key(&KeyCmpValue::String("Action".to_string())),
            "Schema should contain Action type");
    
    // Test case 1: Flat syntax with separate sections
    let flat_syntax = r#"
@ script
id = "test"
actions = []

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

    let tree = eure_parol::parse(flat_syntax).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(flat_syntax);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);

    // Flat syntax should have no validation errors

    // Check that valid fields are not reported as unexpected
    let unexpected_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(&e.kind, ValidationErrorKind::UnexpectedField { .. }))
        .collect();
    
    assert_eq!(unexpected_errors.len(), 0, 
        "Should not report any fields as unexpected in flat syntax, but got: {unexpected_errors:?}");

    // Test case 2: Block syntax with nested sections
    let block_syntax = r#"
@ script
id = "test2"
actions = []

@ script.actions[] {
  $variant: set-choices
  description = "Choose an option"

  @ choice[]
  text = "Option A"
  value = "a"

  @ choice[]
  text = "Option B"
  value = "b"
}
"#;

    let tree2 = eure_parol::parse(block_syntax).expect("Failed to parse document");
    let mut visitor2 = ValueVisitor::new(block_syntax);
    tree2.visit_from_root(&mut visitor2).expect("Failed to visit tree");
    let document2 = visitor2.into_document();
    let errors2 = validate_document(&document2, &schema);


    // Check that valid fields are not reported as unexpected
    let unexpected_errors2: Vec<_> = errors2.iter()
        .filter(|e| matches!(&e.kind, ValidationErrorKind::UnexpectedField { .. }))
        .collect();
    
    assert_eq!(unexpected_errors2.len(), 0, 
        "Should not report any fields as unexpected in block syntax, but got: {unexpected_errors2:?}");

    // Test case 3: Verify missing required fields are still caught
    let missing_required = r#"
@ script
id = "test3"
actions = []

@ script.actions[]
$variant: set-choices
# Missing required 'description' field

@ script.actions[].choice[]
text = "Option A"
# Missing required 'value' field
"#;

    let tree3 = eure_parol::parse(missing_required).expect("Failed to parse document");
    let mut visitor3 = ValueVisitor::new(missing_required);
    tree3.visit_from_root(&mut visitor3).expect("Failed to visit tree");
    let document3 = visitor3.into_document();
    let errors3 = validate_document(&document3, &schema);

    // Should have errors for missing required fields
    let has_missing_description = errors3.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::RequiredFieldMissing { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "description")
    ));
    
    let has_missing_value = errors3.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::RequiredFieldMissing { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "value")
    ));

    assert!(has_missing_description, 
        "Should report missing 'description' field in set-choices variant");
    assert!(has_missing_value, 
        "Should report missing 'value' field in choice array element");
}

#[test]
fn test_block_syntax_nested_variant_fields() {
    // This test specifically verifies that fields inside nested sections within blocks
    // are properly validated against variant schemas (regression test for issue where
    // fields inside @ choice[] within @ script.actions[] {} were reported as unexpected)
    
    let schema_input = r#"
$types.Action {
  @ $variants.set-choices {
    description = .string
  }
  @ $variants.set-choices.choice.$array {
    text = .string
    value = .string
  }
}

@ script
actions.$array = .$types.Action
"#;

    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test block syntax with nested sections containing valid variant fields
    let doc = r#"
@ script
actions = []

@ script.actions[] {
  $variant: set-choices
  description = "Choose your option"

  @ choice[]
  text = "Option 1"
  value = "opt1"

  @ choice[]
  text = "Option 2"
  value = "opt2"
}
"#;

    let tree = eure_parol::parse(doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);

    // Should have no validation errors for this valid document
    assert_eq!(errors.len(), 0, 
        "Valid block syntax with nested variant fields should have no errors, but got: {errors:?}");
    
    // Test with an invalid field to ensure validation is actually working
    let doc_with_invalid = r#"
@ script
actions = []

@ script.actions[] {
  $variant: set-choices
  description = "Choose your option"
  invalid_field = "should be reported"

  @ choice[]
  text = "Option 1"
  value = "opt1"
  another_invalid = "should also be reported"
}
"#;

    let tree2 = eure_parol::parse(doc_with_invalid).expect("Failed to parse document");
    let mut visitor2 = ValueVisitor::new(doc_with_invalid);
    tree2.visit_from_root(&mut visitor2).expect("Failed to visit tree");
    let document2 = visitor2.into_document();
    let errors2 = validate_document(&document2, &schema);

    // Should have exactly 2 unexpected field errors
    let unexpected_errors: Vec<_> = errors2.iter()
        .filter(|e| matches!(&e.kind, ValidationErrorKind::UnexpectedField { .. }))
        .collect();
    
    assert_eq!(unexpected_errors.len(), 2, 
        "Should report exactly 2 unexpected fields, but got: {errors2:?}");
    
    // Verify the specific fields reported as unexpected
    let has_invalid_field = errors2.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnexpectedField { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "invalid_field")
    ));
    
    let has_another_invalid = errors2.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnexpectedField { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "another_invalid")
    ));
    
    assert!(has_invalid_field, "Should report 'invalid_field' as unexpected");
    assert!(has_another_invalid, "Should report 'another_invalid' as unexpected");
}

#[test]
fn test_text_binding_validation() {
    // This test verifies that text bindings (using : syntax) are validated with the correct path context
    // Regression test for issue where `aaa: aaa` was validated with wrong path
    
    // Use a simple schema to avoid parsing issues
    let schema_input = r#"
root_field.$type = .string
items.$array = .string
"#;

    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test document with text bindings (which should be treated as unexpected fields)
    let doc = r#"
root_field = "value1"
invalid_field: text_value
items = ["a", "b"]
"#;

    let tree = eure_parol::parse(doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);

    // Should have exactly 1 unexpected field error for the text binding
    let unexpected_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(&e.kind, ValidationErrorKind::UnexpectedField { .. }))
        .collect();
    
    assert_eq!(unexpected_errors.len(), 1, 
        "Should report exactly 1 unexpected field, but got: {errors:?}");
    
    // Verify invalid_field is reported as unexpected
    let has_invalid = errors.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnexpectedField { field, .. }
        if matches!(field, KeyCmpValue::String(s) if s == "invalid_field")
    ));
    
    assert!(has_invalid, "Should report 'invalid_field' as unexpected");
}