use indexmap::{IndexMap, IndexSet};

use eure_codegen_ir::*;

fn base_union_module() -> IrModule {
    let type_id = TypeId("union-type".to_string());
    let schema_name = QualifiedTypeName::local("union-type");

    let mut nodes = IndexMap::new();
    nodes.insert(
        SchemaNodeIrId(1),
        SchemaNodeIr::new(
            SchemaNodeContentIr::Null,
            SchemaMetadataIr::default(),
            IndexMap::new(),
        ),
    );

    nodes.insert(
        SchemaNodeIrId(0),
        SchemaNodeIr::new(
            SchemaNodeContentIr::Union(UnionSchemaIr::new(
                IndexMap::from([("a".to_string(), SchemaNodeIrId(1))]),
                IndexSet::from(["a".to_string()]),
                IndexSet::new(),
                UnionInteropIr::default(),
            )),
            SchemaMetadataIr::default(),
            IndexMap::new(),
        ),
    );

    let mut module = IrModule::default();
    module.set_emission_defaults(EmissionDefaultsIr {
        serde_serialize: true,
        serde_deserialize: true,
        derive_allow: IndexSet::from([
            "Debug".to_string(),
            "Serialize".to_string(),
            "Deserialize".to_string(),
        ]),
    });

    let type_def = TypeDefIr::new(
        type_id.clone(),
        TypeNamesIr::new("UnionType".to_string(), Some(schema_name.clone())),
        nodes,
        SchemaNodeIrId(0),
        RustBindingIr::new(
            RustTypeKindIr::Enum,
            ContainerAttrsIr::default(),
            Vec::new(),
            vec![RustVariantIr::new(
                "A".to_string(),
                "a".to_string(),
                false,
                VariantShapeIr::Unit,
            )],
            RustGenericsIr::default(),
            WhereClauseIr::default(),
            TypeEmissionConfigIr::default(),
        ),
        TypeCodegenIr::None,
        TypeOriginIr::Schema,
    );

    module.insert_name_index(schema_name, type_id.clone());
    module.push_root(type_id.clone());
    module.insert_type(type_id, type_def);
    module
}

#[test]
fn identical_modules_are_structurally_equal() {
    let lhs = base_union_module();
    let rhs = base_union_module();

    assert!(structural_eq(&lhs, &rhs));
    assert!(assert_structural_eq(&lhs, &rhs).is_ok());
}

#[test]
fn none_and_external_variant_repr_are_not_structurally_equal() {
    let lhs = base_union_module();
    let mut rhs = base_union_module();

    let ty = rhs.types_mut().values_mut().next().unwrap();
    let root = ty.schema_nodes_mut().get_mut(&SchemaNodeIrId(0)).unwrap();
    let SchemaNodeContentIr::Union(union) = root.content_mut() else {
        panic!("expected union");
    };
    union.interop_mut().variant_repr = Some(VariantReprIr::External);

    assert!(!structural_eq(&lhs, &rhs));
    assert!(assert_structural_eq(&lhs, &rhs).is_err());
}

#[test]
fn emission_override_equivalent_to_default_is_not_structurally_equal() {
    let lhs = base_union_module();
    let mut rhs = base_union_module();

    let ty = rhs.types_mut().values_mut().next().unwrap();
    *ty.rust_binding_mut().emission_mut() = TypeEmissionConfigIr {
        serde_serialize: Some(true),
        serde_deserialize: Some(true),
        derive_allow: Some(IndexSet::from([
            "Debug".to_string(),
            "Serialize".to_string(),
            "Deserialize".to_string(),
        ])),
    };

    assert!(!structural_eq(&lhs, &rhs));
    assert!(assert_structural_eq(&lhs, &rhs).is_err());
}

#[test]
fn insertion_order_does_not_affect_structural_equality_for_index_types() {
    let mut lhs = base_union_module();
    let mut rhs = base_union_module();

    let lhs_ty = lhs.types_mut().values_mut().next().unwrap();
    let lhs_root = lhs_ty
        .schema_nodes_mut()
        .get_mut(&SchemaNodeIrId(0))
        .unwrap();
    let SchemaNodeContentIr::Union(lhs_union) = lhs_root.content_mut() else {
        panic!("expected union");
    };
    lhs_union
        .variants_mut()
        .insert("b".to_string(), SchemaNodeIrId(1));
    *lhs_union.unambiguous_mut() = IndexSet::from(["a".to_string(), "b".to_string()]);
    *lhs_union.deny_untagged_mut() = IndexSet::from(["b".to_string(), "a".to_string()]);

    let rhs_ty = rhs.types_mut().values_mut().next().unwrap();
    let rhs_root = rhs_ty
        .schema_nodes_mut()
        .get_mut(&SchemaNodeIrId(0))
        .unwrap();
    let SchemaNodeContentIr::Union(rhs_union) = rhs_root.content_mut() else {
        panic!("expected union");
    };
    rhs_union
        .variants_mut()
        .insert("b".to_string(), SchemaNodeIrId(1));
    *rhs_union.unambiguous_mut() = IndexSet::from(["b".to_string(), "a".to_string()]);
    *rhs_union.deny_untagged_mut() = IndexSet::from(["a".to_string(), "b".to_string()]);

    assert!(structural_eq(&lhs, &rhs));
    assert!(assert_structural_eq(&lhs, &rhs).is_ok());
}

#[test]
fn metadata_examples_none_and_empty_are_not_structurally_equal() {
    let mut lhs = base_union_module();
    let mut rhs = base_union_module();

    lhs.types_mut()
        .values_mut()
        .next()
        .unwrap()
        .schema_nodes_mut()
        .get_mut(&SchemaNodeIrId(0))
        .unwrap()
        .metadata_mut()
        .examples = None;

    rhs.types_mut()
        .values_mut()
        .next()
        .unwrap()
        .schema_nodes_mut()
        .get_mut(&SchemaNodeIrId(0))
        .unwrap()
        .metadata_mut()
        .examples = Some(Vec::new());

    assert!(!structural_eq(&lhs, &rhs));
    assert!(assert_structural_eq(&lhs, &rhs).is_err());
}
