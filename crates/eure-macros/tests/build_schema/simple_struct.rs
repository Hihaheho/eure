//! Test BuildSchema derive for simple structs

use eure::{BuildSchema, SchemaDocument};
use eure_schema::{SchemaNodeContent, UnknownFieldsPolicy};

#[derive(BuildSchema)]
struct SimpleStruct {
    name: String,
    age: u32,
    active: bool,
}

#[test]
fn test_simple_struct_schema() {
    let schema = SchemaDocument::of::<SimpleStruct>();

    // Check root is a record
    let root = schema.node(schema.root);
    let SchemaNodeContent::Record(record) = &root.content else {
        panic!("Expected Record, got {:?}", root.content);
    };

    // Check fields exist
    assert!(record.properties.contains_key("name"));
    assert!(record.properties.contains_key("age"));
    assert!(record.properties.contains_key("active"));

    // Check field types
    let name_field = &record.properties["name"];
    assert!(!name_field.optional);
    assert!(matches!(
        schema.node(name_field.schema).content,
        SchemaNodeContent::Text(_)
    ));

    let age_field = &record.properties["age"];
    assert!(!age_field.optional);
    assert!(matches!(
        schema.node(age_field.schema).content,
        SchemaNodeContent::Integer(_)
    ));

    let active_field = &record.properties["active"];
    assert!(!active_field.optional);
    assert!(matches!(
        schema.node(active_field.schema).content,
        SchemaNodeContent::Boolean
    ));

    // Default unknown fields policy is Deny
    assert!(matches!(record.unknown_fields, UnknownFieldsPolicy::Deny));
}

#[derive(BuildSchema)]
#[eure(type_name = "user")]
struct UserWithTypeName {
    name: String,
}

#[test]
fn test_type_name_registration() {
    let schema = SchemaDocument::of::<UserWithTypeName>();

    // Check type is registered
    let user_type = schema.types.get(&"user".parse().unwrap());
    assert!(user_type.is_some());
}

#[derive(BuildSchema)]
struct WithOptionalField {
    required: String,
    optional: Option<i32>,
}

#[test]
fn test_optional_field() {
    let schema = SchemaDocument::of::<WithOptionalField>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Record(record) = &root.content else {
        panic!("Expected Record");
    };

    // Required field should not be optional
    assert!(!record.properties["required"].optional);

    // Option<T> field should be optional
    assert!(record.properties["optional"].optional);

    // The schema for Option<i32> should be a union
    let optional_schema = schema.node(record.properties["optional"].schema);
    assert!(matches!(optional_schema.content, SchemaNodeContent::Union(_)));
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

    let root = schema.node(schema.root);
    let SchemaNodeContent::Record(record) = &root.content else {
        panic!("Expected Record");
    };

    // Fields should be renamed to kebab-case
    assert!(record.properties.contains_key("user-name"));
    assert!(record.properties.contains_key("email-address"));
    assert!(!record.properties.contains_key("user_name"));
    assert!(!record.properties.contains_key("email_address"));
}

#[derive(BuildSchema)]
#[eure(allow_unknown_fields)]
struct AllowUnknownFields {
    name: String,
}

#[test]
fn test_allow_unknown_fields() {
    let schema = SchemaDocument::of::<AllowUnknownFields>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Record(record) = &root.content else {
        panic!("Expected Record");
    };

    assert!(matches!(record.unknown_fields, UnknownFieldsPolicy::Allow));
}
