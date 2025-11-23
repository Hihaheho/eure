//! Comprehensive test cases for EureDocument to SchemaDocument conversion
//!
//! These tests cover all major schema features supported by EURE Schema:
//! - Primitive types
//! - Code types
//! - Arrays and collections
//! - Records and objects
//! - Union types
//! - Variant types (tagged unions)
//! - Custom type definitions
//! - Type constraints
//! - Metadata and annotations

use eure::document::parse_to_document;
use eure_schema::convert::document_to_schema;
use eure_schema::{
    ArraySchema, BooleanSchema, Bound, CodeSchema, FloatSchema, IntegerSchema, MapSchema,
    PathSchema, SchemaDocument, SchemaMetadata, SchemaNodeContent, SchemaNodeId, StringSchema,
    TupleSchema, UnknownFieldsPolicy,
};
use eure_value::data_model::VariantRepr;
use eure_value::identifier::Identifier;
use num_bigint::BigInt;

// Helper function to parse EURE text and convert to schema
fn parse_and_convert(input: &str) -> SchemaDocument {
    let doc = parse_to_document(input).expect("Failed to parse EURE document");
    document_to_schema(&doc).expect("Failed to convert to schema")
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
        let id1 = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        check1(schema, *id1);
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
        let id1 = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        let id2 = record
            .properties
            .get(name2)
            .unwrap_or_else(|| panic!("Field '{}' missing", name2));
        check1(schema, *id1);
        check2(schema, *id2);
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
        let id1 = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        let id2 = record
            .properties
            .get(name2)
            .unwrap_or_else(|| panic!("Field '{}' missing", name2));
        let id3 = record
            .properties
            .get(name3)
            .unwrap_or_else(|| panic!("Field '{}' missing", name3));
        check1(schema, *id1);
        check2(schema, *id2);
        check3(schema, *id3);
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
        let id1 = record
            .properties
            .get(name1)
            .unwrap_or_else(|| panic!("Field '{}' missing", name1));
        let id2 = record
            .properties
            .get(name2)
            .unwrap_or_else(|| panic!("Field '{}' missing", name2));
        let id3 = record
            .properties
            .get(name3)
            .unwrap_or_else(|| panic!("Field '{}' missing", name3));
        let id4 = record
            .properties
            .get(name4)
            .unwrap_or_else(|| panic!("Field '{}' missing", name4));
        let id5 = record
            .properties
            .get(name5)
            .unwrap_or_else(|| panic!("Field '{}' missing", name5));
        check1(schema, *id1);
        check2(schema, *id2);
        check3(schema, *id3);
        check4(schema, *id4);
        check5(schema, *id5);
    } else {
        panic!("Expected Record type, got {:?}", node.content);
    }
}

/// Assert that a node is a String type
fn assert_string(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::String(_)),
        "Expected String type, got {:?}",
        node.content
    );
}

/// Assert that a node is a String type with specific constraints
fn assert_string_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&StringSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::String(string_schema) = &node.content {
        check(string_schema);
    } else {
        panic!("Expected String type, got {:?}", node.content);
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

/// Assert that a node is a Boolean type with specific constraints
fn assert_boolean_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&BooleanSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Boolean(bool_schema) = &node.content {
        check(bool_schema);
    } else {
        panic!("Expected Boolean type, got {:?}", node.content);
    }
}

/// Assert that a node is a Code type with specific constraints
fn assert_code_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&CodeSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Code(code_schema) = &node.content {
        check(code_schema);
    } else {
        panic!("Expected Code type, got {:?}", node.content);
    }
}

/// Assert that a node is a Path type with specific constraints
fn assert_path_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&PathSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Path(path_schema) = &node.content {
        check(path_schema);
    } else {
        panic!("Expected Path type, got {:?}", node.content);
    }
}

