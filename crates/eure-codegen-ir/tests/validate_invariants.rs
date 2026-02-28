use indexmap::IndexMap;

use eure_codegen_ir::*;

fn base_module() -> IrModule {
    let type_id = TypeId("example".to_string());
    let schema_name = QualifiedTypeName::local("example");

    let mut schema_nodes = IndexMap::new();
    schema_nodes.insert(
        SchemaNodeIrId(0),
        SchemaNodeIr::new(
            SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::new(),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            )),
            SchemaMetadataIr::default(),
            IndexMap::new(),
        ),
    );

    let type_def = TypeDefIr::new(
        type_id.clone(),
        TypeNamesIr::new("Example".to_string(), Some(schema_name.clone())),
        schema_nodes,
        SchemaNodeIrId(0),
        RustBindingIr::new(
            RustTypeKindIr::Record,
            ContainerAttrsIr::default(),
            Vec::new(),
            Vec::new(),
            RustGenericsIr::default(),
            WhereClauseIr::default(),
            TypeEmissionConfigIr::default(),
        ),
        TypeCodegenIr::None,
        TypeOriginIr::Derive,
    );

    let mut module = IrModule::default();
    module.insert_name_index(schema_name, type_id.clone());
    module.push_root(type_id.clone());
    module.insert_type(type_id, type_def);
    module
}

#[test]
fn rejects_proxy_and_opaque_together() {
    let mut module = base_module();
    let ty = module.types_mut().values_mut().next().unwrap();
    *ty.rust_binding_mut().container_mut().proxy_target_mut() =
        Some(RustPathIr::new("external::Proxy"));
    *ty.rust_binding_mut().container_mut().opaque_target_mut() =
        Some(RustPathIr::new("external::Opaque"));

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::ProxyOpaqueConflict { .. }));
}

#[test]
fn rejects_conflicting_field_modes() {
    let mut module = base_module();
    let ty = module.types_mut().values_mut().next().unwrap();

    ty.rust_binding_mut().push_field(RustFieldIr::new(
        "field".to_string(),
        "field".to_string(),
        FieldModeIr::Record,
        FieldSourceAttrsIr {
            ext: true,
            flatten: true,
            flatten_ext: false,
        },
        RustTypeExprIr::Primitive(PrimitiveRustTypeIr::String),
        DefaultValueIr::None,
        None,
    ));

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::FieldModeConflict { .. }));
}

#[test]
fn rejects_allow_unknown_fields_on_non_record_variant() {
    let mut module = base_module();
    let ty = module.types_mut().values_mut().next().unwrap();
    ty.rust_binding_mut().set_kind(RustTypeKindIr::Enum);
    ty.rust_binding_mut().push_variant(RustVariantIr::new(
        "A".to_string(),
        "a".to_string(),
        true,
        VariantShapeIr::Unit,
    ));

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(
        err,
        IrBuildError::VariantAllowUnknownFieldsInvalid { .. }
    ));
}

#[test]
fn rejects_flatten_in_parse_ext_container() {
    let mut module = base_module();
    let ty = module.types_mut().values_mut().next().unwrap();
    *ty.rust_binding_mut().container_mut().parse_ext_mut() = true;
    ty.rust_binding_mut().push_field(RustFieldIr::new(
        "field".to_string(),
        "field".to_string(),
        FieldModeIr::Flatten,
        FieldSourceAttrsIr {
            ext: false,
            flatten: true,
            flatten_ext: false,
        },
        RustTypeExprIr::Primitive(PrimitiveRustTypeIr::String),
        DefaultValueIr::None,
        None,
    ));

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::FlattenInParseExt { .. }));
}

#[test]
fn rejects_empty_codegen_overrides() {
    let mut module = base_module();
    module.root_codegen_mut().type_name_override = Some("".to_string());

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::EmptyRootCodegenOverride { .. }));

    let mut module = base_module();
    let ty = module.types_mut().values_mut().next().unwrap();
    *ty.type_codegen_mut() = TypeCodegenIr::Record(RecordCodegenIr {
        type_name_override: Some("  ".to_string()),
        derive: InheritableCodegenValueIr::InheritCodegenDefaults,
    });

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::EmptyCodegenOverride { .. }));

    let mut module = base_module();
    let ty = module.types_mut().values_mut().next().unwrap();
    *ty.type_codegen_mut() = TypeCodegenIr::Union(UnionCodegenIr {
        type_name_override: Some("ExampleUnion".to_string()),
        derive: InheritableCodegenValueIr::InheritCodegenDefaults,
        variant_types: false,
        variant_types_suffix_override: Some("  ".to_string()),
    });

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::EmptyCodegenOverride { .. }));

    let mut module = base_module();
    {
        let ty = module.types_mut().values_mut().next().unwrap();
        ty.schema_nodes_mut().insert(
            SchemaNodeIrId(1),
            SchemaNodeIr::new(
                SchemaNodeContentIr::Text(TextSchemaIr {
                    language: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    unknown_fields: IndexMap::new(),
                }),
                SchemaMetadataIr::default(),
                IndexMap::new(),
            ),
        );
        let root = ty.schema_nodes_mut().get_mut(&SchemaNodeIrId(0)).unwrap();
        let SchemaNodeContentIr::Record(record) = root.content_mut() else {
            panic!("expected record root");
        };
        record.properties_mut().insert(
            "field".to_string(),
            RecordFieldSchemaIr::new(
                SchemaNodeIrId(1),
                false,
                None,
                FieldCodegenIr {
                    name_override: Some("".to_string()),
                },
            ),
        );
    }

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::EmptyCodegenOverride { .. }));
}

#[test]
fn rejects_conflicting_root_and_type_codegen_type_names() {
    let mut module = base_module();
    module.root_codegen_mut().type_name_override = Some("Root".to_string());

    let ty = module.types_mut().values_mut().next().unwrap();
    *ty.type_codegen_mut() = TypeCodegenIr::Record(RecordCodegenIr {
        type_name_override: Some("NotRoot".to_string()),
        derive: InheritableCodegenValueIr::InheritCodegenDefaults,
    });

    let err = module.clone().into_checked().unwrap_err();
    assert!(matches!(err, IrBuildError::RootTypeNameConflict { .. }));
}
