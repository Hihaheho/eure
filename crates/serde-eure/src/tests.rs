//! Comprehensive integration tests for `serde-eure`.
//!
//! Coverage:
//! - De: all primitive types, array, tuple, map, record (required/optional/unknown-fields),
//!   union (External / Internal / Adjacent / Untagged), Any/schema-free
//! - Ser: all primitive types, array, tuple, map, record, union (all 4 modes)
//! - Roundtrip: ser → JSON string → de → compare original document

use eure::document::EureDocument;
use eure::document::constructor::DocumentConstructor;
use eure::document::path::PathSegment;
use eure::eure;
use eure::value::{ObjectKey, PrimitiveValue, Text};
use eure_schema::interop::{UnionInterop, VariantRepr};
use eure_schema::{
    ArraySchema, FloatSchema, IntegerSchema, MapSchema, RecordFieldSchema, RecordSchema,
    SchemaDocument, SchemaNodeContent, TextSchema, TupleSchema, UnionSchema, UnknownFieldsPolicy,
};
use num_bigint::BigInt;
use serde_json::{Deserializer as JsonDeserializer, json};

use crate::{from_deserializer, to_serializer};

// ============================================================================
// Document builder helpers
// ============================================================================

fn prim_doc(v: PrimitiveValue) -> EureDocument {
    let mut c = DocumentConstructor::new();
    c.bind_primitive(v).unwrap();
    c.finish()
}

fn text_doc(s: &str) -> EureDocument {
    prim_doc(PrimitiveValue::Text(Text::plaintext(s.to_string())))
}

fn int_doc(n: i64) -> EureDocument {
    prim_doc(PrimitiveValue::Integer(BigInt::from(n)))
}

fn bigint_doc(value: BigInt) -> EureDocument {
    prim_doc(PrimitiveValue::Integer(value))
}

fn f64_doc(f: f64) -> EureDocument {
    prim_doc(PrimitiveValue::F64(f))
}

fn bool_doc(b: bool) -> EureDocument {
    prim_doc(PrimitiveValue::Bool(b))
}

fn null_doc() -> EureDocument {
    prim_doc(PrimitiveValue::Null)
}

fn int_array_doc(values: &[i64]) -> EureDocument {
    let mut c = DocumentConstructor::new();
    c.bind_empty_array().unwrap();
    for &v in values {
        let scope = c.begin_scope();
        c.navigate(PathSegment::ArrayIndex(None)).unwrap();
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(v)))
            .unwrap();
        c.end_scope(scope).unwrap();
    }
    c.finish()
}

fn tuple_int_text_doc(i: i64, s: &str) -> EureDocument {
    let mut c = DocumentConstructor::new();
    c.bind_empty_tuple().unwrap();
    let scope = c.begin_scope();
    c.navigate(PathSegment::TupleIndex(0)).unwrap();
    c.bind_primitive(PrimitiveValue::Integer(BigInt::from(i)))
        .unwrap();
    c.end_scope(scope).unwrap();
    let scope = c.begin_scope();
    c.navigate(PathSegment::TupleIndex(1)).unwrap();
    c.bind_primitive(PrimitiveValue::Text(Text::plaintext(s.to_string())))
        .unwrap();
    c.end_scope(scope).unwrap();
    c.finish()
}

fn single_entry_map_doc(key: ObjectKey, value: PrimitiveValue) -> EureDocument {
    let mut c = DocumentConstructor::new();
    c.bind_empty_map().unwrap();
    let scope = c.begin_scope();
    c.navigate(PathSegment::Value(key)).unwrap();
    c.bind_primitive(value).unwrap();
    c.end_scope(scope).unwrap();
    c.finish()
}

fn null_variant_doc(variant: &str) -> EureDocument {
    let mut c = DocumentConstructor::new();
    c.bind_primitive(PrimitiveValue::Null).unwrap();
    c.set_variant(variant).unwrap();
    c.finish()
}

// ============================================================================
// Schema builder helpers
// ============================================================================

fn field(schema_id: eure_schema::SchemaNodeId, optional: bool) -> RecordFieldSchema {
    RecordFieldSchema {
        schema: schema_id,
        optional,
        binding_style: None,
        field_codegen: Default::default(),
    }
}

fn make_text_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    s.root = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    s
}

fn make_integer_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    s.root = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    s
}

fn make_float_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    s.root = s.create_node(SchemaNodeContent::Float(FloatSchema::default()));
    s
}

fn make_bool_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    s.root = s.create_node(SchemaNodeContent::Boolean);
    s
}

fn make_null_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    s.root = s.create_node(SchemaNodeContent::Null);
    s
}

fn make_array_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Array(ArraySchema {
        item: int_id,
        min_length: None,
        max_length: None,
        unique: false,
        contains: None,
        binding_style: None,
    }));
    s
}

fn make_tuple_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Tuple(TupleSchema {
        elements: vec![int_id, text_id],
        binding_style: None,
    }));
    s
}

fn make_map_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Map(MapSchema {
        key: text_id,
        value: int_id,
        min_size: None,
        max_size: None,
    }));
    s
}

fn make_integer_key_map_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Map(MapSchema {
        key: int_id,
        value: text_id,
        min_size: None,
        max_size: None,
    }));
    s
}

fn make_boolean_key_map_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let bool_id = s.create_node(SchemaNodeContent::Boolean);
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Map(MapSchema {
        key: bool_id,
        value: text_id,
        min_size: None,
        max_size: None,
    }));
    s
}

