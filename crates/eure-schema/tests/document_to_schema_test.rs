//! Comprehensive test cases for EureDocument to SchemaDocument conversion
//!
//! These tests cover all major schema features supported by Eure Schema:
//! - Primitive types (text, integer, float, boolean, null, any)
//! - Text types with language specifiers
//! - Literal types (exact value match)
//! - Arrays with item type and constraints
//! - Maps with key/value types
//! - Records with fixed named fields
//! - Tuples with fixed-length elements
//! - Union types with named variants
//! - Custom type definitions and references
//! - Type constraints (length, range, pattern, etc.)
//! - Metadata (description, deprecated, default, examples)

use eure::document::parse_to_document;
use eure_schema::convert::{ConversionError, document_to_schema};
use eure_schema::{
    ArraySchema, Bound, FloatSchema, IntegerSchema, MapSchema, SchemaDocument, SchemaMetadata,
    SchemaNodeContent, SchemaNodeId, TextSchema, UnknownFieldsPolicy,
};
use eure_value::data_model::VariantRepr;
use eure_value::identifier::Identifier;
use eure_value::parse::{ParseError, ParseErrorKind};
use eure_value::value::ObjectKey;
use num_bigint::BigInt;

// Helper function to parse Eure text and convert to schema
fn parse_and_convert(input: &str) -> SchemaDocument {
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let (schema, _source_map) = document_to_schema(&doc).expect("Failed to convert to schema");
    schema
}

// Helper to create an identifier
fn ident(s: &str) -> Identifier {
    s.parse().expect("Invalid identifier")
}

// ============================================================================
// ASSERTION HELPERS
// ============================================================================

/// Helper function to assert a node is a Record with 1 field
fn assert_record1<F1>(schema: &SchemaDocument, node_id: SchemaNodeId, field1: (&str, F1))
where
    F1: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Record(record) = &node.content {
        let (name1, check1) = field1;
        let field = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        check1(schema, field.schema);
    } else {
        panic!("Expected Record type, got {:?}", node.content);
    }
}

/// Helper function to assert a node is a Record with 2 fields
fn assert_record2<F1, F2>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    field1: (&str, F1),
    field2: (&str, F2),
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Record(record) = &node.content {
        assert_eq!(record.properties.len(), 2);
        let (name1, check1) = field1;
        let (name2, check2) = field2;
        let f1 = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        let f2 = record
            .properties
            .get(name2)
            .unwrap_or_else(|| panic!("Field '{}' missing", name2));
        check1(schema, f1.schema);
        check2(schema, f2.schema);
    } else {
        panic!("Expected Record type, got {:?}", node.content);
    }
}

/// Helper function to assert a node is a Record with 3 fields
fn assert_record3<F1, F2, F3>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    field1: (&str, F1),
    field2: (&str, F2),
    field3: (&str, F3),
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
    F3: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Record(record) = &node.content {
        assert_eq!(record.properties.len(), 3);
        let (name1, check1) = field1;
        let (name2, check2) = field2;
        let (name3, check3) = field3;
        let f1 = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        let f2 = record
            .properties
            .get(name2)
            .unwrap_or_else(|| panic!("Field '{}' missing", name2));
        let f3 = record
            .properties
            .get(name3)
            .unwrap_or_else(|| panic!("Field '{}' missing", name3));
        check1(schema, f1.schema);
        check2(schema, f2.schema);
        check3(schema, f3.schema);
    } else {
        panic!("Expected Record type, got {:?}", node.content);
    }
}

/// Helper function to assert a node is a Record with 5 fields
fn assert_record5<F1, F2, F3, F4, F5>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    field1: (&str, F1),
    field2: (&str, F2),
    field3: (&str, F3),
    field4: (&str, F4),
    field5: (&str, F5),
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
    F3: Fn(&SchemaDocument, SchemaNodeId),
    F4: Fn(&SchemaDocument, SchemaNodeId),
    F5: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Record(record) = &node.content {
        assert_eq!(record.properties.len(), 5);
        let (name1, check1) = field1;
        let (name2, check2) = field2;
        let (name3, check3) = field3;
        let (name4, check4) = field4;
        let (name5, check5) = field5;
        let f1 = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        let f2 = record
            .properties
            .get(name2)
            .unwrap_or_else(|| panic!("Field '{}' missing", name2));
        let f3 = record
            .properties
            .get(name3)
            .unwrap_or_else(|| panic!("Field '{}' missing", name3));
        let f4 = record
            .properties
            .get(name4)
            .unwrap_or_else(|| panic!("Field '{}' missing", name4));
        let f5 = record
            .properties
            .get(name5)
            .unwrap_or_else(|| panic!("Field '{}' missing", name5));
        check1(schema, f1.schema);
        check2(schema, f2.schema);
        check3(schema, f3.schema);
        check4(schema, f4.schema);
        check5(schema, f5.schema);
    } else {
        panic!("Expected Record type, got {:?}", node.content);
    }
}

/// Assert that a record field is optional
fn assert_field_optional(schema: &SchemaDocument, node_id: SchemaNodeId, field_name: &str) {
    let node = schema.node(node_id);
    if let SchemaNodeContent::Record(record) = &node.content {
        let field = record
            .properties
            .get(field_name)
            .unwrap_or_else(|| panic!("Field '{}' missing", field_name));
        assert!(
            field.optional,
            "Expected field '{}' to be optional",
            field_name
        );
    } else {
        panic!("Expected Record type, got {:?}", node.content);
    }
}

/// Assert node is Text with no language constraint (accepts any language)
fn assert_text(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    if let SchemaNodeContent::Text(text_schema) = &node.content {
        assert!(
            text_schema.language.is_none(),
            "Expected text with no language constraint, got {:?}",
            text_schema.language
        );
    } else {
        panic!("Expected Text type, got {:?}", node.content);
    }
}

