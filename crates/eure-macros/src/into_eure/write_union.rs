#[cfg(test)]
mod tests;

use darling::{FromField, FromVariant};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataEnum, Fields, Variant};

use crate::attrs::{FieldAttrs, VariantAttrs, extract_eure_attr_spans, extract_variant_attr_spans};
use crate::context::MacroContext;
use crate::util::respan;

pub fn generate_union_writer(context: &MacroContext, input: &DataEnum) -> syn::Result<TokenStream> {
    let DataEnum { variants, .. } = input;

    let mut variant_arms = Vec::new();
    for variant in variants {
        let arm = generate_variant_arm(context, variant)?;
        variant_arms.push(arm);
    }
    let needs_non_exhaustive_fallback = context.config.proxy.is_some()
        && context.config.non_exhaustive
        && context.opaque_target().is_none();
    if needs_non_exhaustive_fallback {
        let write_error = context.WriteError();
        let proxy_target = &context
            .config
            .proxy
            .as_ref()
            .expect("non-exhaustive fallback requires proxy target")
            .target;
        variant_arms.push(quote! {
            _ => Err(#write_error::NonExhaustiveVariant {
                type_name: ::core::any::type_name::<#proxy_target>(),
            })
        });
    }

    let enum_ident = context.ident();

    // For opaque proxy, we need to convert via .into() first
    Ok(if context.opaque_target().is_some() {
        let opaque_span = context.opaque_error_span();
        let into_value = quote_spanned! {opaque_span=>
            let value: #enum_ident = value.into();
        };
        context.impl_into_eure(quote! {
            #into_value
            match value {
                #(#variant_arms)*
            }
        })
    } else {
        context.impl_into_eure(quote! {
            match value {
                #(#variant_arms)*
            }
        })
    })
}

fn generate_variant_arm(context: &MacroContext, variant: &Variant) -> syn::Result<TokenStream> {
    let document_crate = &context.config.document_crate;
    let needs_type_asserts = context.config.proxy.is_some();
    // For opaque: we already converted to definition type, so use ident()
    // For proxy: value is target type, so use target_type()
    // For normal: value is Self, so use ident()
    let enum_type = if context.opaque_target().is_some() {
        // Opaque: value was converted, use definition type
        let ident = context.ident();
        quote!(#ident)
    } else {
        // Proxy or normal: use target_type() which returns target or self
        respan(context.target_type(), variant.ident.span())
    };
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
        Fields::Unit => Ok(generate_unit_variant_arm(
            document_crate,
            &enum_type,
            variant_ident,
            &variant_name,
        )),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(generate_newtype_variant_arm(
            document_crate,
            &enum_type,
            variant_ident,
            &variant_name,
            &fields.unnamed[0].ty,
        )),
        Fields::Unnamed(fields) => Ok(generate_tuple_variant_arm(
            document_crate,
            &enum_type,
            variant_ident,
            &variant_name,
            &fields.unnamed,
            needs_type_asserts,
        )),
        Fields::Named(fields) => generate_struct_variant_arm(
            context,
            document_crate,
            &enum_type,
            variant_ident,
            &variant_name,
            &fields.named,
            needs_type_asserts,
        ),
    }
}

fn generate_unit_variant_arm(
    document_crate: &TokenStream,
    enum_type: &TokenStream,
    variant_ident: &syn::Ident,
    variant_name: &str,
) -> TokenStream {
    quote! {
        #enum_type::#variant_ident => {
            c.set_variant(#variant_name)?;
            c.bind_primitive(#document_crate::value::PrimitiveValue::Text(
                #document_crate::text::Text::plaintext(#variant_name)
            ))?;
            Ok(())
        }
    }
}

fn generate_newtype_variant_arm(
    document_crate: &TokenStream,
    enum_type: &TokenStream,
    variant_ident: &syn::Ident,
    variant_name: &str,
    field_ty: &syn::Type,
) -> TokenStream {
    let field_span = field_ty.span();
    let write = quote_spanned! {field_span=>
        <#field_ty as #document_crate::write::IntoEure>::write(inner, c)
    };
    quote! {
        #enum_type::#variant_ident(inner) => {
            c.set_variant(#variant_name)?;
            #write
        }
    }
}

