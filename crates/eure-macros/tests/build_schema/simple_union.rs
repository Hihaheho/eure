//! Test BuildSchema derive for enums (unions)

use eure::{BuildSchema, SchemaDocument};
use eure_schema::{SchemaNodeContent, SchemaNodeId};

// ============================================================================
// Assertion helpers
// ============================================================================

fn assert_text(schema: &SchemaDocument, id: SchemaNodeId) {
    assert!(matches!(schema.node(id).content, SchemaNodeContent::Text(_)));
}

fn assert_integer(schema: &SchemaDocument, id: SchemaNodeId) {
    assert!(matches!(schema.node(id).content, SchemaNodeContent::Integer(_)));
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

fn assert_tuple<F>(schema: &SchemaDocument, id: SchemaNodeId, check: F)
where
    F: Fn(&SchemaDocument, &eure_schema::TupleSchema),
{
    let SchemaNodeContent::Tuple(tuple) = &schema.node(id).content else {
        panic!("Expected Tuple, got {:?}", schema.node(id).content);
    };
    check(schema, tuple);
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

// ============================================================================
// Tests
// ============================================================================

#[derive(BuildSchema)]
enum SimpleEnum {
    Active,
    Inactive,
    Pending,
}

#[test]
fn test_simple_enum_schema() {
    let schema = SchemaDocument::of::<SimpleEnum>();
    assert_union(&schema, schema.root, |s, union| {
        assert_eq!(union.variants.len(), 3);
        assert_null(s, union.variants["Active"]);
        assert_null(s, union.variants["Inactive"]);
        assert_null(s, union.variants["Pending"]);
    });
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
    assert_union(&schema, schema.root, |s, union| {
        assert!(union.variants.contains_key("user-active"));
        assert!(union.variants.contains_key("user-inactive"));
        assert_null(s, union.variants["user-active"]);
        assert_null(s, union.variants["user-inactive"]);
    });
}

#[derive(BuildSchema)]
enum NewtypeVariants {
    Text(String),
    Number(i32),
}

#[test]
fn test_newtype_variants() {
    let schema = SchemaDocument::of::<NewtypeVariants>();
    assert_union(&schema, schema.root, |s, union| {
        assert_eq!(union.variants.len(), 2);
        assert_text(s, union.variants["Text"]);
        assert_integer(s, union.variants["Number"]);
    });
}

#[derive(BuildSchema)]
enum StructVariants {
    User { name: String, age: u32 },
    Admin { name: String, level: i32 },
}

#[test]
fn test_struct_variants() {
    let schema = SchemaDocument::of::<StructVariants>();
    assert_union(&schema, schema.root, |s, union| {
        assert_eq!(union.variants.len(), 2);

        assert_record(s, union.variants["User"], |s, rec| {
            assert_text(s, rec.properties["name"].schema);
            assert_integer(s, rec.properties["age"].schema);
        });

        assert_record(s, union.variants["Admin"], |s, rec| {
            assert_text(s, rec.properties["name"].schema);
            assert_integer(s, rec.properties["level"].schema);
        });
    });
}

#[derive(BuildSchema)]
enum TupleVariants {
    Point(i32, i32),
    Color(u8, u8, u8),
}

#[test]
fn test_tuple_variants() {
    let schema = SchemaDocument::of::<TupleVariants>();
    assert_union(&schema, schema.root, |s, union| {
        assert_eq!(union.variants.len(), 2);

        assert_tuple(s, union.variants["Point"], |s, tuple| {
            assert_eq!(tuple.elements.len(), 2);
            assert_integer(s, tuple.elements[0]);
            assert_integer(s, tuple.elements[1]);
        });

        assert_tuple(s, union.variants["Color"], |s, tuple| {
            assert_eq!(tuple.elements.len(), 3);
            assert_integer(s, tuple.elements[0]);
            assert_integer(s, tuple.elements[1]);
            assert_integer(s, tuple.elements[2]);
        });
    });
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
    assert_union(&schema, schema.root, |s, union| {
        assert_eq!(union.variants.len(), 4);

        // Unit -> Null
        assert_null(s, union.variants["Unit"]);

        // Newtype -> Text
        assert_text(s, union.variants["Newtype"]);

        // Tuple -> Tuple
        assert_tuple(s, union.variants["Tuple"], |s, tuple| {
            assert_eq!(tuple.elements.len(), 2);
            assert_integer(s, tuple.elements[0]);
            assert_integer(s, tuple.elements[1]);
        });

        // Struct -> Record
        assert_record(s, union.variants["Struct"], |s, rec| {
            assert_text(s, rec.properties["name"].schema);
        });
    });
}
