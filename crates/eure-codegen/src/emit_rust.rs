use std::collections::BTreeSet;

use eure_codegen_ir::{
    EmissionDefaultsIr, InheritableCodegenValueIr, IrModule, RecordSchemaIr, SchemaNodeContentIr,
    SchemaNodeIrId, TypeCodegenIr, TypeDefIr, UnionSchemaIr, UnknownFieldsPolicyIr,
    filter_desired_derives,
};

use crate::GenerationConfig;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmitRustError {
    #[error("type `{type_id}` references missing schema node {node:?}")]
    MissingSchemaNode {
        type_id: String,
        node: SchemaNodeIrId,
    },

    #[error("type `{type_id}` is not yet supported for Rust emission: {reason}")]
    NotYetSupported { type_id: String, reason: String },
}

pub fn emit_rust_types(
    module: &IrModule,
    config: &GenerationConfig,
) -> Result<String, EmitRustError> {
    let mut ids = module.types().keys().cloned().collect::<Vec<_>>();
    ids.sort();

    let mut chunks = Vec::new();
    for type_id in ids {
        let ty = &module.types()[&type_id];
        chunks.push(emit_type(module, ty, config)?);
    }

    Ok(chunks.join("\n\n"))
}

fn emit_type(
    module: &IrModule,
    ty: &TypeDefIr,
    config: &GenerationConfig,
) -> Result<String, EmitRustError> {
    let root = ty.schema_nodes().get(&ty.semantic_root()).ok_or_else(|| {
        EmitRustError::MissingSchemaNode {
            type_id: ty.id().0.clone(),
            node: ty.semantic_root(),
        }
    })?;

    let visibility = match config.visibility {
        crate::Visibility::Pub => "pub ",
        crate::Visibility::PubCrate => "pub(crate) ",
        crate::Visibility::Private => "",
    };

    let mut header = Vec::new();
    if config.allow_warnings {
        header.push("#[allow(dead_code)]".to_string());
    }

    let type_name = effective_type_name(module, ty);
    let derives = effective_derives(module, ty, config);
    if !derives.is_empty() {
        header.push(format!("#[derive({})]", derives.join(", ")));
    }

    let body = match &root.content() {
        SchemaNodeContentIr::Record(record) => {
            emit_record_type(module, ty, &type_name, visibility, record)?
        }
        SchemaNodeContentIr::Union(union) => {
            emit_union_type(module, ty, &type_name, visibility, union)?
        }
        SchemaNodeContentIr::Null => format!("{visibility}struct {type_name};"),
        _ => emit_newtype(module, ty, &type_name, visibility, ty.semantic_root())?,
    };

    if header.is_empty() {
        Ok(body)
    } else {
        Ok(format!("{}\n{body}", header.join("\n")))
    }
}

fn effective_type_name(module: &IrModule, ty: &TypeDefIr) -> String {
    if module.roots().contains(ty.id())
        && let Some(root_type_name) = &module.root_codegen().type_name_override
    {
        return root_type_name.clone();
    }

    match &ty.type_codegen() {
        TypeCodegenIr::Record(record) => record
            .type_name_override
            .clone()
            .unwrap_or_else(|| ty.names().rust_name().to_string()),
        TypeCodegenIr::Union(union) => union
            .type_name_override
            .clone()
            .unwrap_or_else(|| ty.names().rust_name().to_string()),
        TypeCodegenIr::None => ty.names().rust_name().to_string(),
    }
}

fn effective_derives(module: &IrModule, ty: &TypeDefIr, config: &GenerationConfig) -> Vec<String> {
    let desired = match &ty.type_codegen() {
        TypeCodegenIr::Record(record) => resolve_derive_codegen(&record.derive, module),
        TypeCodegenIr::Union(union) => resolve_derive_codegen(&union.derive, module),
        TypeCodegenIr::None => module.codegen_defaults().derive.clone(),
    };

    let defaults = EmissionDefaultsIr {
        serde_serialize: module.emission_defaults().serde_serialize && config.serde_serialize,
        serde_deserialize: module.emission_defaults().serde_deserialize && config.serde_deserialize,
        derive_allow: module.emission_defaults().derive_allow.clone(),
    };

    filter_desired_derives(&desired, &defaults, ty.rust_binding().emission())
}

