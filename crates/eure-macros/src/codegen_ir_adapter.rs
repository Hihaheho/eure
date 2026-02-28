use darling::{FromDeriveInput, FromField, FromVariant};
use indexmap::{IndexMap, IndexSet};
use proc_macro2::Span;
use quote::ToTokens;
use std::collections::HashMap;
use syn::spanned::Spanned;

use crate::attrs::{
    ContainerAttrs, DefaultValue, FieldAttrs, RenameAll, VariantAttrs,
    extract_container_attr_spans, extract_eure_attr_spans, extract_variant_attr_spans,
};
use crate::config::MacroConfig;
use crate::context::MacroContext;
use crate::ir::{FieldMode, RenameScope, analyze_common_named_fields};
use crate::ir_spans::DeriveSpanTable;

use eure_codegen_ir::{
    ConstParamIr, ContainerAttrsIr, DefaultValueIr, FieldCodegenIr, FieldModeIr,
    FieldSourceAttrsIr, IrBuildError, IrModule, LifetimeParamIr, MapImplTypeIr,
    PrimitiveRustTypeIr, QualifiedTypeName, RecordFieldSchemaIr, RecordSchemaIr, RenameRuleIr,
    RustBindingIr, RustFieldIr, RustGenericsIr, RustPathIr, RustTypeExprIr, RustTypeKindIr,
    RustVariantIr, SchemaMetadataIr, SchemaNodeContentIr, SchemaNodeIr, SchemaNodeIrId,
    TupleElementIr, TupleSchemaIr, TypeCodegenIr, TypeDefIr, TypeId, TypeNamesIr, TypeOriginIr,
    TypeParamIr, UnionInteropIr, UnionSchemaIr, UnknownFieldsPolicyIr, VariantShapeIr,
    WhereClauseIr, WrapperKindIr,
};

#[derive(Debug, Clone)]
pub(crate) struct DeriveIrArtifacts {
    pub(crate) module: IrModule,
    pub(crate) spans: DeriveSpanTable,
}

#[cfg(test)]
pub(crate) fn derive_input_to_ir(input: &syn::DeriveInput) -> syn::Result<IrModule> {
    derive_input_to_ir_artifacts(input).map(|artifacts| artifacts.module)
}

