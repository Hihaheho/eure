use eure_schema::{document_to_schema, validate_document, ValidationErrorKind, KeyCmpValue};
use eure_tree::value_visitor::ValueVisitor;

// ============================================================================
// Tagged Variant Representation Tests
// ============================================================================

#[test]
fn test_tagged_variant_basic() {
    let schema_input = r#"
$types.Command {
  @ $variants.echo {
    message = .string
  }
  @ $variants.run {
    command = .string
    args.$array = .string
    args.$optional = true
  }
}

commands.$array = .$types.Command
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test 1: Valid tagged variant
    let valid_doc = r#"
commands = []
@ commands[] {
  echo {
    message = "Hello, World!"
  }
}
"#;
    
    let tree = eure_parol::parse(valid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(valid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid tagged variant should have no errors, but got: {errors:?}");
}

#[test]
fn test_tagged_variant_with_extension() {
    let schema_input = r#"
$types.Command {
  @ $variants.echo {
    message = .string
  }
  @ $variants.run {
    command = .string
  }
}

commands.$array = .$types.Command
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Tagged variant with $variant extension (alternative syntax)
    let doc_with_extension = r#"
commands = []
@ commands[] {
  $variant: echo
  message = "Hello from extension syntax!"
}
"#;
    
    let tree = eure_parol::parse(doc_with_extension).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc_with_extension);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Tagged variant with $variant extension should work, but got: {errors:?}");
}

#[test]
fn test_tagged_variant_multiple_keys_error() {
    let schema_input = r#"
$types.Command {
  @ $variants.echo {
    message = .string
  }
  @ $variants.run {
    command = .string
  }
}

commands.$array = .$types.Command
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Tagged variant with multiple keys (should fail)
    let invalid_doc = r#"
commands = []
@ commands[] {
  echo {
    message = "Hello"
  }
  run {
    command = "ls"
  }
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert!(!errors.is_empty(), "Tagged variant with multiple keys should produce errors");
}

// ============================================================================
// Internally Tagged Variant Representation Tests
// ============================================================================

#[test]
fn test_internally_tagged_variant_basic() {
    let schema_input = r#"
$types.Event {
  $variant-repr = { tag = "type" }
  @ $variants.click {
    x = .number
    y = .number
  }
  @ $variants.keypress {
    key = .string
    shift = .boolean
    shift.$optional = true
  }
}

events.$array = .$types.Event
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test 1: Valid internally tagged variant
    let valid_doc = r#"
events = []
@ events[] {
  type = "click"
  x = 100
  y = 200
}
@ events[] {
  type = "keypress"
  key = "Enter"
  shift = true
}
"#;
    
    let tree = eure_parol::parse(valid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(valid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid internally tagged variants should have no errors, but got: {errors:?}");
}

#[test]
fn test_internally_tagged_missing_tag() {
    let schema_input = r#"
$types.Event {
  $variant-repr = { tag = "type" }
  @ $variants.click {
    x = .number
    y = .number
  }
}

events.$array = .$types.Event
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Missing tag field
    let invalid_doc = r#"
events = []
@ events[] {
  x = 100
  y = 200
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert!(!errors.is_empty(), "Missing tag field should produce errors");
}

#[test]
fn test_internally_tagged_invalid_tag_value() {
    let schema_input = r#"
$types.Event {
  $variant-repr = { tag = "type" }
  @ $variants.click {
    x = .number
    y = .number
  }
}

events.$array = .$types.Event
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Invalid tag value
    let invalid_doc = r#"
events = []
@ events[] {
  type = "invalid_variant"
  x = 100
  y = 200
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    let has_unknown_variant = errors.iter().any(|e| matches!(&e.kind,
        ValidationErrorKind::UnknownVariant { variant, .. }
        if variant == "invalid_variant"
    ));
    
    assert!(has_unknown_variant, "Invalid tag value should produce UnknownVariant error");
}

// ============================================================================
// Adjacently Tagged Variant Representation Tests
// ============================================================================

#[test]
fn test_adjacently_tagged_variant_basic() {
    let schema_input = r#"
$types.Message {
  $variant-repr = { tag = "kind", content = "data" }
  @ $variants.text {
    content = .string
    formatted = .boolean
    formatted.$optional = true
  }
  @ $variants.image {
    url = .string
    alt = .string
    alt.$optional = true
  }
}

messages.$array = .$types.Message
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Valid adjacently tagged variant
    let valid_doc = r#"
messages = []
@ messages[] {
  kind = "text"
  data = {
    content = "Hello, World!"
    formatted = true
  }
}
@ messages[] {
  kind = "image"
  data = {
    url = "https://example.com/image.png"
    alt = "Example image"
  }
}
"#;
    
    let tree = eure_parol::parse(valid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(valid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid adjacently tagged variants should have no errors, but got: {errors:?}");
}

#[test]
fn test_adjacently_tagged_missing_content() {
    let schema_input = r#"
$types.Message {
  $variant-repr = { tag = "kind", content = "data" }
  @ $variants.text {
    content = .string
  }
}

messages.$array = .$types.Message
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Missing content field
    let invalid_doc = r#"
messages = []
@ messages[] {
  kind = "text"
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert!(!errors.is_empty(), "Missing content field should produce errors");
}

// ============================================================================
// Untagged Variant Representation Tests
// ============================================================================

#[test]
fn test_untagged_variant_basic() {
    let schema_input = r#"
$types.Value {
  @ $variants.text {
    text = .string
    lang = .string
    lang.$optional = true
  }
  @ $variants.number {
    value = .number
    unit = .string
    unit.$optional = true
  }
  @ $variants.bool {
    flag = .boolean
  }
  @ $variant-repr = "untagged"
}

values.$array = .$types.Value
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Valid untagged variants (determined by structure)
    let valid_doc = r#"
values = []
@ values[] {
  text = "Hello"
  lang = "en"
}
@ values[] {
  value = 42
  unit = "meters"
}
@ values[] {
  flag = true
}
"#;
    
    let tree = eure_parol::parse(valid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(valid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Valid untagged variants should have no errors, but got: {errors:?}");
}

#[test]
fn test_untagged_variant_ambiguous() {
    let schema_input = r#"
$types.Ambiguous {
  $variant-repr = "untagged"
  @ $variants.a {
    field1 = .string
    field2 = .string
    field2.$optional = true
  }
  @ $variants.b {
    field1 = .string
    field3 = .string
    field3.$optional = true
  }
}

items.$array = .$types.Ambiguous
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Ambiguous untagged variant (matches first valid)
    let doc = r#"
items = []
@ items[] {
  field1 = "value"
}
"#;
    
    let tree = eure_parol::parse(doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    // Should match first variant that fits
    assert_eq!(errors.len(), 0, "Should match first valid variant, but got: {errors:?}");
}

#[test]
fn test_untagged_variant_no_match() {
    let schema_input = r#"
$types.Value {
  $variant-repr = "untagged"
  @ $variants.text {
    text = .string
  }
  @ $variants.number {
    value = .number
  }
}

values.$array = .$types.Value
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: No matching variant
    let invalid_doc = r#"
values = []
@ values[] {
  unknown_field = "value"
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert!(!errors.is_empty(), "No matching variant should produce errors");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_variant() {
    let schema_input = r#"
$types.Empty {
  @ $variants.empty {
  }
  @ $variants.with_optional {
    field = .string
    field.$optional = true
  }
}

items.$array = .$types.Empty
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Empty variant
    let doc = r#"
items = []
@ items[] {
  $variant: empty
}
@ items[] {
  $variant: with_optional
}
"#;
    
    let tree = eure_parol::parse(doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Empty variants should be valid, but got: {errors:?}");
}

#[test]
fn test_deeply_nested_variants() {
    let schema_input = r#"
$types.Level3 {
  @ $variants.leaf {
    value = .string
  }
}

$types.Level2 {
  @ $variants.branch {
    nested = .$types.Level3
  }
}

$types.Level1 {
  @ $variants.root {
    child = .$types.Level2
  }
}

item.$type = .$types.Level1
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Deeply nested variants
    let doc = r#"
item = {
  $variant = "root"
  child = {
    $variant = "branch"
    nested = {
      $variant = "leaf"
      value = "deep value"
    }
  }
}
"#;
    
    let tree = eure_parol::parse(doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Deeply nested variants should validate correctly, but got: {errors:?}");
}

#[test]
fn test_variant_with_complex_types() {
    let schema_input = r#"
$types.ComplexVariant {
  @ $variants.with_array {
    items.$array = .string
    matrix.$array.$array = .number
  }
  @ $variants.with_map {
    config = .object
    metadata = { key = .string, value = .any }
  }
  @ $variants.with_union {
    value.$union[] = .string
    value.$union[] = .number
    value.$union[] = .boolean
  }
}

complex.$type = .$types.ComplexVariant
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Variant with arrays
    let doc_array = r#"
complex = {
  $variant = "with_array"
  items = ["a", "b", "c"]
  matrix = [[1, 2], [3, 4]]
}
"#;
    
    let tree = eure_parol::parse(doc_array).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc_array);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Variant with complex array types should validate, but got: {errors:?}");
}

// ============================================================================
// Error Condition Tests
// ============================================================================

#[test]
fn test_variant_field_type_mismatch() {
    let schema_input = r#"
$types.Typed {
  @ $variants.strict {
    text = .string
    number = .number
    flag = .boolean
  }
}

item.$type = .$types.Typed
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Type mismatches in variant fields
    let invalid_doc = r#"
item = {
  $variant = "strict"
  text = 123
  number = "not a number"
  flag = "not a boolean"
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    // Should have 3 type mismatch errors
    let type_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(&e.kind, ValidationErrorKind::TypeMismatch { .. }))
        .collect();
    
    assert_eq!(type_errors.len(), 3, "Should have 3 type mismatch errors, but got: {errors:?}");
}

#[test]
fn test_variant_required_field_missing() {
    let schema_input = r#"
$types.Required {
  @ $variants.needs_all {
    field1 = .string
    field2 = .number
    field3 = .boolean
  }
}

item.$type = .$types.Required
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Missing required fields
    let invalid_doc = r#"
item = {
  $variant = "needs_all"
  field1 = "present"
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    // Should have errors for missing field2 and field3
    let missing_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(&e.kind, ValidationErrorKind::RequiredFieldMissing { .. }))
        .collect();
    
    assert_eq!(missing_errors.len(), 2, "Should have 2 missing field errors, but got: {errors:?}");
}

#[test]
fn test_variant_unexpected_fields() {
    let schema_input = r#"
$types.Strict {
  @ $variants.defined {
    allowed_field = .string
  }
}

item.$type = .$types.Strict
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Unexpected fields in variant
    let invalid_doc = r#"
item = {
  $variant = "defined"
  allowed_field = "ok"
  unexpected1 = "should error"
  unexpected2 = 123
}
"#;
    
    let tree = eure_parol::parse(invalid_doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(invalid_doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    // Should have 2 unexpected field errors
    let unexpected_errors: Vec<_> = errors.iter()
        .filter(|e| matches!(&e.kind, ValidationErrorKind::UnexpectedField { .. }))
        .collect();
    
    assert_eq!(unexpected_errors.len(), 2, "Should have 2 unexpected field errors, but got: {errors:?}");
}

#[test]
fn test_variant_cascade_type_interaction() {
    let schema_input = r#"
$types.WithCascade {
  @ $variants.variant1 {
    field = .string
  }
  @ $variants.variant2 {
    field = .number
  }
}

$cascade-type.items[] = .$types.WithCascade

items.$array = .object
"#;
    
    let schema_tree = eure_parol::parse(schema_input).expect("Failed to parse schema");
    let mut schema_visitor = ValueVisitor::new(schema_input);
    schema_tree.visit_from_root(&mut schema_visitor).expect("Failed to visit schema tree");
    let schema_doc = schema_visitor.into_document();
    let schema = document_to_schema(&schema_doc).expect("Failed to extract schema");
    
    // Test: Variant with cascade type
    let doc = r#"
items = []
@ items[] {
  $variant: variant1
  field = "text"
}
@ items[] {
  $variant: variant2
  field = 42
}
"#;
    
    let tree = eure_parol::parse(doc).expect("Failed to parse document");
    let mut visitor = ValueVisitor::new(doc);
    tree.visit_from_root(&mut visitor).expect("Failed to visit tree");
    let document = visitor.into_document();
    let errors = validate_document(&document, &schema);
    
    assert_eq!(errors.len(), 0, "Variants with cascade types should validate correctly, but got: {errors:?}");
}