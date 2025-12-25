//! Test BuildSchema derive for enums (unions)

use eure::{BuildSchema, SchemaDocument};
use eure_schema::SchemaNodeContent;

#[derive(BuildSchema)]
enum SimpleEnum {
    Active,
    Inactive,
    Pending,
}

#[test]
fn test_simple_enum_schema() {
    let schema = SchemaDocument::of::<SimpleEnum>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Union(union) = &root.content else {
        panic!("Expected Union, got {:?}", root.content);
    };

    // Check variants exist
    assert!(union.variants.contains_key("Active"));
    assert!(union.variants.contains_key("Inactive"));
    assert!(union.variants.contains_key("Pending"));

    // Unit variants should have Null schema
    for (_, &variant_id) in &union.variants {
        assert!(matches!(
            schema.node(variant_id).content,
            SchemaNodeContent::Null
        ));
    }
}

#[derive(BuildSchema)]
#[eure(rename_all = "kebab-case")]
enum RenamedVariants {
    UserActive,
    UserInactive,
}

#[test]
fn test_renamed_variants() {
    let schema = SchemaDocument::of::<RenamedVariants>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Union(union) = &root.content else {
        panic!("Expected Union");
    };

    assert!(union.variants.contains_key("user-active"));
    assert!(union.variants.contains_key("user-inactive"));
}

#[derive(BuildSchema)]
enum NewtypeVariants {
    Text(String),
    Number(i32),
}

#[test]
fn test_newtype_variants() {
    let schema = SchemaDocument::of::<NewtypeVariants>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Union(union) = &root.content else {
        panic!("Expected Union");
    };

    // Text variant should have Text schema
    let text_schema = schema.node(union.variants["Text"]);
    assert!(matches!(text_schema.content, SchemaNodeContent::Text(_)));

    // Number variant should have Integer schema
    let number_schema = schema.node(union.variants["Number"]);
    assert!(matches!(
        number_schema.content,
        SchemaNodeContent::Integer(_)
    ));
}

#[derive(BuildSchema)]
enum StructVariants {
    User { name: String, age: u32 },
    Admin { name: String, level: i32 },
}

#[test]
fn test_struct_variants() {
    let schema = SchemaDocument::of::<StructVariants>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Union(union) = &root.content else {
        panic!("Expected Union");
    };

    // User variant should be a record
    let user_schema = schema.node(union.variants["User"]);
    let SchemaNodeContent::Record(user_record) = &user_schema.content else {
        panic!("Expected Record for User variant");
    };
    assert!(user_record.properties.contains_key("name"));
    assert!(user_record.properties.contains_key("age"));

    // Admin variant should be a record
    let admin_schema = schema.node(union.variants["Admin"]);
    let SchemaNodeContent::Record(admin_record) = &admin_schema.content else {
        panic!("Expected Record for Admin variant");
    };
    assert!(admin_record.properties.contains_key("name"));
    assert!(admin_record.properties.contains_key("level"));
}

#[derive(BuildSchema)]
enum TupleVariants {
    Point(i32, i32),
    Color(u8, u8, u8),
}

#[test]
fn test_tuple_variants() {
    let schema = SchemaDocument::of::<TupleVariants>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Union(union) = &root.content else {
        panic!("Expected Union");
    };

    // Point variant should be a tuple
    let point_schema = schema.node(union.variants["Point"]);
    let SchemaNodeContent::Tuple(point_tuple) = &point_schema.content else {
        panic!("Expected Tuple for Point variant");
    };
    assert_eq!(point_tuple.elements.len(), 2);

    // Color variant should be a tuple
    let color_schema = schema.node(union.variants["Color"]);
    let SchemaNodeContent::Tuple(color_tuple) = &color_schema.content else {
        panic!("Expected Tuple for Color variant");
    };
    assert_eq!(color_tuple.elements.len(), 3);
}

#[derive(BuildSchema)]
enum MixedVariants {
    Unit,
    Newtype(String),
    Tuple(i32, i32),
    Struct { name: String },
}

#[test]
fn test_mixed_variants() {
    let schema = SchemaDocument::of::<MixedVariants>();

    let root = schema.node(schema.root);
    let SchemaNodeContent::Union(union) = &root.content else {
        panic!("Expected Union");
    };

    assert_eq!(union.variants.len(), 4);

    // Unit -> Null
    assert!(matches!(
        schema.node(union.variants["Unit"]).content,
        SchemaNodeContent::Null
    ));

    // Newtype -> Text
    assert!(matches!(
        schema.node(union.variants["Newtype"]).content,
        SchemaNodeContent::Text(_)
    ));

    // Tuple -> Tuple
    assert!(matches!(
        schema.node(union.variants["Tuple"]).content,
        SchemaNodeContent::Tuple(_)
    ));

    // Struct -> Record
    assert!(matches!(
        schema.node(union.variants["Struct"]).content,
        SchemaNodeContent::Record(_)
    ));
}