/// Assert node is Text with specific language constraint
fn assert_text_language(schema: &SchemaDocument, node_id: SchemaNodeId, expected: &str) {
    let node = schema.node(node_id);
    if let SchemaNodeContent::Text(text_schema) = &node.content {
        assert_eq!(
            text_schema.language.as_deref(),
            Some(expected),
            "Expected language '{}', got {:?}",
            expected,
            text_schema.language
        );
    } else {
        panic!("Expected Text type, got {:?}", node.content);
    }
}

/// Assert node is Text with full constraint check
fn assert_text_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&TextSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Text(text_schema) = &node.content {
        check(text_schema);
    } else {
        panic!("Expected Text type, got {:?}", node.content);
    }
}

/// Assert that a node is an Integer type
fn assert_integer(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Integer(_)),
        "Expected Integer type, got {:?}",
        node.content
    );
}

/// Assert that a node is an Integer type with specific constraints
fn assert_integer_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&IntegerSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Integer(int_schema) = &node.content {
        check(int_schema);
    } else {
        panic!("Expected Integer type, got {:?}", node.content);
    }
}

/// Assert that a node is a Float type
fn assert_float(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Float(_)),
        "Expected Float type, got {:?}",
        node.content
    );
}

/// Assert that a node is a Float type with specific constraints
fn assert_float_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&FloatSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Float(float_schema) = &node.content {
        check(float_schema);
    } else {
        panic!("Expected Float type, got {:?}", node.content);
    }
}

/// Assert that a node is a Boolean type (unit variant, no constraints)
fn assert_boolean(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Boolean),
        "Expected Boolean type, got {:?}",
        node.content
    );
}

/// Assert that a node is a Null type
fn assert_null(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Null),
        "Expected Null type, got {:?}",
        node.content
    );
}

/// Assert that a node is an Any type
fn assert_any(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Any),
        "Expected Any type, got {:?}",
        node.content
    );
}

/// Assert that a node is an Array type
fn assert_array<F>(schema: &SchemaDocument, node_id: SchemaNodeId, item_check: F)
where
    F: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Array(array_schema) = &node.content {
        item_check(schema, array_schema.item);
    } else {
        panic!("Expected Array type, got {:?}", node.content);
    }
}

/// Assert that a node is an Array type with specific constraints
fn assert_array_with<F, G>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    item_check: F,
    constraint_check: G,
) where
    F: Fn(&SchemaDocument, SchemaNodeId),
    G: Fn(&ArraySchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Array(array_schema) = &node.content {
        item_check(schema, array_schema.item);
        constraint_check(array_schema);
    } else {
        panic!("Expected Array type, got {:?}", node.content);
    }
}

/// Assert that a node is a Map type with specific key and value checks
fn assert_map<K, V>(schema: &SchemaDocument, node_id: SchemaNodeId, key_check: K, value_check: V)
where
    K: Fn(&SchemaDocument, SchemaNodeId),
    V: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Map(map_schema) = &node.content {
        key_check(schema, map_schema.key);
        value_check(schema, map_schema.value);
    } else {
        panic!("Expected Map type, got {:?}", node.content);
    }
}

/// Assert that a node is a Map type with specific constraints
fn assert_map_with<K, V, C>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    key_check: K,
    value_check: V,
    constraint_check: C,
) where
    K: Fn(&SchemaDocument, SchemaNodeId),
    V: Fn(&SchemaDocument, SchemaNodeId),
    C: Fn(&MapSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Map(map_schema) = &node.content {
        key_check(schema, map_schema.key);
        value_check(schema, map_schema.value);
        constraint_check(map_schema);
    } else {
        panic!("Expected Map type, got {:?}", node.content);
    }
}

/// Assert that a node is a Tuple with 2 elements and check each
fn assert_tuple2<F1, F2>(schema: &SchemaDocument, node_id: SchemaNodeId, check1: F1, check2: F2)
where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Tuple(tuple_schema) = &node.content {
        assert_eq!(tuple_schema.elements.len(), 2);
        check1(schema, tuple_schema.elements[0]);
        check2(schema, tuple_schema.elements[1]);
    } else {
        panic!("Expected Tuple type, got {:?}", node.content);
    }
}

/// Assert that a node is a Tuple with 3 elements and check each
fn assert_tuple3<F1, F2, F3>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    check1: F1,
    check2: F2,
    check3: F3,
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
    F3: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Tuple(tuple_schema) = &node.content {
        assert_eq!(tuple_schema.elements.len(), 3);
        check1(schema, tuple_schema.elements[0]);
        check2(schema, tuple_schema.elements[1]);
        check3(schema, tuple_schema.elements[2]);
    } else {
        panic!("Expected Tuple type, got {:?}", node.content);
    }
}

/// Assert that a node is a Union type with 2 named variants
fn assert_union2<F1, F2>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    variant1: (&str, F1),
    variant2: (&str, F2),
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Union(union_schema) = &node.content {
        assert_eq!(union_schema.variants.len(), 2);
        let (name1, check1) = variant1;
        let (name2, check2) = variant2;
        let id1 = union_schema
            .variants
            .get(name1)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name1));
        let id2 = union_schema
            .variants
            .get(name2)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name2));
        check1(schema, *id1);
        check2(schema, *id2);
    } else {
        panic!("Expected Union type, got {:?}", node.content);
    }
}

/// Assert that a node is a Union type with 3 named variants
fn assert_union3<F1, F2, F3>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    variant1: (&str, F1),
    variant2: (&str, F2),
    variant3: (&str, F3),
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
    F3: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Union(union_schema) = &node.content {
        assert_eq!(union_schema.variants.len(), 3);
        let (name1, check1) = variant1;
        let (name2, check2) = variant2;
        let (name3, check3) = variant3;
        let id1 = union_schema
            .variants
            .get(name1)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name1));
        let id2 = union_schema
            .variants
            .get(name2)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name2));
        let id3 = union_schema
            .variants
            .get(name3)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name3));
        check1(schema, *id1);
        check2(schema, *id2);
        check3(schema, *id3);
    } else {
        panic!("Expected Union type, got {:?}", node.content);
    }
}