fn make_any_key_map_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let any_id = s.create_node(SchemaNodeContent::Any);
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Map(MapSchema {
        key: any_id,
        value: text_id,
        min_size: None,
        max_size: None,
    }));
    s
}

fn make_adjacent_null_variant_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let null_id = s.create_node(SchemaNodeContent::Null);
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let text_record_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("message".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("unit".to_string(), null_id),
            ("data".to_string(), text_record_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Adjacent {
                tag: "t".to_string(),
                content: "c".to_string(),
            }),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_record_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [
            ("name".to_string(), field(text_id, false)),
            ("age".to_string(), field(int_id, false)),
        ]
        .into_iter()
        .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    s
}

fn make_record_with_optional_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [
            ("name".to_string(), field(text_id, false)),
            ("nickname".to_string(), field(text_id, true)),
            ("age".to_string(), field(int_id, false)),
        ]
        .into_iter()
        .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    s
}

fn make_record_allow_unknown_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("name".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Allow,
    }));
    s
}

/// Two-variant union: "success" -> {message: text}, "failure" -> {code: integer}
fn make_two_variant_union_schema(repr: VariantRepr) -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    let success_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("message".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let failure_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("code".to_string(), field(int_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("success".to_string(), success_id),
            ("failure".to_string(), failure_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(repr),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_two_variant_union_allow_unknown_schema(repr: VariantRepr) -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
    let success_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("message".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Allow,
    }));
    let failure_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("code".to_string(), field(int_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Allow,
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("success".to_string(), success_id),
            ("failure".to_string(), failure_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(repr),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_nested_untagged_union_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let bool_id = s.create_node(SchemaNodeContent::Boolean);
    let leaf_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("value".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let other_leaf_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("flag".to_string(), field(bool_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let inner_union_id = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("leaf".to_string(), leaf_id),
            ("other_leaf".to_string(), other_leaf_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Untagged),
        },
        deny_untagged: Default::default(),
    }));
    let fallback_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("other".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("outer".to_string(), inner_union_id),
            ("fallback".to_string(), fallback_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Untagged),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_deny_untagged_schema() -> SchemaDocument {
    // "explicit" variant must have $variant set; "implicit" variant matches untagged.
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let explicit_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("value".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let implicit_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("value".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("explicit".to_string(), explicit_id),
            ("implicit".to_string(), implicit_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Untagged),
        },
        deny_untagged: ["explicit".to_string()].into_iter().collect(),
    }));
    s
}

fn make_unambiguous_union_schema() -> SchemaDocument {
    // Both variants have matching shapes — only one is in `unambiguous`.
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let v1_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("name".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let v2_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("name".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [("v1".to_string(), v1_id), ("v2".to_string(), v2_id)]
            .into_iter()
            .collect(),
        unambiguous: ["v1".to_string(), "v2".to_string()].into_iter().collect(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Untagged),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_nested_internal_external_union_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let leaf_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("value".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let other_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("other".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let inner_union_id = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("leaf".to_string(), leaf_id),
            ("other_leaf".to_string(), other_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::External),
        },
        deny_untagged: Default::default(),
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [("wrapper".to_string(), inner_union_id)]
            .into_iter()
            .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Internal {
                tag: "kind".to_string(),
            }),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_nested_internal_external_unit_union_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let leaf_id = s.create_node(SchemaNodeContent::Null);
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let other_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("other".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let inner_union_id = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("leaf".to_string(), leaf_id),
            ("other_leaf".to_string(), other_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::External),
        },
        deny_untagged: Default::default(),
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [("wrapper".to_string(), inner_union_id)]
            .into_iter()
            .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Internal {
                tag: "kind".to_string(),
            }),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_nested_internal_internal_unit_union_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    let leaf_id = s.create_node(SchemaNodeContent::Null);
    let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
    let other_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
        properties: [("other".to_string(), field(text_id, false))]
            .into_iter()
            .collect(),
        flatten: vec![],
        unknown_fields: UnknownFieldsPolicy::Deny,
    }));
    let inner_union_id = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [
            ("leaf".to_string(), leaf_id),
            ("other_leaf".to_string(), other_id),
        ]
        .into_iter()
        .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Internal {
                tag: "inner".to_string(),
            }),
        },
        deny_untagged: Default::default(),
    }));
    s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
        variants: [("wrapper".to_string(), inner_union_id)]
            .into_iter()
            .collect(),
        unambiguous: Default::default(),
        interop: UnionInterop {
            variant_repr: Some(VariantRepr::Internal {
                tag: "outer".to_string(),
            }),
        },
        deny_untagged: Default::default(),
    }));
    s
}

fn make_any_schema() -> SchemaDocument {
    let mut s = SchemaDocument::new();
    s.root = s.create_node(SchemaNodeContent::Any);
    s
}

// ============================================================================
// Deserializer tests
// ============================================================================

mod de {
    use super::*;

    fn de_json(json: &str, schema: &SchemaDocument) -> EureDocument {
        let mut d = JsonDeserializer::from_str(json);
        from_deserializer(&mut d, schema).expect("deserialization failed")
    }

