use eure_codegen_ir::{
    ArraySchemaIr, BindingStyleIr, BoundIr, CodegenDefaultsIr, DecimalInt, DescriptionIr,
    ExtTypeIr, FieldCodegenIr, FloatPrecisionIr, FloatSchemaIr, InheritableCodegenValueIr,
    IntegerSchemaIr, IrModule, MapSchemaIr, ObjectKeyIr, QualifiedTypeName, RecordCodegenIr,
    RecordFieldSchemaIr, RecordSchemaIr, RootCodegenIr, RustBindingIr, RustTypeKindIr,
    SchemaMetadataIr, SchemaNodeContentIr, SchemaNodeIr, SchemaNodeIrId, TextLanguageIr,
    TextSchemaIr, TextValueIr, TupleSchemaIr, TypeCodegenIr, TypeDefIr, TypeId, TypeNamesIr,
    TypeOriginIr, UnionCodegenIr, UnionInteropIr, UnionSchemaIr, UnknownFieldsPolicyIr, ValueIr,
    VariantReprIr,
};
use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::plan::Form;
use eure_document::text::Language;
use eure_document::value::{ObjectKey, PrimitiveValue};
use eure_schema::{
    Bound, CodegenDefaults, Description, ExtTypeSchema, FieldCodegen, FloatPrecision,
    RecordCodegen, RootCodegen, SchemaDocument, SchemaMetadata, SchemaNode, SchemaNodeContent,
    SchemaNodeId, TypeCodegen, TypeReference, UnionCodegen, UnknownFieldsPolicy,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaToIrError {
    #[error("schema root node {root} is out of bounds (node count: {node_count})")]
    MissingRoot { root: usize, node_count: usize },

    #[error("named type `{type_name}` references missing schema node {node}")]
    MissingTypeNode { type_name: String, node: usize },

    #[error("schema node {node} references missing schema node {target} at {path}")]
    MissingSchemaNodeReference {
        node: usize,
        target: usize,
        path: String,
    },

    #[error(
        "schema node {node} has local type reference `{name}` that is not declared in schema.types"
    )]
    UnknownLocalTypeReference { node: usize, name: String },

    #[error("schema node {node} has unsupported hole value in {field_path}")]
    UnsupportedHoleValue { node: usize, field_path: String },

    #[error("schema node {node} has unsupported partial map value in {field_path}")]
    UnsupportedPartialMapValue { node: usize, field_path: String },

    #[error("schema node {node} has unsupported value extensions in {field_path}")]
    UnsupportedValueExtensions { node: usize, field_path: String },

    #[error("generated IR failed build checks: {source}")]
    InvalidIr {
        #[from]
        source: eure_codegen_ir::IrBuildError,
    },
}

pub fn schema_to_ir_module(schema: &SchemaDocument) -> Result<IrModule, SchemaToIrError> {
    if schema.root.0 >= schema.nodes.len() {
        return Err(SchemaToIrError::MissingRoot {
            root: schema.root.0,
            node_count: schema.nodes.len(),
        });
    }

    let schema_nodes = convert_schema_nodes(schema)?;

    let mut module = IrModule::default();
    module.set_root_codegen(convert_root_codegen(&schema.root_codegen));
    module.set_codegen_defaults(convert_codegen_defaults(&schema.codegen_defaults));

    for (name, root_id) in &schema.types {
        if root_id.0 >= schema.nodes.len() {
            return Err(SchemaToIrError::MissingTypeNode {
                type_name: name.to_string(),
                node: root_id.0,
            });
        }

        let type_id = TypeId(format!("schema::{}", name));
        let schema_name = QualifiedTypeName::local(name.to_string());

        let type_def = build_type_def(
            type_id.clone(),
            schema_name_to_rust_name(name.as_ref()),
            Some(schema_name.clone()),
            SchemaNodeIrId(root_id.0),
            &schema_nodes,
            convert_type_codegen(&schema.node(*root_id).type_codegen),
        );

        module.insert_name_index(schema_name, type_id.clone());
        module.insert_type(type_id, type_def);
    }

    let mut roots = Vec::new();
    for (name, root_id) in &schema.types {
        if *root_id == schema.root {
            roots.push(TypeId(format!("schema::{}", name)));
        }
    }

    if roots.is_empty() {
        let root_type_id = TypeId("__schema_root__".to_string());
        let root_type = build_type_def(
            root_type_id.clone(),
            "Root".to_string(),
            None,
            SchemaNodeIrId(schema.root.0),
            &schema_nodes,
            convert_type_codegen(&schema.node(schema.root).type_codegen),
        );
        module.insert_type(root_type_id.clone(), root_type);
        roots.push(root_type_id);
    }

    roots.sort();
    roots.dedup();
    module.set_roots(roots);

    let module = module.into_checked()?;
    Ok(module)
}