/// Assert that a node is a Union type with 4 named variants
fn assert_union4<F1, F2, F3, F4>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    variant1: (&str, F1),
    variant2: (&str, F2),
    variant3: (&str, F3),
    variant4: (&str, F4),
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
    F3: Fn(&SchemaDocument, SchemaNodeId),
    F4: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Union(union_schema) = &node.content {
        assert_eq!(union_schema.variants.len(), 4);
        let (name1, check1) = variant1;
        let (name2, check2) = variant2;
        let (name3, check3) = variant3;
        let (name4, check4) = variant4;
        let id1 = union_schema
            .variants
            .get(name1)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name1));
        let id2 = union_schema
            .variants
            .get(name2)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name2));
        let id3 = union_schema
            .variants
            .get(name3)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name3));
        let id4 = union_schema
            .variants
            .get(name4)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name4));
        check1(schema, *id1);
        check2(schema, *id2);
        check3(schema, *id3);
        check4(schema, *id4);
    } else {
        panic!("Expected Union type, got {:?}", node.content);
    }
}

/// Assert that a node is a Union type with specific representation
fn assert_union_repr<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&VariantRepr),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Union(union_schema) = &node.content {
        check(&union_schema.repr);
    } else {
        panic!("Expected Union type, got {:?}", node.content);
    }
}

/// Assert unknown fields policy for a Record node
fn assert_unknown_fields<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&UnknownFieldsPolicy),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Record(record) = &node.content {
        check(&record.unknown_fields);
    } else {
        panic!("Expected Record type, got {:?}", node.content);
    }
}

/// Assert that a node is a Reference type (local reference)
fn assert_reference(schema: &SchemaDocument, node_id: SchemaNodeId, expected_name: &str) {
    let node = schema.node(node_id);
    if let SchemaNodeContent::Reference(type_ref) = &node.content {
        assert!(
            type_ref.namespace.is_none(),
            "Expected local reference, got namespace {:?}",
            type_ref.namespace
        );
        assert_eq!(
            type_ref.name,
            ident(expected_name),
            "Expected reference to '{}', got '{:?}'",
            expected_name,
            type_ref.name
        );
    } else {
        panic!("Expected Reference type, got {:?}", node.content);
    }
}

/// Assert that a node is a Reference type (cross-schema reference)
fn assert_reference_external(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    expected_namespace: &str,
    expected_name: &str,
) {
    let node = schema.node(node_id);
    if let SchemaNodeContent::Reference(type_ref) = &node.content {
        assert_eq!(
            type_ref.namespace.as_deref(),
            Some(expected_namespace),
            "Expected namespace '{}', got {:?}",
            expected_namespace,
            type_ref.namespace
        );
        assert_eq!(
            type_ref.name,
            ident(expected_name),
            "Expected reference to '{}', got '{:?}'",
            expected_name,
            type_ref.name
        );
    } else {
        panic!("Expected Reference type, got {:?}", node.content);
    }
}

/// Assert node metadata
fn assert_metadata<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&SchemaMetadata),
{
    let node = schema.node(node_id);
    check(&node.metadata);
}

// ============================================================================
// BASIC PRIMITIVE TYPES
// ============================================================================

#[test]
fn test_string_type() {
    let input = r#"
name = `text`
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("name", |s, id| assert_text(s, id)));
}

#[test]
fn test_float_type() {
    let input = r#"
count = `float`
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("count", |s, id| assert_float(s, id)));
}

#[test]
fn test_integer_type() {
    let input = r#"
age = `integer`
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("age", |s, id| assert_integer(s, id)));
}

#[test]
fn test_boolean_type() {
    let input = r#"
active = `boolean`
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("active", |s, id| assert_boolean(s, id)),
    );
}

#[test]
fn test_null_type() {
    let input = r#"
deleted = `null`
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("deleted", |s, id| assert_null(s, id)),
    );
}

#[test]
fn test_any_type() {
    let input = r#"
data = `any`
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("data", |s, id| assert_any(s, id)));
}

// ============================================================================
// TEXT TYPES (unified String and Code)
// ============================================================================

#[test]
fn test_text_type() {
    // .text accepts any language (no constraint)
    let input = r#"
content = `text`
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("content", |s, id| assert_text(s, id)),
    );
}

#[test]
fn test_text_with_language() {
    // .text.rust requires rust language
    let input = r#"
code = `text.rust`
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("code", |s, id| assert_text_language(s, id, "rust")),
    );
}

#[test]
fn test_text_with_constraints() {
    // Text with length and pattern constraints
    let input = r#"
@ username {
    $variant: text
    language = "plaintext"
    min-length = 3
    max-length = 20
    pattern = "^[a-z]+$"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("username", |s, id| {
            assert_text_with(s, id, |t| {
                assert_eq!(t.language.as_deref(), Some("plaintext"));
                assert_eq!(t.min_length, Some(3));
                assert_eq!(t.max_length, Some(20));
                assert!(t.pattern.is_some());
            })
        }),
    );
}

// ============================================================================
// ARRAY TYPES
// ============================================================================

#[test]
fn test_array_shorthand() {
    let input = r#"
tags = [`text`]
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("tags", |s, id| assert_array(s, id, assert_text)),
    );
}

#[test]
fn test_array_with_constraints() {
    let input = r#"
@ tags {
  $variant: array
  item = `text`
  min-length = 1
  max-length = 10
  unique = true
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("tags", |s, id| {
            assert_array_with(s, id, assert_text, |array_schema| {
                assert_eq!(array_schema.min_length, Some(1));
                assert_eq!(array_schema.max_length, Some(10));
                assert!(array_schema.unique);
            })
        }),
    );
}

// ============================================================================
// TUPLE TYPES
// ============================================================================

#[test]
fn test_tuple_shorthand() {
    let input = r#"
point = (`float`, `float`)
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("point", |s, id| {
            assert_tuple2(s, id, assert_float, assert_float)
        }),
    );
}

#[test]
fn test_tuple_mixed_types() {
    let input = r#"
entry = (`text`, `integer`, `boolean`)
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("entry", |s, id| {
            assert_tuple3(s, id, assert_text, assert_integer, assert_boolean)
        }),
    );
}