pub(crate) fn derive_input_to_ir_artifacts(
    input: &syn::DeriveInput,
) -> syn::Result<DeriveIrArtifacts> {
    let mut attrs = ContainerAttrs::from_derive_input(input)
        .map_err(|err| syn::Error::new(input.span(), err.to_string()))?;
    attrs.non_exhaustive |= has_non_exhaustive_attr(input);
    let attr_spans = extract_container_attr_spans(input);
    let mut spans = DeriveSpanTable::new(input.span(), attr_spans.clone());
    let mut attrs_for_config = ContainerAttrs::from_derive_input(input)
        .map_err(|err| syn::Error::new(input.span(), err.to_string()))?;
    attrs_for_config.non_exhaustive |= has_non_exhaustive_attr(input);
    let config = MacroConfig::from_attrs(attrs_for_config, attr_spans)?;
    let context = MacroContext::new(config, input.clone());

    let rust_name = input.ident.to_string();
    let type_id = TypeId(rust_name.clone());
    let schema_name = Some(QualifiedTypeName::local(
        attrs.type_name.clone().unwrap_or_else(|| rust_name.clone()),
    ));

    let container = container_attrs_to_ir(&attrs);
    let mut binding = RustBindingIr::new(
        RustTypeKindIr::Unit,
        container,
        Vec::new(),
        Vec::new(),
        generics_to_ir(&input.generics),
        where_clause_to_ir(&input.generics),
        Default::default(),
    );

    let mut schema_nodes = IndexMap::<SchemaNodeIrId, SchemaNodeIr>::new();
    let mut next_node = 1usize;

    let root_content = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => {
                binding.set_kind(RustTypeKindIr::Record);
                let common =
                    analyze_common_named_fields(&context, &fields.named, RenameScope::Container)?;
                let mut properties = IndexMap::new();
                let mut flatten = Vec::new();

                for (field, common_field) in fields.named.iter().zip(common.iter()) {
                    let attrs = FieldAttrs::from_field(field)
                        .map_err(|err| syn::Error::new(field.span(), err.to_string()))?;
                    let field_attr_spans = extract_eure_attr_spans(&field.attrs);
                    spans.upsert_field(
                        common_field.ident.to_string(),
                        field.span(),
                        field.ty.span(),
                        field_attr_spans,
                    );
                    let rust_field = to_rust_field(
                        &common_field.ident,
                        &common_field.wire_name,
                        common_field.mode,
                        &common_field.ty,
                        &attrs,
                    );
                    let node_id = alloc_any_node(&mut schema_nodes, &mut next_node);

                    if matches!(
                        rust_field.mode(),
                        FieldModeIr::Flatten | FieldModeIr::FlattenExt
                    ) {
                        flatten.push(node_id);
                    } else {
                        properties.insert(
                            rust_field.wire_name().to_string(),
                            RecordFieldSchemaIr::new(
                                node_id,
                                is_option_type(&field.ty),
                                None,
                                FieldCodegenIr::default(),
                            ),
                        );
                    }

                    binding.push_field(rust_field);
                }

                SchemaNodeContentIr::Record(RecordSchemaIr::new(
                    properties,
                    flatten,
                    if attrs.allow_unknown_fields {
                        UnknownFieldsPolicyIr::Allow
                    } else {
                        UnknownFieldsPolicyIr::Deny
                    },
                ))
            }
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                binding.set_kind(RustTypeKindIr::Newtype);
                let field = &fields.unnamed[0];
                let field_attrs = FieldAttrs::from_field(field)
                    .map_err(|err| syn::Error::new(field.span(), err.to_string()))?;
                spans.upsert_field(
                    "0".to_string(),
                    field.span(),
                    field.ty.span(),
                    extract_eure_attr_spans(&field.attrs),
                );
                binding.push_field(RustFieldIr::new(
                    "0".to_string(),
                    "0".to_string(),
                    FieldModeIr::Record,
                    FieldSourceAttrsIr::default(),
                    rust_type_expr(&field.ty),
                    default_value_to_ir(&field_attrs.default),
                    field_attrs.via.map(path_from_type),
                ));
                SchemaNodeContentIr::Any
            }
            syn::Fields::Unnamed(fields) => {
                binding.set_kind(RustTypeKindIr::Tuple);
                let mut elements = Vec::new();
                for (idx, field) in fields.unnamed.iter().enumerate() {
                    let field_attrs = FieldAttrs::from_field(field)
                        .map_err(|err| syn::Error::new(field.span(), err.to_string()))?;
                    spans.upsert_field(
                        idx.to_string(),
                        field.span(),
                        field.ty.span(),
                        extract_eure_attr_spans(&field.attrs),
                    );
                    binding.push_field(RustFieldIr::new(
                        idx.to_string(),
                        idx.to_string(),
                        FieldModeIr::Record,
                        FieldSourceAttrsIr::default(),
                        rust_type_expr(&field.ty),
                        default_value_to_ir(&field_attrs.default),
                        field_attrs.via.map(path_from_type),
                    ));
                    elements.push(alloc_any_node(&mut schema_nodes, &mut next_node));
                }
                SchemaNodeContentIr::Tuple(TupleSchemaIr {
                    elements,
                    binding_style: None,
                })
            }
            syn::Fields::Unit => {
                binding.set_kind(RustTypeKindIr::Unit);
                SchemaNodeContentIr::Null
            }
        },
        syn::Data::Enum(data) => {
            binding.set_kind(RustTypeKindIr::Enum);
            let mut variants = IndexMap::new();

            for variant in &data.variants {
                let variant_attrs = VariantAttrs::from_variant(variant)
                    .map_err(|err| syn::Error::new(variant.span(), err.to_string()))?;
                let variant_rust_name = variant.ident.to_string();
                spans.upsert_variant(
                    variant_rust_name.clone(),
                    variant.span(),
                    extract_variant_attr_spans(variant),
                );
                let wire_name = variant_attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_rename(&variant_rust_name));

                let (shape, variant_schema_node) = match &variant.fields {
                    syn::Fields::Unit => (VariantShapeIr::Unit, schema_node_null()),
                    syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        let field = &fields.unnamed[0];
                        let attrs = FieldAttrs::from_field(field)
                            .map_err(|err| syn::Error::new(field.span(), err.to_string()))?;
                        spans.upsert_variant_field(
                            &variant_rust_name,
                            "0".to_string(),
                            field.span(),
                            field.ty.span(),
                            extract_eure_attr_spans(&field.attrs),
                        );
                        (
                            VariantShapeIr::Newtype {
                                ty: rust_type_expr(&field.ty),
                                via: attrs.via.map(path_from_type),
                            },
                            schema_node_any(),
                        )
                    }
                    syn::Fields::Unnamed(fields) => {
                        let mut tuple_elements = Vec::new();
                        let mut schema_elements = Vec::new();

                        for (idx, field) in fields.unnamed.iter().enumerate() {
                            let attrs = FieldAttrs::from_field(field)
                                .map_err(|err| syn::Error::new(field.span(), err.to_string()))?;
                            spans.upsert_variant_field(
                                &variant_rust_name,
                                idx.to_string(),
                                field.span(),
                                field.ty.span(),
                                extract_eure_attr_spans(&field.attrs),
                            );
                            tuple_elements.push(TupleElementIr {
                                ty: rust_type_expr(&field.ty),
                                via: attrs.via.map(path_from_type),
                            });
                            schema_elements.push(alloc_any_node(&mut schema_nodes, &mut next_node));
                        }

                        (
                            VariantShapeIr::Tuple(tuple_elements),
                            SchemaNodeIr::new(
                                SchemaNodeContentIr::Tuple(TupleSchemaIr {
                                    elements: schema_elements,
                                    binding_style: None,
                                }),
                                SchemaMetadataIr::default(),
                                IndexMap::new(),
                            ),
                        )
                    }
                    syn::Fields::Named(fields) => {
                        let common = analyze_common_named_fields(
                            &context,
                            &fields.named,
                            RenameScope::Field,
                        )?;
                        let mut rust_fields = Vec::new();
                        let mut properties = IndexMap::new();
                        let mut flatten = Vec::new();

                        for (field, common_field) in fields.named.iter().zip(common.iter()) {
                            let attrs = FieldAttrs::from_field(field)
                                .map_err(|err| syn::Error::new(field.span(), err.to_string()))?;
                            spans.upsert_variant_field(
                                &variant_rust_name,
                                common_field.ident.to_string(),
                                field.span(),
                                field.ty.span(),
                                extract_eure_attr_spans(&field.attrs),
                            );
                            let rust_field = to_rust_field(
                                &common_field.ident,
                                &common_field.wire_name,
                                common_field.mode,
                                &common_field.ty,
                                &attrs,
                            );
                            let node_id = alloc_any_node(&mut schema_nodes, &mut next_node);

                            if matches!(
                                rust_field.mode(),
                                FieldModeIr::Flatten | FieldModeIr::FlattenExt
                            ) {
                                flatten.push(node_id);
                            } else {
                                properties.insert(
                                    rust_field.wire_name().to_string(),
                                    RecordFieldSchemaIr::new(
                                        node_id,
                                        is_option_type(&field.ty),
                                        None,
                                        FieldCodegenIr::default(),
                                    ),
                                );
                            }
                            rust_fields.push(rust_field);
                        }

                        (
                            VariantShapeIr::Record(rust_fields),
                            SchemaNodeIr::new(
                                SchemaNodeContentIr::Record(RecordSchemaIr::new(
                                    properties,
                                    flatten,
                                    UnknownFieldsPolicyIr::Deny,
                                )),
                                SchemaMetadataIr::default(),
                                IndexMap::new(),
                            ),
                        )
                    }
                };

                let variant_node_id =
                    alloc_schema_node(&mut schema_nodes, &mut next_node, variant_schema_node);
                variants.insert(wire_name.clone(), variant_node_id);
                binding.push_variant(RustVariantIr::new(
                    variant_rust_name,
                    wire_name,
                    variant_attrs.allow_unknown_fields,
                    shape,
                ));
            }

            SchemaNodeContentIr::Union(UnionSchemaIr::new(
                variants,
                IndexSet::new(),
                IndexSet::new(),
                UnionInteropIr::default(),
            ))
        }
        syn::Data::Union(data) => {
            return Err(syn::Error::new(
                data.union_token.span,
                "union types are not supported by eure derive macros",
            ));
        }
    };

    let root_id = SchemaNodeIrId(0);
    schema_nodes.insert(
        root_id,
        SchemaNodeIr::new(root_content, SchemaMetadataIr::default(), IndexMap::new()),
    );

    let type_def = TypeDefIr::new(
        type_id.clone(),
        TypeNamesIr::new(rust_name, schema_name.clone()),
        schema_nodes,
        root_id,
        binding,
        TypeCodegenIr::None,
        TypeOriginIr::Derive,
    );

    let mut module = IrModule::default();
    if let Some(name) = schema_name {
        module.insert_name_index(name, type_id.clone());
    }
    module.push_root(type_id.clone());
    module.insert_type(type_id, type_def);

    let module = module
        .into_checked()
        .map_err(|err| ir_build_error_to_syn(input, err))?;

    Ok(DeriveIrArtifacts { module, spans })
}

