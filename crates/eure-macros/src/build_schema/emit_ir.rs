use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use eure_codegen_ir::{
    FieldModeIr, IrModule, RecordSchemaIr, RustFieldIr, RustTypeKindIr, RustVariantIr,
    SchemaNodeContentIr, UnknownFieldsPolicyIr, VariantShapeIr,
};

use crate::emit_ir_common::{self, DeriveIrType};
use crate::ir_spans::DeriveSpanTable;

pub(super) fn derive(ir: &IrModule, spans: &DeriveSpanTable) -> syn::Result<TokenStream> {
    let emit = DeriveIrType::single_root(ir, spans)?;
    let schema_crate = emit.schema_crate();

    let build_body = match emit.binding().kind() {
        RustTypeKindIr::Record => emit_record_schema(&emit)?,
        RustTypeKindIr::Newtype => emit_newtype_schema(&emit)?,
        RustTypeKindIr::Tuple => emit_tuple_schema(&emit)?,
        RustTypeKindIr::Unit => quote! { #schema_crate::SchemaNodeContent::Null },
        RustTypeKindIr::Enum => emit_enum_schema(&emit)?,
    };

    emit_ir_common::impl_build_schema(&emit, build_body)
}

fn emit_record_schema(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let schema_crate = emit.schema_crate();
    let root_node = emit
        .ty()
        .schema_nodes()
        .get(&emit.ty().semantic_root())
        .ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "derive IR semantic root node is missing for record type",
            )
        })?;

    let record_schema = match root_node.content() {
        SchemaNodeContentIr::Record(record) => record,
        other => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("record Rust binding expected record schema root, found `{other:?}`"),
            ));
        }
    };

    let document_crate = emit.document_crate()?;
    let (builds, content) = emit_record_content(
        emit.binding().fields(),
        record_schema,
        &schema_crate,
        &document_crate,
        "field",
        "flatten",
    )?;
    Ok(quote! {
        #(#builds)*
        #content
    })
}

fn emit_newtype_schema(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let document_crate = emit.document_crate()?;
    let field =
        emit.binding().fields().first().ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "newtype has no field")
        })?;
    let field_ty = emit_ir_common::rust_type_tokens(field.ty(), &document_crate)?;
    Ok(quote! { <#field_ty as BuildSchema>::build_schema(ctx) })
}

fn emit_tuple_schema(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let schema_crate = emit.schema_crate();
    let document_crate = emit.document_crate()?;
    let fields = &emit.binding().fields();

    let field_builds = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            let field_ty = emit_ir_common::rust_type_tokens(field.ty(), &document_crate)?;
            let schema_var = format_ident!("field_{}_schema", idx);
            Ok(quote! { let #schema_var = ctx.build::<#field_ty>(); })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let schema_vars = (0..fields.len())
        .map(|idx| format_ident!("field_{}_schema", idx))
        .collect::<Vec<_>>();

    Ok(quote! {
        #(#field_builds)*
        #schema_crate::SchemaNodeContent::Tuple(#schema_crate::TupleSchema {
            elements: vec![#(#schema_vars),*],
            binding_style: None,
        })
    })
}

fn emit_enum_schema(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let schema_crate = emit.schema_crate();
    let document_crate = emit.document_crate()?;
    let root_node = emit
        .ty()
        .schema_nodes()
        .get(&emit.ty().semantic_root())
        .ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "derive IR semantic root node is missing for enum type",
            )
        })?;

    let union_schema = match root_node.content() {
        SchemaNodeContentIr::Union(union) => union,
        other => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("enum Rust binding expected union schema root, found `{other:?}`"),
            ));
        }
    };

    let variant_schemas = emit
        .binding()
        .variants()
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let schema_var = format_ident!("variant_{}_schema", idx);
            let schema_build = emit_variant_schema(
                emit,
                variant,
                union_schema
                    .variants()
                    .get(variant.wire_name())
                    .ok_or_else(|| {
                        syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!("missing schema node for variant `{}`", variant.wire_name()),
                        )
                    })?,
                &schema_var,
                &schema_crate,
                &document_crate,
                idx,
            )?;
            Ok((variant.wire_name().to_string(), schema_var, schema_build))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let builds = variant_schemas
        .iter()
        .map(|(_, _, build)| build.clone())
        .collect::<Vec<_>>();
    let entries = variant_schemas
        .iter()
        .map(|(name, schema_var, _)| quote! { (#name.to_string(), #schema_var) })
        .collect::<Vec<_>>();

    Ok(quote! {
        #(#builds)*
        #schema_crate::SchemaNodeContent::Union(#schema_crate::UnionSchema {
            variants: [#(#entries),*].into_iter().collect(),
            unambiguous: Default::default(),
            interop: #schema_crate::interop::UnionInterop::default(),
            deny_untagged: Default::default(),
        })
    })
}

fn emit_variant_schema(
    emit: &DeriveIrType<'_>,
    variant: &RustVariantIr,
    schema_node_id: &eure_codegen_ir::SchemaNodeIrId,
    schema_var: &syn::Ident,
    schema_crate: &TokenStream,
    document_crate: &TokenStream,
    variant_index: usize,
) -> syn::Result<TokenStream> {
    let schema_node = emit
        .ty()
        .schema_nodes()
        .get(schema_node_id)
        .ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("missing schema node `{:?}` for variant", schema_node_id),
            )
        })?;

    match (variant.shape(), schema_node.content()) {
        (VariantShapeIr::Unit, SchemaNodeContentIr::Null) => Ok(
            quote! { let #schema_var = ctx.create_node(#schema_crate::SchemaNodeContent::Null); },
        ),
        (VariantShapeIr::Newtype { ty, .. }, _) => {
            let field_ty = emit_ir_common::rust_type_tokens(ty, document_crate)?;
            Ok(quote! { let #schema_var = ctx.build::<#field_ty>(); })
        }
        (VariantShapeIr::Tuple(elements), SchemaNodeContentIr::Tuple(_)) => {
            let field_builds = elements
                .iter()
                .enumerate()
                .map(|(idx, element)| {
                    let field_ty = emit_ir_common::rust_type_tokens(&element.ty, document_crate)?;
                    let field_var = format_ident!("variant_{}_field_{}", variant_index, idx);
                    Ok(quote! { let #field_var = ctx.build::<#field_ty>(); })
                })
                .collect::<syn::Result<Vec<_>>>()?;
            let field_vars = (0..elements.len())
                .map(|idx| format_ident!("variant_{}_field_{}", variant_index, idx))
                .collect::<Vec<_>>();

            Ok(quote! {
                #(#field_builds)*
                let #schema_var = ctx.create_node(#schema_crate::SchemaNodeContent::Tuple(
                    #schema_crate::TupleSchema {
                        elements: vec![#(#field_vars),*],
                        binding_style: None,
                    }
                ));
            })
        }
        (VariantShapeIr::Record(fields), SchemaNodeContentIr::Record(record_schema)) => {
            let (builds, record_content) = emit_record_content(
                fields,
                record_schema,
                schema_crate,
                document_crate,
                &format!("variant_{variant_index}_field"),
                &format!("variant_{variant_index}_flatten"),
            )?;
            Ok(quote! {
                #(#builds)*
                let #schema_var = ctx.create_node(#record_content);
            })
        }
        (shape, content) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("variant shape/schema mismatch: shape=`{shape:?}`, schema=`{content:?}`"),
        )),
    }
}