fn emit_record_type(
    module: &IrModule,
    ty: &TypeDefIr,
    type_name: &str,
    visibility: &str,
    record: &RecordSchemaIr,
) -> Result<String, EmitRustError> {
    if !record.flatten().is_empty() {
        return Err(EmitRustError::NotYetSupported {
            type_id: ty.id().0.clone(),
            reason: "record flatten fields".to_string(),
        });
    }
    if matches!(record.unknown_fields(), UnknownFieldsPolicyIr::Schema(_)) {
        return Err(EmitRustError::NotYetSupported {
            type_id: ty.id().0.clone(),
            reason: "schema-driven unknown_fields policy".to_string(),
        });
    }

    let mut field_lines = Vec::new();
    for (wire_name, field) in record.properties() {
        let rust_name = sanitize_field_name(
            field
                .field_codegen()
                .name_override
                .as_deref()
                .unwrap_or(wire_name.as_str()),
        );
        let field_ty = schema_node_type(module, ty, field.schema(), &mut BTreeSet::new())?;
        let field_ty = if field.optional() {
            format!("Option<{field_ty}>")
        } else {
            field_ty
        };
        field_lines.push(format!("    {visibility}{rust_name}: {field_ty},"));
    }

    Ok(format!(
        "{visibility}struct {type_name} {{\n{}\n}}",
        field_lines.join("\n")
    ))
}

fn emit_union_type(
    module: &IrModule,
    ty: &TypeDefIr,
    type_name: &str,
    visibility: &str,
    union: &UnionSchemaIr,
) -> Result<String, EmitRustError> {
    if matches!(
        &ty.type_codegen(),
        TypeCodegenIr::Union(union_codegen) if union_codegen.variant_types
    ) {
        return Err(EmitRustError::NotYetSupported {
            type_id: ty.id().0.clone(),
            reason: "union-codegen variant_types".to_string(),
        });
    }

    let mut variants = Vec::new();
    for (wire_name, schema_id) in union.variants() {
        let variant_name = sanitize_variant_name(wire_name);
        let node =
            ty.schema_nodes()
                .get(schema_id)
                .ok_or_else(|| EmitRustError::MissingSchemaNode {
                    type_id: ty.id().0.clone(),
                    node: *schema_id,
                })?;

        let variant_line = match &node.content() {
            SchemaNodeContentIr::Null => format!("    {variant_name},"),
            SchemaNodeContentIr::Tuple(tuple) => {
                let mut items = Vec::new();
                for element in &tuple.elements {
                    items.push(schema_node_type(
                        module,
                        ty,
                        *element,
                        &mut BTreeSet::new(),
                    )?);
                }
                format!("    {variant_name}({}),", items.join(", "))
            }
            SchemaNodeContentIr::Record(record) => {
                if !record.flatten().is_empty() {
                    return Err(EmitRustError::NotYetSupported {
                        type_id: ty.id().0.clone(),
                        reason: format!("flattened struct variant `{wire_name}`"),
                    });
                }
                let mut fields = Vec::new();
                for (name, field) in record.properties() {
                    let rust_name = sanitize_field_name(
                        field
                            .field_codegen()
                            .name_override
                            .as_deref()
                            .unwrap_or(name.as_str()),
                    );
                    let ty_str =
                        schema_node_type(module, ty, field.schema(), &mut BTreeSet::new())?;
                    let ty_str = if field.optional() {
                        format!("Option<{ty_str}>")
                    } else {
                        ty_str
                    };
                    fields.push(format!("{rust_name}: {ty_str}"));
                }
                format!("    {variant_name} {{ {} }},", fields.join(", "))
            }
            _ => {
                let ty_str = schema_node_type(module, ty, *schema_id, &mut BTreeSet::new())?;
                format!("    {variant_name}({ty_str}),")
            }
        };
        variants.push(variant_line);
    }

    // Keep the `module` read to make future cross-module type emission explicit.
    let _ = module.types().len();

    Ok(format!(
        "{visibility}enum {type_name} {{\n{}\n}}",
        variants.join("\n")
    ))
}

