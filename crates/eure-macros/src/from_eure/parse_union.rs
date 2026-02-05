#[cfg(test)]
mod tests;

use darling::{FromField, FromVariant};
use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataEnum, Fields, Variant};

use crate::attrs::{FieldAttrs, VariantAttrs, extract_variant_attr_spans};
use crate::config::MacroConfig;
use crate::context::MacroContext;
use crate::util::respan;

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
    // Use target_type() which returns the type to construct (proxy target or self)
    let target_type = respan(context.target_type(), variant.ident.span());
    let opaque_target = context.opaque_target();
    let opaque_span = context.opaque_error_span();
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

    let document_crate = &context.config.document_crate;
    match &variant.fields {
        Fields::Unit => Ok(generate_unit_variant(
            context,
            &target_type,
            opaque_target,
            opaque_span,
            &variant_name,
            variant_ident,
        )),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(generate_newtype_variant(
            document_crate,
            &target_type,
            opaque_target,
            opaque_span,
            &variant_name,
            variant_ident,
            &fields.unnamed[0],
        )),
        Fields::Unnamed(fields) => Ok(generate_tuple_variant(
            document_crate,
            &target_type,
            opaque_target,
            opaque_span,
            &variant_name,
            variant_ident,
            &fields.unnamed,
        )),
        Fields::Named(fields) => Ok(generate_struct_variant(
            context,
            opaque_span,
            &variant_name,
            variant_ident,
            &fields.named,
            &variant_attrs,
        )),
    }
}

fn generate_unit_variant(
    context: &MacroContext,
    target_type: &TokenStream,
    opaque_target: Option<&syn::Type>,
    opaque_span: Span,
    variant_name: &str,
    variant_ident: &syn::Ident,
) -> TokenStream {
    // For opaque: construct target_type then convert via .into()
    // For proxy/normal: construct target_type directly
    let mapper = if opaque_target.is_some() {
        quote_spanned! {opaque_span=> |_| #target_type::#variant_ident.into()}
    } else {
        quote!(|_| #target_type::#variant_ident)
    };
    let variant_parser = context.VariantLiteralParser(quote!(#variant_name), mapper);
    quote! {
        .variant(#variant_name, #variant_parser)
    }
}

fn generate_newtype_variant(
    document_crate: &TokenStream,
    target_type: &TokenStream,
    opaque_target: Option<&syn::Type>,
    opaque_span: Span,
    variant_name: &str,
    variant_ident: &syn::Ident,
    field: &syn::Field,
) -> TokenStream {
    let field_ty = &field.ty;
    let field_span = field_ty.span();
    let attrs = FieldAttrs::from_field(field).expect("failed to parse field attributes");
    // For opaque: construct target_type then convert via .into()
    // For proxy/normal: construct target_type directly
    if let Some(via_type) = attrs.via.as_ref() {
        let body = if opaque_target.is_some() {
            quote_spanned! {opaque_span=>
                Ok(#target_type::#variant_ident(field_0).into())
            }
        } else {
            quote_spanned! {field_span=>
                Ok(#target_type::#variant_ident(field_0))
            }
        };
        quote_spanned! {via_type.span()=>
            .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                let field_0 = ctx.parse_via::<#via_type, #field_ty>()?;
                #body
            })
        }
    } else if opaque_target.is_some() {
        quote_spanned! {opaque_span=>
            .parse_variant::<#field_ty>(#variant_name, |field_0| Ok(#target_type::#variant_ident(field_0).into()))
        }
    } else {
        quote_spanned! {field_span=>
            .parse_variant::<#field_ty>(#variant_name, |field_0| Ok(#target_type::#variant_ident(field_0)))
        }
    }
}

fn generate_tuple_variant(
    document_crate: &TokenStream,
    target_type: &TokenStream,
    opaque_target: Option<&syn::Type>,
    opaque_span: Span,
    variant_name: &str,
    variant_ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let tuple_span = fields.span();
    let mut field_types = Vec::new();
    let mut field_names = Vec::new();
    let mut field_args = Vec::new();
    let mut field_parsers = Vec::new();
    let mut has_via = false;

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;
        let field_name = format_ident!("field_{}", i, span = f.ty.span());
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        if attrs.via.is_some() {
            has_via = true;
        }
        let parser = if let Some(via_type) = attrs.via.as_ref() {
            quote_spanned! {via_type.span()=>
                let #field_name = tuple.next_via::<#via_type, #field_ty>()?;
            }
        } else {
            quote_spanned! {field_ty.span()=>
                let #field_name = tuple.next::<#field_ty>()?;
            }
        };
        field_types.push(field_ty);
        field_names.push(field_name.clone());
        field_args.push(quote_spanned! {field_ty.span()=> #field_name});
        field_parsers.push(parser);
    }

    let body = if opaque_target.is_some() {
        quote_spanned! {opaque_span=>
            let value: #target_type = #target_type::#variant_ident(#(#field_args),*);
            Ok(value.into())
        }
    } else {
        quote_spanned! {tuple_span=>
            let value: #target_type = #target_type::#variant_ident(#(#field_args),*);
            Ok(value)
        }
    };

    if has_via {
        let tuple_len = Literal::usize_unsuffixed(fields.len());
        if opaque_target.is_some() {
            quote_spanned! {opaque_span=>
                .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                    let mut tuple = ctx.parse_tuple()?;
                    tuple.expect_len(#tuple_len)?;
                    #(#field_parsers)*
                    #body
                })
            }
        } else {
            quote_spanned! {tuple_span=>
                .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                    let mut tuple = ctx.parse_tuple()?;
                    tuple.expect_len(#tuple_len)?;
                    #(#field_parsers)*
                    #body
                })
            }
        }
    } else if opaque_target.is_some() {
        quote_spanned! {opaque_span=>
            .parse_variant::<(#(#field_types,)*)>(#variant_name, |(#(#field_names,)*)| {
                #body
            })
        }
    } else {
        quote_spanned! {tuple_span=>
            .parse_variant::<(#(#field_types,)*)>(#variant_name, |(#(#field_names,)*)| {
                #body
            })
        }
    }
}

fn generate_struct_variant(
    context: &MacroContext,
    opaque_span: Span,
    variant_name: &str,
    variant_ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    variant_attrs: &VariantAttrs,
) -> TokenStream {
    let target_type = respan(context.target_type(), fields.span());
    let opaque_target = context.opaque_target();
    let document_crate = &context.config.document_crate;
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

    // For opaque: construct target_type then convert via .into()
    // For proxy/normal: construct target_type directly
    let return_value = if opaque_target.is_some() {
        quote_spanned! {opaque_span=> Ok(value.into()) }
    } else {
        quote! { Ok(value) }
    };

    if has_record {
        let unknown_fields_check = if variant_attrs.allow_unknown_fields {
            quote! { rec.allow_unknown_fields()?; }
        } else {
            quote! { rec.deny_unknown_fields()?; }
        };

        quote! {
            .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                let mut rec = ctx.parse_record()?;
                let value = #target_type::#variant_ident {
                    #(#field_assignments),*
                };
                #unknown_fields_check
                #return_value
            })
        }
    } else {
        quote! {
            .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                let value = #target_type::#variant_ident {
                    #(#field_assignments),*
                };
                ctx.deny_unknown_extensions()?;
                #return_value
            })
        }
    }
}