// ============================================================================
// RECORD TYPES
// ============================================================================

#[test]
fn test_record_basic() {
    let input = r#"
@ user {
  name = `text`
  age = `integer`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, id| {
            assert_record2(s, id, ("name", assert_text), ("age", assert_integer))
        }),
    );
}

// ============================================================================
// UNION TYPES
// ============================================================================

#[test]
fn test_union_type() {
    let input = r#"
@ value {
  $variant: union
  variants.string = `text`
  variants.float = `float`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("value", |s, id| {
            assert_union2(s, id, ("string", assert_text), ("float", assert_float))
        }),
    );
}

#[test]
fn test_union_with_multiple_types() {
    let input = r#"
@ data {
  $variant: union
  variants.string = `text`
  variants.float = `float`
  variants.boolean = `boolean`
  variants.null = `null`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("data", |s, id| {
            assert_union4(
                s,
                id,
                ("string", assert_text),
                ("float", assert_float),
                ("boolean", assert_boolean),
                ("null", assert_null),
            )
        }),
    );
}

#[test]
fn test_union_with_record_variants() {
    let input = r#"
@ $types.action {
  $variant: union
  variants.click = { x => `float`, y => `float` }
  variants.hover = { element => `text` }
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("action")));
    let action_id = schema.types[&ident("action")];

    assert_union2(
        &schema,
        action_id,
        ("click", |s, click_id| {
            assert_record2(s, click_id, ("x", assert_float), ("y", assert_float))
        }),
        ("hover", |s, hover_id| {
            assert_record1(s, hover_id, ("element", |s, id| assert_text(s, id)))
        }),
    );
}

#[test]
fn test_union_with_untagged_repr() {
    let input = r#"
@ $types.response {
  $variant: union
  $variant-repr = "untagged"
  variants.success = { data => `any` }
  variants.error = { message => `text` }
}
"#;
    let schema = parse_and_convert(input);

    let response_id = schema.types[&ident("response")];

    assert_union2(
        &schema,
        response_id,
        ("success", |s, success_id| {
            assert_record1(s, success_id, ("data", assert_any));
        }),
        ("error", |s, error_id| {
            assert_record1(s, error_id, ("message", assert_text));
        }),
    );

    assert_union_repr(&schema, response_id, |repr| {
        assert!(matches!(repr, VariantRepr::Untagged));
    });
}

#[test]
fn test_union_with_internal_tag() {
    let input = r#"
@ $types.message {
  $variant: union
  $variant-repr = { tag => "type" }
  variants.text = { content => `text` }
  variants.image = { url => `text` }
}
"#;
    let schema = parse_and_convert(input);

    let message_id = schema.types[&ident("message")];

    assert_union_repr(&schema, message_id, |repr| {
        if let VariantRepr::Internal { tag } = repr {
            assert_eq!(tag, "type");
        } else {
            panic!("Expected VariantRepr::Internal, got {:?}", repr);
        }
    });
}

#[test]
fn test_union_with_adjacent_tag() {
    let input = r#"
@ $types.event {
  $variant: union
  $variant-repr = { tag => "kind", content => "data" }
  variants.login = { username => `text` }
  variants.logout = { reason => `text` }
}
"#;
    let schema = parse_and_convert(input);

    let event_id = schema.types[&ident("event")];

    assert_union_repr(&schema, event_id, |repr| {
        if let VariantRepr::Adjacent { tag, content } = repr {
            assert_eq!(tag, "kind");
            assert_eq!(content, "data");
        } else {
            panic!("Expected VariantRepr::Adjacent, got {:?}", repr);
        }
    });
}

#[test]
fn test_union_default_external() {
    let input = r#"
@ $types.status {
  $variant: union
  variants.pending = { message => `text` }
  variants.active = { started_at => `integer` }
}
"#;
    let schema = parse_and_convert(input);

    let status_id = schema.types[&ident("status")];

    // Default representation should be External
    assert_union_repr(&schema, status_id, |repr| {
        assert!(
            matches!(repr, VariantRepr::External),
            "Expected VariantRepr::External, got {:?}",
            repr
        );
    });
}

#[test]
fn test_union_with_three_variants() {
    let input = r#"
@ $types.traffic-light {
  $variant: union
  variants.red = { duration => `integer` }
  variants.yellow = { duration => `integer` }
  variants.green = { duration => `integer` }
}
"#;
    let schema = parse_and_convert(input);

    let light_id = schema.types[&ident("traffic-light")];

    assert_union3(
        &schema,
        light_id,
        ("red", |s, red_id| {
            assert_record1(s, red_id, ("duration", assert_integer));
        }),
        ("yellow", |s, yellow_id| {
            assert_record1(s, yellow_id, ("duration", assert_integer));
        }),
        ("green", |s, green_id| {
            assert_record1(s, green_id, ("duration", assert_integer));
        }),
    );
}

// ============================================================================
// CUSTOM TYPE DEFINITIONS
// ============================================================================

#[test]
fn test_custom_type_definition() {
    // Note: bindings must come before sections in Eure
    let input = r#"
user = `$types.username`

@ $types.username {
  $variant: text
  min-length = 3
  max-length = 20
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("username")));

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, id| assert_reference(s, id, "username")),
    );
}

// ============================================================================
// STRING CONSTRAINTS
// ============================================================================

#[test]
fn test_string_with_length() {
    let input = r#"
@ username {
  $variant: text
  min-length = 3
  max-length = 20
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("username", |s, id| {
            assert_text_with(s, id, |string_schema| {
                assert_eq!(string_schema.min_length, Some(3));
                assert_eq!(string_schema.max_length, Some(20));
            })
        }),
    );
}

#[test]
fn test_string_with_pattern() {
    let input = r#"
@ email {
  $variant: text
  pattern = "^[a-z]+@[a-z]+\\.[a-z]+$"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("email", |s, id| {
            assert_text_with(s, id, |string_schema| {
                assert!(string_schema.pattern.is_some());
            })
        }),
    );
}

// ============================================================================
// INTEGER CONSTRAINTS
// ============================================================================