fn emit_newtype(
    module: &IrModule,
    ty: &TypeDefIr,
    type_name: &str,
    visibility: &str,
    node_id: SchemaNodeIrId,
) -> Result<String, EmitRustError> {
    let inner_ty = schema_node_type(module, ty, node_id, &mut BTreeSet::new())?;

    // Keep the `module` read to make future cross-module type emission explicit.
    let _ = module.roots().len();

    Ok(format!(
        "{visibility}struct {type_name}({visibility}{inner_ty});"
    ))
}

fn schema_node_type(
    module: &IrModule,
    ty: &TypeDefIr,
    node_id: SchemaNodeIrId,
    visiting: &mut BTreeSet<SchemaNodeIrId>,
) -> Result<String, EmitRustError> {
    if !visiting.insert(node_id) {
        return Err(EmitRustError::NotYetSupported {
            type_id: ty.id().0.clone(),
            reason: format!("recursive inline node graph at {:?}", node_id),
        });
    }

    let node = ty
        .schema_nodes()
        .get(&node_id)
        .ok_or_else(|| EmitRustError::MissingSchemaNode {
            type_id: ty.id().0.clone(),
            node: node_id,
        })?;

    let out = match &node.content() {
        SchemaNodeContentIr::Any | SchemaNodeContentIr::Literal(_) => {
            "::eure_document::document::EureDocument".to_string()
        }
        SchemaNodeContentIr::Text(_) => "String".to_string(),
        SchemaNodeContentIr::Integer(_) => "i64".to_string(),
        SchemaNodeContentIr::Float(_) => "f64".to_string(),
        SchemaNodeContentIr::Boolean => "bool".to_string(),
        SchemaNodeContentIr::Null => "()".to_string(),
        SchemaNodeContentIr::Array(array) => {
            let item = schema_node_type(module, ty, array.item, visiting)?;
            format!("Vec<{item}>")
        }
        SchemaNodeContentIr::Tuple(tuple) => {
            let mut items = Vec::new();
            for element in &tuple.elements {
                items.push(schema_node_type(module, ty, *element, visiting)?);
            }
            if items.is_empty() {
                "()".to_string()
            } else if items.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        SchemaNodeContentIr::Map(map) => {
            let key = schema_node_type(module, ty, map.key, visiting)?;
            if key != "String" {
                return Err(EmitRustError::NotYetSupported {
                    type_id: ty.id().0.clone(),
                    reason: "map keys other than text".to_string(),
                });
            }
            let value = schema_node_type(module, ty, map.value, visiting)?;
            format!("::std::collections::BTreeMap<String, {value}>")
        }
        SchemaNodeContentIr::Reference(reference) => resolve_reference_type_name(module, reference),
        SchemaNodeContentIr::Record(_) => {
            return Err(EmitRustError::NotYetSupported {
                type_id: ty.id().0.clone(),
                reason: "inline record nodes as field/newtype types".to_string(),
            });
        }
        SchemaNodeContentIr::Union(_) => {
            return Err(EmitRustError::NotYetSupported {
                type_id: ty.id().0.clone(),
                reason: "inline union nodes as field/newtype types".to_string(),
            });
        }
    };

    visiting.remove(&node_id);
    Ok(out)
}

fn resolve_derive_codegen(
    derive: &InheritableCodegenValueIr<Vec<String>>,
    module: &IrModule,
) -> Vec<String> {
    match derive {
        InheritableCodegenValueIr::InheritCodegenDefaults => {
            module.codegen_defaults().derive.clone()
        }
        InheritableCodegenValueIr::Value(derive) => derive.clone(),
    }
}

fn resolve_reference_type_name(
    module: &IrModule,
    reference: &eure_codegen_ir::QualifiedTypeName,
) -> String {
    if let Some(type_id) = module.name_index().get(reference)
        && let Some(ty) = module.types().get(type_id)
    {
        return effective_type_name(module, ty);
    }

    if let Some(namespace) = &reference.namespace {
        format!("{namespace}::{}", reference.name)
    } else {
        reference.name.clone()
    }
}

fn sanitize_field_name(raw: &str) -> String {
    let mut out = String::new();
    let mut previous_is_sep = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            if previous_is_sep && !out.is_empty() {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_is_sep = false;
        } else {
            previous_is_sep = true;
        }
    }
    if out.is_empty() {
        out.push_str("field");
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if is_rust_keyword(&out) {
        format!("r#{out}")
    } else {
        out
    }
}

fn sanitize_variant_name(raw: &str) -> String {
    let mut out = String::new();
    let mut next_upper = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            if next_upper {
                out.push(ch.to_ascii_uppercase());
                next_upper = false;
            } else {
                out.push(ch);
            }
        } else {
            next_upper = true;
        }
    }
    if out.is_empty() {
        out.push_str("Variant");
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn is_rust_keyword(ident: &str) -> bool {
    matches!(
        ident,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use eure_codegen_ir::{
        FieldCodegenIr, IrModule, QualifiedTypeName, RecordFieldSchemaIr, RecordSchemaIr,
        RootCodegenIr, RustBindingIr, RustTypeKindIr, SchemaMetadataIr, SchemaNodeContentIr,
        SchemaNodeIr, SchemaNodeIrId, TextSchemaIr, TupleSchemaIr, TypeCodegenIr, TypeDefIr,
        TypeId, TypeNamesIr, TypeOriginIr, UnionSchemaIr, UnknownFieldsPolicyIr,
    };

    use super::*;

    fn node(content: SchemaNodeContentIr) -> SchemaNodeIr {
        SchemaNodeIr::new(content, SchemaMetadataIr::default(), IndexMap::new())
    }

    fn binding(kind: RustTypeKindIr) -> RustBindingIr {
        RustBindingIr::new(
            kind,
            Default::default(),
            Vec::new(),
            Vec::new(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    fn type_def(
        id: &str,
        rust_name: &str,
        schema_name: Option<QualifiedTypeName>,
        nodes: IndexMap<SchemaNodeIrId, SchemaNodeIr>,
        semantic_root: SchemaNodeIrId,
        kind: RustTypeKindIr,
    ) -> TypeDefIr {
        TypeDefIr::new(
            TypeId(id.to_string()),
            TypeNamesIr::new(rust_name.to_string(), schema_name),
            nodes,
            semantic_root,
            binding(kind),
            TypeCodegenIr::None,
            TypeOriginIr::Schema,
        )
    }

    fn base_module(type_def: TypeDefIr) -> IrModule {
        let mut module = IrModule::default();
        module.insert_type(type_def.id().clone(), type_def.clone());
        module.push_root(type_def.id().clone());
        module
    }

    #[test]
    fn emits_record_and_union_shapes() {
        let mut nodes = IndexMap::new();
        nodes.insert(
            SchemaNodeIrId(1),
            node(SchemaNodeContentIr::Text(eure_codegen_ir::TextSchemaIr {
                language: None,
                min_length: None,
                max_length: None,
                pattern: None,
                unknown_fields: IndexMap::new(),
            })),
        );
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "user-name".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(1),
                        false,
                        None,
                        FieldCodegenIr::default(),
                    ),
                )]),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            ))),
        );
        let record = type_def(
            "record",
            "User",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        );

        let module = base_module(record);
        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(generated.contains("struct User"));
        assert!(generated.contains("user_name: String"));
    }

    #[test]
    fn rejects_flattened_records_for_now() {
        let mut nodes = IndexMap::new();
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::new(),
                vec![SchemaNodeIrId(1)],
                UnknownFieldsPolicyIr::Deny,
            ))),
        );
        nodes.insert(
            SchemaNodeIrId(1),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::new(),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            ))),
        );
        let ty = type_def(
            "flattened",
            "Flattened",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        );

        let err = emit_rust_types(&base_module(ty), &GenerationConfig::builder().build())
            .expect_err("flatten should be unsupported for now");
        assert!(matches!(err, EmitRustError::NotYetSupported { .. }));
    }

    #[test]
    fn emits_simple_union_enum() {
        let mut nodes = IndexMap::new();
        nodes.insert(SchemaNodeIrId(1), node(SchemaNodeContentIr::Null));
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Union(UnionSchemaIr::new(
                IndexMap::from([("ok".to_string(), SchemaNodeIrId(1))]),
                Default::default(),
                Default::default(),
                Default::default(),
            ))),
        );
        let ty = type_def(
            "union",
            "ResultType",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Enum,
        );

        let generated = emit_rust_types(&base_module(ty), &GenerationConfig::builder().build())
            .expect("emit rust");
        assert!(generated.contains("enum ResultType"));
        assert!(generated.contains("Ok"));
    }

    #[test]
    fn emits_reference_fields_using_referenced_rust_type_name() {
        let referenced_type_id = TypeId("a_referenced".to_string());
        let referenced_schema_name = QualifiedTypeName::local("user-profile");
        let root_type_id = TypeId("b_root".to_string());
        let root_schema_name = QualifiedTypeName::local("root");

        let referenced_nodes =
            IndexMap::from([(SchemaNodeIrId(0), node(SchemaNodeContentIr::Null))]);
        let referenced = type_def(
            "a_referenced",
            "UserProfile",
            Some(referenced_schema_name.clone()),
            referenced_nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Unit,
        );

        let root_nodes = IndexMap::from([
            (
                SchemaNodeIrId(1),
                node(SchemaNodeContentIr::Reference(
                    referenced_schema_name.clone(),
                )),
            ),
            (
                SchemaNodeIrId(0),
                node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    IndexMap::from([(
                        "profile".to_string(),
                        RecordFieldSchemaIr::new(
                            SchemaNodeIrId(1),
                            false,
                            None,
                            FieldCodegenIr::default(),
                        ),
                    )]),
                    Vec::new(),
                    UnknownFieldsPolicyIr::Deny,
                ))),
            ),
        ]);
        let root = type_def(
            "b_root",
            "Root",
            Some(root_schema_name.clone()),
            root_nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        );

        let mut module = IrModule::default();
        module.insert_name_index(referenced_schema_name, referenced_type_id.clone());
        module.insert_name_index(root_schema_name, root_type_id.clone());
        module.insert_type(referenced_type_id, referenced);
        module.insert_type(root_type_id.clone(), root);
        module.push_root(root_type_id);

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("profile: UserProfile"),
            "expected reference field to use referenced rust type name, got:\n{generated}"
        );
    }

    #[test]
    fn applies_root_codegen_type_name_override_for_root_type() {
        let mut nodes = IndexMap::new();
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::new(),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            ))),
        );

        let mut module = base_module(type_def(
            "root",
            "SchemaRoot",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        ));
        module.set_root_codegen(RootCodegenIr {
            type_name_override: Some("ConfiguredRoot".to_string()),
        });

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("struct ConfiguredRoot"),
            "expected root override type name in generated rust, got:\n{generated}"
        );
    }

    #[test]
    fn emits_single_element_tuple_types_with_trailing_comma() {
        let mut nodes = IndexMap::new();
        nodes.insert(
            SchemaNodeIrId(2),
            node(SchemaNodeContentIr::Text(TextSchemaIr {
                language: None,
                min_length: None,
                max_length: None,
                pattern: None,
                unknown_fields: IndexMap::new(),
            })),
        );
        nodes.insert(
            SchemaNodeIrId(1),
            node(SchemaNodeContentIr::Tuple(TupleSchemaIr {
                elements: vec![SchemaNodeIrId(2)],
                binding_style: None,
            })),
        );
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "coords".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(1),
                        false,
                        None,
                        FieldCodegenIr::default(),
                    ),
                )]),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            ))),
        );

        let module = base_module(type_def(
            "tuple-field",
            "TupleField",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        ));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("coords: (String,),"),
            "expected single-element tuple type syntax with trailing comma, got:\n{generated}"
        );
    }
}
