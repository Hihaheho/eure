use indexmap::IndexMap;

use eure_codegen_ir::*;

#[test]
fn preserves_root_and_default_codegen_metadata() {
    let type_id = TypeId("t".to_string());
    let schema_name = QualifiedTypeName::local("t");

    let mut nodes = IndexMap::new();
    nodes.insert(
        SchemaNodeIrId(0),
        SchemaNodeIr::new(
            SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "name".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(1),
                        false,
                        None,
                        FieldCodegenIr {
                            name_override: Some("username".to_string()),
                        },
                    ),
                )]),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            )),
            SchemaMetadataIr::default(),
            IndexMap::new(),
        ),
    );

    nodes.insert(
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

    let mut module = IrModule::default();
    module.set_root_codegen(RootCodegenIr {
        type_name_override: Some("RootGenerated".to_string()),
    });
    *module.codegen_defaults_mut() = CodegenDefaultsIr {
        derive: vec!["Debug".to_string(), "Clone".to_string()],
        ext_types_field_prefix: "ext_".to_string(),
        ext_types_type_prefix: "Ext".to_string(),
        document_node_id_field: "doc_node".to_string(),
    };

    module.insert_name_index(schema_name.clone(), type_id.clone());
    module.push_root(type_id.clone());
    module.insert_type(
        type_id,
        TypeDefIr::new(
            TypeId("t".to_string()),
            TypeNamesIr::new("T".to_string(), Some(schema_name)),
            nodes,
            SchemaNodeIrId(0),
            RustBindingIr::default(),
            TypeCodegenIr::Record(RecordCodegenIr {
                type_name_override: Some("User".to_string()),
                derive: InheritableCodegenValueIr::Value(vec![
                    "Debug".to_string(),
                    "Serialize".to_string(),
                ]),
            }),
            TypeOriginIr::Schema,
        ),
    );

    assert_eq!(
        module.root_codegen().type_name_override.as_deref(),
        Some("RootGenerated")
    );
    assert_eq!(module.codegen_defaults().document_node_id_field, "doc_node");

    let ty = module.types().values().next().unwrap();
    let TypeCodegenIr::Record(record_codegen) = ty.type_codegen() else {
        panic!("expected record codegen");
    };
    assert_eq!(record_codegen.type_name_override.as_deref(), Some("User"));

    let root = &ty.schema_nodes()[&SchemaNodeIrId(0)];
    let SchemaNodeContentIr::Record(record) = root.content() else {
        panic!("expected record root");
    };
    assert_eq!(
        record.properties()["name"]
            .field_codegen()
            .name_override
            .as_deref(),
        Some("username")
    );
}
