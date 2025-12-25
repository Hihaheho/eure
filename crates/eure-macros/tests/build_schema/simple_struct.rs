//! Test BuildSchema derive for simple structs

use eure::{BuildSchema, SchemaDocument};
use eure_schema::{SchemaNodeContent, SchemaNodeId, UnknownFieldsPolicy};

// ============================================================================
// Assertion helpers
// ============================================================================

fn assert_text(schema: &SchemaDocument, id: SchemaNodeId) {
    assert!(matches!(schema.node(id).content, SchemaNodeContent::Text(_)));
}

fn assert_integer(schema: &SchemaDocument, id: SchemaNodeId) {
    assert!(matches!(schema.node(id).content, SchemaNodeContent::Integer(_)));
}

fn assert_boolean(schema: &SchemaDocument, id: SchemaNodeId) {
    assert!(matches!(schema.node(id).content, SchemaNodeContent::Boolean));
}

fn assert_null(schema: &SchemaDocument, id: SchemaNodeId) {
    assert!(matches!(schema.node(id).content, SchemaNodeContent::Null));
}

fn assert_record<F>(schema: &SchemaDocument, id: SchemaNodeId, check: F)
where
    F: Fn(&SchemaDocument, &eure_schema::RecordSchema),
{
    let SchemaNodeContent::Record(record) = &schema.node(id).content else {
        panic!("Expected Record, got {:?}", schema.node(id).content);
    };
    check(schema, record);
}

fn assert_union<F>(schema: &SchemaDocument, id: SchemaNodeId, check: F)
where
    F: Fn(&SchemaDocument, &eure_schema::UnionSchema),
{
    let SchemaNodeContent::Union(union) = &schema.node(id).content else {
        panic!("Expected Union, got {:?}", schema.node(id).content);
    };
    check(schema, union);
}

fn assert_reference(schema: &SchemaDocument, id: SchemaNodeId, type_name: &str) {
    let SchemaNodeContent::Reference(r) = &schema.node(id).content else {
        panic!("Expected Reference, got {:?}", schema.node(id).content);
    };
    assert_eq!(r.name.as_ref(), type_name);
}

// ============================================================================
// Tests
// ============================================================================

#[derive(BuildSchema)]
struct SimpleStruct {
    name: String,
    age: u32,
    active: bool,
}

#[test]
fn test_simple_struct_schema() {
    let schema = SchemaDocument::of::<SimpleStruct>();
    assert_record(&schema, schema.root, |s, rec| {
        assert_eq!(rec.properties.len(), 3);
        assert_text(s, rec.properties["name"].schema);
        assert_integer(s, rec.properties["age"].schema);
        assert_boolean(s, rec.properties["active"].schema);
    });
}

#[derive(BuildSchema)]
#[eure(type_name = "user")]
struct UserWithTypeName {
    name: String,
}

#[test]
fn test_type_name_registration() {
    let schema = SchemaDocument::of::<UserWithTypeName>();

    // Root should reference the type
    assert_reference(&schema, schema.root, "user");

    // Type should be registered
    let user_id = *schema.types.values().next().expect("type registered");
    assert_record(&schema, user_id, |s, rec| {
        assert_text(s, rec.properties["name"].schema);
    });
}

#[derive(BuildSchema)]
struct WithOptionalField {
    required: String,
    optional: Option<i32>,
}

#[test]
fn test_optional_field() {
    let schema = SchemaDocument::of::<WithOptionalField>();
    assert_record(&schema, schema.root, |s, rec| {
        assert_text(s, rec.properties["required"].schema);
        // Optional field should have a union schema (some|none)
        assert_union(s, rec.properties["optional"].schema, |s, union| {
            assert_eq!(union.variants.len(), 2);
            assert_integer(s, union.variants["some"]);
            assert_null(s, union.variants["none"]);
        });
        // Optional field should be marked as optional
        assert!(rec.properties["optional"].optional);
    });
}

#[derive(BuildSchema)]
#[eure(rename_all = "kebab-case")]
struct RenamedFields {
    user_name: String,
    email_address: String,
}

#[test]
fn test_rename_all() {
    let schema = SchemaDocument::of::<RenamedFields>();
    assert_record(&schema, schema.root, |s, rec| {
        assert!(rec.properties.contains_key("user-name"));
        assert!(rec.properties.contains_key("email-address"));
        assert_text(s, rec.properties["user-name"].schema);
        assert_text(s, rec.properties["email-address"].schema);
    });
}

#[derive(BuildSchema)]
#[eure(allow_unknown_fields)]
struct AllowUnknownFields {
    name: String,
}

#[test]
fn test_allow_unknown_fields() {
    let schema = SchemaDocument::of::<AllowUnknownFields>();
    assert_record(&schema, schema.root, |s, rec| {
        assert_text(s, rec.properties["name"].schema);
        assert!(matches!(rec.unknown_fields, UnknownFieldsPolicy::Allow));
    });
}