    #[test]
    fn primitive_text() {
        let actual = de_json(r#""hello""#, &make_text_schema());
        assert_eq!(actual, text_doc("hello"));
    }

    #[test]
    fn primitive_integer() {
        let actual = de_json("42", &make_integer_schema());
        assert_eq!(actual, int_doc(42));
    }

    #[test]
    fn primitive_float() {
        let actual = de_json("3.5", &make_float_schema());
        assert_eq!(actual, f64_doc(3.5));
    }

    #[test]
    fn primitive_bool_true() {
        let actual = de_json("true", &make_bool_schema());
        assert_eq!(actual, bool_doc(true));
    }

    #[test]
    fn primitive_bool_false() {
        let actual = de_json("false", &make_bool_schema());
        assert_eq!(actual, bool_doc(false));
    }

    #[test]
    fn primitive_null() {
        let actual = de_json("null", &make_null_schema());
        assert_eq!(actual, null_doc());
    }

    #[test]
    fn array_of_integers() {
        let actual = de_json("[1, 2, 3]", &make_array_schema());
        assert_eq!(actual, int_array_doc(&[1, 2, 3]));
    }

    #[test]
    fn empty_array() {
        let actual = de_json("[]", &make_array_schema());
        assert_eq!(actual, int_array_doc(&[]));
    }

    #[test]
    fn tuple_int_text() {
        let actual = de_json(r#"[7, "world"]"#, &make_tuple_schema());
        assert_eq!(actual, tuple_int_text_doc(7, "world"));
    }

    #[test]
    fn map_text_to_integer() {
        let actual = de_json(r#"{"a": 1, "b": 2}"#, &make_map_schema());
        assert_eq!(actual, eure!({ a = 1, b = 2 }));
    }

    #[test]
    fn record_all_fields() {
        let actual = de_json(r#"{"name": "Alice", "age": 30}"#, &make_record_schema());
        assert_eq!(actual, eure!({ name = "Alice", age = 30 }));
    }

    #[test]
    fn record_optional_field_present() {
        let actual = de_json(
            r#"{"name": "Bob", "nickname": "Bobby", "age": 25}"#,
            &make_record_with_optional_schema(),
        );
        assert_eq!(
            actual,
            eure!({ name = "Bob", nickname = "Bobby", age = 25 })
        );
    }

    #[test]
    fn record_optional_field_absent() {
        let actual = de_json(
            r#"{"name": "Carol", "age": 40}"#,
            &make_record_with_optional_schema(),
        );
        assert_eq!(actual, eure!({ name = "Carol", age = 40 }));
    }

    #[test]
    fn record_required_field_missing_errors() {
        let mut d = JsonDeserializer::from_str(r#"{"name": "Dave"}"#);
        assert!(from_deserializer(&mut d, &make_record_schema()).is_err());
    }

    #[test]
    fn record_unknown_field_denied_errors() {
        let mut d = JsonDeserializer::from_str(r#"{"name": "Eve", "age": 20, "extra": "oops"}"#);
        assert!(from_deserializer(&mut d, &make_record_schema()).is_err());
    }

    #[test]
    fn record_unknown_field_allowed() {
        // Fix 2: Allow policy now preserves unknown fields instead of dropping them.
        let actual = de_json(
            r#"{"name": "Frank", "extra": "ignored"}"#,
            &make_record_allow_unknown_schema(),
        );
        assert_eq!(actual, eure!({ name = "Frank", extra = "ignored" }));
    }

    #[test]
    fn union_external_success_variant() {
        let actual = de_json(
            r#"{"success": {"message": "ok"}}"#,
            &make_two_variant_union_schema(VariantRepr::External),
        );
        assert_eq!(actual, eure!({ message = "ok", %variant = "success" }));
    }

    #[test]
    fn union_external_failure_variant() {
        let actual = de_json(
            r#"{"failure": {"code": 404}}"#,
            &make_two_variant_union_schema(VariantRepr::External),
        );
        assert_eq!(actual, eure!({ code = 404, %variant = "failure" }));
    }

    #[test]
    fn union_external_unknown_variant_errors() {
        let mut d = JsonDeserializer::from_str(r#"{"unknown": {}}"#);
        assert!(
            from_deserializer(
                &mut d,
                &make_two_variant_union_schema(VariantRepr::External)
            )
            .is_err()
        );
    }

    #[test]
    fn union_internal_success_variant() {
        let actual = de_json(
            r#"{"type": "success", "message": "ok"}"#,
            &make_two_variant_union_schema(VariantRepr::Internal {
                tag: "type".to_string(),
            }),
        );
        assert_eq!(actual, eure!({ message = "ok", %variant = "success" }));
    }

    #[test]
    fn union_internal_failure_variant() {
        let actual = de_json(
            r#"{"type": "failure", "code": 503}"#,
            &make_two_variant_union_schema(VariantRepr::Internal {
                tag: "type".to_string(),
            }),
        );
        assert_eq!(actual, eure!({ code = 503, %variant = "failure" }));
    }

    #[test]
    fn union_adjacent_success_variant() {
        let actual = de_json(
            r#"{"type": "success", "content": {"message": "ok"}}"#,
            &make_two_variant_union_schema(VariantRepr::Adjacent {
                tag: "type".to_string(),
                content: "content".to_string(),
            }),
        );
        assert_eq!(actual, eure!({ message = "ok", %variant = "success" }));
    }

    #[test]
    fn union_adjacent_failure_variant() {
        let actual = de_json(
            r#"{"type": "failure", "content": {"code": 500}}"#,
            &make_two_variant_union_schema(VariantRepr::Adjacent {
                tag: "type".to_string(),
                content: "content".to_string(),
            }),
        );
        assert_eq!(actual, eure!({ code = 500, %variant = "failure" }));
    }

    #[test]
    fn de_adjacent_extra_sibling_preserved() {
        let actual = de_json(
            r#"{"type": "success", "content": {"message": "ok"}, "trace": "abc-123"}"#,
            &make_two_variant_union_allow_unknown_schema(VariantRepr::Adjacent {
                tag: "type".to_string(),
                content: "content".to_string(),
            }),
        );
        assert_eq!(
            actual,
            eure!({ message = "ok", trace = "abc-123", %variant = "success" })
        );
    }

    #[test]
    fn de_adjacent_unit_variant() {
        // Regression: Adjacent-tagged unit variant has no content field — must deserialize without error.
        let actual = de_json(r#"{"t": "unit"}"#, &make_adjacent_null_variant_schema());
        // Unit variant root is Primitive(Null) + %variant extension, not an empty Map.
        let expected = {
            let mut c = DocumentConstructor::new();
            c.set_variant("unit").unwrap();
            c.bind_primitive(PrimitiveValue::Null).unwrap();
            c.finish()
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn union_untagged_success_variant() {
        let actual = de_json(
            r#"{"message": "ok"}"#,
            &make_two_variant_union_schema(VariantRepr::Untagged),
        );
        assert_eq!(actual, eure!({ message = "ok", %variant = "success" }));
    }

    #[test]
    fn union_untagged_failure_variant() {
        let actual = de_json(
            r#"{"code": 404}"#,
            &make_two_variant_union_schema(VariantRepr::Untagged),
        );
        assert_eq!(actual, eure!({ code = 404, %variant = "failure" }));
    }

    #[test]
    fn de_untagged_nested() {
        let actual = de_json(r#"{"value": "deep"}"#, &make_nested_untagged_union_schema());
        assert_eq!(actual, eure!({ value = "deep", %variant = "outer.leaf" }));
    }

    #[test]
    fn de_deny_untagged_allowed_variant_matches() {
        // "implicit" is not deny_untagged, so untagged matching works.
        let actual = de_json(r#"{"value": "hello"}"#, &make_deny_untagged_schema());
        assert_eq!(actual, eure!({ value = "hello", %variant = "implicit" }));
    }

    #[test]
    fn de_unambiguous_conflict_errors() {
        // Both v1 and v2 match {"name":"x"} and both are unambiguous → must error.
        let schema = make_unambiguous_union_schema();
        let result = serde_json::from_str::<serde_json::Value>(r#"{"name": "x"}"#)
            .ok()
            .and_then(|v| from_deserializer(v, &schema).err());
        assert!(
            result.is_some(),
            "expected ambiguity error but deserialization succeeded"
        );
    }

    #[test]
    fn any_schema_string() {
        let actual = de_json(r#""hello""#, &make_any_schema());
        assert_eq!(actual, text_doc("hello"));
    }

    #[test]
    fn any_schema_integer() {
        let actual = de_json("99", &make_any_schema());
        assert_eq!(actual, int_doc(99));
    }

    #[test]
    fn any_schema_bool() {
        let actual = de_json("true", &make_any_schema());
        assert_eq!(actual, bool_doc(true));
    }

    #[test]
    fn any_schema_null() {
        let actual = de_json("null", &make_any_schema());
        assert_eq!(actual, null_doc());
    }

    #[test]
    fn any_schema_object() {
        let actual = de_json(r#"{"x": 1, "y": 2}"#, &make_any_schema());
        assert_eq!(actual, eure!({ x = 1, y = 2 }));
    }

    #[test]
    fn any_schema_array() {
        // schema-free array: each element deserialized as Any
        let actual = de_json("[1, 2, 3]", &make_any_schema());
        assert_eq!(actual, int_array_doc(&[1, 2, 3]));
    }

    #[test]
    fn nested_record() {
        let mut s = SchemaDocument::new();
        let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let inner_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("label".to_string(), field(text_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("inner".to_string(), field(inner_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        let actual = de_json(r#"{"inner": {"label": "nested"}}"#, &s);
        assert_eq!(actual, eure!({ inner.label = "nested" }));
    }

    #[test]
    fn array_of_records() {
        let mut s = SchemaDocument::new();
        let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let item_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("tag".to_string(), field(text_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        s.root = s.create_node(SchemaNodeContent::Array(ArraySchema {
            item: item_id,
            min_length: None,
            max_length: None,
            unique: false,
            contains: None,
            binding_style: None,
        }));
        let actual = de_json(r#"[{"tag": "a"}, {"tag": "b"}]"#, &s);
        // Build expected: [{tag="a"}, {tag="b"}]
        let mut c = DocumentConstructor::new();
        c.bind_empty_array().unwrap();
        for tag_val in &["a", "b"] {
            let scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(None)).unwrap();
            c.bind_empty_map().unwrap();
            let inner = c.begin_scope();
            c.navigate(PathSegment::Value(ObjectKey::String("tag".to_string())))
                .unwrap();
            c.bind_primitive(PrimitiveValue::Text(Text::plaintext(
                (*tag_val).to_string(),
            )))
            .unwrap();
            c.end_scope(inner).unwrap();
            c.end_scope(scope).unwrap();
        }
        let expected = c.finish();
        assert_eq!(actual, expected);
    }
}

// ============================================================================
// Serializer tests
// ============================================================================

mod ser {
    use super::*;
    use std::fmt;

    use serde::ser::{Error as _, Impossible, Serializer};

    #[derive(Debug, PartialEq, Eq)]
    enum CapturedInteger {
        I64(i64),
        U64(u64),
        I128(i128),
        U128(u128),
    }

    #[derive(Debug, PartialEq, Eq)]
    struct CaptureError(String);

    impl fmt::Display for CaptureError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl std::error::Error for CaptureError {}

    impl serde::ser::Error for CaptureError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            Self(msg.to_string())
        }
    }

    struct IntegerCaptureSerializer;

    impl Serializer for IntegerCaptureSerializer {
        type Ok = CapturedInteger;
        type Error = CaptureError;
        type SerializeSeq = Impossible<CapturedInteger, CaptureError>;
        type SerializeTuple = Impossible<CapturedInteger, CaptureError>;
        type SerializeTupleStruct = Impossible<CapturedInteger, CaptureError>;
        type SerializeTupleVariant = Impossible<CapturedInteger, CaptureError>;
        type SerializeMap = Impossible<CapturedInteger, CaptureError>;
        type SerializeStruct = Impossible<CapturedInteger, CaptureError>;
        type SerializeStructVariant = Impossible<CapturedInteger, CaptureError>;

        fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
            Ok(CapturedInteger::I64(value))
        }

        fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
            Ok(CapturedInteger::U64(value))
        }

        fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
            Ok(CapturedInteger::I128(value))
        }

        fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
            Ok(CapturedInteger::U128(value))
        }

        fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected bool"))
        }

        fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected i8"))
        }

        fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected i16"))
        }

        fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected i32"))
        }

        fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected u8"))
        }

        fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected u16"))
        }

        fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected u32"))
        }

        fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected f32"))
        }

        fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected f64"))
        }

        fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected char"))
        }

        fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected str"))
        }

        fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected bytes"))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected none"))
        }

        fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + serde::Serialize,
        {
            Err(CaptureError::custom("unexpected some"))
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected unit"))
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected unit struct"))
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            Err(CaptureError::custom("unexpected unit variant"))
        }

        fn serialize_newtype_struct<T>(
            self,
            _name: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + serde::Serialize,
        {
            Err(CaptureError::custom("unexpected newtype struct"))
        }

        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + serde::Serialize,
        {
            Err(CaptureError::custom("unexpected newtype variant"))
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Err(CaptureError::custom("unexpected seq"))
        }

        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Err(CaptureError::custom("unexpected tuple"))
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Err(CaptureError::custom("unexpected tuple struct"))
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Err(CaptureError::custom("unexpected tuple variant"))
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Err(CaptureError::custom("unexpected map"))
        }

        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Err(CaptureError::custom("unexpected struct"))
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Err(CaptureError::custom("unexpected struct variant"))
        }
    }

    fn ser_to_json(doc: &EureDocument, schema: &SchemaDocument) -> serde_json::Value {
        to_serializer(serde_json::value::Serializer, doc, schema).expect("serialization failed")
    }

    #[test]
    fn primitive_text() {
        let doc = text_doc("hello");
        assert_eq!(ser_to_json(&doc, &make_text_schema()), json!("hello"));
    }

    #[test]
    fn primitive_integer() {
        let doc = int_doc(42);
        assert_eq!(ser_to_json(&doc, &make_integer_schema()), json!(42));
    }

    #[test]
    fn primitive_float() {
        let doc = f64_doc(3.5);
        assert_eq!(ser_to_json(&doc, &make_float_schema()), json!(3.5));
    }

    #[test]
    fn primitive_bool_true() {
        let doc = bool_doc(true);
        assert_eq!(ser_to_json(&doc, &make_bool_schema()), json!(true));
    }

    #[test]
    fn primitive_bool_false() {
        let doc = bool_doc(false);
        assert_eq!(ser_to_json(&doc, &make_bool_schema()), json!(false));
    }

    #[test]
    fn primitive_null() {
        let doc = null_doc();
        assert_eq!(ser_to_json(&doc, &make_null_schema()), json!(null));
    }

    #[test]
    fn array_of_integers() {
        let doc = int_array_doc(&[10, 20, 30]);
        assert_eq!(ser_to_json(&doc, &make_array_schema()), json!([10, 20, 30]));
    }

    #[test]
    fn empty_array() {
        let doc = int_array_doc(&[]);
        assert_eq!(ser_to_json(&doc, &make_array_schema()), json!([]));
    }

    #[test]
    fn tuple_int_text() {
        let doc = tuple_int_text_doc(7, "world");
        assert_eq!(ser_to_json(&doc, &make_tuple_schema()), json!([7, "world"]));
    }

    #[test]
    fn map_text_to_integer() {
        let doc = eure!({ a = 1, b = 2 });
        let result = ser_to_json(&doc, &make_map_schema());
        assert_eq!(result["a"], json!(1));
        assert_eq!(result["b"], json!(2));
    }

    #[test]
    fn ser_map_integer_key() {
        let doc = single_entry_map_doc(
            ObjectKey::Number(BigInt::from(42)),
            PrimitiveValue::Text(Text::plaintext("answer".to_string())),
        );
        let result = ser_to_json(&doc, &make_integer_key_map_schema());
        assert_eq!(result, json!({"42": "answer"}));
    }

    #[test]
    fn ser_map_boolean_key() {
        let doc = single_entry_map_doc(
            ObjectKey::from(true),
            PrimitiveValue::Text(Text::plaintext("flag".to_string())),
        );
        let result = ser_to_json(&doc, &make_boolean_key_map_schema());
        assert_eq!(result, json!({"true": "flag"}));
    }

    #[test]
    fn ser_map_any_string_key() {
        // Regression: Any-schema map keys must NOT be wrapped in Display quotes.
        let doc = single_entry_map_doc(
            ObjectKey::String("hello".to_string()),
            PrimitiveValue::Text(Text::plaintext("world".to_string())),
        );
        let result = ser_to_json(&doc, &make_any_key_map_schema());
        assert_eq!(result, json!({"hello": "world"}));
    }

    #[test]
    fn ser_adjacent_unit_variant() {
        // Unit variant (Null schema) with Adjacent repr must emit only the tag, no content field.
        let doc = eure!({ %variant = "unit" });
        let result = ser_to_json(&doc, &make_adjacent_null_variant_schema());
        assert_eq!(result, json!({"t": "unit"}));
    }

    #[test]
    fn ser_deny_untagged_variant_errors() {
        // "explicit" is in deny_untagged — serializing it as Untagged must error.
        let doc = eure!({ value = "hello", %variant = "explicit" });
        let err = to_serializer(
            serde_json::value::Serializer,
            &doc,
            &make_deny_untagged_schema(),
        )
        .expect_err("deny_untagged variant should error on untagged serialization");
        assert!(err.to_string().contains("explicit"));
    }

    #[test]
    fn record_all_fields() {
        let doc = eure!({ name = "Alice", age = 30 });
        assert_eq!(
            ser_to_json(&doc, &make_record_schema()),
            json!({"name": "Alice", "age": 30})
        );
    }

    #[test]
    fn ser_required_field_missing_errors() {
        let doc = eure!({ name = "Alice" });
        let err = to_serializer(serde_json::value::Serializer, &doc, &make_record_schema())
            .expect_err("missing required field should error");
        assert!(err.to_string().contains("missing field: age"));
    }

    #[test]
    fn ser_required_flatten_field_missing_errors() {
        let mut s = SchemaDocument::new();
        let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let extra_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("score".to_string(), field(int_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("name".to_string(), field(text_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![extra_id],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));

        let doc = eure!({ name = "Alice" });
        let err = to_serializer(serde_json::value::Serializer, &doc, &s)
            .expect_err("missing flattened required field should error");
        assert!(err.to_string().contains("missing field: score"));
    }

    #[test]
    fn union_external_success() {
        let doc = eure!({ message = "ok", %variant = "success" });
        let result = ser_to_json(&doc, &make_two_variant_union_schema(VariantRepr::External));
        assert_eq!(result, json!({"success": {"message": "ok"}}));
    }

    #[test]
    fn union_external_failure() {
        let doc = eure!({ code = 404, %variant = "failure" });
        let result = ser_to_json(&doc, &make_two_variant_union_schema(VariantRepr::External));
        assert_eq!(result, json!({"failure": {"code": 404}}));
    }

    #[test]
    fn union_internal_success() {
        let doc = eure!({ message = "ok", %variant = "success" });
        let result = ser_to_json(
            &doc,
            &make_two_variant_union_schema(VariantRepr::Internal {
                tag: "type".to_string(),
            }),
        );
        assert_eq!(result, json!({"type": "success", "message": "ok"}));
    }

    #[test]
    fn ser_internal_non_map_like_variant_errors() {
        // Regression (P2): an internal-tagged variant with a non-map-like schema (e.g., Text)
        // must error, not emit untyped garbage.
        let mut s = SchemaDocument::new();
        let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
        s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
            variants: [("plain".to_string(), text_id)].into_iter().collect(),
            unambiguous: Default::default(),
            interop: UnionInterop {
                variant_repr: Some(VariantRepr::Internal {
                    tag: "type".to_string(),
                }),
            },
            deny_untagged: Default::default(),
        }));
        let doc = eure!({ %variant = "plain" });
        let err = to_serializer(serde_json::value::Serializer, &doc, &s)
            .expect_err("non-map-like internal-tagged variant should error");
        let msg = err.to_string();
        assert!(
            msg.contains("cannot be serialized as a map-like"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn ser_internal_nested_union_variant_path_forwarded() {
        let doc = eure!({ value = "deep", %variant = "wrapper.leaf" });
        let result = ser_to_json(&doc, &make_nested_internal_external_union_schema());
        assert_eq!(
            result,
            json!({"kind": "wrapper", "leaf": {"value": "deep"}})
        );
    }

    #[test]
    fn ser_internal_nested_unit_variant_path_forwarded() {
        let doc = null_variant_doc("wrapper.leaf");
        let result = ser_to_json(&doc, &make_nested_internal_external_unit_union_schema());
        assert_eq!(result, json!({"kind": "wrapper", "leaf": null}));
    }

    #[test]
    fn ser_internal_nested_map_like_unit_variant_path_forwarded() {
        let doc = null_variant_doc("wrapper.leaf");
        let result = ser_to_json(&doc, &make_nested_internal_internal_unit_union_schema());
        assert_eq!(result, json!({"outer": "wrapper", "inner": "leaf"}));
    }

    #[test]
    fn ser_internal_variant_path_remainder_on_non_union_errors() {
        let doc = eure!({ message = "ok", %variant = "success.garbage" });
        let err = to_serializer(
            serde_json::value::Serializer,
            &doc,
            &make_two_variant_union_schema(VariantRepr::Internal {
                tag: "type".to_string(),
            }),
        )
        .expect_err("dangling variant path should error");
        assert!(
            err.to_string()
                .contains("variant path has remaining components but schema is not a union")
        );
    }

    #[test]
    fn union_adjacent_success() {
        let doc = eure!({ message = "ok", %variant = "success" });
        let result = ser_to_json(
            &doc,
            &make_two_variant_union_schema(VariantRepr::Adjacent {
                tag: "type".to_string(),
                content: "content".to_string(),
            }),
        );
        assert_eq!(
            result,
            json!({"type": "success", "content": {"message": "ok"}})
        );
    }

    #[test]
    fn union_untagged_success() {
        let doc = eure!({ message = "ok", %variant = "success" });
        let result = ser_to_json(&doc, &make_two_variant_union_schema(VariantRepr::Untagged));
        // Untagged: content emitted directly without wrapper
        assert_eq!(result, json!({"message": "ok"}));
    }

    #[test]
    fn ser_bigint_i128() {
        let value = i64::MIN as i128 - 1;
        let doc = bigint_doc(BigInt::from(value));
        let actual = to_serializer(IntegerCaptureSerializer, &doc, &make_integer_schema()).unwrap();
        assert_eq!(actual, CapturedInteger::I128(value));
    }

    #[test]
    fn deny_rejects_extra_field_in_document() {
        // Fix 3: Deny policy must error when the document has unknown fields.
        let schema = make_record_schema(); // unknown_fields = Deny
        // Insert an extra field beyond what the schema allows.
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();
        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("name".to_string())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext("Alice".to_string())))
            .unwrap();
        c.end_scope(scope).unwrap();
        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("age".to_string())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(30)))
            .unwrap();
        c.end_scope(scope).unwrap();
        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("extra".to_string())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext("oops".to_string())))
            .unwrap();
        c.end_scope(scope).unwrap();
        let doc = c.finish();
        let result = to_serializer(serde_json::value::Serializer, &doc, &schema);
        assert!(
            result.is_err(),
            "expected error for extra field under Deny policy"
        );
    }

    #[test]
    fn tuple_arity_mismatch_errors() {
        // Fix 4: Tuple serializer must reject arity mismatches.
        let schema = make_tuple_schema(); // 2-element tuple: (i64, text)
        // Build a 3-element tuple document.
        let mut c = DocumentConstructor::new();
        c.bind_empty_tuple().unwrap();
        for (idx, val) in [
            (0u8, PrimitiveValue::Integer(BigInt::from(1))),
            (1, PrimitiveValue::Text(Text::plaintext("x".to_string()))),
            (2, PrimitiveValue::Integer(BigInt::from(3))),
        ] {
            let scope = c.begin_scope();
            c.navigate(PathSegment::TupleIndex(idx)).unwrap();
            c.bind_primitive(val).unwrap();
            c.end_scope(scope).unwrap();
        }
        let doc = c.finish();
        let result = to_serializer(serde_json::value::Serializer, &doc, &schema);
        assert!(result.is_err(), "expected arity mismatch error");
    }
}