fn emit_record_content(
    fields: &[RustFieldIr],
    record_schema: &RecordSchemaIr,
    schema_crate: &TokenStream,
    document_crate: &TokenStream,
    field_prefix: &str,
    flatten_prefix: &str,
) -> syn::Result<(Vec<TokenStream>, TokenStream)> {
    let mut regular = Vec::new();
    let mut flatten = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let field_ty = emit_ir_common::rust_type_tokens(field.ty(), document_crate)?;
        if matches!(field.mode(), FieldModeIr::Flatten | FieldModeIr::FlattenExt) {
            let schema_var = format_ident!("{}_{}_schema", flatten_prefix, idx);
            flatten.push((schema_var, field_ty));
        } else {
            let schema_var = format_ident!("{}_{}_schema", field_prefix, idx);
            let optional = record_schema
                .properties()
                .get(field.wire_name())
                .map(|prop| prop.optional())
                .unwrap_or(false);
            regular.push((
                field.wire_name().to_string(),
                schema_var,
                field_ty,
                optional,
            ));
        }
    }

    let field_builds = regular
        .iter()
        .map(|(_, schema_var, field_ty, _)| quote! { let #schema_var = ctx.build::<#field_ty>(); })
        .collect::<Vec<_>>();
    let flatten_builds = flatten
        .iter()
        .map(|(schema_var, field_ty)| quote! { let #schema_var = ctx.build::<#field_ty>(); })
        .collect::<Vec<_>>();
    let property_entries = regular
        .iter()
        .map(|(name, schema_var, _, optional)| {
            quote! {
                (
                    #name.to_string(),
                    #schema_crate::RecordFieldSchema {
                        schema: #schema_var,
                        optional: #optional,
                        binding_style: None,
                        field_codegen: ::core::default::Default::default(),
                    }
                )
            }
        })
        .collect::<Vec<_>>();
    let flatten_entries = flatten
        .iter()
        .map(|(schema_var, _)| quote! { #schema_var })
        .collect::<Vec<_>>();

    let unknown_fields = match record_schema.unknown_fields() {
        UnknownFieldsPolicyIr::Deny => quote! { #schema_crate::UnknownFieldsPolicy::Deny },
        UnknownFieldsPolicyIr::Allow => quote! { #schema_crate::UnknownFieldsPolicy::Allow },
        UnknownFieldsPolicyIr::Schema(_) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "derive BuildSchema emission does not support schema-based unknown_fields policy",
            ));
        }
    };

    let builds = field_builds
        .into_iter()
        .chain(flatten_builds)
        .collect::<Vec<_>>();
    let content = quote! {
        #schema_crate::SchemaNodeContent::Record(#schema_crate::RecordSchema {
            properties: [#(#property_entries),*].into_iter().collect(),
            flatten: vec![#(#flatten_entries),*],
            unknown_fields: #unknown_fields,
        })
    };
    Ok((builds, content))
}
