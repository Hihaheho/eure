#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, Fields, Variant};

use crate::{config::MacroConfig, context::MacroContext};

pub fn generate_union_parser(context: &MacroContext, input: &DataEnum) -> TokenStream {
    let ident = context.ident();
    let MacroConfig { document_crate, .. } = &context.config;
    let DataEnum { variants, .. } = input;
    let impl_generics = context.impl_generics();
    let for_generics = context.for_generics();
    let variant_repr = variant_repr(document_crate);
    let variants = variants
        .iter()
        .map(|variant| generate_variant(context, variant));
    let parse_document = context.config.ParseDocument();
    let parse_error = context.config.ParseError();
    let parse_context = context.config.ParseContext();
    quote! {
        impl<'doc, #(#impl_generics),*> #parse_document<'doc> for #ident<#(#for_generics),*> {
            type Error = #parse_error;

            fn parse(ctx: &#parse_context<'doc>) -> Result<Self, Self::Error> {
                ctx.parse_union(#variant_repr)?
                    #(#variants)*
                    .parse()
            }
        }
    }
}

fn variant_repr(document_crate: &TokenStream) -> TokenStream {
    // TODO: Support custom variant repr via attributes
    quote! { #document_crate::data_model::VariantRepr::default() }
}

fn generate_variant(context: &MacroContext, variant: &Variant) -> TokenStream {
    let ident = context.ident();
    let MacroConfig { document_crate, .. } = &context.config;
    let variant_ident = &variant.ident;
    let variant_name = variant_ident.to_string();

    match &variant.fields {
        Fields::Unit => generate_unit_variant(ident, &variant_name, variant_ident),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            generate_newtype_variant(ident, &variant_name, variant_ident, &fields.unnamed[0].ty)
        }
        Fields::Unnamed(fields) => {
            generate_tuple_variant(ident, &variant_name, variant_ident, &fields.unnamed)
        }
        Fields::Named(fields) => generate_struct_variant(
            ident,
            document_crate,
            &variant_name,
            variant_ident,
            &fields.named,
        ),
    }
}

fn generate_unit_variant(
    enum_ident: &syn::Ident,
    variant_name: &str,
    variant_ident: &syn::Ident,
) -> TokenStream {
    quote! {
        .parse_variant::<()>(#variant_name, |_| Ok(#enum_ident::#variant_ident))
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
    enum_ident: &syn::Ident,
    document_crate: &TokenStream,
    variant_name: &str,
    variant_ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let field_names: Vec<_> = fields
        .iter()
        .map(|f| f.ident.as_ref().expect("struct fields must have names"))
        .collect();
    let field_name_strs: Vec<_> = field_names.iter().map(|n| n.to_string()).collect();

    quote! {
        .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'_>| {
            let mut rec = ctx.parse_record()?;
            let value = #enum_ident::#variant_ident {
                #(#field_names: rec.parse_field(#field_name_strs)?),*
            };
            rec.deny_unknown_fields()?;
            Ok(value)
        })
    }
}