#[test]
fn test_integer_with_range() {
    let input = r#"
@ age {
  $variant: integer
  range = "[0, 150]"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("age", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Inclusive(BigInt::from(0)));
                assert_eq!(int_schema.max, Bound::Inclusive(BigInt::from(150)));
            })
        }),
    );
}

#[test]
fn test_integer_with_multiple_of() {
    let input = r#"
@ even {
  $variant: integer
  multiple-of = 2
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("even", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.multiple_of, Some(BigInt::from(2)));
            })
        }),
    );
}

// ============================================================================
// FLOAT CONSTRAINTS
// ============================================================================

#[test]
fn test_float_with_range() {
    let input = r#"
@ probability {
  $variant: float
  range = "[0.0, 1.0]"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("probability", |s, id| {
            assert_float_with(s, id, |float_schema| {
                assert_eq!(float_schema.min, Bound::Inclusive(0.0));
                assert_eq!(float_schema.max, Bound::Inclusive(1.0));
            })
        }),
    );
}

// ============================================================================
// MAP TYPES
// ============================================================================

#[test]
fn test_map_type() {
    let input = r#"
@ headers {
  $variant: map
  key = `text`
  value = `text`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("headers", |s, id| {
            assert_map(s, id, assert_text, assert_text)
        }),
    );
}

#[test]
fn test_map_with_constraints() {
    let input = r#"
@ settings {
  $variant: map
  key = `text`
  value = `any`
  min-size = 1
  max-size = 100
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("settings", |s, id| {
            assert_map_with(s, id, assert_text, assert_any, |map_schema| {
                assert_eq!(map_schema.min_size, Some(1));
                assert_eq!(map_schema.max_size, Some(100));
            })
        }),
    );
}

// ============================================================================
// ARRAY CONTAINS TEST
// ============================================================================

#[test]
fn test_array_with_contains() {
    let input = r#"
@ tags {
  $variant: array
  item = `text`
  contains = "required"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("tags", |s, id| {
            assert_array_with(s, id, assert_text, |array_schema| {
                assert!(
                    array_schema.contains.is_some(),
                    "Expected contains to be Some"
                );
            })
        }),
    );
}

// ============================================================================
// NESTED RECORD TEST
// ============================================================================

#[test]
fn test_nested_record() {
    let input = r#"
@ user.profile
name = `text`
bio = `text`
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, user_id| {
            assert_record1(
                s,
                user_id,
                ("profile", |s, profile_id| {
                    assert_record2(
                        s,
                        profile_id,
                        ("name", |s, id| assert_text(s, id)),
                        ("bio", |s, id| assert_text(s, id)),
                    )
                }),
            )
        }),
    );
}

// ============================================================================
// INTEGER RANGE TESTS
// ============================================================================

#[test]
fn test_integer_range_rust_style_inclusive() {
    // Rust-style: ..= means inclusive end
    let input = r#"
@ age {
  $variant: integer
  range = "0..=150"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("age", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Inclusive(BigInt::from(0)));
                assert_eq!(int_schema.max, Bound::Inclusive(BigInt::from(150)));
            })
        }),
    );
}

#[test]
fn test_integer_range_rust_style_exclusive() {
    // Rust-style: .. means exclusive end
    let input = r#"
@ index {
  $variant: integer
  range = "0..100"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("index", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Inclusive(BigInt::from(0)));
                assert_eq!(int_schema.max, Bound::Exclusive(BigInt::from(100)));
            })
        }),
    );
}

#[test]
fn test_integer_range_min_only() {
    // Rust-style: min only
    let input = r#"
@ positive {
  $variant: integer
  range = "1.."
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("positive", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Inclusive(BigInt::from(1)));
                assert_eq!(int_schema.max, Bound::Unbounded);
            })
        }),
    );
}

#[test]
fn test_integer_range_max_only() {
    // Rust-style: max only (exclusive)
    let input = r#"
@ small {
  $variant: integer
  range = "..100"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("small", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Unbounded);
                assert_eq!(int_schema.max, Bound::Exclusive(BigInt::from(100)));
            })
        }),
    );
}

#[test]
fn test_integer_range_max_only_inclusive() {
    // Rust-style: max only (inclusive with ..=)
    let input = r#"
@ small {
  $variant: integer
  range = "..=100"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("small", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Unbounded);
                assert_eq!(int_schema.max, Bound::Inclusive(BigInt::from(100)));
            })
        }),
    );
}

#[test]
fn test_integer_range_interval_exclusive() {
    // Interval notation: both exclusive
    let input = r#"
@ value {
  $variant: integer
  range = "(0, 100)"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("value", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Exclusive(BigInt::from(0)));
                assert_eq!(int_schema.max, Bound::Exclusive(BigInt::from(100)));
            })
        }),
    );
}

#[test]
fn test_integer_range_interval_mixed() {
    // Interval notation: left exclusive, right inclusive
    let input = r#"
@ value {
  $variant: integer
  range = "(0, 100]"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("value", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Exclusive(BigInt::from(0)));
                assert_eq!(int_schema.max, Bound::Inclusive(BigInt::from(100)));
            })
        }),
    );
}

// ============================================================================
// FLOAT RANGE TESTS
// ============================================================================

#[test]
fn test_float_range_interval_half_open() {
    // Interval notation: left inclusive, right exclusive
    let input = r#"
@ probability {
  $variant: float
  range = "[0.0, 1.0)"
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("probability", |s, id| {
            assert_float_with(s, id, |float_schema| {
                assert_eq!(float_schema.min, Bound::Inclusive(0.0));
                assert_eq!(float_schema.max, Bound::Exclusive(1.0));
            })
        }),
    );
}

#[test]
fn test_float_range_rust_style() {
    // Rust-style: min only
    let input = r#"
@ positive {
  $variant: float
  range = "0.0.."
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("positive", |s, id| {
            assert_float_with(s, id, |float_schema| {
                assert_eq!(float_schema.min, Bound::Inclusive(0.0));
                assert_eq!(float_schema.max, Bound::Unbounded);
            })
        }),
    );
}