fn container_attrs_to_ir(attrs: &ContainerAttrs) -> ContainerAttrsIr {
    ContainerAttrsIr::new(
        attrs
            .crate_path
            .as_ref()
            .map(|path| RustPathIr::new(path.to_token_stream().to_string())),
        attrs.rename_all.map(rename_rule_to_ir),
        attrs.rename_all_fields.map(rename_rule_to_ir),
        attrs.parse_ext,
        attrs.allow_unknown_fields,
        attrs.allow_unknown_extensions,
        attrs
            .parse_error
            .as_ref()
            .map(|path| RustPathIr::new(path.to_token_stream().to_string())),
        attrs
            .write_error
            .as_ref()
            .map(|path| RustPathIr::new(path.to_token_stream().to_string())),
        attrs.type_name.clone(),
        attrs.non_exhaustive,
        attrs
            .proxy
            .as_ref()
            .map(|target| RustPathIr::new(target.to_token_stream().to_string())),
        attrs
            .opaque
            .as_ref()
            .map(|target| RustPathIr::new(target.to_token_stream().to_string())),
    )
}

fn rename_rule_to_ir(rule: RenameAll) -> RenameRuleIr {
    match rule {
        RenameAll::Lower => RenameRuleIr::Lower,
        RenameAll::Upper => RenameRuleIr::Upper,
        RenameAll::Pascal => RenameRuleIr::Pascal,
        RenameAll::Camel => RenameRuleIr::Camel,
        RenameAll::Snake => RenameRuleIr::Snake,
        RenameAll::ScreamingSnake => RenameRuleIr::ScreamingSnake,
        RenameAll::Kebab => RenameRuleIr::Kebab,
        RenameAll::Cobol => RenameRuleIr::Cobol,
    }
}