/// Assert that a node is a Tuple type with specific checks
fn assert_tuple_with<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&TupleSchema),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Tuple(tuple_schema) = &node.content {
        check(tuple_schema);
    } else {
        panic!("Expected Tuple type, got {:?}", node.content);
    }
}
fn assert_boolean(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Boolean(_)),
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

/// Assert that a node is a Path type
fn assert_path(schema: &SchemaDocument, node_id: SchemaNodeId) {
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Path(_)),
        "Expected Path type, got {:?}",
        node.content
    );
}

/// Assert that a node is a Code type with optional language check
fn assert_code(schema: &SchemaDocument, node_id: SchemaNodeId, expected_language: Option<&str>) {
    let node = schema.node(node_id);
    if let SchemaNodeContent::Code(code_schema) = &node.content {
        if let Some(expected_lang) = expected_language {
            assert_eq!(
                code_schema.language.as_deref(),
                Some(expected_lang),
                "Expected language '{}', got {:?}",
                expected_lang,
                code_schema.language
            );
        }
    } else {
        panic!("Expected Code type, got {:?}", node.content);
    }
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
        assert_eq!(tuple_schema.items.len(), 2);
        check1(schema, tuple_schema.items[0]);
        check2(schema, tuple_schema.items[1]);
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
        assert_eq!(tuple_schema.items.len(), 3);
        check1(schema, tuple_schema.items[0]);
        check2(schema, tuple_schema.items[1]);
        check3(schema, tuple_schema.items[2]);
    } else {
        panic!("Expected Tuple type, got {:?}", node.content);
    }
}

/// Assert that a node is a Union type with 2 variants
fn assert_union2<F1, F2>(schema: &SchemaDocument, node_id: SchemaNodeId, check1: F1, check2: F2)
where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Union(union_variants) = &node.content {
        assert_eq!(union_variants.len(), 2);
        check1(schema, union_variants[0]);
        check2(schema, union_variants[1]);
    } else {
        panic!("Expected Union type, got {:?}", node.content);
    }
}

/// Assert that a node is a Union type with 3 variants
fn assert_union3<F1, F2, F3>(
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
    if let SchemaNodeContent::Union(union_variants) = &node.content {
        assert_eq!(union_variants.len(), 3);
        check1(schema, union_variants[0]);
        check2(schema, union_variants[1]);
        check3(schema, union_variants[2]);
    } else {
        panic!("Expected Union type, got {:?}", node.content);
    }
}

/// Assert that a node is a Union type with 4 variants
fn assert_union4<F1, F2, F3, F4>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    check1: F1,
    check2: F2,
    check3: F3,
    check4: F4,
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
    F3: Fn(&SchemaDocument, SchemaNodeId),
    F4: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Union(union_variants) = &node.content {
        assert_eq!(union_variants.len(), 4);
        check1(schema, union_variants[0]);
        check2(schema, union_variants[1]);
        check3(schema, union_variants[2]);
        check4(schema, union_variants[3]);
    } else {
        panic!("Expected Union type, got {:?}", node.content);
    }
}

/// Assert that a node is a Variant type with 2 variants
fn assert_variant2<F1, F2>(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    variant1: (&str, F1),
    variant2: (&str, F2),
) where
    F1: Fn(&SchemaDocument, SchemaNodeId),
    F2: Fn(&SchemaDocument, SchemaNodeId),
{
    let node = schema.node(node_id);
    if let SchemaNodeContent::Variant(variant_schema) = &node.content {
        assert_eq!(variant_schema.variants.len(), 2);
        let (name1, check1) = variant1;
        let (name2, check2) = variant2;
        let id1 = variant_schema
            .variants
            .get(name1)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name1));
        let id2 = variant_schema
            .variants
            .get(name2)
            .unwrap_or_else(|| panic!("Variant '{}' missing", name2));
        check1(schema, *id1);
        check2(schema, *id2);
    } else {
        panic!("Expected Variant type, got {:?}", node.content);
    }
}

