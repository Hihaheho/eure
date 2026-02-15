#[cfg(test)]
mod tests;

use std::collections::HashMap;

use darling::FromVariant;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, Fields};

use crate::attrs::{VariantAttrs, extract_variant_attr_spans};
use crate::context::MacroContext;

pub fn derive(context: MacroContext) -> TokenStream {
    derive_inner(&context).unwrap_or_else(syn::Error::into_compile_error)
}

fn derive_inner(context: &MacroContext) -> syn::Result<TokenStream> {
    let data = match &context.input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                &context.input.ident,
                "ObjectKey can only be derived for enums",
            ));
        }
    };

    // Reject generics
    if !context.generics().params.is_empty() {
        return Err(syn::Error::new_spanned(
            &context.generics().params,
            "ObjectKey does not support generic enums",
        ));
    }

    // Validate all variants are unit variants and collect (variant_ident, renamed_name) pairs
    let variants = collect_variants(context, data)?;

    let document_crate = &context.config.document_crate;
    let ident = context.ident();

    let parse_impl = generate_parse_object_key(document_crate, ident, &variants);
    let from_impl = generate_from_object_key(document_crate, ident, &variants);

    Ok(quote! {
        #parse_impl
        #from_impl
    })
}

struct VariantInfo {
    ident: syn::Ident,
    name: String,
}

fn collect_variants(context: &MacroContext, data: &DataEnum) -> syn::Result<Vec<VariantInfo>> {
    let mut variants = Vec::new();
    let mut seen_names: HashMap<String, (syn::Ident, proc_macro2::Span)> = HashMap::new();
    for variant in &data.variants {
        // Reject non-unit variants
        match &variant.fields {
            Fields::Unit => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "ObjectKey only supports unit variants",
                ));
            }
        }

        let attrs = VariantAttrs::from_variant(variant).map_err(syn::Error::from)?;
        let name_span = variant_name_span(variant, &attrs);
        let name = attrs
            .rename
            .unwrap_or_else(|| context.apply_rename(&variant.ident.to_string()));

        if let Some((first_variant_ident, first_span)) = seen_names.get(&name) {
            let mut err = syn::Error::new(
                name_span,
                format!(
                    "duplicate ObjectKey name `{name}`: variants `{first_variant_ident}` and `{}` resolve to the same key",
                    variant.ident
                ),
            );
            err.combine(syn::Error::new(
                *first_span,
                format!("first mapping for `{name}` appears here"),
            ));
            return Err(err);
        }

        seen_names.insert(name.clone(), (variant.ident.clone(), name_span));

        variants.push(VariantInfo {
            ident: variant.ident.clone(),
            name,
        });
    }
    Ok(variants)
}

fn variant_name_span(variant: &syn::Variant, attrs: &VariantAttrs) -> proc_macro2::Span {
    if attrs.rename.is_some() {
        let variant_attr_spans = extract_variant_attr_spans(variant);
        variant_attr_spans
            .get("rename")
            .copied()
            .unwrap_or_else(|| variant.ident.span())
    } else {
        variant.ident.span()
    }
}

fn generate_parse_object_key(
    document_crate: &TokenStream,
    ident: &syn::Ident,
    variants: &[VariantInfo],
) -> TokenStream {
    let string_match_arms: Vec<_> = variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            let name = &v.name;
            quote! { #name => Ok(#ident::#variant_ident), }
        })
        .collect();

    let ident_match_arms: Vec<_> = variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            let name = &v.name;
            quote! { #name => Ok(#ident::#variant_ident), }
        })
        .collect();

    quote! {
        impl #document_crate::parse::ParseObjectKey<'_> for #ident {
            fn from_object_key(key: &#document_crate::value::ObjectKey) -> Result<Self, #document_crate::parse::ParseErrorKind> {
                match key {
                    #document_crate::value::ObjectKey::String(s) => match s.as_str() {
                        #(#string_match_arms)*
                        other => Err(#document_crate::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                    },
                    _ => Err(#document_crate::parse::ParseErrorKind::TypeMismatch {
                        expected: #document_crate::value::ValueKind::Text,
                        actual: match key {
                            #document_crate::value::ObjectKey::Number(_) => #document_crate::value::ValueKind::Integer,
                            #document_crate::value::ObjectKey::String(_) => #document_crate::value::ValueKind::Text,
                            #document_crate::value::ObjectKey::Tuple(_) => #document_crate::value::ValueKind::Tuple,
                        },
                    }),
                }
            }

            fn from_extension_ident(ident: &#document_crate::identifier::Identifier) -> Result<Self, #document_crate::parse::ParseErrorKind> {
                match ident.as_ref() {
                    #(#ident_match_arms)*
                    other => Err(#document_crate::parse::ParseErrorKind::UnknownVariant(other.to_string())),
                }
            }
        }
    }
}

fn generate_from_object_key(
    document_crate: &TokenStream,
    ident: &syn::Ident,
    variants: &[VariantInfo],
) -> TokenStream {
    let match_arms: Vec<_> = variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            let name = &v.name;
            quote! { #ident::#variant_ident => #name, }
        })
        .collect();

    quote! {
        impl From<#ident> for #document_crate::value::ObjectKey {
            fn from(value: #ident) -> Self {
                #document_crate::value::ObjectKey::String(match value {
                    #(#match_arms)*
                }.to_string())
            }
        }
    }
}