// ============================================================================
// Roundtrip tests: ser → JSON string → de → compare
// ============================================================================

mod roundtrip {
    use super::*;

    fn roundtrip(doc: &EureDocument, schema: &SchemaDocument) -> EureDocument {
        let json_val =
            to_serializer(serde_json::value::Serializer, doc, schema).expect("ser failed");
        let json_str = json_val.to_string();
        let mut d = JsonDeserializer::from_str(&json_str);
        from_deserializer(&mut d, schema).expect("de failed")
    }

    #[test]
    fn record() {
        let schema = make_record_schema();
        let doc = eure!({ name = "Alice", age = 30 });
        assert_eq!(roundtrip(&doc, &schema), doc);
    }

    #[test]
    fn array_of_integers() {
        let doc = int_array_doc(&[1, 2, 3]);
        assert_eq!(roundtrip(&doc, &make_array_schema()), doc);
    }

    #[test]
    fn tuple() {
        let doc = tuple_int_text_doc(7, "world");
        assert_eq!(roundtrip(&doc, &make_tuple_schema()), doc);
    }

    #[test]
    fn union_external_success() {
        let schema = make_two_variant_union_schema(VariantRepr::External);
        let doc = eure!({ message = "ok", %variant = "success" });
        assert_eq!(roundtrip(&doc, &schema), doc);
    }