/// Assert that a node is a Variant type with specific representation
fn assert_variant_repr<F>(schema: &SchemaDocument, node_id: SchemaNodeId, check: F)
where
    F: Fn(&VariantRepr),
{
    let node = schema.node(node_id);
    assert!(
        matches!(node.content, SchemaNodeContent::Variant(_)),
        "Expected Variant type, got {:?}",
        node.content
    );
    if let SchemaNodeContent::Variant(variant_schema) = &node.content {
        check(&variant_schema.repr);
    } else {
        panic!("Expected Variant type, got {:?}", node.content);
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

/// Assert that a node is a Reference type
fn assert_reference(schema: &SchemaDocument, node_id: SchemaNodeId, expected_name: &str) {
    let node = schema.node(node_id);
    if let SchemaNodeContent::Reference(ref_name) = &node.content {
        assert_eq!(
            ref_name,
            &ident(expected_name),
            "Expected reference to '{}', got '{:?}'",
            expected_name,
            ref_name
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
name.$type = .string
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("name", |s, id| assert_string(s, id)));
}

#[test]
fn test_number_type() {
    let input = r#"
count.$type = .float
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("count", |s, id| assert_float(s, id)));
}

#[test]
fn test_integer_type() {
    let input = r#"
age.$type = .integer
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("age", |s, id| assert_integer(s, id)));
}

#[test]
fn test_boolean_type() {
    let input = r#"
active.$type = .boolean
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
value.$type = .null
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("value", |s, id| assert_null(s, id)));
}

#[test]
fn test_any_type() {
    let input = r#"
data.$type = .any
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("data", |s, id| assert_any(s, id)));
}

#[test]
fn test_path_type() {
    let input = r#"
ref.$type = .path
"#;
    let schema = parse_and_convert(input);

    assert_record1(&schema, schema.root, ("ref", |s, id| assert_path(s, id)));
}

// ============================================================================
// CODE TYPES
// ============================================================================

#[test]
fn test_code_email_type() {
    let input = r#"
email.$type = .code.email
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("email", |s, id| assert_code(s, id, Some("email"))),
    );
}

#[test]
fn test_code_javascript_type() {
    let input = r#"
script.$type = .code.javascript
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("script", |s, id| {
            assert_code_with(s, id, |code_schema| {
                assert_eq!(code_schema.language, Some("javascript".to_string()));
            })
        }),
    );
}

// ============================================================================
// ARRAY TYPES
// ============================================================================

#[test]
fn test_array_of_strings() {
    let input = r#"
tags.$array = .string
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("tags", |s, id| assert_array(s, id, assert_string)),
    );
}

#[test]
fn test_array_with_unique_constraint() {
    let input = r#"
tags.$array = .string
tags.$unique = true
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("tags", |s, id| {
            assert_array_with(s, id, assert_string, |array_schema| {
                assert!(array_schema.unique)
            })
        }),
    );
}

#[test]
fn test_array_with_min_max_items() {
    let input = r#"
items.$array = .string
items.$min-items = 1
items.$max-items = 10
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("items", |s, id| {
            assert_array_with(s, id, assert_string, |array_schema| {
                assert_eq!(array_schema.min_items, Some(1));
                assert_eq!(array_schema.max_items, Some(10));
            })
        }),
    );
}

#[test]
fn test_array_with_contains() {
    let input = r#"
tags.$array = .string
tags.$contains = "required"
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("tags", |s, id| {
            assert_array_with(s, id, assert_string, |array_schema| {
                assert!(array_schema.contains.is_some())
            })
        }),
    );
}

// ============================================================================
// RECORD / OBJECT TYPES
// ============================================================================

#[test]
fn test_record_with_multiple_fields() {
    let input = r#"
@ user
name.$type = .string
age.$type = .integer
email.$type = .code.email
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, user_id| {
            assert_record3(
                s,
                user_id,
                ("name", |s, id| assert_string(s, id)),
                ("age", |s, id| assert_integer(s, id)),
                ("email", |s, id| assert_code(s, id, Some("email"))),
            )
        }),
    );
}

