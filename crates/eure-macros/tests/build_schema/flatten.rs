//! Test BuildSchema derive for flatten

use eure::{BuildSchema, SchemaDocument};
use eure_schema::SchemaNodeContent;

#[derive(BuildSchema)]
struct Address {
    city: String,
    country: String,
}

#[derive(BuildSchema)]
struct Person {
    name: String,
    #[eure(flatten)]
    address: Address,
}

#[test]
fn test_flatten_record() {
    let schema = SchemaDocument::of::<Person>();

    // Root should be a Record with direct properties + flatten target
    let SchemaNodeContent::Record(record) = &schema.node(schema.root).content else {
        panic!("Expected Record");
    };

    // Should have "name" as direct property
    assert!(record.properties.contains_key("name"));

    // Should NOT have address as a property (it's flattened)
    assert!(!record.properties.contains_key("address"));
    assert!(!record.properties.contains_key("city"));
    assert!(!record.properties.contains_key("country"));

    // Should have one flatten target
    assert_eq!(record.flatten.len(), 1);

    // Flatten target should be the Address schema
    let flatten_id = record.flatten[0];
    let SchemaNodeContent::Record(addr_record) = &schema.node(flatten_id).content else {
        panic!("Expected flattened Record");
    };
    assert!(addr_record.properties.contains_key("city"));
    assert!(addr_record.properties.contains_key("country"));
}

#[derive(BuildSchema)]
enum ContactMethod {
    Email { email: String },
    Phone { phone: String },
}

#[derive(BuildSchema)]
struct Contact {
    name: String,
    #[eure(flatten)]
    method: ContactMethod,
}

#[test]
fn test_flatten_union() {
    let schema = SchemaDocument::of::<Contact>();

    let SchemaNodeContent::Record(record) = &schema.node(schema.root).content else {
        panic!("Expected Record");
    };

    assert!(record.properties.contains_key("name"));
    assert_eq!(record.flatten.len(), 1);

    // Flatten target should be a Union
    let flatten_id = record.flatten[0];
    let SchemaNodeContent::Union(union) = &schema.node(flatten_id).content else {
        panic!("Expected flattened Union");
    };
    // Enum variants use PascalCase by default
    assert!(union.variants.contains_key("Email"));
    assert!(union.variants.contains_key("Phone"));
}

#[derive(BuildSchema)]
struct Level2 {
    field_c: String,
}

#[derive(BuildSchema)]
struct Level1 {
    field_b: String,
    #[eure(flatten)]
    level2: Level2,
}

#[derive(BuildSchema)]
struct RootLevel {
    field_a: String,
    #[eure(flatten)]
    level1: Level1,
}

#[test]
fn test_nested_flatten() {
    let schema = SchemaDocument::of::<RootLevel>();

    let SchemaNodeContent::Record(record) = &schema.node(schema.root).content else {
        panic!("Expected Record");
    };

    assert!(record.properties.contains_key("field_a"));
    assert_eq!(record.flatten.len(), 1);

    // Level1 should also have its own flatten
    let level1_id = record.flatten[0];
    let SchemaNodeContent::Record(level1) = &schema.node(level1_id).content else {
        panic!("Expected Record for level1");
    };
    assert!(level1.properties.contains_key("field_b"));
    assert_eq!(level1.flatten.len(), 1);
}
