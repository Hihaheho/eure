use crate::codegen::TypeCodegenIr;
use crate::error::{IrBuildError, type_id_string};
use crate::ids::SchemaNodeIrId;
use crate::module::IrModule;
use crate::rust_binding::{FieldModeIr, VariantShapeIr};
use crate::schema::{
    ArraySchemaIr, MapSchemaIr, RecordSchemaIr, SchemaNodeContentIr, TupleSchemaIr, UnionSchemaIr,
};

pub(crate) fn ensure_module_invariants(module: &IrModule) -> Result<(), IrBuildError> {
    if let Some(root_type_name) = &module.root_codegen().type_name_override
        && root_type_name.trim().is_empty()
    {
        return Err(IrBuildError::EmptyRootCodegenOverride {
            path: "root_codegen.type_name_override".to_string(),
        });
    }

    for root in module.roots() {
        if !module.types().contains_key(root) {
            return Err(IrBuildError::RootMissingType {
                type_id: root.0.clone(),
            });
        }
    }

    for (name, id) in module.name_index() {
        let Some(ty) = module.types().get(id) else {
            return Err(IrBuildError::NameIndexMissingType {
                name: name.clone(),
                missing: id.0.clone(),
            });
        };

        if ty.names().schema_name() != Some(name) {
            return Err(IrBuildError::NameIndexMismatch {
                name: name.clone(),
                pointed: id.0.clone(),
                actual: ty.names().schema_name().cloned(),
            });
        }
    }

    for (id, ty) in module.types() {
        let type_id = type_id_string(id);

        if ty.rust_binding().container().proxy_target().is_some()
            && ty.rust_binding().container().opaque_target().is_some()
        {
            return Err(IrBuildError::ProxyOpaqueConflict { type_id });
        }

        validate_type_codegen(module, id, &type_id)?;
        validate_rust_binding(module, id, &type_id)?;
        validate_schema_graph(&type_id, ty.schema_nodes(), ty.semantic_root())?;
    }

    Ok(())
}

fn validate_type_codegen(
    module: &IrModule,
    type_id: &crate::ids::TypeId,
    type_id_str: &str,
) -> Result<(), IrBuildError> {
    let ty = module.types().get(type_id).expect("type must exist");

    let explicit_type_name = match ty.type_codegen() {
        TypeCodegenIr::Record(record) => {
            if let Some(type_name) = &record.type_name_override {
                if type_name.trim().is_empty() {
                    return Err(IrBuildError::EmptyCodegenOverride {
                        type_id: type_id_str.to_string(),
                        path: "type_codegen.record.type_name_override".to_string(),
                    });
                }
                Some(type_name)
            } else {
                None
            }
        }
        TypeCodegenIr::Union(union) => {
            if let Some(type_name) = &union.type_name_override {
                if type_name.trim().is_empty() {
                    return Err(IrBuildError::EmptyCodegenOverride {
                        type_id: type_id_str.to_string(),
                        path: "type_codegen.union.type_name_override".to_string(),
                    });
                }
                Some(type_name)
            } else {
                None
            }
        }
        TypeCodegenIr::None => None,
    };

    if let TypeCodegenIr::Union(union) = ty.type_codegen()
        && let Some(suffix) = &union.variant_types_suffix_override
        && suffix.trim().is_empty()
    {
        return Err(IrBuildError::EmptyCodegenOverride {
            type_id: type_id_str.to_string(),
            path: "type_codegen.union.variant_types_suffix_override".to_string(),
        });
    }

    if module.roots().contains(type_id)
        && let Some(type_name) = explicit_type_name
        && let Some(root_type_name) = &module.root_codegen().type_name_override
        && root_type_name != type_name
    {
        return Err(IrBuildError::RootTypeNameConflict {
            type_id: type_id_str.to_string(),
            root_type_name: root_type_name.clone(),
            type_type_name: type_name.clone(),
        });
    }

    Ok(())
}