#[test]
fn test_nested_record() {
    let input = r#"
@ user.profile
name.$type = .string
bio.$type = .string
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
                        ("name", |s, id| assert_string(s, id)),
                        ("bio", |s, id| assert_string(s, id)),
                    )
                }),
            )
        }),
    );
}

// ============================================================================
// TUPLE TYPES
// ============================================================================

#[test]
fn test_tuple_type() {
    let input = r#"
point.#0.$type = .float
point.#1.$type = .integer
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("point", |s, id| {
            assert_tuple2(s, id, assert_float, assert_integer)
        }),
    );
}

// ============================================================================
// UNION TYPES
// ============================================================================

#[test]
fn test_union_type() {
    let input = r#"
value.$union = [.string, .float]
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("value", |s, id| {
            assert_union2(s, id, assert_string, assert_float)
        }),
    );
}

#[test]
fn test_union_with_multiple_types() {
    let input = r#"
data.$union = [.string, .float, .boolean, .null]
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("data", |s, id| {
            assert_union4(
                s,
                id,
                assert_string,
                assert_float,
                assert_boolean,
                assert_null,
            )
        }),
    );
}

// ============================================================================
// VARIANT TYPES (TAGGED UNIONS)
// ============================================================================

#[test]
fn test_variants_basic() {
    let input = r#"
$types.action {
  @ $variants.click {
    x.$type = .float
    y.$type = .float
  }
  @ $variants.hover {
    element.$type = .string
  }
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("action")));
    let action_id = schema.types[&ident("action")];

    assert_variant2(
        &schema,
        action_id,
        ("click", |s, click_id| {
            assert_record2(s, click_id, ("x", assert_float), ("y", assert_float))
        }),
        ("hover", |s, hover_id| {
            assert_record1(s, hover_id, ("element", |s, id| assert_string(s, id)))
        }),
    );
}

#[test]
fn test_variants_with_untagged_repr() {
    let input = r#"
$types.response {
  $variant-repr = "untagged"
  @ $variants.success {
    data.$type = .any
  }
  @ $variants.error {
    message.$type = .string
  }
}
"#;
    let schema = parse_and_convert(input);

    let response_id = schema.types[&ident("response")];

    assert_variant2(
        &schema,
        response_id,
        ("success", |s, success_id| {
            assert_record1(s, success_id, ("data", assert_any));
        }),
        ("error", |s, error_id| {
            assert_record1(s, error_id, ("message", assert_string));
        }),
    );

    assert_variant_repr(&schema, response_id, |repr| {
        assert!(matches!(repr, VariantRepr::Untagged));
    });
}

#[test]
fn test_variants_with_internally_tagged_repr() {
    let input = r#"
$types.message {
  $variant-repr = { tag = "type" }
  @ $variants.text {
    content.$type = .string
  }
  @ $variants.image {
    url.$type = .string
  }
}
"#;
    let schema = parse_and_convert(input);

    let message_id = schema.types[&ident("message")];

    assert_variant2(
        &schema,
        message_id,
        ("text", |s, text_id| {
            assert_record1(s, text_id, ("content", assert_string));
        }),
        ("image", |s, image_id| {
            assert_record1(s, image_id, ("url", assert_string));
        }),
    );

    assert_variant_repr(&schema, message_id, |repr| {
        if let VariantRepr::Internal { tag } = repr {
            assert_eq!(tag, "type");
        } else {
            panic!("Expected VariantRepr::Internal, got {:?}", repr);
        }
    });
}