fn to_rust_field(
    ident: &syn::Ident,
    wire_name: &str,
    mode: FieldMode,
    ty: &syn::Type,
    attrs: &FieldAttrs,
) -> RustFieldIr {
    RustFieldIr::new(
        ident.to_string(),
        wire_name.to_string(),
        match mode {
            FieldMode::Record => FieldModeIr::Record,
            FieldMode::Ext => FieldModeIr::Ext,
            FieldMode::Flatten => FieldModeIr::Flatten,
            FieldMode::FlattenExt => FieldModeIr::FlattenExt,
        },
        FieldSourceAttrsIr {
            ext: attrs.ext,
            flatten: attrs.flatten,
            flatten_ext: attrs.flatten_ext,
        },
        rust_type_expr(ty),
        default_value_to_ir(&attrs.default),
        attrs.via.clone().map(path_from_type),
    )
}

fn default_value_to_ir(default: &DefaultValue) -> DefaultValueIr {
    match default {
        DefaultValue::None => DefaultValueIr::None,
        DefaultValue::Default => DefaultValueIr::DefaultTrait,
        DefaultValue::Path(path) => {
            DefaultValueIr::Function(RustPathIr::new(path.to_token_stream().to_string()))
        }
    }
}

fn generics_to_ir(generics: &syn::Generics) -> RustGenericsIr {
    let mut out = RustGenericsIr::default();

    for param in &generics.params {
        match param {
            syn::GenericParam::Type(ty) => {
                out.type_params.push(TypeParamIr {
                    name: ty.ident.to_string(),
                    bounds: ty
                        .bounds
                        .iter()
                        .map(|bound| bound.to_token_stream().to_string())
                        .collect(),
                });
            }
            syn::GenericParam::Lifetime(lt) => {
                out.lifetime_params.push(LifetimeParamIr {
                    name: lt.lifetime.to_string(),
                    bounds: lt
                        .bounds
                        .iter()
                        .map(|bound| bound.to_token_stream().to_string())
                        .collect(),
                });
            }
            syn::GenericParam::Const(cnst) => {
                out.const_params.push(ConstParamIr {
                    name: cnst.ident.to_string(),
                    ty: cnst.ty.to_token_stream().to_string(),
                });
            }
        }
    }

    out
}