fn validate_rust_binding(
    module: &IrModule,
    type_id: &crate::ids::TypeId,
    type_id_str: &str,
) -> Result<(), IrBuildError> {
    let ty = module.types().get(type_id).expect("type must exist");

    if let Some(schema_name) = ty.names().schema_name()
        && let Some(other_id) = module.name_index().get(schema_name)
        && other_id != type_id
    {
        return Err(IrBuildError::DuplicateSchemaName {
            type_id: type_id_str.to_string(),
            schema_name: schema_name.clone(),
        });
    }

    let parse_ext = ty.rust_binding().container().parse_ext();

    for field in ty.rust_binding().fields() {
        let flags = field.source_attrs();

        let active =
            usize::from(flags.ext) + usize::from(flags.flatten) + usize::from(flags.flatten_ext);
        if active > 1 {
            return Err(IrBuildError::FieldModeConflict {
                type_id: type_id_str.to_string(),
                field: field.rust_name().to_string(),
                detail: "ext/flatten/flatten_ext are mutually exclusive".to_string(),
            });
        }

        if field.via().is_some()
            && matches!(field.mode(), FieldModeIr::Flatten | FieldModeIr::FlattenExt)
        {
            return Err(IrBuildError::ViaWithFlatten {
                type_id: type_id_str.to_string(),
                field: field.rust_name().to_string(),
            });
        }

        if !matches!(field.default(), crate::rust_binding::DefaultValueIr::None)
            && matches!(field.mode(), FieldModeIr::Flatten | FieldModeIr::FlattenExt)
        {
            return Err(IrBuildError::DefaultWithFlatten {
                type_id: type_id_str.to_string(),
                field: field.rust_name().to_string(),
            });
        }

        if parse_ext && matches!(field.mode(), FieldModeIr::Flatten) {
            return Err(IrBuildError::FlattenInParseExt {
                type_id: type_id_str.to_string(),
                field: field.rust_name().to_string(),
            });
        }
    }

    for variant in ty.rust_binding().variants() {
        if variant.allow_unknown_fields() && !matches!(variant.shape(), VariantShapeIr::Record(_)) {
            return Err(IrBuildError::VariantAllowUnknownFieldsInvalid {
                type_id: type_id_str.to_string(),
                variant: variant.rust_name().to_string(),
            });
        }
    }

    Ok(())
}

fn validate_schema_graph(
    type_id_str: &str,
    nodes: &indexmap::IndexMap<SchemaNodeIrId, crate::schema::SchemaNodeIr>,
    root: SchemaNodeIrId,
) -> Result<(), IrBuildError> {
    if !nodes.contains_key(&root) {
        return Err(IrBuildError::MissingSemanticRoot {
            type_id: type_id_str.to_string(),
            node: root,
        });
    }

    for (node_id, node) in nodes {
        match node.content() {
            SchemaNodeContentIr::Array(schema) => {
                validate_array_refs(type_id_str, *node_id, schema, nodes)?;
            }
            SchemaNodeContentIr::Map(schema) => {
                validate_map_refs(type_id_str, *node_id, schema, nodes)?;
            }
            SchemaNodeContentIr::Record(schema) => {
                validate_record_refs(type_id_str, *node_id, schema, nodes)?;
                validate_record_field_codegen(type_id_str, *node_id, schema)?;
            }
            SchemaNodeContentIr::Tuple(schema) => {
                validate_tuple_refs(type_id_str, *node_id, schema, nodes)?;
            }
            SchemaNodeContentIr::Union(schema) => {
                validate_union_refs(type_id_str, *node_id, schema, nodes)?;
            }
            _ => {}
        }

        for (ext_name, ext_schema) in node.ext_types() {
            ensure_node_exists(
                type_id_str,
                *node_id,
                ext_schema.schema,
                format!("ext_types.{ext_name}"),
                nodes,
            )?;
        }
    }

    Ok(())
}

fn validate_record_field_codegen(
    type_id_str: &str,
    node_id: SchemaNodeIrId,
    schema: &RecordSchemaIr,
) -> Result<(), IrBuildError> {
    for (field_name, field) in schema.properties() {
        if let Some(name) = &field.field_codegen().name_override
            && name.trim().is_empty()
        {
            return Err(IrBuildError::EmptyCodegenOverride {
                type_id: type_id_str.to_string(),
                path: format!(
                    "schema_nodes.{node_id:?}.record.properties.{field_name}.field_codegen.name_override"
                ),
            });
        }
    }
    Ok(())
}