fn build_type_def(
    id: TypeId,
    rust_name: String,
    schema_name: Option<QualifiedTypeName>,
    semantic_root: SchemaNodeIrId,
    schema_nodes: &indexmap::IndexMap<SchemaNodeIrId, SchemaNodeIr>,
    type_codegen: TypeCodegenIr,
) -> TypeDefIr {
    let kind = schema_nodes
        .get(&semantic_root)
        .map_or(RustTypeKindIr::Newtype, |node| {
            rust_kind_for_schema_content(node.content())
        });

    let rust_binding = RustBindingIr::new(
        kind,
        Default::default(),
        Vec::new(),
        Vec::new(),
        Default::default(),
        Default::default(),
        Default::default(),
    );

    TypeDefIr::new(
        id,
        TypeNamesIr::new(rust_name, schema_name),
        schema_nodes.clone(),
        semantic_root,
        rust_binding,
        type_codegen,
        TypeOriginIr::Schema,
    )
}

fn rust_kind_for_schema_content(content: &SchemaNodeContentIr) -> RustTypeKindIr {
    match content {
        SchemaNodeContentIr::Record(_) => RustTypeKindIr::Record,
        SchemaNodeContentIr::Union(_) => RustTypeKindIr::Enum,
        SchemaNodeContentIr::Tuple(_) => RustTypeKindIr::Tuple,
        SchemaNodeContentIr::Null => RustTypeKindIr::Unit,
        _ => RustTypeKindIr::Newtype,
    }
}

fn convert_root_codegen(codegen: &RootCodegen) -> RootCodegenIr {
    RootCodegenIr {
        type_name_override: codegen.type_name.clone(),
    }
}

fn convert_codegen_defaults(defaults: &CodegenDefaults) -> CodegenDefaultsIr {
    CodegenDefaultsIr {
        derive: defaults.derive.clone().unwrap_or_default(),
        ext_types_field_prefix: defaults.ext_types_field_prefix.clone().unwrap_or_default(),
        ext_types_type_prefix: defaults.ext_types_type_prefix.clone().unwrap_or_default(),
        document_node_id_field: defaults.document_node_id_field.clone().unwrap_or_default(),
    }
}

fn convert_type_codegen(codegen: &TypeCodegen) -> TypeCodegenIr {
    match codegen {
        TypeCodegen::None => TypeCodegenIr::None,
        TypeCodegen::Record(record) => TypeCodegenIr::Record(convert_record_codegen(record)),
        TypeCodegen::Union(union) => TypeCodegenIr::Union(convert_union_codegen(union)),
    }
}

fn convert_record_codegen(codegen: &RecordCodegen) -> RecordCodegenIr {
    RecordCodegenIr {
        type_name_override: codegen.type_name.clone(),
        derive: codegen.derive.clone().map_or(
            InheritableCodegenValueIr::InheritCodegenDefaults,
            InheritableCodegenValueIr::Value,
        ),
    }
}

fn convert_union_codegen(codegen: &UnionCodegen) -> UnionCodegenIr {
    UnionCodegenIr {
        type_name_override: codegen.type_name.clone(),
        derive: codegen.derive.clone().map_or(
            InheritableCodegenValueIr::InheritCodegenDefaults,
            InheritableCodegenValueIr::Value,
        ),
        variant_types: codegen.variant_types.unwrap_or_default(),
        variant_types_suffix_override: codegen.variant_types_suffix.clone(),
    }
}

fn convert_field_codegen(codegen: &FieldCodegen) -> FieldCodegenIr {
    FieldCodegenIr {
        name_override: codegen.name.clone(),
    }
}

fn schema_name_to_rust_name(name: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if out.is_empty() && ch.is_ascii_digit() {
                out.push('_');
            }
            if capitalize {
                out.extend(ch.to_uppercase());
                capitalize = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalize = true;
        }
    }

    if out.is_empty() {
        "GeneratedType".to_string()
    } else {
        out
    }
}

