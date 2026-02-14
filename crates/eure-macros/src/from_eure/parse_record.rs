#[cfg(test)]
mod tests;

use darling::FromField;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataStruct, Fields};

use crate::attrs::{DefaultValue, FieldAttrs, extract_eure_attr_spans};
use crate::context::MacroContext;
use crate::util::respan;

pub fn generate_record_parser(
    context: &MacroContext,
    input: &DataStruct,
) -> syn::Result<TokenStream> {
    match &input.fields {
        Fields::Named(fields) => generate_named_struct(context, &fields.named),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            Ok(generate_newtype_struct(context, &fields.unnamed[0]))
        }
        Fields::Unnamed(fields) => Ok(generate_tuple_struct(context, &fields.unnamed)),
        Fields::Unit => Ok(generate_unit_struct(context)),
    }
}

fn generate_named_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> syn::Result<TokenStream> {
    if context.config.parse_ext {
        generate_named_struct_from_ext(context, fields)
    } else {
        generate_named_struct_from_record(context, fields)
    }
}

fn generate_named_struct_from_record(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> syn::Result<TokenStream> {
    let target_span = fields.span();
    let has_record = fields.iter().any(|f| {
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        !attrs.flatten && !attrs.ext && !attrs.flatten_ext
    });
    let mut field_assignments = Vec::new();
    for f in fields {
        let field_name = f.ident.as_ref().expect("named fields must have names");
        let field_ty = &f.ty;
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        let spans = extract_eure_attr_spans(&f.attrs);

        // Validate incompatible attribute combinations
        if attrs.flatten && attrs.flatten_ext {
            let span = spans.get("flatten").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                "cannot use both #[eure(flatten)] and #[eure(flatten_ext)] on the same field",
            ));
        }
        if attrs.flatten && attrs.ext {
            let span = spans.get("flatten").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                "cannot use both #[eure(flatten)] and #[eure(ext)] on the same field",
            ));
        }
        if attrs.ext && attrs.flatten_ext {
            let span = spans.get("ext").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                "cannot use both #[eure(ext)] and #[eure(flatten_ext)] on the same field",
            ));
        }
        if attrs.default.is_some() && attrs.flatten {
            let span = spans.get("default").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "cannot use #[eure(default)] with #[eure(flatten)] on field `{}`; \
                    flatten parses entire nested types, not optional fields",
                    field_name
                ),
            ));
        }
        if attrs.default.is_some() && attrs.flatten_ext {
            let span = spans.get("default").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "cannot use #[eure(default)] with #[eure(flatten_ext)] on field `{}`; \
                    flatten_ext parses entire nested types, not optional fields",
                    field_name
                ),
            ));
        }
        if attrs.via.is_some() && (attrs.flatten || attrs.flatten_ext) {
            let span = spans.get("via").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "cannot use #[eure(via = \"...\")] with #[eure(flatten)] or #[eure(flatten_ext)] on field `{}`",
                    field_name
                ),
            ));
        }

        let assignment = if attrs.flatten {
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
                .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
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
                .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
            generate_record_field(
                field_name,
                field_ty,
                &field_name_str,
                &attrs.default,
                attrs.via.as_ref(),
            )
        };
        field_assignments.push(assignment);
    }

    let unknown_fields_check = if has_record {
        if context.config.allow_unknown_fields {
            quote! { rec.allow_unknown_fields()?; }
        } else {
            quote! { rec.deny_unknown_fields()?; }
        }
    } else if context.config.allow_unknown_fields {
        quote! {
            if let Ok(rec) = ctx.parse_record() {
                rec.allow_unknown_fields()?;
            }
        }
    } else {
        quote! {
            if let Ok(rec) = ctx.parse_record() {
                rec.deny_unknown_fields()?;
            }
        }
    };

    let unknown_extensions_check = if context.config.allow_unknown_extensions {
        quote! {}
    } else {
        quote! { ctx.deny_unknown_extensions()?; }
    };

    // Use target_type() which returns the type to construct (proxy target or self)
    let target_type = respan(context.target_type(), target_span);

    // For opaque proxy, we need to convert via .into().
    // Note: `ctx.parse_record()?` is only emitted when `has_record` is true.
    // In content mode (`has_record == false`) we intentionally avoid forcing record
    // parsing so flattened non-record targets (e.g. Vec<T>, NodeId) can parse from
    // `ctx.flatten()`, while unknown-field checks remain best-effort via `if let Ok(rec) = ...`.
    Ok(if let Some(opaque_target) = context.opaque_target() {
        let opaque_span = context.opaque_error_span();
        let opaque_target = quote_spanned! {opaque_span=> #opaque_target};
        let into_value = quote_spanned! {opaque_span=>
            let value: #opaque_target = value.into();
        };
        if has_record {
            context.impl_from_eure(quote! {
                let rec = ctx.parse_record()?;
                let value = #target_type {
                    #(#field_assignments),*
                };
                #unknown_fields_check
                #unknown_extensions_check
                #into_value
                Ok(value)
            })
        } else {
            context.impl_from_eure(quote! {
                let value = #target_type {
                    #(#field_assignments),*
                };
                #unknown_fields_check
                #unknown_extensions_check
                #into_value
                Ok(value)
            })
        }
    } else if has_record {
        context.impl_from_eure(quote! {
            let rec = ctx.parse_record()?;
            let value = #target_type {
                #(#field_assignments),*
            };
            #unknown_fields_check
            #unknown_extensions_check
            Ok(value)
        })
    } else {
        context.impl_from_eure(quote! {
            let value = #target_type {
                #(#field_assignments),*
            };
            #unknown_fields_check
            #unknown_extensions_check
            Ok(value)
        })
    })
}