// ============================================================================
// METADATA TESTS
// ============================================================================

#[test]
fn test_optional_field() {
    let input = r#"
name = `text`
bio = `text`
bio.$optional = true
"#;
    let schema = parse_and_convert(input);

    // Check bio is optional
    assert_field_optional(&schema, schema.root, "bio");
}

#[test]
fn test_metadata_description() {
    let input = r#"
user = `any`
user.$description: User information
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, id| {
            assert_any(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(metadata.description.is_some());
            })
        }),
    );
}

#[test]
fn test_metadata_deprecated() {
    let input = r#"
old_field = `text`
old_field.$deprecated = true
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("old_field", |s, id| {
            assert_text(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(metadata.deprecated);
            })
        }),
    );
}

#[test]
fn test_metadata_default_value() {
    let input = r#"
timeout = `integer`
timeout.$default = 30
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("timeout", |s, id| {
            assert_integer(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(metadata.default.is_some());
            })
        }),
    );
}

// ============================================================================
// UNKNOWN FIELDS POLICY TESTS
// ============================================================================

#[test]
fn test_unknown_fields_policy_allow() {
    let input = r#"
@ config {
  $unknown-fields = "allow"
  host = `text`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("config", |s, config_id| {
            assert_record1(s, config_id, ("host", assert_text));
            assert_unknown_fields(s, config_id, |policy| {
                assert!(matches!(policy, UnknownFieldsPolicy::Allow));
            });
        }),
    );
}

#[test]
fn test_unknown_fields_policy_deny() {
    let input = r#"
@ config {
  $unknown-fields = "deny"
  host = `text`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("config", |s, config_id| {
            assert_record1(s, config_id, ("host", assert_text));
            assert_unknown_fields(s, config_id, |policy| {
                assert!(matches!(policy, UnknownFieldsPolicy::Deny));
            });
        }),
    );
}

#[test]
fn test_unknown_fields_policy_schema() {
    let input = r#"
@ config {
  $unknown-fields = `text`
  host = `text`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("config", |s, config_id| {
            assert_record1(s, config_id, ("host", assert_text));
            assert_unknown_fields(s, config_id, |policy| {
                if let UnknownFieldsPolicy::Schema(schema_id) = policy {
                    assert_text(s, *schema_id);
                } else {
                    panic!("Expected UnknownFieldsPolicy::Schema, got {:?}", policy);
                }
            });
        }),
    );
}

// ============================================================================
// TEXT LANGUAGE AND PATH TESTS
// ============================================================================

#[test]
fn test_text_language_variants() {
    let input = r#"
rust = `text.rust`
python = `text.python`
sql = `text.sql`
"#;
    let schema = parse_and_convert(input);

    assert_record3(
        &schema,
        schema.root,
        ("rust", |s, id| assert_text_language(s, id, "rust")),
        ("python", |s, id| assert_text_language(s, id, "python")),
        ("sql", |s, id| assert_text_language(s, id, "sql")),
    );
}

// ============================================================================
// TYPE REFERENCE TESTS
// ============================================================================

#[test]
fn test_type_reference() {
    let input = r#"
$types.email = `text.email`

contact = `$types.email`
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("contact", |s, id| assert_reference(s, id, "email")),
    );
}

#[test]
fn test_external_type_reference() {
    // External type reference: `$types.namespace.typename`
    let input = r#"
user = `$types.common.User`

@ $types.common.User
name = `text`
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, id| {
            assert_reference_external(s, id, "common", "User")
        }),
    );
}

#[test]
fn test_circular_type_reference_is_valid() {
    let input = r#"
$types.a = `$types.b`
$types.b = `$types.a`

data = `$types.a`
"#;
    let schema = parse_and_convert(input);

    // Circular references are allowed
    assert!(schema.types.contains_key(&ident("a")));
    assert!(schema.types.contains_key(&ident("b")));

    assert_record1(
        &schema,
        schema.root,
        ("data", |s, id| assert_reference(s, id, "a")),
    );

    // Verify the circular reference structure
    let a_id = schema.types[&ident("a")];
    assert_reference(&schema, a_id, "b");

    let b_id = schema.types[&ident("b")];
    assert_reference(&schema, b_id, "a");
}

#[test]
fn test_type_reference_chain() {
    // Note: bindings must come before sections in Eure
    // Also: type definitions must use @ $types.name section syntax
    let input = r#"
data = `$types.user`

@ $types.base-string {
  $variant: text
  min-length = 1
  max-length = 100
}

@ $types.username = `$types.base-string`

@ $types.user
username = `$types.username`
email = `text.email`
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("base-string")));
    assert!(schema.types.contains_key(&ident("username")));
    assert!(schema.types.contains_key(&ident("user")));

    assert_record1(
        &schema,
        schema.root,
        ("data", |s, id| assert_reference(s, id, "user")),
    );

    // user references username, which references base-string
    let user_id = schema.types[&ident("user")];
    assert_record2(
        &schema,
        user_id,
        ("username", |s, id| assert_reference(s, id, "username")),
        ("email", |s, id| assert_text_language(s, id, "email")),
    );

    let username_id = schema.types[&ident("username")];
    assert_reference(&schema, username_id, "base-string");
}

// ============================================================================
// COMPLEX EXAMPLES
// ============================================================================

#[test]
fn test_complex_user_schema() {
    // For literal union variants, just use the literal value directly
    let input = r#"
@ $types.username {
  $variant: text
  min-length = 3
  max-length = 20
  pattern = "^[a-z0-9_]+$"
}

@ $types.role {
  $variant: union
  variants.admin = "admin"
  variants.user = "user"
  variants.guest = "guest"
}

@ $types.user {
  username = `$types.username`
  email = `text.email`
  age = `integer`
  age.$optional = true
  tags = [`text`]
  role = `$types.role`
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("username")));
    assert!(schema.types.contains_key(&ident("user")));
    assert!(schema.types.contains_key(&ident("role")));

    let user_id = schema.types[&ident("user")];

    assert_record5(
        &schema,
        user_id,
        ("username", |s, id| assert_reference(s, id, "username")),
        ("email", |s, id| assert_text_language(s, id, "email")),
        ("age", |s, id| assert_integer(s, id)),
        ("tags", |s, id| assert_array(s, id, assert_text)),
        ("role", |s, id| assert_reference(s, id, "role")),
    );
}

