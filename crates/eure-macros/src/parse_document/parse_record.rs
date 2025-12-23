#[cfg(test)]
mod tests;

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataStruct, Fields};

use crate::attrs::{DefaultValue, FieldAttrs};
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

            // Validate incompatible attribute combinations
            if attrs.flatten && attrs.flatten_ext {
                panic!(
                    "cannot use both #[eure(flatten)] and #[eure(flatten_ext)] on the same field"
                );
            }
            if attrs.flatten && attrs.ext {
                panic!("cannot use both #[eure(flatten)] and #[eure(ext)] on the same field");
            }
            if attrs.ext && attrs.flatten_ext {
                panic!("cannot use both #[eure(ext)] and #[eure(flatten_ext)] on the same field");
            }
            if attrs.default.is_some() && attrs.flatten {
                panic!(
                    "cannot use #[eure(default)] with #[eure(flatten)] on field `{}`; \
                    flatten parses entire nested types, not optional fields",
                    field_name
                );
            }
            if attrs.default.is_some() && attrs.flatten_ext {
                panic!(
                    "cannot use #[eure(default)] with #[eure(flatten_ext)] on field `{}`; \
                    flatten_ext parses entire nested types, not optional fields",
                    field_name
                );
            }

            if attrs.flatten {
                quote! { #field_name: <#field_ty>::parse(&rec.flatten())? }
            } else if attrs.flatten_ext {
                quote! { #field_name: <#field_ty>::parse(&ctx.flatten_ext())? }
            } else if attrs.ext {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
                generate_ext_field(field_name, field_ty, &field_name_str, &attrs.default)
            } else {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
                generate_record_field(field_name, field_ty, &field_name_str, &attrs.default)
            }
        })
        .collect();

    context.impl_parse_document(quote! {
        let rec = ctx.parse_record()?;
        let value = #ident {
            #(#field_assignments),*
        };
        rec.deny_unknown_fields()?;
        ctx.deny_unknown_extensions()?;
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
                panic!("#[eure(flatten)] cannot be used in #[eure(parse_ext)] context; use #[eure(flatten_ext)] instead");
            }
            if attrs.default.is_some() && attrs.flatten_ext {
                panic!(
                    "cannot use #[eure(default)] with #[eure(flatten_ext)] on field `{}`; \
                    flatten_ext parses entire nested types, not optional fields",
                    field_name
                );
            }

            if attrs.flatten_ext {
                quote! { #field_name: <#field_ty>::parse(&ctx.flatten_ext())? }
            } else {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
                generate_ext_field(field_name, field_ty, &field_name_str, &attrs.default)
            }
        })
        .collect();

    // No need to call deny_unknown_extensions in parse_ext context
    // (the caller is responsible for validation)
    context.impl_parse_document(quote! {
        let value = #ident {
            #(#field_assignments),*
        };
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

fn generate_record_field(
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    field_name_str: &str,
    default: &DefaultValue,
) -> TokenStream {
    match default {
        DefaultValue::None => {
            quote! { #field_name: rec.parse_field(#field_name_str)? }
        }
        DefaultValue::Default => {
            quote! {
                #field_name: rec.parse_field_optional(#field_name_str)?
                    .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
            }
        }
        DefaultValue::Path(path) => {
            quote! {
                #field_name: rec.parse_field_optional(#field_name_str)?
                    .unwrap_or_else(#path)
            }
        }
    }
}

fn generate_ext_field(
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    field_name_str: &str,
    default: &DefaultValue,
) -> TokenStream {
    match default {
        DefaultValue::None => {
            quote! { #field_name: ctx.parse_ext(#field_name_str)? }
        }
        DefaultValue::Default => {
            quote! {
                #field_name: ctx.parse_ext_optional(#field_name_str)?
                    .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
            }
        }
        DefaultValue::Path(path) => {
            quote! {
                #field_name: ctx.parse_ext_optional(#field_name_str)?
                    .unwrap_or_else(#path)
            }
        }
    }
}