fn generate_tuple_variant_arm(
    _document_crate: &TokenStream,
    enum_type: &TokenStream,
    variant_ident: &syn::Ident,
    variant_name: &str,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    needs_type_asserts: bool,
) -> TokenStream {
    let pattern_span = fields.span();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_names: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| format_ident!("field_{}", i, span = f.ty.span()))
        .collect();

    let type_asserts: Vec<_> = if needs_type_asserts {
        field_names
            .iter()
            .zip(field_types.iter())
            .map(|(name, ty)| {
                quote_spanned! {ty.span()=>
                    let _: &#ty = &#name;
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let field_writes: Vec<_> = field_names
        .iter()
        .map(|name| {
            quote! { t.next(#name)?; }
        })
        .collect();

    let pattern = quote_spanned! {pattern_span=>
        #enum_type::#variant_ident(#(#field_names),*)
    };
    quote! {
        #pattern => {
            #(#type_asserts)*
            c.set_variant(#variant_name)?;
            c.tuple(|t| {
                #(#field_writes)*
                Ok(())
            })
        }
    }
}

fn generate_struct_variant_arm(
    context: &MacroContext,
    document_crate: &TokenStream,
    enum_type: &TokenStream,
    variant_ident: &syn::Ident,
    variant_name: &str,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    needs_type_asserts: bool,
) -> syn::Result<TokenStream> {
    let field_names: Vec<_> = fields
        .iter()
        .map(|f| f.ident.as_ref().expect("struct fields must have names"))
        .collect();

    let mut field_writes = Vec::new();
    let mut field_asserts = Vec::new();
    for f in fields {
        let field_name = f.ident.as_ref().expect("struct fields must have names");
        let field_ty = &f.ty;
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        let spans = extract_eure_attr_spans(&f.attrs);

        // Validate incompatible attribute combinations
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
                .unwrap_or_else(|| context.apply_field_rename(&field_name.to_string()));
            generate_ext_write_variant(
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
                .unwrap_or_else(|| context.apply_field_rename(&field_name.to_string()));
            generate_record_field_write_variant(
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
                let _: &#field_ty = &#field_name;
            });
        }
    }

    Ok(quote! {
        #enum_type::#variant_ident { #(#field_names),* } => {
            #(#field_asserts)*
            c.set_variant(#variant_name)?;
            c.record(|rec| {
                #(#field_writes)*
                Ok(())
            })
        }
    })
}

/// Generate field write for a struct variant (bindings are direct variables, not value.field)
fn generate_record_field_write_variant(
    _document_crate: &TokenStream,
    field_name: &syn::Ident,
    _field_ty: &syn::Type,
    field_name_str: &str,
    via: Option<&syn::Type>,
) -> TokenStream {
    // When via is specified, we use field_via to call the marker type's write method
    if let Some(via_type) = via {
        quote! {
            rec.field_via::<#via_type, _>(#field_name_str, #field_name)?;
        }
    } else {
        quote! {
            rec.field(#field_name_str, #field_name)?;
        }
    }
}

/// Generate extension write for a struct variant (bindings are direct variables, not value.field)
fn generate_ext_write_variant(
    document_crate: &TokenStream,
    field_name: &syn::Ident,
    _field_ty: &syn::Type,
    field_name_str: &str,
    via: Option<&syn::Type>,
) -> TokenStream {
    // Extension writes go through set_extension
    if let Some(via_type) = via {
        quote! {
            {
                let scope = rec.constructor().begin_scope();
                let ident: #document_crate::identifier::Identifier = #field_name_str.parse()
                    .map_err(|_| #document_crate::write::WriteError::InvalidIdentifier(#field_name_str.into()))?;
                rec.constructor().navigate(#document_crate::path::PathSegment::Extension(ident))?;
                rec.constructor().write_via::<#via_type, _>(#field_name)?;
                rec.constructor().end_scope(scope)?;
            }
        }
    } else {
        quote! {
            rec.constructor().set_extension(#field_name_str, #field_name)?;
        }
    }
}