#[test]
fn test_variants_with_adjacently_tagged_repr() {
    let input = r#"
$types.event {
  $variant-repr = { tag = "kind", content = "data" }
  @ $variants.login {
    username.$type = .string
  }
  @ $variants.logout {
    reason.$type = .string
  }
}
"#;
    let schema = parse_and_convert(input);

    let event_id = schema.types[&ident("event")];

    assert_variant2(
        &schema,
        event_id,
        ("login", |s, login_id| {
            assert_record1(s, login_id, ("username", assert_string));
        }),
        ("logout", |s, logout_id| {
            assert_record1(s, logout_id, ("reason", assert_string));
        }),
    );

    assert_variant_repr(&schema, event_id, |repr| {
        if let VariantRepr::Adjacent { tag, content } = repr {
            assert_eq!(tag, "kind");
            assert_eq!(content, "data");
        } else {
            panic!("Expected VariantRepr::Adjacent, got {:?}", repr);
        }
    });
}

// ============================================================================
// CUSTOM TYPE DEFINITIONS
// ============================================================================

#[test]
fn test_custom_type_definition() {
    let input = r#"
$types.username {
  $type = .string
  $length = (3, 20)
}

user.$type = .$types.username
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("username")));

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, id| assert_reference(s, id, "username")),
    );
}

#[test]
fn test_type_reference() {
    let input = r#"
$types.email {
  $type = .code.email
}

contact.$type = .$types.email
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("contact", |s, id| assert_reference(s, id, "email")),
    );
}

// ============================================================================
// STRING CONSTRAINTS
// ============================================================================

#[test]
fn test_string_length_constraint() {
    let input = r#"
username.$type = .string
username.$length = (3, 20)
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("username", |s, id| {
            assert_string_with(s, id, |string_schema| {
                assert_eq!(string_schema.length, Some((3, 20)));
            })
        }),
    );
}

#[test]
fn test_string_pattern_constraint() {
    let input = r#"
username.$type = .string
username.$pattern = "^[a-z0-9_]+$"
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("username", |s, id| {
            assert_string_with(s, id, |string_schema| {
                assert_eq!(string_schema.pattern, Some("^[a-z0-9_]+$".to_string()));
            })
        }),
    );
}

#[test]
fn test_string_format_constraint() {
    let input = r#"
email.$type = .string
email.$format = "email"
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("email", |s, id| {
            assert_string_with(s, id, |string_schema| {
                assert_eq!(string_schema.format, Some("email".to_string()));
            })
        }),
    );
}

// ============================================================================
// NUMBER CONSTRAINTS
// ============================================================================

#[test]
fn test_integer_range_constraint() {
    let input = r#"
age.$type = .integer
age.$range = (0, 150)
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("age", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Inclusive(0.into()));
                assert_eq!(int_schema.max, Bound::Inclusive(150.into()));
            })
        }),
    );
}

#[test]
fn test_integer_minimum_constraint() {
    let input = r#"
count.$type = .integer
count.$minimum = 0
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("count", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                if let Bound::Inclusive(val) = &int_schema.min {
                    assert_eq!(*val, BigInt::from(0));
                } else {
                    panic!("Expected Inclusive minimum");
                }
            })
        }),
    );
}

#[test]
fn test_integer_maximum_constraint() {
    let input = r#"
count.$type = .integer
count.$maximum = 100
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("count", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                if let Bound::Inclusive(val) = &int_schema.max {
                    assert_eq!(*val, BigInt::from(100));
                } else {
                    panic!("Expected Inclusive maximum");
                }
            })
        }),
    );
}

#[test]
fn test_integer_exclusive_min_max() {
    let input = r#"
value.$type = .integer
value.$exclusive-min = 0
value.$exclusive-max = 100
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("value", |s, id| {
            assert_integer_with(s, id, |int_schema| {
                assert_eq!(int_schema.min, Bound::Exclusive(0.into()));
                assert_eq!(int_schema.max, Bound::Exclusive(100.into()));
            })
        }),
    );
}

#[test]
fn test_integer_multiple_of() {
    let input = r#"
even.$type = .integer
even.$multiple-of = 2
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

#[test]
fn test_float_range_constraint() {
    let input = r#"
temperature.$type = .float
temperature.$range = (-273.15, 1000.0)
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("temperature", |s, id| {
            assert_float_with(s, id, |float_schema| {
                assert_eq!(float_schema.min, Bound::Inclusive(-273.15));
                assert_eq!(float_schema.max, Bound::Inclusive(1000.0));
            })
        }),
    );
}