fn where_clause_to_ir(generics: &syn::Generics) -> WhereClauseIr {
    let mut out = WhereClauseIr::default();
    if let Some(where_clause) = &generics.where_clause {
        out.predicates = where_clause
            .predicates
            .iter()
            .map(|pred| pred.to_token_stream().to_string())
            .collect();
    }
    out
}

fn rust_type_expr(ty: &syn::Type) -> RustTypeExprIr {
    match ty {
        syn::Type::Reference(r) => {
            if let syn::Type::Path(p) = r.elem.as_ref()
                && p.path.is_ident("str")
            {
                return RustTypeExprIr::Primitive(PrimitiveRustTypeIr::String);
            }
            RustTypeExprIr::Path(RustPathIr::new(ty.to_token_stream().to_string()))
        }
        syn::Type::Tuple(tuple) => {
            let elements = tuple.elems.iter().map(rust_type_expr).collect::<Vec<_>>();
            if elements.is_empty() {
                RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Unit)
            } else {
                RustTypeExprIr::Tuple(elements)
            }
        }
        syn::Type::Path(type_path) => path_type_expr(type_path),
        _ => RustTypeExprIr::Path(RustPathIr::new(ty.to_token_stream().to_string())),
    }
}

fn path_type_expr(type_path: &syn::TypePath) -> RustTypeExprIr {
    let Some(segment) = type_path.path.segments.last() else {
        return RustTypeExprIr::Path(RustPathIr::new(type_path.to_token_stream().to_string()));
    };

    let ident = segment.ident.to_string();

    match ident.as_str() {
        "String" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::String),
        "bool" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Bool),
        "i8" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::I8),
        "i16" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::I16),
        "i32" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::I32),
        "i64" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::I64),
        "i128" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::I128),
        "isize" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Isize),
        "u8" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::U8),
        "u16" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::U16),
        "u32" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::U32),
        "u64" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::U64),
        "u128" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::U128),
        "usize" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Usize),
        "f32" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::F32),
        "f64" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::F64),
        "Text" => RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Text),
        "Option" => unary_generic_type(segment, RustTypeExprIr::Option),
        "Vec" => unary_generic_type(segment, RustTypeExprIr::Vec),
        "Box" => wrapper_generic_type(segment, WrapperKindIr::Box),
        "Rc" => wrapper_generic_type(segment, WrapperKindIr::Rc),
        "Arc" => wrapper_generic_type(segment, WrapperKindIr::Arc),
        "Result" => result_generic_type(segment),
        "HashMap" => map_generic_type(segment, MapImplTypeIr::HashMap),
        "BTreeMap" => map_generic_type(segment, MapImplTypeIr::BTreeMap),
        "IndexMap" => map_generic_type(segment, MapImplTypeIr::IndexMap),
        _ => RustTypeExprIr::Path(RustPathIr::new(type_path.to_token_stream().to_string())),
    }
}

