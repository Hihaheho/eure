#[cfg(test)]
mod tests;

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataStruct, Fields};

use crate::attrs::{FieldAttrs, extract_eure_attr_spans};
use crate::context::MacroContext;
use crate::util::respan;

pub fn generate_record_writer(
    context: &MacroContext,
    input: &DataStruct,
) -> syn::Result<TokenStream> {
    match &input.fields {
        Fields::Named(fields) => generate_named_struct(context, &fields.named),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            Ok(generate_newtype_struct(context, &fields.unnamed[0].ty))
        }
        Fields::Unnamed(fields) => Ok(generate_tuple_struct(context, &fields.unnamed)),
        Fields::Unit => Ok(generate_unit_struct(context)),
    }
}

fn generate_named_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> syn::Result<TokenStream> {
    let document_crate = &context.config.document_crate;

    let needs_type_asserts = context.config.proxy.is_some();
    let mut field_writes = Vec::new();
    let mut field_asserts = Vec::new();
    for f in fields {
        let field_name = f.ident.as_ref().expect("named fields must have names");
        let field_ty = &f.ty;
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        let spans = extract_eure_attr_spans(&f.attrs);

        // Validate incompatible attribute combinations (similar to from_eure)
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

        // For IntoEure, flatten is not yet implemented
        if attrs.flatten {
            let span = spans.get("flatten").copied().unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "#[eure(flatten)] is not yet supported for IntoEure derive on field `{}`",
                    field_name
                ),
            ));
        }
        if attrs.flatten_ext {
            let span = spans
                .get("flatten_ext")
                .copied()
                .unwrap_or_else(|| f.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "#[eure(flatten_ext)] is not yet supported for IntoEure derive on field `{}`",
                    field_name
                ),
            ));
        }

        let write = if attrs.ext {
            let field_name_str = attrs
                .rename
                .clone()
                .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
            generate_ext_write(
                document_crate,
                field_name,
                field_ty,
                &field_name_str,
                attrs.via.as_ref(),
            )
        } else {
            let field_name_str = attrs
                .rename
                .clone()
                .unwrap_or_else(|| context.apply_rename(&field_name.to_string()));
            generate_record_field_write(
                document_crate,
                field_name,
                field_ty,
                &field_name_str,
                attrs.via.as_ref(),
            )
        };
        field_writes.push(write);
        if needs_type_asserts {
            field_asserts.push(quote_spanned! {field_ty.span()=>
                let _: &#field_ty = &value.#field_name;
            });
        }
    }

    // For opaque proxy, we need to convert via .into() first
    Ok(if context.opaque_target().is_some() {
        let ident = context.ident();
        let opaque_span = context.opaque_error_span();
        let into_value = quote_spanned! {opaque_span=>
            let value: #ident = value.into();
        };
        context.impl_into_eure(quote! {
            #into_value
            c.record(|rec| {
                #(#field_asserts)*
                #(#field_writes)*
                Ok(())
            })
        })
    } else {
        context.impl_into_eure(quote! {
            c.record(|rec| {
                #(#field_asserts)*
                #(#field_writes)*
                Ok(())
            })
        })
    })
}

fn generate_unit_struct(context: &MacroContext) -> TokenStream {
    let document_crate = &context.config.document_crate;
    let span = context.ident().span();

    // For opaque proxy, we need to convert via .into() first
    if context.opaque_target().is_some() {
        let ident = context.ident();
        let opaque_span = context.opaque_error_span();
        let into_value = quote_spanned! {opaque_span=>
            let _: #ident = value.into();
        };
        context.impl_into_eure(quote_spanned! {span=>
            #into_value
            c.bind_primitive(#document_crate::value::PrimitiveValue::Null)?;
            Ok(())
        })
    } else {
        context.impl_into_eure(quote_spanned! {span=>
            let _ = value;
            c.bind_primitive(#document_crate::value::PrimitiveValue::Null)?;
            Ok(())
        })
    }
}

fn generate_tuple_struct(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let span = fields.span();
    let target_type = respan(context.target_type(), span);
    let field_names: Vec<_> = (0..fields.len())
        .map(|i| format_ident!("field_{}", i))
        .collect();

    let field_writes: Vec<_> = field_names
        .iter()
        .map(|name| {
            quote! { t.next(#name)?; }
        })
        .collect();

    // For opaque proxy, we need to convert via .into() first
    if context.opaque_target().is_some() {
        let ident = context.ident();
        let opaque_span = context.opaque_error_span();
        let into_value = quote_spanned! {opaque_span=>
            let value: #ident = value.into();
        };
        context.impl_into_eure(quote_spanned! {span=>
            #into_value
            let #ident(#(#field_names),*) = value;
            c.tuple(|t| {
                #(#field_writes)*
                Ok(())
            })
        })
    } else {
        context.impl_into_eure(quote_spanned! {span=>
            let #target_type(#(#field_names),*) = value;
            c.tuple(|t| {
                #(#field_writes)*
                Ok(())
            })
        })
    }
}

fn generate_newtype_struct(context: &MacroContext, field_ty: &syn::Type) -> TokenStream {
    let document_crate = &context.config.document_crate;
    let span = field_ty.span();
    let target_type = respan(context.target_type(), span);

    // For opaque proxy, we need to convert via .into() first
    if context.opaque_target().is_some() {
        let ident = context.ident();
        let opaque_span = context.opaque_error_span();
        let into_value = quote_spanned! {opaque_span=>
            let value: #ident = value.into();
        };
        context.impl_into_eure(quote_spanned! {span=>
            #into_value
            let #ident(inner) = value;
            <#field_ty as #document_crate::write::IntoEure>::write(inner, c)
        })
    } else {
        context.impl_into_eure(quote_spanned! {span=>
            let #target_type(inner) = value;
            <#field_ty as #document_crate::write::IntoEure>::write(inner, c)
        })
    }
}

pub(super) fn generate_record_field_write(
    _document_crate: &TokenStream,
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    field_name_str: &str,
    via: Option<&syn::Type>,
) -> TokenStream {
    let span = field_ty.span();

    // When via is specified, we use field_via to call the marker type's write method
    if let Some(via_type) = via {
        quote_spanned! {span=>
            rec.field_via::<#via_type, _>(#field_name_str, value.#field_name)?;
        }
    } else {
        quote_spanned! {span=>
            rec.field(#field_name_str, value.#field_name)?;
        }
    }
}

pub(super) fn generate_ext_write(
    document_crate: &TokenStream,
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    field_name_str: &str,
    via: Option<&syn::Type>,
) -> TokenStream {
    let span = field_ty.span();

    // Extension writes go through set_extension
    // When via is specified, we need to use write_via
    if let Some(via_type) = via {
        quote_spanned! {span=>
            {
                let scope = rec.constructor().begin_scope();
                let ident: #document_crate::identifier::Identifier = #field_name_str.parse()
                    .map_err(|_| #document_crate::write::WriteError::InvalidIdentifier(#field_name_str.into()))?;
                rec.constructor().navigate(#document_crate::path::PathSegment::Extension(ident))?;
                rec.constructor().write_via::<#via_type, _>(value.#field_name)?;
                rec.constructor().end_scope(scope)?;
            }
        }
    } else {
        quote_spanned! {span=>
            rec.constructor().set_extension(#field_name_str, value.#field_name)?;
        }
    }
}