// ============================================================================
// METADATA
// ============================================================================

#[test]
fn test_optional_field() {
    let input = r#"
name.$type = .string
bio.$type = .string
bio.$optional = true
"#;
    let schema = parse_and_convert(input);

    // Check bio is optional
    assert_record1(
        &schema,
        schema.root,
        ("bio", |s, id| {
            assert_string(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(metadata.optional);
            })
        }),
    );

    // Check name is required
    assert_record1(
        &schema,
        schema.root,
        ("name", |s, id| {
            assert_string(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(!metadata.optional);
            })
        }),
    );
}

#[test]
fn test_cascade_type() {
    let input = r#"
@ config
$cascade-type = .string

@ config.server.host
@ config.server.port
@ config.database.name
"#;
    let schema = parse_and_convert(input);

    // This test verifies that cascade-type applies to all descendant fields
    // Implementation should handle this during conversion
    assert_record1(
        &schema,
        schema.root,
        ("config", |s, config_id| {
            // Verify config has server field with host
            assert_record1(
                s,
                config_id,
                ("server", |s, server_id| {
                    assert_record1(s, server_id, ("host", assert_string));
                }),
            );
        }),
    );
}

#[test]
fn test_prefer_section() {
    let input = r#"
profile.$type = .any
profile.$prefer.section = true
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("profile", |s, id| {
            assert_any(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(metadata.prefer_section);
            })
        }),
    );
}

#[test]
fn test_rename() {
    let input = r#"
user_name.$type = .string
user_name.$rename = "userName"
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user_name", |s, id| {
            assert_string(s, id);
            assert_metadata(s, id, |metadata| {
                assert_eq!(metadata.rename, Some("userName".to_string()));
            })
        }),
    );
}

#[test]
fn test_rename_all() {
    let input = r#"
@ config
$rename-all = "camelCase"
server_host.$type = .string
database_name.$type = .string
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("config", |s, config_id| {
            // Config is a record with fields
            assert_record1(s, config_id, ("host", assert_string));
            assert_metadata(s, config_id, |metadata| {
                assert_eq!(metadata.rename_all, Some("camelCase".to_string()));
            })
        }),
    );
}

// ============================================================================
// COMPLEX EXAMPLES
// ============================================================================

#[test]
fn test_complex_user_schema() {
    let input = r#"
$types.username {
  $type = .string
  $length = (3, 20)
  $pattern = "^[a-z0-9_]+$"
}

$types.user {
  @ username
  $type = .$types.username

  @ email
  $type = .code.email

  @ age
  $type = .integer
  $range = (0, 150)
  $optional = true

  @ tags
  $array = .string
  $unique = true

  @ role
  $union = ["admin", "user", "guest"]
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("username")));
    assert!(schema.types.contains_key(&ident("user")));

    let user_id = schema.types[&ident("user")];

    // Use assert_record5 to check all fields properly
    assert_record5(
        &schema,
        user_id,
        ("username", |s, id| assert_reference(s, id, "username")),
        ("email", |s, id| assert_code(s, id, Some("email"))),
        ("age", |s, id| {
            assert_integer(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(metadata.optional);
            });
        }),
        ("tags", |s, id| {
            assert_array_with(s, id, assert_string, |array_schema| {
                assert!(array_schema.unique);
            });
        }),
        ("role", |s, id| {
            assert_union3(
                s,
                id,
                |s, v_id| {
                    // Union literals are represented as their types
                    assert_string(s, v_id);
                },
                |s, v_id| {
                    assert_string(s, v_id);
                },
                |s, v_id| {
                    assert_string(s, v_id);
                },
            )
        }),
    );
}

