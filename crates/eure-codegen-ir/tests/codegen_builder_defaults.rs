use eure_codegen_ir::{
    CodegenDefaultsIr, FieldCodegenIr, InheritableCodegenValueIr, RecordCodegenIr, RootCodegenIr,
    UnionCodegenIr,
};

#[test]
fn builders_use_default_values_for_non_inheriting_fields() {
    let root = RootCodegenIr::builder().build();
    assert!(root.type_name_override.is_none());

    let defaults = CodegenDefaultsIr::builder().build();
    assert!(defaults.derive.is_empty());
    assert!(defaults.ext_types_field_prefix.is_empty());
    assert!(defaults.ext_types_type_prefix.is_empty());
    assert!(defaults.document_node_id_field.is_empty());

    let field = FieldCodegenIr::builder().build();
    assert!(field.name_override.is_none());
}

#[test]
fn builders_default_to_inherit_for_codegen_default_aware_fields() {
    let record = RecordCodegenIr::builder().build();
    assert!(record.type_name_override.is_none());
    assert_eq!(
        record.derive,
        InheritableCodegenValueIr::InheritCodegenDefaults
    );

    let union = UnionCodegenIr::builder().build();
    assert!(union.type_name_override.is_none());
    assert_eq!(
        union.derive,
        InheritableCodegenValueIr::InheritCodegenDefaults
    );
    assert!(!union.variant_types);
    assert!(union.variant_types_suffix_override.is_none());
}