fn unary_generic_type(
    segment: &syn::PathSegment,
    constructor: impl FnOnce(Box<RustTypeExprIr>) -> RustTypeExprIr,
) -> RustTypeExprIr {
    let inner = generic_args(segment)
        .into_iter()
        .next()
        .map(|ty| Box::new(rust_type_expr(ty)))
        .unwrap_or_else(|| Box::new(RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Any)));
    constructor(inner)
}

fn wrapper_generic_type(segment: &syn::PathSegment, wrapper: WrapperKindIr) -> RustTypeExprIr {
    let inner = generic_args(segment)
        .into_iter()
        .next()
        .map(|ty| Box::new(rust_type_expr(ty)))
        .unwrap_or_else(|| Box::new(RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Any)));

    RustTypeExprIr::Wrapper { inner, wrapper }
}

fn result_generic_type(segment: &syn::PathSegment) -> RustTypeExprIr {
    let args = generic_args(segment);
    let ok = args
        .first()
        .map(|ty| Box::new(rust_type_expr(ty)))
        .unwrap_or_else(|| Box::new(RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Any)));
    let err = args
        .get(1)
        .map(|ty| Box::new(rust_type_expr(ty)))
        .unwrap_or_else(|| Box::new(RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Any)));

    RustTypeExprIr::Result { ok, err }
}

fn map_generic_type(segment: &syn::PathSegment, impl_type: MapImplTypeIr) -> RustTypeExprIr {
    let args = generic_args(segment);
    let key = args
        .first()
        .map(|ty| Box::new(rust_type_expr(ty)))
        .unwrap_or_else(|| Box::new(RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Any)));
    let value = args
        .get(1)
        .map(|ty| Box::new(rust_type_expr(ty)))
        .unwrap_or_else(|| Box::new(RustTypeExprIr::Primitive(PrimitiveRustTypeIr::Any)));

    RustTypeExprIr::Map {
        key,
        value,
        impl_type,
    }
}

fn generic_args(segment: &syn::PathSegment) -> Vec<&syn::Type> {
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

fn schema_node_any() -> SchemaNodeIr {
    SchemaNodeIr::new(
        SchemaNodeContentIr::Any,
        SchemaMetadataIr::default(),
        IndexMap::new(),
    )
}

fn schema_node_null() -> SchemaNodeIr {
    SchemaNodeIr::new(
        SchemaNodeContentIr::Null,
        SchemaMetadataIr::default(),
        IndexMap::new(),
    )
}

fn alloc_any_node(
    schema_nodes: &mut IndexMap<SchemaNodeIrId, SchemaNodeIr>,
    next_node: &mut usize,
) -> SchemaNodeIrId {
    alloc_schema_node(schema_nodes, next_node, schema_node_any())
}

fn alloc_schema_node(
    schema_nodes: &mut IndexMap<SchemaNodeIrId, SchemaNodeIr>,
    next_node: &mut usize,
    node: SchemaNodeIr,
) -> SchemaNodeIrId {
    let id = SchemaNodeIrId(*next_node);
    *next_node += 1;
    schema_nodes.insert(id, node);
    id
}

fn path_from_type(ty: syn::Type) -> RustPathIr {
    RustPathIr::new(ty.to_token_stream().to_string())
}

fn has_non_exhaustive_attr(input: &syn::DeriveInput) -> bool {
    input
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("non_exhaustive"))
}

