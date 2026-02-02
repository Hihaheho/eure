#[cfg(test)]
mod tests;

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataStruct, Fields};

use crate::attrs::{DefaultValue, FieldAttrs};
use crate::context::MacroContext;

pub fn generate_record_parser(context: &MacroContext, input: &DataStruct) -> TokenStream {
    match &input.fields {
        Fields::Named(fields) => generate_named_struct(context, &fields.named),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            generate_newtype_struct(context, &fields.unnamed[0].ty)
        }
        Fields::Unnamed(fields) => generate_tuple_struct(context, &fields.unnamed),
        Fields::Unit => generate_unit_struct(context),
    }
}

fn generate_named_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    if context.config.parse_ext {
        generate_named_struct_from_ext(context, fields)
    } else {
        generate_named_struct_from_record(context, fields)
    }
}

fn generate_named_struct_from_record(
    context: &MacroContext,
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
            if attrs.via.is_some() && (attrs.flatten || attrs.flatten_ext) {
                panic!(
                    "cannot use #[eure(via = \"...\")] with #[eure(flatten)] or #[eure(flatten_ext)] on field `{}`",
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
                generate_ext_field(field_name, field_ty, &field_name_str, &attrs.default, attrs.via.as_ref())
            } else {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
                generate_record_field(field_name, field_ty, &field_name_str, &attrs.default, attrs.via.as_ref())
            }
        })
        .collect();

    let unknown_fields_check = if context.config.allow_unknown_fields {
        quote! { rec.allow_unknown_fields()?; }
    } else {
        quote! { rec.deny_unknown_fields()?; }
    };

    let unknown_extensions_check = if context.config.allow_unknown_extensions {
        quote! {}
    } else {
        quote! { ctx.deny_unknown_extensions()?; }
    };

    // Use target_type() which returns the type to construct (proxy target or self)
    let target_type = context.target_type();

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        context.impl_from_eure(quote! {
            let rec = ctx.parse_record()?;
            let value = #target_type {
                #(#field_assignments),*
            };
            #unknown_fields_check
            #unknown_extensions_check
            let value: #opaque_target = value.into();
            Ok(value)
        })
    } else {
        context.impl_from_eure(quote! {
            let rec = ctx.parse_record()?;
            let value = #target_type {
                #(#field_assignments),*
            };
            #unknown_fields_check
            #unknown_extensions_check
            Ok(value)
        })
    }
}

fn generate_named_struct_from_ext(
    context: &MacroContext,
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
            if attrs.via.is_some() && attrs.flatten_ext {
                panic!(
                    "cannot use #[eure(via = \"...\")] with #[eure(flatten_ext)] on field `{}`",
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
                generate_ext_field(field_name, field_ty, &field_name_str, &attrs.default, attrs.via.as_ref())
            }
        })
        .collect();

    // No need to call deny_unknown_extensions in parse_ext context
    // (the caller is responsible for validation)
    // Use target_type() which returns the type to construct (proxy target or self)
    let target_type = context.target_type();

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        context.impl_from_eure(quote! {
            let value = #target_type {
                #(#field_assignments),*
            };
            let value: #opaque_target = value.into();
            Ok(value)
        })
    } else {
        context.impl_from_eure(quote! {
            let value = #target_type {
                #(#field_assignments),*
            };
            Ok(value)
        })
    }
}

fn generate_unit_struct(context: &MacroContext) -> TokenStream {
    let target_type = context.target_type();

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        context.impl_from_eure(quote! {
            ctx.parse::<()>()?;
            let value: #opaque_target = #target_type.into();
            Ok(value)
        })
    } else {
        context.impl_from_eure(quote! {
            ctx.parse::<()>()?;
            Ok(#target_type)
        })
    }
}

fn generate_tuple_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let target_type = context.target_type();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_names: Vec<_> = (0..fields.len())
        .map(|i| format_ident!("field_{}", i))
        .collect();

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        context.impl_from_eure(quote! {
            let (#(#field_names,)*) = ctx.parse::<(#(#field_types,)*)>()?;
            let value: #opaque_target = #target_type(#(#field_names),*).into();
            Ok(value)
        })
    } else {
        context.impl_from_eure(quote! {
            let (#(#field_names,)*) = ctx.parse::<(#(#field_types,)*)>()?;
            Ok(#target_type(#(#field_names),*))
        })
    }
}

fn generate_newtype_struct(context: &MacroContext, field_ty: &syn::Type) -> TokenStream {
    let target_type = context.target_type();

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        context.impl_from_eure(quote! {
            let field_0 = ctx.parse::<#field_ty>()?;
            let value: #opaque_target = #target_type(field_0).into();
            Ok(value)
        })
    } else {
        context.impl_from_eure(quote! {
            let field_0 = ctx.parse::<#field_ty>()?;
            Ok(#target_type(field_0))
        })
    }
}

pub(super) fn generate_record_field(
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    field_name_str: &str,
    default: &DefaultValue,
    via: Option<&syn::Type>,
) -> TokenStream {
    let span = field_ty.span();

    // When via is specified, we use parse_field_with to call the marker type's parse method
    if let Some(via_type) = via {
        return match default {
            DefaultValue::None => {
                quote_spanned! {span=>
                    #field_name: rec.parse_field_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                }
            }
            DefaultValue::Default => {
                quote_spanned! {span=>
                    #field_name: rec.parse_field_optional_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
                }
            }
            DefaultValue::Path(path) => {
                quote_spanned! {span=>
                    #field_name: rec.parse_field_optional_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(#path)
                }
            }
        };
    }

    match default {
        DefaultValue::None => {
            quote_spanned! {span=> #field_name: rec.parse_field::<#field_ty>(#field_name_str)? }
        }
        DefaultValue::Default => {
            quote_spanned! {span=>
                #field_name: rec.parse_field_optional::<#field_ty>(#field_name_str)?
                    .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
            }
        }
        DefaultValue::Path(path) => {
            quote_spanned! {span=>
                #field_name: rec.parse_field_optional::<#field_ty>(#field_name_str)?
                    .unwrap_or_else(#path)
            }
        }
    }
}

pub(super) fn generate_ext_field(
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    field_name_str: &str,
    default: &DefaultValue,
    via: Option<&syn::Type>,
) -> TokenStream {
    let span = field_ty.span();

    // When via is specified, we use parse_ext_with to call the marker type's parse method
    if let Some(via_type) = via {
        return match default {
            DefaultValue::None => {
                quote_spanned! {span=>
                    #field_name: ctx.parse_ext_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                }
            }
            DefaultValue::Default => {
                quote_spanned! {span=>
                    #field_name: ctx.parse_ext_optional_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
                }
            }
            DefaultValue::Path(path) => {
                quote_spanned! {span=>
                    #field_name: ctx.parse_ext_optional_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(#path)
                }
            }
        };
    }

    match default {
        DefaultValue::None => {
            quote_spanned! {span=> #field_name: ctx.parse_ext::<#field_ty>(#field_name_str)? }
        }
        DefaultValue::Default => {
            quote_spanned! {span=>
                #field_name: ctx.parse_ext_optional::<#field_ty>(#field_name_str)?
                    .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
            }
        }
        DefaultValue::Path(path) => {
            quote_spanned! {span=>
                #field_name: ctx.parse_ext_optional::<#field_ty>(#field_name_str)?
                    .unwrap_or_else(#path)
            }
        }
    }
}
