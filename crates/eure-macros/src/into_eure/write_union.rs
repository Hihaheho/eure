#[cfg(test)]
mod tests;

use darling::{FromField, FromVariant};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, Fields, Variant};

use crate::attrs::{FieldAttrs, VariantAttrs};
use crate::context::MacroContext;

pub fn generate_union_writer(context: &MacroContext, input: &DataEnum) -> TokenStream {
    let DataEnum { variants, .. } = input;

    let variant_arms: Vec<_> = variants
        .iter()
        .map(|variant| generate_variant_arm(context, variant))
        .collect();

    let enum_ident = context.ident();

    // For opaque proxy, we need to convert via .into() first
    if context.opaque_target().is_some() {
        context.impl_into_eure(quote! {
            let value: #enum_ident = value.into();
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
    }
}

fn generate_variant_arm(context: &MacroContext, variant: &Variant) -> TokenStream {
    let document_crate = &context.config.document_crate;
    let enum_ident = context.ident();
    let variant_ident = &variant.ident;
    let variant_attrs =
        VariantAttrs::from_variant(variant).expect("failed to parse variant attributes");
    let variant_name = variant_attrs
        .rename
        .clone()
        .unwrap_or_else(|| context.apply_rename(&variant_ident.to_string()));

    match &variant.fields {
        Fields::Unit => {
            generate_unit_variant_arm(document_crate, enum_ident, variant_ident, &variant_name)
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => generate_newtype_variant_arm(
            document_crate,
            enum_ident,
            variant_ident,
            &variant_name,
            &fields.unnamed[0].ty,
        ),
        Fields::Unnamed(fields) => generate_tuple_variant_arm(
            document_crate,
            enum_ident,
            variant_ident,
            &variant_name,
            &fields.unnamed,
        ),
        Fields::Named(fields) => generate_struct_variant_arm(
            context,
            document_crate,
            enum_ident,
            variant_ident,
            &variant_name,
            &fields.named,
        ),
    }
}

fn generate_unit_variant_arm(
    document_crate: &TokenStream,
    enum_ident: &syn::Ident,
    variant_ident: &syn::Ident,
    variant_name: &str,
) -> TokenStream {
    quote! {
        #enum_ident::#variant_ident => {
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
    enum_ident: &syn::Ident,
    variant_ident: &syn::Ident,
    variant_name: &str,
    field_ty: &syn::Type,
) -> TokenStream {
    quote! {
        #enum_ident::#variant_ident(inner) => {
            c.set_variant(#variant_name)?;
            <#field_ty as #document_crate::write::IntoEure>::write(inner, c)
        }
    }
}

fn generate_tuple_variant_arm(
    _document_crate: &TokenStream,
    enum_ident: &syn::Ident,
    variant_ident: &syn::Ident,
    variant_name: &str,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let field_names: Vec<_> = (0..fields.len())
        .map(|i| format_ident!("field_{}", i))
        .collect();

    let field_writes: Vec<_> = field_names
        .iter()
        .map(|name| {
            quote! { t.next(#name)?; }
        })
        .collect();

    quote! {
        #enum_ident::#variant_ident(#(#field_names),*) => {
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
    enum_ident: &syn::Ident,
    variant_ident: &syn::Ident,
    variant_name: &str,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let field_names: Vec<_> = fields
        .iter()
        .map(|f| f.ident.as_ref().expect("struct fields must have names"))
        .collect();

    let field_writes: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().expect("struct fields must have names");
            let field_ty = &f.ty;
            let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");

            // Validate incompatible attribute combinations
            if attrs.flatten || attrs.flatten_ext {
                panic!(
                    "#[eure(flatten)] and #[eure(flatten_ext)] are not yet supported for IntoEure derive on field `{}`",
                    field_name
                );
            }

            if attrs.ext {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_field_rename(&field_name.to_string()));
                generate_ext_write_variant(document_crate, field_name, field_ty, &field_name_str, attrs.via.as_ref())
            } else {
                let field_name_str = attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| context.apply_field_rename(&field_name.to_string()));
                generate_record_field_write_variant(document_crate, field_name, field_ty, &field_name_str, attrs.via.as_ref())
            }
        })
        .collect();

    quote! {
        #enum_ident::#variant_ident { #(#field_names),* } => {
            c.set_variant(#variant_name)?;
            c.record(|rec| {
                #(#field_writes)*
                Ok(())
            })
        }
    }
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
