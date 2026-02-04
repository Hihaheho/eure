#[cfg(test)]
mod tests;

use darling::{FromField, FromVariant};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{DataEnum, Fields, Variant};

use crate::attrs::{FieldAttrs, VariantAttrs, extract_variant_attr_spans};
use crate::{config::MacroConfig, context::MacroContext};

use super::parse_record::{generate_ext_field, generate_record_field};

pub fn generate_union_parser(context: &MacroContext, input: &DataEnum) -> syn::Result<TokenStream> {
    let MacroConfig { document_crate, .. } = &context.config;
    let DataEnum { variants, .. } = input;
    let variant_repr = variant_repr(document_crate);
    let mut variant_tokens = Vec::new();
    for variant in variants {
        variant_tokens.push(generate_variant(context, variant)?);
    }
    Ok(context.impl_from_eure(quote! {
        ctx.parse_union(#variant_repr)?
            #(#variant_tokens)*
            .parse()
    }))
}

fn variant_repr(document_crate: &TokenStream) -> TokenStream {
    // TODO: Support custom variant repr via attributes
    quote! { #document_crate::data_model::VariantRepr::default() }
}

fn generate_variant(context: &MacroContext, variant: &Variant) -> syn::Result<TokenStream> {
    let ident = context.ident();
    let MacroConfig { document_crate, .. } = &context.config;
    let variant_ident = &variant.ident;
    let variant_attrs =
        VariantAttrs::from_variant(variant).expect("failed to parse variant attributes");
    let variant_attr_spans = extract_variant_attr_spans(variant);
    let variant_name = variant_attrs
        .rename
        .clone()
        .unwrap_or_else(|| context.apply_rename(&variant_ident.to_string()));

    // Validate allow_unknown_fields is only on struct variants
    if variant_attrs.allow_unknown_fields && !matches!(&variant.fields, Fields::Named(_)) {
        let span = variant_attr_spans
            .get("allow_unknown_fields")
            .copied()
            .unwrap_or_else(|| variant.span());
        return Err(syn::Error::new(
            span,
            "#[eure(allow_unknown_fields)] is only valid on struct variants with named fields",
        ));
    }

    match &variant.fields {
        Fields::Unit => Ok(generate_unit_variant(
            context,
            ident,
            &variant_name,
            variant_ident,
        )),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(generate_newtype_variant(
            ident,
            &variant_name,
            variant_ident,
            &fields.unnamed[0].ty,
        )),
        Fields::Unnamed(fields) => Ok(generate_tuple_variant(
            ident,
            &variant_name,
            variant_ident,
            &fields.unnamed,
        )),
        Fields::Named(fields) => Ok(generate_struct_variant(
            context,
            ident,
            document_crate,
            &variant_name,
            variant_ident,
            &fields.named,
            &variant_attrs,
        )),
    }
}

fn generate_unit_variant(
    context: &MacroContext,
    enum_ident: &syn::Ident,
    variant_name: &str,
    variant_ident: &syn::Ident,
) -> TokenStream {
    let variant_parser = context.VariantLiteralParser(
        quote!(#variant_name),
        quote!(|_| #enum_ident::#variant_ident),
    );
    quote! {
        .variant(#variant_name, #variant_parser)
    }
}

fn generate_newtype_variant(
    enum_ident: &syn::Ident,
    variant_name: &str,
    variant_ident: &syn::Ident,
    field_ty: &syn::Type,
) -> TokenStream {
    quote! {
        .parse_variant::<#field_ty>(#variant_name, |field_0| Ok(#enum_ident::#variant_ident(field_0)))
    }
}

fn generate_tuple_variant(
    enum_ident: &syn::Ident,
    variant_name: &str,
    variant_ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_names: Vec<_> = (0..fields.len())
        .map(|i| format_ident!("field_{}", i))
        .collect();

    quote! {
        .parse_variant::<(#(#field_types,)*)>(#variant_name, |(#(#field_names,)*)| Ok(#enum_ident::#variant_ident(#(#field_names),*)))
    }
}

fn generate_struct_variant(
    context: &MacroContext,
    enum_ident: &syn::Ident,
    document_crate: &TokenStream,
    variant_name: &str,
    variant_ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    variant_attrs: &VariantAttrs,
) -> TokenStream {
    // Check if there are any "regular" record fields (not flatten, ext, or flatten_ext)
    let has_record = fields.iter().any(|f| {
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        !attrs.flatten && !attrs.ext && !attrs.flatten_ext
    });

    let field_assignments: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().expect("struct fields must have names");
            let field_ty = &f.ty;
            let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");

            if attrs.flatten {
                // Use rec.flatten() when we have a record, ctx.flatten() otherwise
                if has_record {
                    quote! { #field_name: <#field_ty>::parse(&rec.flatten())? }
                } else {
                    quote! { #field_name: <#field_ty>::parse(&ctx.flatten())? }
                }
            } else if attrs.flatten_ext {
                quote! { #field_name: <#field_ty>::parse(&ctx.flatten_ext())? }
            } else if attrs.ext {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_field_rename(&field_name.to_string()));
                generate_ext_field(
                    field_name,
                    field_ty,
                    &field_name_str,
                    &attrs.default,
                    attrs.via.as_ref(),
                )
            } else {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_field_rename(&field_name.to_string()));
                generate_record_field(
                    field_name,
                    field_ty,
                    &field_name_str,
                    &attrs.default,
                    attrs.via.as_ref(),
                )
            }
        })
        .collect();

    if has_record {
        let unknown_fields_check = if variant_attrs.allow_unknown_fields {
            quote! { rec.allow_unknown_fields()?; }
        } else {
            quote! { rec.deny_unknown_fields()?; }
        };

        quote! {
            .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value = #enum_ident::#variant_ident {
                    #(#field_assignments),*
                };
                #unknown_fields_check
                Ok(value)
            })
        }
    } else {
        quote! {
            .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'_>| {
                let value = #enum_ident::#variant_ident {
                    #(#field_assignments),*
                };
                ctx.deny_unknown_extensions()?;
                Ok(value)
            })
        }
    }
}
