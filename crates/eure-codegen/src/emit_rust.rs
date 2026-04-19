use std::collections::BTreeSet;

use eure_codegen_ir::{
    DEFAULT_VARIANT_TYPES_SUFFIX, EmissionDefaultsIr, InheritableCodegenValueIr, IrModule,
    RecordSchemaIr, SchemaNodeContentIr, SchemaNodeIrId, TypeCodegenIr, TypeDefIr, UnionSchemaIr,
    UnknownFieldsPolicyIr, filter_desired_derives,
};

use crate::GenerationConfig;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmitRustUnsupportedReason {
    #[error("record flatten fields")]
    RecordFlattenFields,

    #[error("schema-driven unknown_fields policy")]
    SchemaDrivenUnknownFieldsPolicy,

    #[error("recursive inline node graph at {node:?}")]
    RecursiveInlineNodeGraph { node: SchemaNodeIrId },

    #[error("map keys other than text")]
    MapKeysOtherThanText,

    #[error("inline union nodes as field/newtype types")]
    InlineUnionNodesAsFieldOrNewtypeTypes,

    #[error("flattened struct variant `{variant_wire_name}`")]
    FlattenedStructVariant { variant_wire_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmitRustError {
    #[error("type `{type_id}` references missing schema node {node:?}")]
    MissingSchemaNode {
        type_id: String,
        node: SchemaNodeIrId,
    },

    #[error("type `{type_id}` is not yet supported for Rust emission: {reason}")]
    NotYetSupported {
        type_id: String,
        reason: EmitRustUnsupportedReason,
    },
}

struct NestedTypeEmitter<'a> {
    module: &'a IrModule,
    ty: &'a TypeDefIr,
    allow_warnings: bool,
    visibility: &'a str,
    derives: Vec<String>,
    used_type_names: BTreeSet<String>,
    generated_chunks: Vec<String>,
}

impl<'a> NestedTypeEmitter<'a> {
    fn new(
        module: &'a IrModule,
        ty: &'a TypeDefIr,
        allow_warnings: bool,
        visibility: &'a str,
        derives: Vec<String>,
    ) -> Self {
        let used_type_names = module
            .types()
            .values()
            .map(|candidate| effective_type_name(module, candidate))
            .collect();

        Self {
            module,
            ty,
            allow_warnings,
            visibility,
            derives,
            used_type_names,
            generated_chunks: Vec::new(),
        }
    }

    fn type_id(&self) -> &str {
        &self.ty.id().0
    }

    fn render_type_block(&self, body: String) -> String {
        let mut header = Vec::new();
        if self.allow_warnings {
            header.push("#[allow(dead_code)]".to_string());
        }
        if !self.derives.is_empty() {
            header.push(format!("#[derive({})]", self.derives.join(", ")));
        }

        if header.is_empty() {
            body
        } else {
            format!("{}\n{body}", header.join("\n"))
        }
    }

    fn push_generated_type(&mut self, body: String) {
        self.generated_chunks.push(self.render_type_block(body));
    }

    fn type_name_attr(&self) -> Option<String> {
        let schema_name = self.ty.names().schema_name()?;
        let rust_name = effective_type_name(self.module, self.ty);
        if schema_name.name != rust_name {
            Some(format!("#[eure(type_name = \"{}\")]", schema_name.name))
        } else {
            None
        }
    }

    fn reserve_type_name(&mut self, preferred: String) -> String {
        if self.used_type_names.insert(preferred.clone()) {
            return preferred;
        }

        let mut index = 2;
        loop {
            let candidate = format!("{preferred}{index}");
            if self.used_type_names.insert(candidate.clone()) {
                return candidate;
            }
            index += 1;
        }
    }
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

    let type_name = effective_type_name(module, ty);
    let derives = effective_derives(module, ty, config);
    let mut emitter =
        NestedTypeEmitter::new(module, ty, config.allow_warnings, visibility, derives);

    let body = match &root.content() {
        SchemaNodeContentIr::Record(record) => {
            let visiting = BTreeSet::from([ty.semantic_root()]);
            emit_record_type(&mut emitter, ty, &type_name, record, &visiting)?
        }
        SchemaNodeContentIr::Union(union) => {
            let visiting = BTreeSet::from([ty.semantic_root()]);
            emit_union_type(&mut emitter, ty, &type_name, union, &visiting)?
        }
        SchemaNodeContentIr::Null => format!("{visibility}struct {type_name};"),
        _ => emit_newtype(&mut emitter, ty, &type_name, ty.semantic_root())?,
    };

    let type_name_attr = emitter.type_name_attr();
    let rendered_body = emitter.render_type_block(body);
    let main_chunk = match type_name_attr {
        Some(attr) => format!("{attr}\n{rendered_body}"),
        None => rendered_body,
    };
    let mut chunks = emitter.generated_chunks;
    chunks.push(main_chunk);
    Ok(chunks.join("\n\n"))
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
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    type_name: &str,
    record: &RecordSchemaIr,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<String, EmitRustError> {
    if matches!(record.unknown_fields(), UnknownFieldsPolicyIr::Schema(_)) {
        return Err(EmitRustError::NotYetSupported {
            type_id: emitter.type_id().to_string(),
            reason: EmitRustUnsupportedReason::SchemaDrivenUnknownFieldsPolicy,
        });
    }

    let field_specs =
        collect_record_field_specs(emitter, schema_owner, type_name, record, base_visiting)?;
    let mut field_lines = Vec::new();
    for spec in field_specs {
        if let Some(attr) = spec.rename_attr() {
            field_lines.push(format!("    {attr}"));
        }
        field_lines.push(format!(
            "    {}{}: {},",
            emitter.visibility,
            spec.rust_name,
            spec.rendered_ty()
        ));
    }

    Ok(format!(
        "{}struct {type_name} {{\n{}\n}}",
        emitter.visibility,
        field_lines.join("\n")
    ))
}

fn emit_union_type(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    type_name: &str,
    union: &UnionSchemaIr,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<String, EmitRustError> {
    let (variant_types, variant_types_suffix) = match emitter.ty.type_codegen() {
        TypeCodegenIr::Union(union_codegen) => (
            union_codegen.variant_types,
            union_codegen
                .variant_types_suffix_override
                .as_deref()
                .unwrap_or(DEFAULT_VARIANT_TYPES_SUFFIX),
        ),
        _ => (false, DEFAULT_VARIANT_TYPES_SUFFIX),
    };
    let variant_types_suffix =
        sanitize_type_name_segment(variant_types_suffix, DEFAULT_VARIANT_TYPES_SUFFIX);

    let mut variants = Vec::new();
    for (wire_name, schema_id) in union.variants() {
        let variant_name = sanitize_variant_name(wire_name);
        let node = lookup_schema_node(emitter, schema_owner, *schema_id)?;

        let rename_prefix = if variant_name != wire_name.as_str() {
            format!("    #[eure(rename = \"{wire_name}\")]\n")
        } else {
            String::new()
        };

        if variant_types {
            let companion_name =
                emitter.reserve_type_name(format!("{variant_name}{variant_types_suffix}"));
            let mut companion_visiting = base_visiting.clone();
            companion_visiting.insert(*schema_id);
            emit_named_schema_type(
                emitter,
                schema_owner,
                &companion_name,
                *schema_id,
                &companion_visiting,
            )?;
            variants.push(format!(
                "{rename_prefix}    {variant_name}({companion_name}),"
            ));
            continue;
        }

        let parent_variant_type_name = format!("{type_name}{variant_name}");
        let mut variant_visiting = base_visiting.clone();
        variant_visiting.insert(*schema_id);

        let variant_line = match &node.content() {
            SchemaNodeContentIr::Null => format!("{rename_prefix}    {variant_name},"),
            SchemaNodeContentIr::Tuple(tuple) => {
                let mut items = Vec::new();
                for (index, element) in tuple.elements.iter().enumerate() {
                    let mut visiting = variant_visiting.clone();
                    items.push(schema_node_type(
                        emitter,
                        schema_owner,
                        *element,
                        Some(tuple_item_type_name(&parent_variant_type_name, index)),
                        &mut visiting,
                    )?);
                }
                format!("{rename_prefix}    {variant_name}({}),", items.join(", "))
            }
            SchemaNodeContentIr::Record(record) => {
                if !record.flatten().is_empty() {
                    return Err(EmitRustError::NotYetSupported {
                        type_id: emitter.type_id().to_string(),
                        reason: EmitRustUnsupportedReason::FlattenedStructVariant {
                            variant_wire_name: wire_name.clone(),
                        },
                    });
                }
                let fields = collect_record_field_specs(
                    emitter,
                    schema_owner,
                    &parent_variant_type_name,
                    record,
                    &variant_visiting,
                )?
                .into_iter()
                .map(|spec| {
                    let attr = spec
                        .rename_attr()
                        .map(|a| format!("{a} "))
                        .unwrap_or_default();
                    format!("{attr}{}: {}", spec.rust_name, spec.rendered_ty())
                })
                .collect::<Vec<_>>();
                format!(
                    "{rename_prefix}    {variant_name} {{ {} }},",
                    fields.join(", ")
                )
            }
            _ => {
                let mut visiting = base_visiting.clone();
                let ty_str = schema_node_type(
                    emitter,
                    schema_owner,
                    *schema_id,
                    Some(parent_variant_type_name),
                    &mut visiting,
                )?;
                format!("{rename_prefix}    {variant_name}({ty_str}),")
            }
        };
        variants.push(variant_line);
    }

    Ok(format!(
        "{}enum {type_name} {{\n{}\n}}",
        emitter.visibility,
        variants.join("\n")
    ))
}

fn emit_newtype(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    type_name: &str,
    node_id: SchemaNodeIrId,
) -> Result<String, EmitRustError> {
    let inner_ty = schema_node_type(
        emitter,
        schema_owner,
        node_id,
        Some(type_name.to_string()),
        &mut BTreeSet::new(),
    )?;

    Ok(format!(
        "{}struct {type_name}({}{inner_ty});",
        emitter.visibility, emitter.visibility
    ))
}

fn schema_node_type(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    node_id: SchemaNodeIrId,
    preferred_inline_name: Option<String>,
    visiting: &mut BTreeSet<SchemaNodeIrId>,
) -> Result<String, EmitRustError> {
    if !visiting.insert(node_id) {
        return Err(EmitRustError::NotYetSupported {
            type_id: emitter.type_id().to_string(),
            reason: EmitRustUnsupportedReason::RecursiveInlineNodeGraph { node: node_id },
        });
    }

    let node = lookup_schema_node(emitter, schema_owner, node_id)?;

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
            let item = schema_node_type(
                emitter,
                schema_owner,
                array.item,
                preferred_inline_name.as_deref().map(array_item_type_name),
                visiting,
            )?;
            format!("Vec<{item}>")
        }
        SchemaNodeContentIr::Tuple(tuple) => {
            let mut items = Vec::new();
            for (index, element) in tuple.elements.iter().enumerate() {
                items.push(schema_node_type(
                    emitter,
                    schema_owner,
                    *element,
                    preferred_inline_name
                        .as_deref()
                        .map(|parent| tuple_item_type_name(parent, index)),
                    visiting,
                )?);
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
            let key = resolve_map_key_type(emitter, schema_owner, map.key, &mut BTreeSet::new())?
                .ok_or_else(|| EmitRustError::NotYetSupported {
                type_id: emitter.type_id().to_string(),
                reason: EmitRustUnsupportedReason::MapKeysOtherThanText,
            })?;
            let value = schema_node_type(
                emitter,
                schema_owner,
                map.value,
                preferred_inline_name.as_deref().map(map_value_type_name),
                visiting,
            )?;
            format!("::std::collections::BTreeMap<{key}, {value}>")
        }
        SchemaNodeContentIr::Reference(reference) => {
            resolve_reference_type_name(emitter.module, reference)
        }
        SchemaNodeContentIr::Record(_) => {
            let type_name = emitter.reserve_type_name(preferred_inline_name.unwrap_or_else(|| {
                append_type_name_segment(
                    &effective_type_name(emitter.module, emitter.ty),
                    &format!("node-{}", node_id.0),
                    "Node",
                )
            }));
            let base_visiting = visiting.clone();
            emit_named_schema_type(emitter, schema_owner, &type_name, node_id, &base_visiting)?;
            type_name
        }
        SchemaNodeContentIr::Union(_) => {
            let type_name = emitter.reserve_type_name(preferred_inline_name.unwrap_or_else(|| {
                append_type_name_segment(
                    &effective_type_name(emitter.module, emitter.ty),
                    &format!("node-{}", node_id.0),
                    "Node",
                )
            }));
            let base_visiting = visiting.clone();
            emit_named_schema_type(emitter, schema_owner, &type_name, node_id, &base_visiting)?;
            type_name
        }
    };

    visiting.remove(&node_id);
    Ok(out)
}

struct FieldSpec {
    rust_name: String,
    wire_name: String,
    field_ty: String,
    optional: bool,
}

impl FieldSpec {
    fn rename_attr(&self) -> Option<String> {
        let bare = self.rust_name.strip_prefix("r#").unwrap_or(&self.rust_name);
        if bare != self.wire_name {
            Some(format!("#[eure(rename = \"{}\")]", self.wire_name))
        } else {
            None
        }
    }

    fn rendered_ty(&self) -> String {
        if self.optional {
            format!("Option<{}>", self.field_ty)
        } else {
            self.field_ty.clone()
        }
    }
}

fn collect_record_field_specs(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    parent_type_name: &str,
    record: &RecordSchemaIr,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<Vec<FieldSpec>, EmitRustError> {
    let mut field_specs = Vec::new();
    for flatten_node_id in record.flatten() {
        field_specs.extend(collect_flattened_record_field_specs(
            emitter,
            schema_owner,
            parent_type_name,
            *flatten_node_id,
            base_visiting,
        )?);
    }
    for (wire_name, field) in record.properties() {
        let raw_field_name = field
            .field_codegen()
            .name_override
            .as_deref()
            .unwrap_or(wire_name.as_str());
        let rust_name = sanitize_field_name(raw_field_name);
        let mut visiting = base_visiting.clone();
        let field_ty = schema_node_type(
            emitter,
            schema_owner,
            field.schema(),
            Some(suggest_inline_type_name(parent_type_name, raw_field_name)),
            &mut visiting,
        )?;
        field_specs.push(FieldSpec {
            rust_name,
            wire_name: wire_name.clone(),
            field_ty,
            optional: field.optional(),
        });
    }

    Ok(field_specs)
}

fn emit_named_schema_type(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    type_name: &str,
    node_id: SchemaNodeIrId,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<(), EmitRustError> {
    let node = lookup_schema_node(emitter, schema_owner, node_id)?;

    let body = match node.content() {
        SchemaNodeContentIr::Record(record) => {
            emit_record_type(emitter, schema_owner, type_name, record, base_visiting)?
        }
        SchemaNodeContentIr::Tuple(tuple) => {
            emit_tuple_struct_type(emitter, schema_owner, type_name, tuple, base_visiting)?
        }
        SchemaNodeContentIr::Null => format!("{}struct {type_name};", emitter.visibility),
        SchemaNodeContentIr::Union(union) => {
            emit_union_type(emitter, schema_owner, type_name, union, base_visiting)?
        }
        _ => emit_named_newtype(emitter, schema_owner, type_name, node_id, base_visiting)?,
    };
    emitter.push_generated_type(body);
    Ok(())
}

fn emit_tuple_struct_type(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    type_name: &str,
    tuple: &eure_codegen_ir::TupleSchemaIr,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<String, EmitRustError> {
    let mut fields = Vec::new();
    for (index, element) in tuple.elements.iter().enumerate() {
        let mut visiting = base_visiting.clone();
        let item_ty = schema_node_type(
            emitter,
            schema_owner,
            *element,
            Some(tuple_item_type_name(type_name, index)),
            &mut visiting,
        )?;
        fields.push(format!("{}{}", emitter.visibility, item_ty));
    }

    if fields.is_empty() {
        Ok(format!("{}struct {type_name};", emitter.visibility))
    } else {
        Ok(format!(
            "{}struct {type_name}({});",
            emitter.visibility,
            fields.join(", ")
        ))
    }
}

fn emit_named_newtype(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    type_name: &str,
    node_id: SchemaNodeIrId,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<String, EmitRustError> {
    let mut visiting = base_visiting.clone();
    visiting.remove(&node_id);
    let inner_ty = schema_node_type(
        emitter,
        schema_owner,
        node_id,
        Some(type_name.to_string()),
        &mut visiting,
    )?;

    Ok(format!(
        "{}struct {type_name}({}{inner_ty});",
        emitter.visibility, emitter.visibility
    ))
}

fn lookup_schema_node<'a>(
    emitter: &NestedTypeEmitter<'_>,
    schema_owner: &'a TypeDefIr,
    node_id: SchemaNodeIrId,
) -> Result<&'a eure_codegen_ir::SchemaNodeIr, EmitRustError> {
    schema_owner
        .schema_nodes()
        .get(&node_id)
        .ok_or_else(|| EmitRustError::MissingSchemaNode {
            type_id: emitter.type_id().to_string(),
            node: node_id,
        })
}

fn collect_flattened_record_field_specs(
    emitter: &mut NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    parent_type_name: &str,
    flatten_node_id: SchemaNodeIrId,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<Vec<FieldSpec>, EmitRustError> {
    let Some(target) = resolve_flatten_target_schema(
        emitter,
        schema_owner,
        flatten_node_id,
        &mut BTreeSet::new(),
    )?
    else {
        return Err(EmitRustError::NotYetSupported {
            type_id: emitter.type_id().to_string(),
            reason: EmitRustUnsupportedReason::RecordFlattenFields,
        });
    };

    match target {
        FlattenTarget::Record {
            owner,
            node_id,
            record,
        } => {
            let visiting = flatten_target_visiting(
                emitter,
                schema_owner,
                owner,
                node_id,
                flatten_node_id,
                base_visiting,
            )?;
            collect_record_field_specs(emitter, owner, parent_type_name, record, &visiting)
        }
        FlattenTarget::Union {
            owner,
            node_id,
            union,
        } => {
            let visiting = flatten_target_visiting(
                emitter,
                schema_owner,
                owner,
                node_id,
                flatten_node_id,
                base_visiting,
            )?;
            collect_flattened_union_field_specs(emitter, owner, parent_type_name, union, &visiting)
        }
    }
}

enum FlattenTarget<'a> {
    Record {
        owner: &'a TypeDefIr,
        node_id: SchemaNodeIrId,
        record: &'a RecordSchemaIr,
    },
    Union {
        owner: &'a TypeDefIr,
        node_id: SchemaNodeIrId,
        union: &'a UnionSchemaIr,
    },
}

fn resolve_flatten_target_schema<'a>(
    emitter: &NestedTypeEmitter<'a>,
    schema_owner: &'a TypeDefIr,
    node_id: SchemaNodeIrId,
    visited_types: &mut BTreeSet<String>,
) -> Result<Option<FlattenTarget<'a>>, EmitRustError> {
    let node = lookup_schema_node(emitter, schema_owner, node_id)?;
    match node.content() {
        SchemaNodeContentIr::Record(record) => Ok(Some(FlattenTarget::Record {
            owner: schema_owner,
            node_id,
            record,
        })),
        SchemaNodeContentIr::Union(union) => Ok(Some(FlattenTarget::Union {
            owner: schema_owner,
            node_id,
            union,
        })),
        SchemaNodeContentIr::Reference(reference) => {
            let Some(target_ty) = emitter.module.get_type_by_name(reference) else {
                return Ok(None);
            };
            if !visited_types.insert(target_ty.id().0.clone()) {
                return Err(EmitRustError::NotYetSupported {
                    type_id: emitter.type_id().to_string(),
                    reason: EmitRustUnsupportedReason::RecursiveInlineNodeGraph { node: node_id },
                });
            }
            let result = resolve_flatten_target_schema(
                emitter,
                target_ty,
                target_ty.semantic_root(),
                visited_types,
            );
            visited_types.remove(&target_ty.id().0);
            result
        }
        _ => Ok(None),
    }
}

fn flatten_target_visiting(
    emitter: &NestedTypeEmitter<'_>,
    current_owner: &TypeDefIr,
    target_owner: &TypeDefIr,
    target_node_id: SchemaNodeIrId,
    source_node_id: SchemaNodeIrId,
    base_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<BTreeSet<SchemaNodeIrId>, EmitRustError> {
    let mut visiting = if std::ptr::eq(current_owner, target_owner) {
        base_visiting.clone()
    } else {
        BTreeSet::new()
    };
    if !visiting.insert(target_node_id) {
        return Err(EmitRustError::NotYetSupported {
            type_id: emitter.type_id().to_string(),
            reason: EmitRustUnsupportedReason::RecursiveInlineNodeGraph {
                node: source_node_id,
            },
        });
    }
    Ok(visiting)
}

fn collect_flattened_union_field_specs(
    emitter: &mut NestedTypeEmitter<'_>,
    union_owner: &TypeDefIr,
    parent_type_name: &str,
    union: &UnionSchemaIr,
    union_visiting: &BTreeSet<SchemaNodeIrId>,
) -> Result<Vec<FieldSpec>, EmitRustError> {
    use indexmap::map::Entry;

    let total_variants = union.variants().len();
    let mut merged = indexmap::IndexMap::<String, FieldSpec>::new();
    let mut counts = indexmap::IndexMap::<String, usize>::new();

    for schema_id in union.variants().values() {
        let node = lookup_schema_node(emitter, union_owner, *schema_id)?;
        let variant_specs = match node.content() {
            SchemaNodeContentIr::Record(record) => {
                let mut variant_visiting = union_visiting.clone();
                variant_visiting.insert(*schema_id);
                collect_record_field_specs(
                    emitter,
                    union_owner,
                    parent_type_name,
                    record,
                    &variant_visiting,
                )?
            }
            SchemaNodeContentIr::Null => Vec::new(),
            _ => {
                return Err(EmitRustError::NotYetSupported {
                    type_id: emitter.type_id().to_string(),
                    reason: EmitRustUnsupportedReason::RecordFlattenFields,
                });
            }
        };

        let mut seen_in_variant = BTreeSet::new();
        for spec in variant_specs {
            let key = spec.wire_name.clone();
            if seen_in_variant.insert(key.clone()) {
                *counts.entry(key.clone()).or_default() += 1;
            }

            match merged.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(spec);
                }
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut();
                    if existing.rust_name != spec.rust_name || existing.field_ty != spec.field_ty {
                        return Err(EmitRustError::NotYetSupported {
                            type_id: emitter.type_id().to_string(),
                            reason: EmitRustUnsupportedReason::RecordFlattenFields,
                        });
                    }
                    existing.optional |= spec.optional;
                }
            }
        }
    }

    for (wire_name, spec) in &mut merged {
        if counts.get(wire_name).copied().unwrap_or_default() < total_variants {
            spec.optional = true;
        }
    }

    Ok(merged.into_values().collect())
}