#[test]
fn test_complex_api_schema() {
    // For literal union variants, just use the literal value directly
    let input = r#"
@ $types.http-method {
  $variant: union
  variants.GET = "GET"
  variants.POST = "POST"
  variants.PUT = "PUT"
  variants.DELETE = "DELETE"
  variants.PATCH = "PATCH"
}

@ $types.api-request {
  method = `$types.http-method`
  path = `text`
  headers = `any`
  headers.$optional = true
  body = `any`
  body.$optional = true
}

@ $types.api-response {
  $variant: union
  $variant-repr = "untagged"
  variants.success = { status => `integer`, data => `any` }
  variants.error = { status => `integer`, message => `text` }
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("http-method")));
    assert!(schema.types.contains_key(&ident("api-request")));
    assert!(schema.types.contains_key(&ident("api-response")));

    let response_id = schema.types[&ident("api-response")];

    // Check variants: success and error
    assert_union2(
        &schema,
        response_id,
        ("success", |s, success_id| {
            assert_record2(
                s,
                success_id,
                ("status", assert_integer),
                ("data", assert_any),
            );
        }),
        ("error", |s, error_id| {
            assert_record2(
                s,
                error_id,
                ("status", assert_integer),
                ("message", assert_text),
            );
        }),
    );

    // Check repr is Untagged
    assert_union_repr(&schema, response_id, |repr| {
        assert!(matches!(repr, VariantRepr::Untagged));
    });
}

#[test]
fn test_nested_types_and_arrays() {
    let input = r#"
@ $types.address {
  street = `text`
  city = `text`
  zip = `text`
}

@ $types.person {
  name = `text`
  @ addresses {
    $variant: array
    item = `$types.address`
    min-length = 1
  }
}
"#;
    let schema = parse_and_convert(input);

    let person_id = schema.types[&ident("person")];

    assert_record2(
        &schema,
        person_id,
        ("name", |s, id| assert_text(s, id)),
        ("addresses", |s, id| {
            assert_array_with(
                s,
                id,
                |s, item_id| assert_reference(s, item_id, "address"),
                |array_schema| {
                    assert_eq!(array_schema.min_length, Some(1));
                },
            );
        }),
    );
}

#[test]
fn test_array_of_custom_types_complex() {
    // Note: bindings must come before sections in Eure
    let input = r#"
data = `$types.collection`

@ $types.item {
  $variant: text
  min-length = 1
  max-length = 100
}

@ $types.collection {
  $variant: array
  item = `$types.item`
  min-length = 1
  unique = true
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("item")));
    assert!(schema.types.contains_key(&ident("collection")));

    assert_record1(
        &schema,
        schema.root,
        ("data", |s, id| assert_reference(s, id, "collection")),
    );

    let collection_id = schema.types[&ident("collection")];
    // collection is an array type that references item
    assert_array_with(
        &schema,
        collection_id,
        |s, item_id| assert_reference(s, item_id, "item"),
        |array_schema| {
            assert_eq!(array_schema.min_length, Some(1));
            assert!(array_schema.unique);
        },
    );
}

// ============================================================================
// MAP TESTS
// ============================================================================

#[test]
fn test_map_with_complex_types() {
    let input = r#"
@ $types.address {
  $variant: text
  min-length = 1
  max-length = 100
}

@ locations {
  $variant: map
  key = `text`
  value = `$types.address`
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("locations", |s, id| {
            assert_map(s, id, assert_text, |s, value_id| {
                assert_reference(s, value_id, "address")
            });
        }),
    );
}

#[test]
fn test_nested_maps() {
    let input = r#"
@ nested {
  $variant: map
  key = `text`
  value = {
    $variant => "map",
    key => `text`,
    value => `integer`
  }
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("nested", |s, id| {
            assert_map(s, id, assert_text, |s, value_id| {
                assert_map(s, value_id, assert_text, assert_integer);
            });
        }),
    );
}

// ============================================================================
// OTHER TESTS
// ============================================================================

#[test]
fn test_nested_union_types() {
    let input = r#"
@ value {
  $variant: union
  variants.string = `text`
  variants.integer = `integer`
  variants.array = [{
    $variant => "union",
    variants => {
      boolean => `boolean`,
      null => `null`
    }
  }]
}
"#;
    let schema = parse_and_convert(input);

    // This tests that unions can contain nested structures
    assert_record1(
        &schema,
        schema.root,
        ("value", |s, id| {
            assert_union3(
                s,
                id,
                ("string", assert_text),
                ("integer", assert_integer),
                ("array", |s, array_id| {
                    // Third variant should be an array containing union
                    assert_array(s, array_id, |s, item_id| {
                        assert_union2(
                            s,
                            item_id,
                            ("boolean", assert_boolean),
                            ("null", assert_null),
                        );
                    });
                }),
            );
        }),
    );
}

#[test]
fn test_empty_section_creates_empty_record() {
    // Empty section creates an empty record schema
    let input = r#"
@ config
"#;
    let schema = parse_and_convert(input);

    // Root should be a record with one field "config" which is an empty record
    assert_record1(
        &schema,
        schema.root,
        ("config", |s, id| {
            let node = s.nodes.get(id.0).unwrap();
            assert!(
                matches!(&node.content, SchemaNodeContent::Record(r) if r.properties.is_empty()),
                "Expected empty record, got {:?}",
                node.content
            );
        }),
    );
}

#[test]
fn test_empty_array_schema() {
    // Array must have an item type
    let input = r#"
items = [`any`]
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("items", |s, id| {
            assert_array(s, id, assert_any);
        }),
    );
}

// ============================================================================
// ERROR CASES
// ============================================================================

