use indexmap::IndexMap;

use eure_codegen_ir::*;

#[test]
fn decimal_int_canonicalization_preserves_large_integers_without_narrowing() {
    let large = DecimalInt::new("1234567890123456789012345678901234567890");
    let canonical = large.canonicalized();
    assert_eq!(
        canonical.to_string(),
        "1234567890123456789012345678901234567890"
    );
}

#[test]
fn decimal_int_canonicalization_normalizes_sign_and_zeroes() {
    assert_eq!(DecimalInt::new("+00042").canonicalized().to_string(), "42");
    assert_eq!(DecimalInt::new("-00042").canonicalized().to_string(), "-42");
    assert_eq!(DecimalInt::new("-0").canonicalized().to_string(), "0");
}

#[test]
fn module_preserves_non_string_object_keys() {
    let mut map = IndexMap::new();
    map.insert(
        ObjectKeyIr::Integer(DecimalInt::new("10")),
        ValueIr::Bool(true),
    );
    map.insert(
        ObjectKeyIr::Tuple(vec![
            ObjectKeyIr::String("a".to_string()),
            ObjectKeyIr::Integer(DecimalInt::new("2")),
        ]),
        ValueIr::Bool(false),
    );

    let type_id = TypeId("example".to_string());
    let schema_name = QualifiedTypeName::local("example");

    let mut nodes = IndexMap::new();
    nodes.insert(
        SchemaNodeIrId(0),
        SchemaNodeIr::new(
            SchemaNodeContentIr::Literal(ValueIr::Map(map)),
            SchemaMetadataIr::default(),
            IndexMap::new(),
        ),
    );

    let mut module = IrModule::default();
    module.insert_name_index(schema_name.clone(), type_id.clone());
    module.push_root(type_id.clone());
    module.insert_type(
        type_id,
        TypeDefIr::new(
            TypeId("example".to_string()),
            TypeNamesIr::new("Example".to_string(), Some(schema_name)),
            nodes,
            SchemaNodeIrId(0),
            RustBindingIr::default(),
            TypeCodegenIr::None,
            TypeOriginIr::Schema,
        ),
    );

    let node = &module.types().values().next().unwrap().schema_nodes()[&SchemaNodeIrId(0)];

    let SchemaNodeContentIr::Literal(ValueIr::Map(result)) = node.content() else {
        panic!("expected literal map");
    };

    assert!(result.contains_key(&ObjectKeyIr::Integer(DecimalInt::new("10"))));
    assert!(result.contains_key(&ObjectKeyIr::Tuple(vec![
        ObjectKeyIr::String("a".to_string()),
        ObjectKeyIr::Integer(DecimalInt::new("2")),
    ])));
}