#[test]
fn test_complex_api_schema() {
    let input = r#"
$types.http-method {
  $union = ["GET", "POST", "PUT", "DELETE", "PATCH"]
}

$types.api-request {
  @ method
  $type = .$types.http-method

  @ path
  $type = .string
  $pattern = "^/"

  @ headers
  $type = .any
  $optional = true

  @ body
  $type = .any
  $optional = true
}

$types.api-response {
  $variant-repr = "untagged"

  @ $variants.success {
    status.$type = .integer
    status.$range = (200, 299)
    data.$type = .any
  }

  @ $variants.error {
    status.$type = .integer
    status.$range = (400, 599)
    message.$type = .string
  }
}
"#;
    let schema = parse_and_convert(input);

    assert!(schema.types.contains_key(&ident("http-method")));
    assert!(schema.types.contains_key(&ident("api-request")));
    assert!(schema.types.contains_key(&ident("api-response")));

    let response_id = schema.types[&ident("api-response")];

    // Check variants: success and error
    assert_variant2(
        &schema,
        response_id,
        ("success", |s, success_id| {
            // success variant has status and data fields
            assert_record2(
                s,
                success_id,
                ("status", assert_integer),
                ("data", assert_any),
            );
        }),
        ("error", |s, error_id| {
            // error variant has status and message fields
            assert_record2(
                s,
                error_id,
                ("status", assert_integer),
                ("message", assert_string),
            );
        }),
    );

    // Check repr is Untagged
    assert_variant_repr(&schema, response_id, |repr| {
        assert!(matches!(repr, VariantRepr::Untagged));
    });
}

#[test]
fn test_nested_types_and_arrays() {
    let input = r#"
$types.address {
  @ street
  $type = .string

  @ city
  $type = .string

  @ zip
  $type = .string
  $pattern = "^[0-9]{5}$"
}

$types.person {
  @ name
  $type = .string

  @ addresses
  $array = .$types.address
  $min-items = 1
}
"#;
    let schema = parse_and_convert(input);

    let person_id = schema.types[&ident("person")];

    assert_record2(
        &schema,
        person_id,
        ("name", |s, id| assert_string(s, id)),
        ("addresses", |s, id| {
            assert_array_with(
                s,
                id,
                |s, item_id| assert_reference(s, item_id, "address"),
                |array_schema| {
                    assert_eq!(array_schema.min_items, Some(1));
                },
            );
        }),
    );
}

// ============================================================================
// ADDITIONAL TESTS FOR FULL COVERAGE
// ============================================================================

#[test]
fn test_unknown_fields_policy_allow() {
    let input = r#"
@ config
$unknown-fields = "allow"
host.$type = .string
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("config", |s, config_id| {
            assert_record1(s, config_id, ("host", assert_string));
            assert_unknown_fields(s, config_id, |policy| {
                assert!(matches!(policy, UnknownFieldsPolicy::Allow));
            });
        }),
    );
}

#[test]
fn test_unknown_fields_policy_schema() {
    let input = r#"
@ config
$unknown-fields = .string
host.$type = .string
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("config", |s, config_id| {
            assert_record1(s, config_id, ("host", assert_string));
            assert_unknown_fields(s, config_id, |policy| {
                if let UnknownFieldsPolicy::Schema(schema_id) = policy {
                    assert_string(s, *schema_id);
                } else {
                    panic!("Expected UnknownFieldsPolicy::Schema, got {:?}", policy);
                }
            });
        }),
    );
}

#[test]
fn test_metadata_description() {
    let input = r#"
user.$type = .any
user.$description = "User information"
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, id| {
            assert_any(s, id);
            assert_metadata(s, id, |metadata| {
                assert_eq!(metadata.description, Some("User information".to_string()));
            })
        }),
    );
}

#[test]
fn test_metadata_deprecated() {
    let input = r#"
old_field.$type = .string
old_field.$deprecated = true
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("old_field", |s, id| {
            assert_string(s, id);
            assert_metadata(s, id, |metadata| {
                assert!(metadata.deprecated);
            })
        }),
    );
}

