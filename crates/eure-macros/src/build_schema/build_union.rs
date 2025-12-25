//! BuildSchema derive implementation for enums (unions)

use darling::FromField;
use darling::FromVariant;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DataEnum;

use crate::attrs::{FieldAttrs, VariantAttrs};
use crate::context::MacroContext;

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
                    // Unit variant -> null schema
                    quote! {
                        let #schema_var = ctx.create_node(#schema_crate::SchemaNodeContent::Null);
                    }
                }
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    // Newtype variant -> delegate to inner type
                    let field_ty = &fields.unnamed[0].ty;
                    quote! {
                        let #schema_var = ctx.build::<#field_ty>();
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    // Tuple variant -> tuple schema
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
                    // Struct variant -> record schema
                    let field_builds: Vec<_> = fields
                        .named
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

                    let property_entries: Vec<_> = fields
                        .named
                        .iter()
                        .enumerate()
                        .map(|(fidx, f)| {
                            let field_name = f.ident.as_ref().unwrap();
                            let field_attrs = FieldAttrs::from_field(f)
                                .expect("failed to parse field attributes");
                            let field_name_str = field_attrs.rename.clone().unwrap_or_else(|| {
                                context.apply_field_rename(&field_name.to_string())
                            });
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
                                unknown_fields: #schema_crate::UnknownFieldsPolicy::Deny,
                            }
                        ));
                    }
                }
            };

            (variant_name, schema_var, schema_build)
        })
        .collect();

    // Collect all schema builds
    let all_builds: Vec<_> = variant_schemas
        .iter()
        .map(|(_, _, build)| build.clone())
        .collect();

    // Create the variants BTreeMap entries
    let variant_entries: Vec<_> = variant_schemas
        .iter()
        .map(|(name, schema_var, _)| {
            quote! {
                (#name.to_string(), #schema_var)
            }
        })
        .collect();

    let content = quote! {
        use std::collections::{BTreeMap, HashSet};

        #(#all_builds)*

        #schema_crate::SchemaNodeContent::Union(#schema_crate::UnionSchema {
            variants: BTreeMap::from([#(#variant_entries),*]),
            unambiguous: HashSet::new(),
            repr: ::eure_document::data_model::VariantRepr::default(),
            deny_untagged: HashSet::new(),
        })
    };

    context.impl_build_schema(content)
}

/// Check if a type is Option<T>
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}