fn convert_schema_nodes(
    schema: &SchemaDocument,
) -> Result<indexmap::IndexMap<SchemaNodeIrId, SchemaNodeIr>, SchemaToIrError> {
    let mut nodes = indexmap::IndexMap::new();

    for (idx, node) in schema.nodes.iter().enumerate() {
        let node_id = SchemaNodeIrId(idx);
        nodes.insert(node_id, convert_schema_node(schema, idx, node)?);
    }

    Ok(nodes)
}

fn convert_schema_node(
    schema: &SchemaDocument,
    node_idx: usize,
    node: &SchemaNode,
) -> Result<SchemaNodeIr, SchemaToIrError> {
    let content = convert_schema_content(schema, node_idx, &node.content)?;
    let metadata = convert_schema_metadata(node_idx, &node.metadata)?;

    let mut ext_types = indexmap::IndexMap::new();
    for (ext_name, ext_schema) in &node.ext_types {
        assert_node_exists(
            schema,
            node_idx,
            ext_schema.schema,
            format!("ext_types.{ext_name}"),
        )?;
        ext_types.insert(ext_name.to_string(), convert_ext_type(ext_schema));
    }

    Ok(SchemaNodeIr::new(content, metadata, ext_types))
}

fn convert_schema_content(
    schema: &SchemaDocument,
    node_idx: usize,
    content: &SchemaNodeContent,
) -> Result<SchemaNodeContentIr, SchemaToIrError> {
    let out = match content {
        SchemaNodeContent::Any => SchemaNodeContentIr::Any,
        SchemaNodeContent::Text(text) => {
            let mut unknown_fields = indexmap::IndexMap::new();
            for (name, value) in &text.unknown_fields {
                unknown_fields.insert(
                    name.clone(),
                    convert_document_value(value, node_idx, format!("text.unknown_fields.{name}"))?,
                );
            }

            SchemaNodeContentIr::Text(TextSchemaIr {
                language: text.language.clone(),
                min_length: text.min_length,
                max_length: text.max_length,
                pattern: text.pattern.as_ref().map(|re| re.as_str().to_string()),
                unknown_fields,
            })
        }
        SchemaNodeContent::Integer(integer) => SchemaNodeContentIr::Integer(IntegerSchemaIr {
            min: convert_bigint_bound(&integer.min),
            max: convert_bigint_bound(&integer.max),
            multiple_of: integer
                .multiple_of
                .as_ref()
                .map(|n| DecimalInt::new(n.to_string())),
        }),
        SchemaNodeContent::Float(float) => SchemaNodeContentIr::Float(FloatSchemaIr {
            min: convert_float_bound(&float.min),
            max: convert_float_bound(&float.max),
            multiple_of: float.multiple_of,
            precision: match float.precision {
                FloatPrecision::F32 => FloatPrecisionIr::F32,
                FloatPrecision::F64 => FloatPrecisionIr::F64,
            },
        }),
        SchemaNodeContent::Boolean => SchemaNodeContentIr::Boolean,
        SchemaNodeContent::Null => SchemaNodeContentIr::Null,
        SchemaNodeContent::Literal(doc) => SchemaNodeContentIr::Literal(convert_document_value(
            doc,
            node_idx,
            "literal".to_string(),
        )?),
        SchemaNodeContent::Array(array) => {
            assert_node_exists(schema, node_idx, array.item, "array.item".to_string())?;
            if let Some(contains) = array.contains {
                assert_node_exists(schema, node_idx, contains, "array.contains".to_string())?;
            }

            SchemaNodeContentIr::Array(ArraySchemaIr {
                item: SchemaNodeIrId(array.item.0),
                min_length: array.min_length,
                max_length: array.max_length,
                unique: array.unique,
                contains: array.contains.map(|id| SchemaNodeIrId(id.0)),
                binding_style: array.binding_style.map(convert_binding_style),
            })
        }
        SchemaNodeContent::Map(map) => {
            assert_node_exists(schema, node_idx, map.key, "map.key".to_string())?;
            assert_node_exists(schema, node_idx, map.value, "map.value".to_string())?;

            SchemaNodeContentIr::Map(MapSchemaIr {
                key: SchemaNodeIrId(map.key.0),
                value: SchemaNodeIrId(map.value.0),
                min_size: map.min_size,
                max_size: map.max_size,
            })
        }
        SchemaNodeContent::Record(record) => {
            let mut properties = indexmap::IndexMap::new();
            for (name, field) in &record.properties {
                assert_node_exists(
                    schema,
                    node_idx,
                    field.schema,
                    format!("record.properties.{name}.schema"),
                )?;

                properties.insert(
                    name.clone(),
                    RecordFieldSchemaIr::new(
                        SchemaNodeIrId(field.schema.0),
                        field.optional,
                        field.binding_style.map(convert_binding_style),
                        convert_field_codegen(&field.field_codegen),
                    ),
                );
            }

            let mut flatten = Vec::new();
            for (flatten_index, flatten_id) in record.flatten.iter().enumerate() {
                assert_node_exists(
                    schema,
                    node_idx,
                    *flatten_id,
                    format!("record.flatten[{flatten_index}]"),
                )?;
                flatten.push(SchemaNodeIrId(flatten_id.0));
            }

            let unknown_fields = match record.unknown_fields {
                UnknownFieldsPolicy::Deny => UnknownFieldsPolicyIr::Deny,
                UnknownFieldsPolicy::Allow => UnknownFieldsPolicyIr::Allow,
                UnknownFieldsPolicy::Schema(id) => {
                    assert_node_exists(
                        schema,
                        node_idx,
                        id,
                        "record.unknown_fields.schema".to_string(),
                    )?;
                    UnknownFieldsPolicyIr::Schema(SchemaNodeIrId(id.0))
                }
            };

            SchemaNodeContentIr::Record(RecordSchemaIr::new(properties, flatten, unknown_fields))
        }
        SchemaNodeContent::Tuple(tuple) => {
            let mut elements = Vec::new();
            for (element_index, element) in tuple.elements.iter().enumerate() {
                assert_node_exists(
                    schema,
                    node_idx,
                    *element,
                    format!("tuple.elements[{element_index}]"),
                )?;
                elements.push(SchemaNodeIrId(element.0));
            }

            SchemaNodeContentIr::Tuple(TupleSchemaIr {
                elements,
                binding_style: tuple.binding_style.map(convert_binding_style),
            })
        }
        SchemaNodeContent::Union(union) => {
            let mut variants = indexmap::IndexMap::new();
            for (variant_name, variant_schema) in &union.variants {
                assert_node_exists(
                    schema,
                    node_idx,
                    *variant_schema,
                    format!("union.variants.{variant_name}"),
                )?;
                variants.insert(variant_name.clone(), SchemaNodeIrId(variant_schema.0));
            }

            let mut unambiguous = indexmap::IndexSet::new();
            for variant in &union.unambiguous {
                unambiguous.insert(variant.clone());
            }

            let mut deny_untagged = indexmap::IndexSet::new();
            for variant in &union.deny_untagged {
                deny_untagged.insert(variant.clone());
            }

            let interop = UnionInteropIr {
                variant_repr: union
                    .interop
                    .variant_repr
                    .as_ref()
                    .map(convert_variant_repr),
            };

            SchemaNodeContentIr::Union(UnionSchemaIr::new(
                variants,
                unambiguous,
                deny_untagged,
                interop,
            ))
        }
        SchemaNodeContent::Reference(reference) => {
            validate_local_reference(schema, node_idx, reference)?;
            SchemaNodeContentIr::Reference(QualifiedTypeName::new(
                reference.namespace.clone(),
                reference.name.to_string(),
            ))
        }
    };

    Ok(out)
}

