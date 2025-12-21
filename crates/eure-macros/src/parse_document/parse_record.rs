#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataStruct, Fields};

use crate::context::MacroContext;

pub fn generate_record_parser(context: &MacroContext, input: &DataStruct) -> TokenStream {
    let ident = context.ident();

    match &input.fields {
        Fields::Named(fields) => generate_named_struct(context, ident, &fields.named),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            generate_newtype_struct(context, ident, &fields.unnamed[0].ty)
        }
        Fields::Unnamed(fields) => generate_tuple_struct(context, ident, &fields.unnamed),
        Fields::Unit => generate_unit_struct(context, ident),
    }
}

fn generate_named_struct(
    context: &MacroContext,
    ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    if context.config.parse_ext {
        generate_named_struct_from_ext(context, ident, fields)
    } else {
        generate_named_struct_from_record(context, ident, fields)
    }
}

fn generate_named_struct_from_record(
    context: &MacroContext,
    ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let field_names: Vec<_> = fields
        .iter()
        .map(|f| f.ident.as_ref().expect("named fields must have names"))
        .collect();
    let field_name_strs: Vec<_> = field_names
        .iter()
        .map(|n| context.apply_rename(&n.to_string()))
        .collect();

    context.impl_parse_document(quote! {
        let mut rec = ctx.parse_record()?;
        let value = #ident {
            #(#field_names: rec.parse_field(#field_name_strs)?),*
        };
        rec.deny_unknown_fields()?;
        Ok(value)
    })
}

fn generate_named_struct_from_ext(
    context: &MacroContext,
    ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let field_names: Vec<_> = fields
        .iter()
        .map(|f| f.ident.as_ref().expect("named fields must have names"))
        .collect();
    let field_name_strs: Vec<_> = field_names
        .iter()
        .map(|n| context.apply_rename(&n.to_string()))
        .collect();

    context.impl_parse_document(quote! {
        let mut ext = ctx.parse_extension();
        let value = #ident {
            #(#field_names: ext.parse_ext(#field_name_strs)?),*
        };
        ext.allow_unknown_extensions();
        Ok(value)
    })
}

fn generate_unit_struct(context: &MacroContext, ident: &syn::Ident) -> TokenStream {
    context.impl_parse_document(quote! {
        ctx.parse::<()>()?;
        Ok(#ident)
    })
}

fn generate_tuple_struct(
    context: &MacroContext,
    ident: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_names: Vec<_> = (0..fields.len())
        .map(|i| format_ident!("field_{}", i))
        .collect();

    context.impl_parse_document(quote! {
        let (#(#field_names,)*) = ctx.parse::<(#(#field_types,)*)>()?;
        Ok(#ident(#(#field_names),*))
    })
}

fn generate_newtype_struct(
    context: &MacroContext,
    ident: &syn::Ident,
    field_ty: &syn::Type,
) -> TokenStream {
    context.impl_parse_document(quote! {
        let field_0 = ctx.parse::<#field_ty>()?;
        Ok(#ident(field_0))
    })
}
