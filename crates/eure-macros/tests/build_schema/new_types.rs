//! Test BuildSchema for decimal, bytes, and set field types

use std::collections::{BTreeSet, HashSet};

use eure::{BuildSchema, FromEure, IntoEure, SchemaDocument};
use eure_document::bytes::Bytes;
use eure_document::decimal::Decimal;
use eure_document::document::constructor::DocumentConstructor;
use eure_schema::{SchemaNodeContent, SchemaNodeId};
use indexmap::IndexSet;

fn content(schema: &SchemaDocument, id: SchemaNodeId) -> &SchemaNodeContent {
    &schema.node(id).content
}

fn set_item(schema: &SchemaDocument, id: SchemaNodeId) -> &SchemaNodeContent {
    let SchemaNodeContent::Set(set) = content(schema, id) else {
        panic!("Expected Set, got {:?}", content(schema, id));
    };
    content(schema, set.item)
}

#[derive(Debug, PartialEq, BuildSchema, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
struct Order {
    price: Decimal,
    signature: Bytes,
    tags: BTreeSet<String>,
    ids: HashSet<i64>,
    pairs: IndexSet<(i64, String)>,
}

#[test]
fn test_new_types_schema() {
    let schema = SchemaDocument::of::<Order>();
    let SchemaNodeContent::Record(record) = content(&schema, schema.root) else {
        panic!("Expected Record");
    };
    assert!(matches!(
        content(&schema, record.properties["price"].schema),
        SchemaNodeContent::Decimal(_)
    ));
    assert!(matches!(
        content(&schema, record.properties["signature"].schema),
        SchemaNodeContent::Bytes(_)
    ));
    assert!(matches!(
        set_item(&schema, record.properties["tags"].schema),
        SchemaNodeContent::Text(_)
    ));
    assert!(matches!(
        set_item(&schema, record.properties["ids"].schema),
        SchemaNodeContent::Integer(_)
    ));
    assert!(matches!(
        set_item(&schema, record.properties["pairs"].schema),
        SchemaNodeContent::Tuple(_)
    ));
}

/// What `IntoEure` writes must validate against the schema `BuildSchema` derives,
/// and parse back to the same value.
#[test]
fn test_written_value_matches_derived_schema() {
    let order = Order {
        price: "19.90".parse().unwrap(),
        signature: Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
        tags: BTreeSet::from(["a".to_string(), "b".to_string()]),
        ids: HashSet::from([3, 1, 2]),
        pairs: IndexSet::from([(1, "x".to_string()), (2, "y".to_string())]),
    };

    let mut c = DocumentConstructor::new();
    c.write(order).unwrap();
    let doc = c.finish();

    let output = eure_schema::validate::validate(&doc, &SchemaDocument::of::<Order>());
    assert!(output.is_valid, "{:?}", output.errors);

    let parsed: Order = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(parsed.price, "19.90".parse::<Decimal>().unwrap());
    assert_eq!(parsed.signature, Bytes(vec![0xde, 0xad, 0xbe, 0xef]));
    assert_eq!(parsed.ids, HashSet::from([1, 2, 3]));
}