fn validate_local_reference(
    schema: &SchemaDocument,
    node_idx: usize,
    reference: &TypeReference,
) -> Result<(), SchemaToIrError> {
    if reference.namespace.is_none() && !schema.types.contains_key(&reference.name) {
        return Err(SchemaToIrError::UnknownLocalTypeReference {
            node: node_idx,
            name: reference.name.to_string(),
        });
    }
    Ok(())
}

fn convert_variant_repr(repr: &eure_schema::interop::VariantRepr) -> VariantReprIr {
    match repr {
        eure_schema::interop::VariantRepr::External => VariantReprIr::External,
        eure_schema::interop::VariantRepr::Internal { tag } => {
            VariantReprIr::Internal { tag: tag.clone() }
        }
        eure_schema::interop::VariantRepr::Adjacent { tag, content } => VariantReprIr::Adjacent {
            tag: tag.clone(),
            content: content.clone(),
        },
        eure_schema::interop::VariantRepr::Untagged => VariantReprIr::Untagged,
    }
}

fn convert_binding_style(style: Form) -> BindingStyleIr {
    match style {
        Form::Inline => BindingStyleIr::Inline,
        Form::BindingBlock => BindingStyleIr::BindingBlock,
        Form::BindingValueBlock => BindingStyleIr::BindingValueBlock,
        Form::Section => BindingStyleIr::Section,
        Form::SectionBlock => BindingStyleIr::SectionBlock,
        Form::SectionValueBlock => BindingStyleIr::SectionValueBlock,
        Form::Flatten => BindingStyleIr::Flatten,
    }
}