fn generate_named_struct_from_ext(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> syn::Result<TokenStream> {
    let target_span = fields.span();
    let mut field_assignments = Vec::new();
    for f in fields {
        let field_name = f.ident.as_ref().expect("named fields must have names");
        let field_ty = &f.ty;
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        let spans = extract_eure_attr_spans(&f.attrs);

        if attrs.flatten {
            let span = spans.get("flatten").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                "#[eure(flatten)] cannot be used in #[eure(parse_ext)] context; use #[eure(flatten_ext)] instead",
            ));
        }
        if attrs.default.is_some() && attrs.flatten_ext {
            let span = spans.get("default").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "cannot use #[eure(default)] with #[eure(flatten_ext)] on field `{}`; \
                    flatten_ext parses entire nested types, not optional fields",
                    field_name
                ),
            ));
        }
        if attrs.via.is_some() && attrs.flatten_ext {
            let span = spans.get("via").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "cannot use #[eure(via = \"...\")] with #[eure(flatten_ext)] on field `{}`",
                    field_name
                ),
            ));
        }

        let assignment = if attrs.flatten_ext {
            quote! { #field_name: <#field_ty>::parse(&ctx.flatten_ext())? }
        } else {
            let field_name_str = attrs
                .rename
                .clone()
                .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
            generate_ext_field(
                field_name,
                field_ty,
                &field_name_str,
                &attrs.default,
                attrs.via.as_ref(),
            )
        };
        field_assignments.push(assignment);
    }

    // No need to call deny_unknown_extensions in parse_ext context
    // (the caller is responsible for validation)
    // Use target_type() which returns the type to construct (proxy target or self)
    let target_type = respan(context.target_type(), target_span);

    // For opaque proxy, we need to convert via .into()
    Ok(if let Some(opaque_target) = context.opaque_target() {
        let opaque_span = context.opaque_error_span();
        let opaque_target = quote_spanned! {opaque_span=> #opaque_target};
        let into_value = quote_spanned! {opaque_span=>
            let value: #opaque_target = value.into();
        };
        context.impl_from_eure(quote! {
            let value = #target_type {
                #(#field_assignments),*
            };
            #into_value
            Ok(value)
        })
    } else {
        context.impl_from_eure(quote! {
            let value = #target_type {
                #(#field_assignments),*
            };
            Ok(value)
        })
    })
}