    #[test]
    fn union_external_failure() {
        let schema = make_two_variant_union_schema(VariantRepr::External);
        let doc = eure!({ code = 404, %variant = "failure" });
        assert_eq!(roundtrip(&doc, &schema), doc);
    }

    #[test]
    fn union_internal() {
        let schema = make_two_variant_union_schema(VariantRepr::Internal {
            tag: "type".to_string(),
        });
        let doc = eure!({ message = "ok", %variant = "success" });
        assert_eq!(roundtrip(&doc, &schema), doc);
    }

    #[test]
    fn union_adjacent() {
        let schema = make_two_variant_union_schema(VariantRepr::Adjacent {
            tag: "type".to_string(),
            content: "content".to_string(),
        });
        let doc = eure!({ message = "ok", %variant = "success" });
        assert_eq!(roundtrip(&doc, &schema), doc);
    }

    #[test]
    fn primitive_text() {
        let doc = text_doc("hello");
        assert_eq!(roundtrip(&doc, &make_text_schema()), doc);
    }

    #[test]
    fn primitive_integer() {
        let doc = int_doc(42);
        assert_eq!(roundtrip(&doc, &make_integer_schema()), doc);
    }

    #[test]
    fn primitive_bool() {
        let doc = bool_doc(true);
        assert_eq!(roundtrip(&doc, &make_bool_schema()), doc);
    }