fn ir_build_error_to_syn(input: &syn::DeriveInput, err: IrBuildError) -> syn::Error {
    match err {
        IrBuildError::ProxyOpaqueConflict { type_id } => {
            let span = find_container_attr_span(input, "proxy")
                .or_else(|| find_container_attr_span(input, "opaque"))
                .unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                format!("type `{type_id}` declares both proxy and opaque targets"),
            )
        }
        IrBuildError::VariantAllowUnknownFieldsInvalid {
            type_id: _,
            variant,
        } => {
            let span = find_variant_attr_span(input, &variant, "allow_unknown_fields")
                .unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                "#[eure(allow_unknown_fields)] is only valid on struct variants with named fields",
            )
        }
        IrBuildError::FieldModeConflict {
            type_id,
            field,
            detail,
        } => {
            let span = find_field_attr_spans(input, &field)
                .and_then(|spans| {
                    for key in ["ext", "flatten", "flatten_ext"] {
                        if let Some(span) = spans.get(key) {
                            return Some(*span);
                        }
                    }
                    None
                })
                .unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                format!("field `{field}` in type `{type_id}` has conflicting mode attrs: {detail}"),
            )
        }
        IrBuildError::ViaWithFlatten { type_id, field } => {
            let span = find_field_attr_spans(input, &field)
                .and_then(|spans| {
                    for key in ["via", "flatten", "flatten_ext"] {
                        if let Some(span) = spans.get(key) {
                            return Some(*span);
                        }
                    }
                    None
                })
                .unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                format!(
                    "field `{field}` in type `{type_id}` cannot use `via` with flatten/flatten_ext"
                ),
            )
        }
        IrBuildError::DefaultWithFlatten { type_id: _, field } => {
            let attr_spans = find_field_attr_spans(input, &field);
            let span = attr_spans
                .as_ref()
                .and_then(|spans| spans.get("default").copied())
                .unwrap_or_else(|| input.span());
            let uses_flatten_ext = attr_spans
                .as_ref()
                .is_some_and(|spans| spans.contains_key("flatten_ext"));

            if uses_flatten_ext {
                syn::Error::new(
                    span,
                    format!(
                        "cannot use #[eure(default)] with #[eure(flatten_ext)] on field `{field}`; \
flatten_ext parses entire nested types, not optional fields"
                    ),
                )
            } else {
                syn::Error::new(
                    span,
                    format!(
                        "cannot use #[eure(default)] with #[eure(flatten)] on field `{field}`; \
flatten parses entire nested types, not optional fields"
                    ),
                )
            }
        }
        IrBuildError::FlattenInParseExt { type_id: _, field } => {
            let span = find_field_attr_spans(input, &field)
                .and_then(|spans| spans.get("flatten").copied())
                .unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                "#[eure(flatten)] cannot be used in #[eure(parse_ext)] context; use #[eure(flatten_ext)] instead",
            )
        }
        IrBuildError::NameIndexMissingType { name, missing } => syn::Error::new(
            input.span(),
            format!("name_index entry `{name:?}` references missing type `{missing}`"),
        ),
        IrBuildError::NameIndexMismatch {
            name,
            pointed,
            actual,
        } => syn::Error::new(
            input.span(),
            format!(
                "name_index entry `{name:?}` points to type `{pointed}` but type carries schema name `{actual:?}`"
            ),
        ),
        IrBuildError::MissingSemanticRoot { type_id, node } => syn::Error::new(
            input.span(),
            format!("type `{type_id}` root node `{node:?}` does not exist"),
        ),
        IrBuildError::MissingSchemaNodeReference {
            type_id,
            node,
            target,
            path,
        } => syn::Error::new(
            input.span(),
            format!(
                "type `{type_id}` node `{node:?}` references missing schema node `{target:?}` at {path}"
            ),
        ),
        IrBuildError::UnionPolicyUnknownVariant {
            type_id,
            node,
            variant,
        } => syn::Error::new(
            input.span(),
            format!(
                "type `{type_id}` union node `{node:?}` has policy entry `{variant}` not present in variants"
            ),
        ),
        IrBuildError::DuplicateSchemaName {
            type_id,
            schema_name,
        } => syn::Error::new(
            input.span(),
            format!(
                "type `{type_id}` exists in name_index but is duplicated for schema name `{schema_name:?}`"
            ),
        ),
        IrBuildError::RootMissingType { type_id } => syn::Error::new(
            input.span(),
            format!("type `{type_id}` is missing from module roots while declared as root"),
        ),
        IrBuildError::EmptyCodegenOverride { type_id, path } => {
            let span = find_container_attr_span(input, "codegen").unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                format!("codegen override at `{path}` in type `{type_id}` cannot be empty"),
            )
        }
        IrBuildError::EmptyRootCodegenOverride { path } => {
            let span = find_container_attr_span(input, "codegen")
                .or_else(|| find_container_attr_span(input, "codegen_defaults"))
                .unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                format!("root codegen override at `{path}` cannot be empty"),
            )
        }
        IrBuildError::RootTypeNameConflict {
            type_id,
            root_type_name,
            type_type_name,
        } => {
            let span = find_container_attr_span(input, "type_name").unwrap_or_else(|| input.span());
            syn::Error::new(
                span,
                format!(
                    "root codegen type name `{root_type_name}` conflicts with root type `{type_id}` codegen type name `{type_type_name}`"
                ),
            )
        }
    }
}

