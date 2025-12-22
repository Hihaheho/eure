#[cfg(test)]
mod tests;

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataStruct, Fields};

use crate::attrs::FieldAttrs;
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
    let field_assignments: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().expect("named fields must have names");
            let field_ty = &f.ty;
            let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");

            if attrs.flatten {
                quote! { #field_name: #field_ty::parse(&rec.flatten())? }
            } else {
                let field_name_str = context.apply_rename(&field_name.to_string());
                quote! { #field_name: rec.parse_field(#field_name_str)? }
            }
        })
        .collect();

    context.impl_parse_document(quote! {
        let mut rec = ctx.parse_record()?;
        let value = #ident {
            #(#field_assignments),*
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
    let field_assignments: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().expect("named fields must have names");
            let field_ty = &f.ty;
            let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");

            if attrs.flatten {
                quote! { #field_name: #field_ty::parse(&ext.flatten())? }
            } else {
                let field_name_str = context.apply_rename(&field_name.to_string());
                quote! { #field_name: ext.parse_ext(#field_name_str)? }
            }
        })
        .collect();

    context.impl_parse_document(quote! {
        let mut ext = ctx.parse_extension();
        let value = #ident {
            #(#field_assignments),*
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