fn resolve_map_key_type(
    emitter: &NestedTypeEmitter<'_>,
    schema_owner: &TypeDefIr,
    node_id: SchemaNodeIrId,
    visited_types: &mut BTreeSet<String>,
) -> Result<Option<&'static str>, EmitRustError> {
    let node = lookup_schema_node(emitter, schema_owner, node_id)?;
    match node.content() {
        SchemaNodeContentIr::Any => Ok(Some("::eure_document::value::ObjectKey")),
        SchemaNodeContentIr::Text(_) => Ok(Some("String")),
        SchemaNodeContentIr::Integer(_) => Ok(Some("i64")),
        SchemaNodeContentIr::Boolean => Ok(Some("bool")),
        SchemaNodeContentIr::Reference(reference) => {
            let Some(target_ty) = emitter.module.get_type_by_name(reference) else {
                return Ok(None);
            };
            if !visited_types.insert(target_ty.id().0.clone()) {
                return Err(EmitRustError::NotYetSupported {
                    type_id: emitter.type_id().to_string(),
                    reason: EmitRustUnsupportedReason::RecursiveInlineNodeGraph { node: node_id },
                });
            }
            let result =
                resolve_map_key_type(emitter, target_ty, target_ty.semantic_root(), visited_types);
            visited_types.remove(&target_ty.id().0);
            result
        }
        _ => Ok(None),
    }
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