fn validate_array_refs(
    type_id_str: &str,
    node_id: SchemaNodeIrId,
    schema: &ArraySchemaIr,
    nodes: &indexmap::IndexMap<SchemaNodeIrId, crate::schema::SchemaNodeIr>,
) -> Result<(), IrBuildError> {
    ensure_node_exists(
        type_id_str,
        node_id,
        schema.item,
        "array.item".to_string(),
        nodes,
    )?;
    if let Some(contains) = schema.contains {
        ensure_node_exists(
            type_id_str,
            node_id,
            contains,
            "array.contains".to_string(),
            nodes,
        )?;
    }
    Ok(())
}

fn validate_map_refs(
    type_id_str: &str,
    node_id: SchemaNodeIrId,
    schema: &MapSchemaIr,
    nodes: &indexmap::IndexMap<SchemaNodeIrId, crate::schema::SchemaNodeIr>,
) -> Result<(), IrBuildError> {
    ensure_node_exists(
        type_id_str,
        node_id,
        schema.key,
        "map.key".to_string(),
        nodes,
    )?;
    ensure_node_exists(
        type_id_str,
        node_id,
        schema.value,
        "map.value".to_string(),
        nodes,
    )?;
    Ok(())
}

fn validate_record_refs(
    type_id_str: &str,
    node_id: SchemaNodeIrId,
    schema: &RecordSchemaIr,
    nodes: &indexmap::IndexMap<SchemaNodeIrId, crate::schema::SchemaNodeIr>,
) -> Result<(), IrBuildError> {
    for (name, field) in schema.properties() {
        ensure_node_exists(
            type_id_str,
            node_id,
            field.schema(),
            format!("record.properties.{name}.schema"),
            nodes,
        )?;
    }

    for (idx, flatten_id) in schema.flatten().iter().enumerate() {
        ensure_node_exists(
            type_id_str,
            node_id,
            *flatten_id,
            format!("record.flatten[{idx}]"),
            nodes,
        )?;
    }

    if let crate::schema::UnknownFieldsPolicyIr::Schema(ref_id) = schema.unknown_fields() {
        ensure_node_exists(
            type_id_str,
            node_id,
            *ref_id,
            "record.unknown_fields.schema".to_string(),
            nodes,
        )?;
    }

    Ok(())
}

fn validate_tuple_refs(
    type_id_str: &str,
    node_id: SchemaNodeIrId,
    schema: &TupleSchemaIr,
    nodes: &indexmap::IndexMap<SchemaNodeIrId, crate::schema::SchemaNodeIr>,
) -> Result<(), IrBuildError> {
    for (idx, element) in schema.elements.iter().enumerate() {
        ensure_node_exists(
            type_id_str,
            node_id,
            *element,
            format!("tuple.elements[{idx}]"),
            nodes,
        )?;
    }
    Ok(())
}

fn validate_union_refs(
    type_id_str: &str,
    node_id: SchemaNodeIrId,
    schema: &UnionSchemaIr,
    nodes: &indexmap::IndexMap<SchemaNodeIrId, crate::schema::SchemaNodeIr>,
) -> Result<(), IrBuildError> {
    for (variant, schema_id) in schema.variants() {
        ensure_node_exists(
            type_id_str,
            node_id,
            *schema_id,
            format!("union.variants.{variant}"),
            nodes,
        )?;
    }

    for variant in schema.unambiguous().iter().chain(schema.deny_untagged()) {
        if !schema.variants().contains_key(variant) {
            return Err(IrBuildError::UnionPolicyUnknownVariant {
                type_id: type_id_str.to_string(),
                node: node_id,
                variant: variant.clone(),
            });
        }
    }

    Ok(())
}

fn ensure_node_exists(
    type_id_str: &str,
    node: SchemaNodeIrId,
    target: SchemaNodeIrId,
    path: String,
    nodes: &indexmap::IndexMap<SchemaNodeIrId, crate::schema::SchemaNodeIr>,
) -> Result<(), IrBuildError> {
    if !nodes.contains_key(&target) {
        return Err(IrBuildError::MissingSchemaNodeReference {
            type_id: type_id_str.to_string(),
            node,
            target,
            path,
        });
    }
    Ok(())
}