fn convert_ext_type(ext: &ExtTypeSchema) -> ExtTypeIr {
    ExtTypeIr {
        schema: SchemaNodeIrId(ext.schema.0),
        optional: ext.optional,
        binding_style: ext.binding_style.map(convert_binding_style),
    }
}

fn convert_schema_metadata(
    node_idx: usize,
    metadata: &SchemaMetadata,
) -> Result<SchemaMetadataIr, SchemaToIrError> {
    let description = metadata
        .description
        .as_ref()
        .map(|description| match description {
            Description::String(text) => DescriptionIr::String(text.clone()),
            Description::Markdown(text) => DescriptionIr::Markdown(text.clone()),
        });

    let default = metadata
        .default
        .as_ref()
        .map(|doc| convert_document_value(doc, node_idx, "metadata.default".to_string()))
        .transpose()?;

    let examples = metadata
        .examples
        .as_ref()
        .map(|docs| {
            docs.iter()
                .enumerate()
                .map(|(idx, doc)| {
                    convert_document_value(doc, node_idx, format!("metadata.examples[{idx}]"))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    Ok(SchemaMetadataIr {
        description,
        deprecated: metadata.deprecated,
        default,
        examples,
    })
}

fn convert_bigint_bound<T>(bound: &Bound<T>) -> BoundIr<DecimalInt>
where
    T: ToString,
{
    match bound {
        Bound::Unbounded => BoundIr::Unbounded,
        Bound::Inclusive(value) => BoundIr::Inclusive(DecimalInt::new(value.to_string())),
        Bound::Exclusive(value) => BoundIr::Exclusive(DecimalInt::new(value.to_string())),
    }
}

fn convert_float_bound(bound: &Bound<f64>) -> BoundIr<f64> {
    match bound {
        Bound::Unbounded => BoundIr::Unbounded,
        Bound::Inclusive(value) => BoundIr::Inclusive(*value),
        Bound::Exclusive(value) => BoundIr::Exclusive(*value),
    }
}

fn assert_node_exists(
    schema: &SchemaDocument,
    node_idx: usize,
    target: SchemaNodeId,
    path: String,
) -> Result<(), SchemaToIrError> {
    if target.0 >= schema.nodes.len() {
        return Err(SchemaToIrError::MissingSchemaNodeReference {
            node: node_idx,
            target: target.0,
            path,
        });
    }
    Ok(())
}

fn convert_document_value(
    doc: &EureDocument,
    schema_node: usize,
    path: String,
) -> Result<ValueIr, SchemaToIrError> {
    convert_node_value(doc, doc.get_root_id(), schema_node, path)
}

fn convert_node_value(
    doc: &EureDocument,
    node_id: NodeId,
    schema_node: usize,
    path: String,
) -> Result<ValueIr, SchemaToIrError> {
    let node = doc.node(node_id);
    if !node.extensions.is_empty() {
        return Err(SchemaToIrError::UnsupportedValueExtensions {
            node: schema_node,
            field_path: path,
        });
    }

    match &node.content {
        NodeValue::Hole(_) => Err(SchemaToIrError::UnsupportedHoleValue {
            node: schema_node,
            field_path: path,
        }),
        NodeValue::PartialMap(_) => Err(SchemaToIrError::UnsupportedPartialMapValue {
            node: schema_node,
            field_path: path,
        }),
        NodeValue::Primitive(primitive) => Ok(convert_primitive_value(primitive)),
        NodeValue::Array(array) => {
            let mut out = Vec::with_capacity(array.len());
            for (index, child) in array.iter().enumerate() {
                out.push(convert_node_value(
                    doc,
                    *child,
                    schema_node,
                    format!("{path}[{index}]"),
                )?);
            }
            Ok(ValueIr::Array(out))
        }
        NodeValue::Tuple(tuple) => {
            let mut out = Vec::with_capacity(tuple.len());
            for (index, child) in tuple.iter().enumerate() {
                out.push(convert_node_value(
                    doc,
                    *child,
                    schema_node,
                    format!("{path}[{index}]"),
                )?);
            }
            Ok(ValueIr::Tuple(out))
        }
        NodeValue::Map(map) => {
            let mut out = indexmap::IndexMap::new();
            for (key, child) in map {
                out.insert(
                    convert_object_key(key),
                    convert_node_value(doc, *child, schema_node, format!("{path}[{}]", key))?,
                );
            }
            Ok(ValueIr::Map(out))
        }
    }
}

fn convert_primitive_value(value: &PrimitiveValue) -> ValueIr {
    match value {
        PrimitiveValue::Null => ValueIr::Null,
        PrimitiveValue::Bool(value) => ValueIr::Bool(*value),
        PrimitiveValue::Integer(value) => ValueIr::Integer(DecimalInt::new(value.to_string())),
        PrimitiveValue::F32(value) => ValueIr::Float(f64::from(*value)),
        PrimitiveValue::F64(value) => ValueIr::Float(*value),
        PrimitiveValue::Text(text) => ValueIr::Text(TextValueIr {
            value: text.content.clone(),
            language: convert_text_language(&text.language),
        }),
    }
}

fn convert_text_language(language: &Language) -> TextLanguageIr {
    match language {
        Language::Plaintext => TextLanguageIr::Plain,
        Language::Implicit => TextLanguageIr::Implicit,
        Language::Other(tag) => TextLanguageIr::Tagged(tag.to_string()),
    }
}

fn convert_object_key(key: &ObjectKey) -> ObjectKeyIr {
    match key {
        ObjectKey::String(value) => ObjectKeyIr::String(value.clone()),
        ObjectKey::Number(value) => ObjectKeyIr::Integer(DecimalInt::new(value.to_string())),
        ObjectKey::Tuple(items) => {
            ObjectKeyIr::Tuple(items.0.iter().map(convert_object_key).collect::<Vec<_>>())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::value::{ObjectKey, PrimitiveValue, Tuple};
    use eure_schema::{
        BindingStyle, CodegenDefaults, FieldCodegen, IntegerSchema, RecordCodegen,
        RecordFieldSchema, RecordSchema, RootCodegen, SchemaMetadata, SchemaNode,
        SchemaNodeContent, SchemaNodeId, TextSchema, TypeCodegen, TypeReference, UnionCodegen,
        UnionSchema,
    };

    #[test]
    fn preserves_record_and_union_schema_semantics() {
        let text_node = SchemaNode {
            content: SchemaNodeContent::Text(TextSchema {
                language: Some("rust".to_string()),
                min_length: Some(1),
                max_length: Some(16),
                pattern: None,
                unknown_fields: {
                    let mut fields = indexmap::IndexMap::new();
                    fields.insert(
                        "x".to_string(),
                        EureDocument::new_primitive(PrimitiveValue::Bool(true)),
                    );
                    fields
                },
            }),
            metadata: SchemaMetadata::default(),
            ext_types: Default::default(),
            type_codegen: TypeCodegen::None,
        };

        let integer_node = SchemaNode {
            content: SchemaNodeContent::Integer(IntegerSchema {
                min: Bound::Inclusive(0.into()),
                max: Bound::Unbounded,
                multiple_of: Some(3.into()),
            }),
            metadata: SchemaMetadata::default(),
            ext_types: Default::default(),
            type_codegen: TypeCodegen::None,
        };

        let union_node = SchemaNode {
            content: SchemaNodeContent::Union(UnionSchema {
                variants: {
                    let mut variants = indexmap::IndexMap::new();
                    variants.insert("ok".to_string(), SchemaNodeId(4));
                    variants.insert("err".to_string(), SchemaNodeId(5));
                    variants
                },
                unambiguous: {
                    let mut set = indexmap::IndexSet::new();
                    set.insert("ok".to_string());
                    set
                },
                interop: eure_schema::interop::UnionInterop {
                    variant_repr: Some(eure_schema::interop::VariantRepr::Adjacent {
                        tag: "type".to_string(),
                        content: "content".to_string(),
                    }),
                },
                deny_untagged: {
                    let mut set = indexmap::IndexSet::new();
                    set.insert("err".to_string());
                    set
                },
            }),
            metadata: SchemaMetadata::default(),
            ext_types: Default::default(),
            type_codegen: TypeCodegen::Union(UnionCodegen {
                type_name: Some("ApiResult".to_string()),
                derive: Some(vec!["Debug".to_string(), "Clone".to_string()]),
                variant_types: Some(true),
                variant_types_suffix: Some("Type".to_string()),
            }),
        };

        let record_node = SchemaNode {
            content: SchemaNodeContent::Record(RecordSchema {
                properties: {
                    let mut properties = indexmap::IndexMap::new();
                    properties.insert(
                        "name".to_string(),
                        RecordFieldSchema {
                            schema: SchemaNodeId(1),
                            optional: false,
                            binding_style: Some(BindingStyle::Inline),
                            field_codegen: FieldCodegen {
                                name: Some("display_name".to_string()),
                            },
                        },
                    );
                    properties
                },
                flatten: vec![SchemaNodeId(3)],
                unknown_fields: UnknownFieldsPolicy::Schema(SchemaNodeId(2)),
            }),
            metadata: SchemaMetadata::default(),
            ext_types: Default::default(),
            type_codegen: TypeCodegen::Record(RecordCodegen {
                type_name: Some("RootRecord".to_string()),
                derive: Some(vec!["Debug".to_string()]),
            }),
        };

        let schema = SchemaDocument {
            nodes: vec![
                record_node,
                text_node,
                integer_node,
                union_node,
                SchemaNode {
                    content: SchemaNodeContent::Boolean,
                    metadata: SchemaMetadata::default(),
                    ext_types: Default::default(),
                    type_codegen: TypeCodegen::None,
                },
                SchemaNode {
                    content: SchemaNodeContent::Null,
                    metadata: SchemaMetadata::default(),
                    ext_types: Default::default(),
                    type_codegen: TypeCodegen::None,
                },
            ],
            root: SchemaNodeId(0),
            types: {
                let mut types = indexmap::IndexMap::new();
                types.insert("root-record".parse().unwrap(), SchemaNodeId(0));
                types
            },
            root_codegen: RootCodegen {
                type_name: Some("RootRecord".to_string()),
            },
            codegen_defaults: CodegenDefaults {
                derive: Some(vec!["Debug".to_string(), "Clone".to_string()]),
                ext_types_field_prefix: Some("ext_".to_string()),
                ext_types_type_prefix: Some("Ext".to_string()),
                document_node_id_field: Some("node_id".to_string()),
            },
        };

        let module = schema_to_ir_module(&schema).expect("schema conversion should succeed");
        let ty = module
            .get_type_by_name(&QualifiedTypeName::local("root-record"))
            .expect("type should exist");

        assert!(matches!(ty.rust_binding().kind(), RustTypeKindIr::Record));

        let root = ty
            .schema_nodes()
            .get(&SchemaNodeIrId(0))
            .expect("root node should exist");
        let SchemaNodeContentIr::Record(record) = root.content() else {
            panic!("expected record root")
        };
        assert!(matches!(
            record.unknown_fields(),
            UnknownFieldsPolicyIr::Schema(SchemaNodeIrId(2))
        ));
        assert_eq!(record.flatten(), [SchemaNodeIrId(3)]);
        assert_eq!(
            record
                .properties()
                .get("name")
                .map(|field| field.binding_style()),
            Some(Some(BindingStyleIr::Inline))
        );

        let text = ty
            .schema_nodes()
            .get(&SchemaNodeIrId(1))
            .expect("text node should exist");
        let SchemaNodeContentIr::Text(text) = text.content() else {
            panic!("expected text node")
        };
        assert_eq!(text.language.as_deref(), Some("rust"));
        assert!(matches!(
            text.unknown_fields.get("x"),
            Some(ValueIr::Bool(true))
        ));

        let union = ty
            .schema_nodes()
            .get(&SchemaNodeIrId(3))
            .expect("union node should exist");
        let SchemaNodeContentIr::Union(union) = union.content() else {
            panic!("expected union node")
        };
        assert!(matches!(
            union.interop().variant_repr,
            Some(VariantReprIr::Adjacent { .. })
        ));
        assert!(union.unambiguous().contains("ok"));
        assert!(union.deny_untagged().contains("err"));
        assert_eq!(
            module.root_codegen().type_name_override.as_deref(),
            Some("RootRecord")
        );
        assert_eq!(module.codegen_defaults().document_node_id_field, "node_id");
        let TypeCodegenIr::Record(record_codegen) = ty.type_codegen() else {
            panic!("expected record codegen")
        };
        assert_eq!(
            record_codegen.type_name_override.as_deref(),
            Some("RootRecord")
        );
        assert_eq!(
            record
                .properties()
                .get("name")
                .and_then(|f| f.field_codegen().name_override.as_deref()),
            Some("display_name")
        );
    }

    #[test]
    fn preserves_literal_value_object_keys() {
        let mut doc = EureDocument::new();
        let root = doc.get_root_id();

        let int_child = doc
            .add_map_child(ObjectKey::from(10_i64), root)
            .expect("insert int key")
            .node_id;
        doc.set_content(int_child, PrimitiveValue::Bool(true).into());

        let tuple_key = ObjectKey::Tuple(Tuple(vec![ObjectKey::from("a"), ObjectKey::from(2_i64)]));
        let tuple_child = doc
            .add_map_child(tuple_key, root)
            .expect("insert tuple key")
            .node_id;
        doc.set_content(tuple_child, PrimitiveValue::Bool(false).into());

        let schema = SchemaDocument {
            nodes: vec![SchemaNode {
                content: SchemaNodeContent::Literal(doc),
                metadata: SchemaMetadata::default(),
                ext_types: Default::default(),
                type_codegen: TypeCodegen::None,
            }],
            root: SchemaNodeId(0),
            types: {
                let mut types = indexmap::IndexMap::new();
                types.insert("literal-map".parse().unwrap(), SchemaNodeId(0));
                types
            },
            root_codegen: RootCodegen::default(),
            codegen_defaults: CodegenDefaults::default(),
        };

        let module = schema_to_ir_module(&schema).expect("schema conversion should succeed");
        let ty = module
            .get_type_by_name(&QualifiedTypeName::local("literal-map"))
            .expect("type should exist");

        let SchemaNodeContentIr::Literal(ValueIr::Map(map)) = ty
            .schema_nodes()
            .get(&SchemaNodeIrId(0))
            .expect("literal root")
            .content()
        else {
            panic!("expected literal map")
        };

        assert!(map.contains_key(&ObjectKeyIr::Integer(DecimalInt::new("10"))));
        assert!(map.contains_key(&ObjectKeyIr::Tuple(vec![
            ObjectKeyIr::String("a".to_string()),
            ObjectKeyIr::Integer(DecimalInt::new("2")),
        ])));
    }

    #[test]
    fn rejects_value_extensions_that_ir_cannot_represent() {
        let mut value = EureDocument::new_primitive(PrimitiveValue::Bool(true));
        let root = value.get_root_id();
        let ext_node = value
            .add_extension("meta".parse().unwrap(), root)
            .expect("insert extension")
            .node_id;
        value.set_content(ext_node, PrimitiveValue::Bool(false).into());

        let schema = SchemaDocument {
            nodes: vec![SchemaNode {
                content: SchemaNodeContent::Reference(TypeReference {
                    namespace: None,
                    name: "self".parse().unwrap(),
                }),
                metadata: SchemaMetadata {
                    description: None,
                    deprecated: false,
                    default: Some(value),
                    examples: None,
                },
                ext_types: Default::default(),
                type_codegen: TypeCodegen::None,
            }],
            root: SchemaNodeId(0),
            types: {
                let mut types = indexmap::IndexMap::new();
                types.insert("self".parse().unwrap(), SchemaNodeId(0));
                types
            },
            root_codegen: RootCodegen::default(),
            codegen_defaults: CodegenDefaults::default(),
        };

        let err = schema_to_ir_module(&schema).expect_err("conversion should reject extensions");
        assert!(matches!(
            err,
            SchemaToIrError::UnsupportedValueExtensions { .. }
        ));
    }
}