#[test]
fn test_metadata_default_value() {
    let input = r#"
timeout.$type = .integer
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

#[test]
fn test_tuple_with_mixed_types() {
    let input = r#"
coordinate.#0.$type = .float
coordinate.#1.$type = .integer
coordinate.#2.$type = .string
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("coordinate", |s, id| {
            assert_tuple3(s, id, assert_float, assert_integer, assert_string)
        }),
    );
}

#[test]
fn test_code_language_variants() {
    let input = r#"
rust.$type = .code.rust
python.$type = .code.python
sql.$type = .code.sql
"#;
    let schema = parse_and_convert(input);

    assert_record3(
        &schema,
        schema.root,
        ("rust", |s, id| assert_code(s, id, Some("rust"))),
        ("python", |s, id| assert_code(s, id, Some("python"))),
        ("sql", |s, id| assert_code(s, id, Some("sql"))),
    );
}

#[test]
fn test_nested_union_types() {
    let input = r#"
value.$union = [
    .string,
    .integer,
    [.boolean, .null]
]
"#;
    let schema = parse_and_convert(input);

    // This tests that unions can contain nested structures
    assert_record1(
        &schema,
        schema.root,
        ("value", |s, id| {
            assert_union2(s, id, assert_string, assert_integer);
        }),
    );
}

#[test]
fn test_map_type() {
    let input = r#"
headers.$type = .any
headers.$key = .string
headers.$value = .string
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("headers", |s, id| {
            assert_map(s, id, assert_string, assert_string);
        }),
    );
}

#[test]
fn test_map_with_constraints() {
    let input = r#"
attributes.$type = .any
attributes.$key = .string
attributes.$value = .any
attributes.$min-pairs = 1
attributes.$max-pairs = 10
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("attributes", |s, id| {
            assert_map_with(s, id, assert_string, assert_any, |map_schema| {
                assert_eq!(map_schema.min_pairs, Some(1));
                assert_eq!(map_schema.max_pairs, Some(10));
            });
        }),
    );
}

// ============================================================================
// TESTS USING SCHEMA TYPES DIRECTLY
// ============================================================================

#[test]
fn test_boolean_schema_details() {
    let input = r#"
always_true.$type = .boolean
always_true.$const = true
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("always_true", |s, id| {
            assert_boolean_with(s, id, |bool_schema| {
                assert_eq!(bool_schema.r#const, Some(true));
            })
        }),
    );
}

#[test]
fn test_code_schema_details() {
    let input = r#"
script.$type = .code.rust
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("script", |s, id| {
            assert_code_with(s, id, |code_schema| {
                assert_eq!(code_schema.language, Some("rust".to_string()));
            })
        }),
    );
}

#[test]
fn test_path_schema_details() {
    let input = r#"
ref.$type = .path
ref.$const = .config.server
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("ref", |s, id| {
            assert_path_with(s, id, |path_schema| {
                assert!(path_schema.r#const.is_some());
            })
        }),
    );
}

#[test]
fn test_tuple_schema_details() {
    let input = r#"
point.#0.$type = .float
point.#1.$type = .integer
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("point", |s, id| {
            assert_tuple_with(s, id, |tuple_schema| {
                assert_eq!(tuple_schema.items.len(), 2);
                assert_float(s, tuple_schema.items[0]);
                assert_integer(s, tuple_schema.items[1]);
            })
        }),
    );
}

#[test]
fn test_schema_node_extensions_access() {
    let input = r#"
user.$type = .string
user.$description = "User name"
"#;
    let schema = parse_and_convert(input);

    assert_record1(
        &schema,
        schema.root,
        ("user", |s, id| {
            assert_string(s, id);
            assert_metadata(s, id, |metadata| {
                assert_eq!(metadata.description, Some("User name".to_string()));
            })
        }),
    );
}