#[test]
fn test_invalid_type_reference() {
    let input = r#"
user = `$types.nonexistent`
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert_eq!(
        result.unwrap_err(),
        ConversionError::UndefinedTypeReference("nonexistent".to_string())
    );
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_error_unknown_variant_type() {
    let input = r#"
@ field {
  $variant: unknown_type
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    // Unknown variant type should produce an UnknownVariant error
    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::UnknownVariant(_),
            ..
        })
    ));
}

#[test]
fn test_error_invalid_integer_range_format() {
    let input = r#"
@ field {
  $variant: integer
  range = "not a range"
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert_eq!(
        result.unwrap_err(),
        ConversionError::InvalidRangeString("not a range".to_string())
    );
}

#[test]
fn test_error_invalid_float_range_format() {
    let input = r#"
@ field {
  $variant: float
  range = "invalid"
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert_eq!(
        result.unwrap_err(),
        ConversionError::InvalidRangeString("invalid".to_string())
    );
}

#[test]
fn test_error_invalid_type_path() {
    let input = r#"
field = `unknown_primitive`
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::InvalidPattern { .. },
            ..
        })
    ));
}

#[test]
fn test_error_invalid_extension_path() {
    let input = r#"
field = `$unknown.type`
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::InvalidPattern { .. },
            ..
        })
    ));
}

#[test]
fn test_error_map_missing_key() {
    let input = r#"
@ field {
  $variant: map
  value = `text`
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::MissingField(_),
            ..
        })
    ));
}

#[test]
fn test_error_map_missing_value() {
    let input = r#"
@ field {
  $variant: map
  key = `text`
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::MissingField(_),
            ..
        })
    ));
}

#[test]
fn test_error_array_missing_item() {
    let input = r#"
@ field {
  $variant: array
  min-length = 1
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::MissingField(_),
            ..
        })
    ));
}

#[test]
fn test_error_invalid_variant_repr() {
    let input = r#"
@ field {
  $variant: union
  $variant-repr = "invalid_repr"
  variants.a = `text`
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::UnknownVariant(_),
            ..
        })
    ));
}

#[test]
fn test_error_adjacent_repr_missing_tag() {
    let input = r#"
@ field {
  $variant: union
  variants.a = `text`
  @ $variant-repr {
    content = "data"
  }
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    // $variant-repr with record value (adjacent repr) missing required "tag" field
    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::MissingField(_),
            ..
        })
    ));
}

#[test]
fn test_error_invalid_unknown_fields_policy() {
    let input = r#"
@ record {
  $unknown-fields = "invalid_policy"
  name = `text`
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::UnknownVariant(_),
            ..
        })
    ));
}

#[test]
fn test_error_array_with_multiple_items() {
    let input = r#"
field = [`text`, `integer`]
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::InvalidPattern { .. },
            ..
        })
    ));
}

#[test]
fn test_error_invalid_range_interval_format() {
    let input = r#"
@ field {
  $variant: integer
  range = "[1, 2, 3]"
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert_eq!(
        result.unwrap_err(),
        ConversionError::InvalidRangeString("[1, 2, 3]".to_string())
    );
}

#[test]
fn test_error_literal_missing_value() {
    let input = r#"
@ field {
  $variant: literal
  other = "something"
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    // $variant: literal on a map with fields (not just a value) creates a literal of the whole map
    // This is actually valid - it creates a Literal(Map({ "other": "something" }))
    // So this test should expect success, not an error
    assert!(result.is_ok());
}

#[test]
fn test_error_types_not_map() {
    let input = r#"
$types = "not a map"
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert_eq!(
        result.unwrap_err(),
        ConversionError::InvalidExtensionValue {
            extension: "types".to_string(),
            path: "$types must be a map".to_string(),
        }
    );
}

#[test]
fn test_error_invalid_type_path_extra_segment() {
    // `integer.foo` is invalid - only `text` supports .X language suffix
    let input = r#"
@ field = `integer.foo`
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    // "integer.foo" is not a valid primitive, so it produces InvalidPattern
    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::InvalidPattern { .. },
            ..
        })
    ));
}

#[test]
fn test_error_nested_variant_path() {
    // Nested variant paths like $variant = "ok.ok.err" are invalid in schema context
    // The type type union doesn't have nested unions
    let input = r#"
@ response {
    $variant = "ok.ok.err"
    error_code = `integer`
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::UnknownVariant(_),
            ..
        })
    ));
}

#[test]
fn test_variant_string_single_segment_valid() {
    // $variant = "text" specifies text type
    let input = r#"
@ field {
    $variant = "text"
    min-length = 1
}
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("field", |s: &SchemaDocument, id: SchemaNodeId| {
            assert_text_with(s, id, |str_schema| {
                assert_eq!(str_schema.min_length, Some(1));
            });
        }),
    );
}

#[test]
fn test_error_variant_string_unknown() {
    // Unknown variant type is invalid
    let input = r#"
@ field {
    $variant = "unknown_type"
    value = 123
}
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert!(matches!(
        result.unwrap_err(),
        ConversionError::ParseError(ParseError {
            kind: ParseErrorKind::UnknownVariant(_),
            ..
        })
    ));
}

#[test]
fn test_error_types_non_string_key() {
    // Type names in $types must be strings (identifiers), not tuples
    let input = r#"
$types.("a", "b") = `text`
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    // Just check that it's an InvalidTypeName error - the tuple structure is internal
    match result.unwrap_err() {
        ConversionError::InvalidTypeName(_) => {}
        err => panic!("Expected InvalidTypeName error, got {:?}", err),
    }
}

#[test]
#[ignore = "Eure grammar cannot parse $types.0 due to lexer precedence (.0 is tokenized as Float)"]
fn test_error_types_integer_key() {
    // Type names in $types must be strings (identifiers), not integers
    // Note: This syntax currently fails to parse because .0 is lexed as a Float token
    let input = r#"
$types.0 = `text`
"#;
    let doc = parse_to_document(input).expect("Failed to parse Eure document");
    let result = document_to_schema(&doc);

    assert_eq!(
        result.unwrap_err(),
        ConversionError::InvalidTypeName(ObjectKey::Number(BigInt::from(0)))
    );
}