fn generate_unit_struct(context: &MacroContext) -> TokenStream {
    let span = context.ident().span();
    let target_type = respan(context.target_type(), span);

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        let opaque_span = context.opaque_error_span();
        let opaque_target = quote_spanned! {opaque_span=> #opaque_target};
        let into_value = quote_spanned! {opaque_span=>
            let value: #opaque_target = #target_type.into();
            Ok(value)
        };
        context.impl_from_eure(quote_spanned! {span=>
            ctx.parse::<()>()?;
            #into_value
        })
    } else {
        context.impl_from_eure(quote_spanned! {span=>
            ctx.parse::<()>()?;
            Ok(#target_type)
        })
    }
}

fn generate_tuple_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let span = fields.span();
    let target_type = respan(context.target_type(), span);
    let mut field_types = Vec::new();
    let mut field_names = Vec::new();
    let mut field_parsers = Vec::new();
    let mut has_via = false;

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;
        let field_name = format_ident!("field_{}", i);
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
        field_names.push(field_name);
        field_parsers.push(parser);
    }

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        let opaque_span = context.opaque_error_span();
        let opaque_target = quote_spanned! {opaque_span=> #opaque_target};
        let into_value = quote_spanned! {opaque_span=>
            let value: #opaque_target = #target_type(#(#field_names),*).into();
            Ok(value)
        };
        if has_via {
            let tuple_len = Literal::usize_unsuffixed(fields.len());
            context.impl_from_eure(quote_spanned! {span=>
                let mut tuple = ctx.parse_tuple()?;
                tuple.expect_len(#tuple_len)?;
                #(#field_parsers)*
                #into_value
            })
        } else {
            context.impl_from_eure(quote_spanned! {span=>
                let (#(#field_names,)*) = ctx.parse::<(#(#field_types,)*)>()?;
                #into_value
            })
        }
    } else if has_via {
        let tuple_len = Literal::usize_unsuffixed(fields.len());
        context.impl_from_eure(quote_spanned! {span=>
            let mut tuple = ctx.parse_tuple()?;
            tuple.expect_len(#tuple_len)?;
            #(#field_parsers)*
            Ok(#target_type(#(#field_names),*))
        })
    } else {
        context.impl_from_eure(quote_spanned! {span=>
            let (#(#field_names,)*) = ctx.parse::<(#(#field_types,)*)>()?;
            Ok(#target_type(#(#field_names),*))
        })
    }
}

fn generate_newtype_struct(context: &MacroContext, field: &syn::Field) -> TokenStream {
    let field_ty = &field.ty;
    let span = field_ty.span();
    let target_type = respan(context.target_type(), span);
    let attrs = FieldAttrs::from_field(field).expect("failed to parse field attributes");
    let parse = if let Some(via_type) = attrs.via.as_ref() {
        quote_spanned! {via_type.span()=>
            let field_0 = ctx.parse_via::<#via_type, #field_ty>()?;
        }
    } else {
        quote_spanned! {span=>
            let field_0 = ctx.parse::<#field_ty>()?;
        }
    };

    // For opaque proxy, we need to convert via .into()
    if let Some(opaque_target) = context.opaque_target() {
        let opaque_span = context.opaque_error_span();
        let opaque_target = quote_spanned! {opaque_span=> #opaque_target};
        let into_value = quote_spanned! {opaque_span=>
            let value: #opaque_target = #target_type(field_0).into();
            Ok(value)
        };
        context.impl_from_eure(quote_spanned! {span=>
            #parse
            #into_value
        })
    } else {
        context.impl_from_eure(quote_spanned! {span=>
            #parse
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
        let via_span = via_type.span();
        return match default {
            DefaultValue::None => {
                quote_spanned! {via_span=>
                    #field_name: rec.parse_field_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                }
            }
            DefaultValue::Default => {
                quote_spanned! {via_span=>
                    #field_name: rec.parse_field_optional_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
                }
            }
            DefaultValue::Path(path) => {
                quote_spanned! {via_span=>
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
        let via_span = via_type.span();
        return match default {
            DefaultValue::None => {
                quote_spanned! {via_span=>
                    #field_name: ctx.parse_ext_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                }
            }
            DefaultValue::Default => {
                quote_spanned! {via_span=>
                    #field_name: ctx.parse_ext_optional_with(#field_name_str, <#via_type as ::eure::document::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
                }
            }
            DefaultValue::Path(path) => {
                quote_spanned! {via_span=>
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
