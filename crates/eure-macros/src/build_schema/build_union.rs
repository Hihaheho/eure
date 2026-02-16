//! BuildSchema derive implementation for enums (unions)

use darling::FromVariant;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DataEnum;

use crate::attrs::VariantAttrs;
use crate::context::MacroContext;
use crate::ir::{RenameScope, analyze_common_named_fields};

pub fn generate_union_schema(context: &MacroContext, input: &DataEnum) -> TokenStream {
    let schema_crate = context.schema_crate();

    let variant_schemas: Vec<_> = input
        .variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let variant_attrs =
                VariantAttrs::from_variant(variant).expect("failed to parse variant attributes");

            let variant_name = variant_attrs
                .rename
                .clone()
                .unwrap_or_else(|| context.apply_rename(&variant.ident.to_string()));

            let schema_var = format_ident!("variant_{}_schema", idx);

            let schema_build = match &variant.fields {
                syn::Fields::Unit => {
                    quote! {
                        let #schema_var = ctx.create_node(#schema_crate::SchemaNodeContent::Null);
                    }
                }
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let field_ty = &fields.unnamed[0].ty;
                    quote! {
                        let #schema_var = ctx.build::<#field_ty>();
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    let field_builds: Vec<_> = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(fidx, f)| {
                            let field_ty = &f.ty;
                            let field_var = format_ident!("variant_{}_field_{}", idx, fidx);
                            quote! {
                                let #field_var = ctx.build::<#field_ty>();
                            }
                        })
                        .collect();

                    let field_vars: Vec<_> = (0..fields.unnamed.len())
                        .map(|fidx| format_ident!("variant_{}_field_{}", idx, fidx))
                        .collect();

                    quote! {
                        #(#field_builds)*
                        let #schema_var = ctx.create_node(#schema_crate::SchemaNodeContent::Tuple(
                            #schema_crate::TupleSchema {
                                elements: vec![#(#field_vars),*],
                                binding_style: None,
                            }
                        ));
                    }
                }
                syn::Fields::Named(fields) => {
                    let common_fields =
                        analyze_common_named_fields(context, &fields.named, RenameScope::Field)
                            .expect("failed to analyze variant fields");

                    let field_builds: Vec<_> = common_fields
                        .iter()
                        .enumerate()
                        .map(|(fidx, f)| {
                            let field_ty = &f.ty;
                            let field_var = format_ident!("variant_{}_field_{}", idx, fidx);
                            quote! {
                                let #field_var = ctx.build::<#field_ty>();
                            }
                        })
                        .collect();

                    let property_entries: Vec<_> = common_fields
                        .iter()
                        .enumerate()
                        .map(|(fidx, f)| {
                            let field_name_str = &f.wire_name;
                            let field_var = format_ident!("variant_{}_field_{}", idx, fidx);
                            let is_optional = is_option_type(&f.ty);

                            quote! {
                                (
                                    #field_name_str.to_string(),
                                    #schema_crate::RecordFieldSchema {
                                        schema: #field_var,
                                        optional: #is_optional,
                                        binding_style: None,
                                    }
                                )
                            }
                        })
                        .collect();

                    quote! {
                        #(#field_builds)*
                        let #schema_var = ctx.create_node(#schema_crate::SchemaNodeContent::Record(
                            #schema_crate::RecordSchema {
                                properties: [#(#property_entries),*].into_iter().collect(),
                                flatten: vec![],
                                unknown_fields: #schema_crate::UnknownFieldsPolicy::Deny,
                            }
                        ));
                    }
                }
            };

            (variant_name, schema_var, schema_build)
        })
        .collect();

    let all_builds: Vec<_> = variant_schemas
        .iter()
        .map(|(_, _, build)| build.clone())
        .collect();

    let variant_entries: Vec<_> = variant_schemas
        .iter()
        .map(|(name, schema_var, _)| {
            quote! {
                (#name.to_string(), #schema_var)
            }
        })
        .collect();

    let content = quote! {
        #(#all_builds)*

        #schema_crate::SchemaNodeContent::Union(#schema_crate::UnionSchema {
            variants: [#(#variant_entries),*].into_iter().collect(),
            unambiguous: Default::default(),
            repr: ::eure_document::data_model::VariantRepr::default(),
            repr_explicit: false,
            deny_untagged: Default::default(),
        })
    };

    context.impl_build_schema(content)
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}