fn suggest_inline_type_name(parent_type_name: &str, raw_segment: &str) -> String {
    append_type_name_segment(parent_type_name, raw_segment, "Field")
}

fn array_item_type_name(parent_type_name: &str) -> String {
    append_type_name_segment(parent_type_name, "item", "Item")
}

fn map_value_type_name(parent_type_name: &str) -> String {
    append_type_name_segment(parent_type_name, "value", "Value")
}

fn tuple_item_type_name(parent_type_name: &str, index: usize) -> String {
    append_type_name_segment(parent_type_name, &format!("item-{}", index + 1), "Item")
}

fn append_type_name_segment(parent_type_name: &str, raw_segment: &str, default: &str) -> String {
    format!(
        "{parent_type_name}{}",
        sanitize_type_name_segment(raw_segment, default)
    )
}

fn sanitize_type_name_segment(raw: &str, default: &str) -> String {
    sanitize_pascal_ident(raw, default)
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
    sanitize_pascal_ident(raw, "Variant")
}

fn sanitize_pascal_ident(raw: &str, default: &str) -> String {
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
        out.push_str(default);
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
        ArraySchemaIr, FieldCodegenIr, IntegerSchemaIr, IrModule, MapSchemaIr, QualifiedTypeName,
        RecordFieldSchemaIr, RecordSchemaIr, RootCodegenIr, RustBindingIr, RustTypeKindIr,
        SchemaMetadataIr, SchemaNodeContentIr, SchemaNodeIr, SchemaNodeIrId, TextSchemaIr,
        TupleSchemaIr, TypeCodegenIr, TypeDefIr, TypeId, TypeNamesIr, TypeOriginIr, UnionCodegenIr,
        UnionSchemaIr, UnknownFieldsPolicyIr,
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
    fn emits_flattened_record_fields() {
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
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "city".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(2),
                        false,
                        None,
                        FieldCodegenIr::default(),
                    ),
                )]),
                vec![SchemaNodeIrId(1)],
                UnknownFieldsPolicyIr::Deny,
            ))),
        );
        nodes.insert(
            SchemaNodeIrId(1),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "street".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(2),
                        false,
                        None,
                        FieldCodegenIr::default(),
                    ),
                )]),
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

        let generated = emit_rust_types(&base_module(ty), &GenerationConfig::builder().build())
            .expect("emit rust");
        assert!(
            generated.contains("street: String"),
            "expected flattened field in parent struct, got:\n{generated}"
        );
        assert!(
            generated.contains("city: String"),
            "expected parent record field after flatten, got:\n{generated}"
        );
    }

    #[test]
    fn emits_flattened_fields_from_referenced_records() {
        let address_schema_name = QualifiedTypeName::local("address");
        let root_schema_name = QualifiedTypeName::local("root");

        let address_nodes = IndexMap::from([
            (
                SchemaNodeIrId(1),
                node(SchemaNodeContentIr::Text(TextSchemaIr {
                    language: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    unknown_fields: IndexMap::new(),
                })),
            ),
            (
                SchemaNodeIrId(0),
                node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    IndexMap::from([(
                        "street".to_string(),
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
        let address = type_def(
            "a_address",
            "Address",
            Some(address_schema_name.clone()),
            address_nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        );

        let root_nodes = IndexMap::from([
            (
                SchemaNodeIrId(2),
                node(SchemaNodeContentIr::Text(TextSchemaIr {
                    language: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    unknown_fields: IndexMap::new(),
                })),
            ),
            (
                SchemaNodeIrId(1),
                node(SchemaNodeContentIr::Reference(address_schema_name.clone())),
            ),
            (
                SchemaNodeIrId(0),
                node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    IndexMap::from([(
                        "name".to_string(),
                        RecordFieldSchemaIr::new(
                            SchemaNodeIrId(2),
                            false,
                            None,
                            FieldCodegenIr::default(),
                        ),
                    )]),
                    vec![SchemaNodeIrId(1)],
                    UnknownFieldsPolicyIr::Deny,
                ))),
            ),
        ]);
        let root = type_def(
            "b_root",
            "User",
            Some(root_schema_name.clone()),
            root_nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        );

        let mut module = IrModule::default();
        module.insert_name_index(address_schema_name, TypeId("a_address".to_string()));
        module.insert_name_index(root_schema_name, TypeId("b_root".to_string()));
        module.insert_type(TypeId("a_address".to_string()), address);
        module.insert_type(TypeId("b_root".to_string()), root);
        module.push_root(TypeId("b_root".to_string()));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("street: String"),
            "expected flattened referenced field in parent struct, got:\n{generated}"
        );
        assert!(
            generated.contains("name: String"),
            "expected parent-owned field to remain present, got:\n{generated}"
        );
    }

    #[test]
    fn emits_flattened_fields_from_unions_of_records() {
        let data_schema_name = QualifiedTypeName::local("data");
        let root_schema_name = QualifiedTypeName::local("root");

        let data_nodes = IndexMap::from([
            (
                SchemaNodeIrId(2),
                node(SchemaNodeContentIr::Text(TextSchemaIr {
                    language: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    unknown_fields: IndexMap::new(),
                })),
            ),
            (
                SchemaNodeIrId(3),
                node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    IndexMap::from([(
                        "json".to_string(),
                        RecordFieldSchemaIr::new(
                            SchemaNodeIrId(2),
                            false,
                            None,
                            FieldCodegenIr::default(),
                        ),
                    )]),
                    Vec::new(),
                    UnknownFieldsPolicyIr::Deny,
                ))),
            ),
            (
                SchemaNodeIrId(4),
                node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    IndexMap::from([
                        (
                            "input-json".to_string(),
                            RecordFieldSchemaIr::new(
                                SchemaNodeIrId(2),
                                false,
                                None,
                                FieldCodegenIr::default(),
                            ),
                        ),
                        (
                            "output-json".to_string(),
                            RecordFieldSchemaIr::new(
                                SchemaNodeIrId(2),
                                false,
                                None,
                                FieldCodegenIr::default(),
                            ),
                        ),
                    ]),
                    Vec::new(),
                    UnknownFieldsPolicyIr::Deny,
                ))),
            ),
            (
                SchemaNodeIrId(0),
                node(SchemaNodeContentIr::Union(UnionSchemaIr::new(
                    IndexMap::from([
                        ("both".to_string(), SchemaNodeIrId(3)),
                        ("separate".to_string(), SchemaNodeIrId(4)),
                    ]),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ))),
            ),
        ]);
        let data = type_def(
            "a_data",
            "Data",
            Some(data_schema_name.clone()),
            data_nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Enum,
        );

        let root_nodes = IndexMap::from([
            (
                SchemaNodeIrId(2),
                node(SchemaNodeContentIr::Text(TextSchemaIr {
                    language: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    unknown_fields: IndexMap::new(),
                })),
            ),
            (
                SchemaNodeIrId(1),
                node(SchemaNodeContentIr::Reference(data_schema_name.clone())),
            ),
            (
                SchemaNodeIrId(0),
                node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    IndexMap::from([(
                        "name".to_string(),
                        RecordFieldSchemaIr::new(
                            SchemaNodeIrId(2),
                            false,
                            None,
                            FieldCodegenIr::default(),
                        ),
                    )]),
                    vec![SchemaNodeIrId(1)],
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
        module.insert_name_index(data_schema_name, TypeId("a_data".to_string()));
        module.insert_name_index(root_schema_name, TypeId("b_root".to_string()));
        module.insert_type(TypeId("a_data".to_string()), data);
        module.insert_type(TypeId("b_root".to_string()), root);
        module.push_root(TypeId("b_root".to_string()));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("json: Option<String>"),
            "expected union-flattened field to become optional, got:\n{generated}"
        );
        assert!(
            generated.contains("input_json: Option<String>"),
            "expected per-variant field to become optional, got:\n{generated}"
        );
        assert!(
            generated.contains("output_json: Option<String>"),
            "expected per-variant field to become optional, got:\n{generated}"
        );
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
    fn emits_inline_record_field_types_as_named_structs() {
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
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "street".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(2),
                        false,
                        None,
                        FieldCodegenIr::default(),
                    ),
                )]),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            ))),
        );
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "address".to_string(),
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
            "inline-record-field",
            "User",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        ));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("struct UserAddress"),
            "expected generated inline struct name, got:\n{generated}"
        );
        assert!(
            generated.contains("address: UserAddress"),
            "expected parent field to reference named inline struct, got:\n{generated}"
        );
    }

    #[test]
    fn emits_inline_union_field_types_as_named_enums() {
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
        nodes.insert(SchemaNodeIrId(3), node(SchemaNodeContentIr::Null));
        nodes.insert(
            SchemaNodeIrId(1),
            node(SchemaNodeContentIr::Union(UnionSchemaIr::new(
                IndexMap::from([
                    ("active".to_string(), SchemaNodeIrId(2)),
                    ("none".to_string(), SchemaNodeIrId(3)),
                ]),
                Default::default(),
                Default::default(),
                Default::default(),
            ))),
        );
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "status".to_string(),
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
            "inline-union-field",
            "User",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Record,
        ));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("enum UserStatus"),
            "expected generated inline enum name, got:\n{generated}"
        );
        assert!(
            generated.contains("status: UserStatus"),
            "expected parent field to reference generated inline enum, got:\n{generated}"
        );
    }

    #[test]
    fn emits_inline_records_inside_newtype_containers() {
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
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "name".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(2),
                        false,
                        None,
                        FieldCodegenIr::default(),
                    ),
                )]),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            ))),
        );
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Array(ArraySchemaIr {
                item: SchemaNodeIrId(1),
                min_length: None,
                max_length: None,
                unique: false,
                contains: None,
                binding_style: None,
            })),
        );

        let module = base_module(type_def(
            "inline-record-array",
            "Users",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Newtype,
        ));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("struct UsersItem"),
            "expected generated item struct for inline record array items, got:\n{generated}"
        );
        assert!(
            generated.contains("struct Users(pub Vec<UsersItem>);"),
            "expected newtype to use generated item struct, got:\n{generated}"
        );
    }

    #[test]
    fn emits_inline_unions_inside_newtype_containers() {
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
        nodes.insert(SchemaNodeIrId(3), node(SchemaNodeContentIr::Null));
        nodes.insert(
            SchemaNodeIrId(1),
            node(SchemaNodeContentIr::Union(UnionSchemaIr::new(
                IndexMap::from([
                    ("text".to_string(), SchemaNodeIrId(2)),
                    ("none".to_string(), SchemaNodeIrId(3)),
                ]),
                Default::default(),
                Default::default(),
                Default::default(),
            ))),
        );
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Array(ArraySchemaIr {
                item: SchemaNodeIrId(1),
                min_length: None,
                max_length: None,
                unique: false,
                contains: None,
                binding_style: None,
            })),
        );

        let module = base_module(type_def(
            "inline-union-array",
            "Statuses",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Newtype,
        ));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("enum StatusesItem"),
            "expected generated item enum for inline union array items, got:\n{generated}"
        );
        assert!(
            generated.contains("struct Statuses(pub Vec<StatusesItem>);"),
            "expected newtype to use generated item enum, got:\n{generated}"
        );
    }

    #[test]
    fn emits_maps_with_integer_boolean_any_and_referenced_text_keys() {
        let key_schema_name = QualifiedTypeName::local("key-text");
        let root_schema_name = QualifiedTypeName::local("root");

        let key_nodes = IndexMap::from([(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Text(TextSchemaIr {
                language: None,
                min_length: None,
                max_length: None,
                pattern: None,
                unknown_fields: IndexMap::new(),
            })),
        )]);
        let key_ty = type_def(
            "a_key_text",
            "KeyText",
            Some(key_schema_name.clone()),
            key_nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Newtype,
        );

        let root_nodes = IndexMap::from([
            (
                SchemaNodeIrId(1),
                node(SchemaNodeContentIr::Integer(IntegerSchemaIr {
                    min: Default::default(),
                    max: Default::default(),
                    multiple_of: None,
                })),
            ),
            (SchemaNodeIrId(2), node(SchemaNodeContentIr::Boolean)),
            (
                SchemaNodeIrId(3),
                node(SchemaNodeContentIr::Reference(key_schema_name.clone())),
            ),
            (
                SchemaNodeIrId(4),
                node(SchemaNodeContentIr::Text(TextSchemaIr {
                    language: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    unknown_fields: IndexMap::new(),
                })),
            ),
            (
                SchemaNodeIrId(5),
                node(SchemaNodeContentIr::Map(MapSchemaIr {
                    key: SchemaNodeIrId(1),
                    value: SchemaNodeIrId(4),
                    min_size: None,
                    max_size: None,
                })),
            ),
            (
                SchemaNodeIrId(6),
                node(SchemaNodeContentIr::Map(MapSchemaIr {
                    key: SchemaNodeIrId(2),
                    value: SchemaNodeIrId(4),
                    min_size: None,
                    max_size: None,
                })),
            ),
            (
                SchemaNodeIrId(7),
                node(SchemaNodeContentIr::Map(MapSchemaIr {
                    key: SchemaNodeIrId(3),
                    value: SchemaNodeIrId(4),
                    min_size: None,
                    max_size: None,
                })),
            ),
            (SchemaNodeIrId(8), node(SchemaNodeContentIr::Any)),
            (
                SchemaNodeIrId(9),
                node(SchemaNodeContentIr::Map(MapSchemaIr {
                    key: SchemaNodeIrId(8),
                    value: SchemaNodeIrId(4),
                    min_size: None,
                    max_size: None,
                })),
            ),
            (
                SchemaNodeIrId(0),
                node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    IndexMap::from([
                        (
                            "counts".to_string(),
                            RecordFieldSchemaIr::new(
                                SchemaNodeIrId(5),
                                false,
                                None,
                                FieldCodegenIr::default(),
                            ),
                        ),
                        (
                            "flags".to_string(),
                            RecordFieldSchemaIr::new(
                                SchemaNodeIrId(6),
                                false,
                                None,
                                FieldCodegenIr::default(),
                            ),
                        ),
                        (
                            "lookup".to_string(),
                            RecordFieldSchemaIr::new(
                                SchemaNodeIrId(7),
                                false,
                                None,
                                FieldCodegenIr::default(),
                            ),
                        ),
                        (
                            "key-tuples".to_string(),
                            RecordFieldSchemaIr::new(
                                SchemaNodeIrId(9),
                                false,
                                None,
                                FieldCodegenIr::default(),
                            ),
                        ),
                    ]),
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
        module.insert_name_index(key_schema_name, TypeId("a_key_text".to_string()));
        module.insert_name_index(root_schema_name, TypeId("b_root".to_string()));
        module.insert_type(TypeId("a_key_text".to_string()), key_ty);
        module.insert_type(TypeId("b_root".to_string()), root);
        module.push_root(TypeId("b_root".to_string()));

        let generated =
            emit_rust_types(&module, &GenerationConfig::builder().build()).expect("emit rust");
        assert!(
            generated.contains("counts: ::std::collections::BTreeMap<i64, String>"),
            "expected integer-keyed map type, got:\n{generated}"
        );
        assert!(
            generated.contains("flags: ::std::collections::BTreeMap<bool, String>"),
            "expected boolean-keyed map type, got:\n{generated}"
        );
        assert!(
            generated.contains("lookup: ::std::collections::BTreeMap<String, String>"),
            "expected referenced text key to collapse to String, got:\n{generated}"
        );
        assert!(
            generated.contains("key_tuples: ::std::collections::BTreeMap<::eure_document::value::ObjectKey, String>"),
            "expected any-keyed map type to use ObjectKey, got:\n{generated}"
        );
    }

    #[test]
    fn emits_union_variant_companion_types_when_enabled() {
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
            node(SchemaNodeContentIr::Record(RecordSchemaIr::new(
                IndexMap::from([(
                    "message".to_string(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(2),
                        false,
                        None,
                        FieldCodegenIr::default(),
                    ),
                )]),
                Vec::new(),
                UnknownFieldsPolicyIr::Deny,
            ))),
        );
        nodes.insert(
            SchemaNodeIrId(3),
            node(SchemaNodeContentIr::Text(TextSchemaIr {
                language: None,
                min_length: None,
                max_length: None,
                pattern: None,
                unknown_fields: IndexMap::new(),
            })),
        );
        nodes.insert(
            SchemaNodeIrId(0),
            node(SchemaNodeContentIr::Union(UnionSchemaIr::new(
                IndexMap::from([
                    ("ok".to_string(), SchemaNodeIrId(1)),
                    ("err".to_string(), SchemaNodeIrId(3)),
                ]),
                Default::default(),
                Default::default(),
                Default::default(),
            ))),
        );

        let mut ty = type_def(
            "union-variant-types",
            "ResultType",
            None,
            nodes,
            SchemaNodeIrId(0),
            RustTypeKindIr::Enum,
        );
        *ty.type_codegen_mut() = TypeCodegenIr::Union(UnionCodegenIr {
            type_name_override: None,
            derive: InheritableCodegenValueIr::inherit(),
            variant_types: true,
            variant_types_suffix_override: None,
        });

        let generated = emit_rust_types(&base_module(ty), &GenerationConfig::builder().build())
            .expect("emit rust");
        assert!(
            generated.contains("struct OkData"),
            "expected default Data suffix for record companion, got:\n{generated}"
        );
        assert!(
            generated.contains("struct ErrData(pub String);"),
            "expected companion newtype for scalar variant, got:\n{generated}"
        );
        assert!(
            generated.contains("Ok(OkData)"),
            "expected enum variant to reference companion type, got:\n{generated}"
        );
        assert!(
            generated.contains("Err(ErrData)"),
            "expected enum variant to reference scalar companion type, got:\n{generated}"
        );
        assert!(
            generated.find("struct OkData").expect("ok companion")
                < generated.find("enum ResultType").expect("enum"),
            "expected companion types before enum, got:\n{generated}"
        );
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