fn find_container_attr_span(input: &syn::DeriveInput, attr_name: &str) -> Option<Span> {
    extract_container_attr_spans(input).get(attr_name).copied()
}

fn find_variant_attr_span(
    input: &syn::DeriveInput,
    variant_name: &str,
    attr_name: &str,
) -> Option<Span> {
    let syn::Data::Enum(data) = &input.data else {
        return None;
    };

    data.variants
        .iter()
        .find(|variant| variant.ident == variant_name)
        .and_then(|variant| extract_variant_attr_spans(variant).get(attr_name).copied())
}

fn find_field_attr_spans(
    input: &syn::DeriveInput,
    field_name: &str,
) -> Option<HashMap<String, Span>> {
    match &input.data {
        syn::Data::Struct(data) => find_field_attr_spans_in_fields(&data.fields, field_name),
        syn::Data::Enum(data) => data
            .variants
            .iter()
            .find_map(|variant| find_field_attr_spans_in_fields(&variant.fields, field_name)),
        syn::Data::Union(_) => None,
    }
}

fn find_field_attr_spans_in_fields(
    fields: &syn::Fields,
    field_name: &str,
) -> Option<HashMap<String, Span>> {
    match fields {
        syn::Fields::Named(named) => named
            .named
            .iter()
            .find(|field| {
                field
                    .ident
                    .as_ref()
                    .is_some_and(|ident| ident == field_name)
            })
            .map(|field| extract_eure_attr_spans(&field.attrs)),
        syn::Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .find(|(idx, _)| idx.to_string() == field_name)
            .map(|(_, field)| extract_eure_attr_spans(&field.attrs)),
        syn::Fields::Unit => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_captures_container_and_field_semantics() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[eure(rename_all = "kebab-case", parse_ext, allow_unknown_fields, type_name = "my-type")]
            struct Sample {
                #[eure(rename = "display-name")]
                name: String,
                #[eure(default)]
                enabled: Option<bool>,
                #[eure(flatten_ext)]
                meta: Meta,
            }
        };

        let module = derive_input_to_ir(&input).expect("adapter should succeed");
        let ty = module.types().values().next().expect("type exists");

        assert_eq!(ty.names().rust_name(), "Sample");
        assert_eq!(
            ty.names().schema_name().map(|n| n.name.as_str()),
            Some("my-type")
        );
        assert!(ty.rust_binding().container().parse_ext());
        assert!(ty.rust_binding().container().allow_unknown_fields());

        assert_eq!(ty.rust_binding().fields().len(), 3);
        assert_eq!(ty.rust_binding().fields()[0].wire_name(), "display-name");
        assert!(matches!(
            ty.rust_binding().fields()[1].default(),
            DefaultValueIr::DefaultTrait
        ));
        assert!(matches!(
            ty.rust_binding().fields()[2].mode(),
            FieldModeIr::FlattenExt
        ));

        module
            .clone()
            .into_checked()
            .expect("produced IR should validate");
    }

    #[test]
    fn adapter_rejects_invalid_field_mode_combinations() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Bad {
                #[eure(flatten, ext)]
                value: String,
            }
        };

        let err = derive_input_to_ir(&input).expect_err("adapter should reject conflicts");
        assert!(
            err.to_string()
                .contains("cannot use both #[eure(flatten)] and #[eure(ext)]")
        );
    }
}