    #[test]
    fn map() {
        // Single-key to avoid JSON key-ordering issues on roundtrip comparison
        let schema = make_map_schema();
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();
        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("x".to_string())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(5)))
            .unwrap();
        c.end_scope(scope).unwrap();
        let doc = c.finish();
        assert_eq!(roundtrip(&doc, &schema), doc);
    }

    #[test]
    fn nested_record() {
        let mut s = SchemaDocument::new();
        let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let inner_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("label".to_string(), field(text_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("inner".to_string(), field(inner_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        let doc = eure!({ inner.label = "nested" });
        assert_eq!(roundtrip(&doc, &s), doc);
    }

    #[test]
    fn record_allow_unknown_preserved() {
        // Fix 2 + roundtrip: unknown fields under Allow policy survive ser→de.
        let schema = make_record_allow_unknown_schema();
        // Build doc with known + unknown field
        let doc = eure!({ name = "Grace", extra = "kept" });
        assert_eq!(roundtrip(&doc, &schema), doc);
    }

    #[test]
    fn record_with_flatten() {
        // Fix 6: flattened schema fields are serialized and deserialized correctly.
        let mut s = SchemaDocument::new();
        let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        // Extra record to be flattened: { score: integer }
        let extra_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("score".to_string(), field(int_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        // Root record: { name: text } + flatten extra
        s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("name".to_string(), field(text_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![extra_id],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        let doc = eure!({ name = "Heidi", score = 99 });
        assert_eq!(roundtrip(&doc, &s), doc);
    }
}

mod de_fixes {
    use super::*;

    use serde::de::value::{Error as ValueError, MapDeserializer};
    use serde::de::{self, IntoDeserializer, Visitor};

    fn de_json(json: &str, schema: &SchemaDocument) -> EureDocument {
        let mut d = JsonDeserializer::from_str(json);
        from_deserializer(&mut d, schema).expect("deserialization failed")
    }

    fn make_internal_float_record_union_schema() -> SchemaDocument {
        let mut s = SchemaDocument::new();
        let float_id = s.create_node(SchemaNodeContent::Float(FloatSchema::default()));
        let record_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("value".to_string(), field(float_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        s.root = s.create_node(SchemaNodeContent::Union(UnionSchema {
            variants: [("measure".to_string(), record_id)].into_iter().collect(),
            unambiguous: Default::default(),
            interop: UnionInterop {
                variant_repr: Some(VariantRepr::Internal {
                    tag: "type".to_string(),
                }),
            },
            deny_untagged: Default::default(),
        }));
        s
    }

    #[derive(Clone)]
    enum TestValue {
        Str(&'static str),
        I32(i32),
    }

    impl<'de> de::Deserializer<'de> for TestValue {
        type Error = ValueError;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self {
                Self::Str(value) => visitor.visit_borrowed_str(value),
                Self::I32(value) => visitor.visit_i32(value),
            }
        }

        fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self {
                Self::I32(value) => visitor.visit_i32(value),
                Self::Str(_) => Err(de::Error::custom("expected i32")),
            }
        }

        fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self {
                Self::Str(value) => visitor.visit_borrowed_str(value),
                Self::I32(_) => Err(de::Error::custom("expected str")),
            }
        }

        fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            self.deserialize_str(visitor)
        }

        fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            self.deserialize_any(visitor)
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i64 i128 u8 u16 u32 u64 u128 f32 f64 char string bytes
            byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
            map struct enum
        }
    }

    impl<'de> IntoDeserializer<'de, ValueError> for TestValue {
        type Deserializer = Self;

        fn into_deserializer(self) -> Self::Deserializer {
            self
        }
    }

    #[test]
    fn record_with_flatten_de() {
        // Fix 6: deserializer recognises flattened fields.
        let mut s = SchemaDocument::new();
        let text_id = s.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = s.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let extra_id = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("score".to_string(), field(int_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        s.root = s.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [("name".to_string(), field(text_id, false))]
                .into_iter()
                .collect(),
            flatten: vec![extra_id],
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        let actual = de_json(r#"{"name": "Heidi", "score": 99}"#, &s);
        assert_eq!(actual, eure!({ name = "Heidi", score = 99 }));
    }

    #[test]
    fn de_buffered_union_float_from_integer() {
        let schema = make_internal_float_record_union_schema();
        let de = MapDeserializer::new(
            [
                ("type", TestValue::Str("measure")),
                ("value", TestValue::I32(7)),
            ]
            .into_iter(),
        );
        let actual = from_deserializer(de, &schema).expect("deserialization failed");
        assert_eq!(actual, eure!({ value = 7.0, %variant = "measure" }));
    }
}
