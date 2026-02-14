#[cfg(test)]
mod tests;

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use std::collections::BTreeSet;
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
    let document_crate = &context.config.document_crate;

    let needs_type_asserts = context.config.proxy.is_some();
    let generic_type_params: BTreeSet<String> = context
        .generics()
        .type_params()
        .map(|tp| tp.ident.to_string())
        .collect();
    let into_eure_record = context.IntoEureRecord();

    let mut field_writes = Vec::new();
    let mut field_asserts = Vec::new();
    let mut flatten_bounds = Vec::new();
    let mut seen_flatten_type_params = BTreeSet::new();
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
        if (attrs.flatten || attrs.flatten_ext)
            && let Some(type_param) = flattened_type_param_ident(field_ty)
        {
            let type_param_name = type_param.to_string();
            if generic_type_params.contains(&type_param_name)
                && seen_flatten_type_params.insert(type_param_name)
            {
                flatten_bounds.push(quote_spanned! {type_param.span()=>
                    #type_param: #into_eure_record<#type_param>
                });
            }
        }

        let write = if attrs.flatten {
            let span = field_ty.span();
            quote_spanned! {span=>
                rec.flatten::<#field_ty, _>(value.#field_name)?;
            }
        } else if attrs.flatten_ext {
            let span = field_ty.span();
            quote_spanned! {span=>
                rec.flatten_ext::<#field_ty, _>(value.#field_name)?;
            }
        } else if attrs.ext {
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

    // Generate both IntoEureRecord and IntoEure for named structs
    let record_body = quote! {
        #(#field_asserts)*
        #(#field_writes)*
        Ok(())
    };
    let record_impl = if flatten_bounds.is_empty() {
        context.impl_into_eure_record(record_body)
    } else {
        context.impl_into_eure_record_with_where(record_body, &flatten_bounds)
    };

    // For opaque proxy, we need to convert via .into() first
    let into_eure_impl = if context.opaque_target().is_some() {
        let ident = context.ident();
        let opaque_span = context.opaque_error_span();
        let into_value = quote_spanned! {opaque_span=>
            let value: #ident = value.into();
        };
        context.impl_into_eure_with_where(
            quote! {
                #into_value
                c.record(|rec| {
                    <Self as #into_eure_record>::write_to_record(value, rec)
                })
            },
            &flatten_bounds,
        )
    } else if let Some(ref proxy) = context.config.proxy {
        // Non-opaque proxy: value type is the target, need explicit type param
        let target = &proxy.target;
        context.impl_into_eure_with_where(
            quote! {
                c.record(|rec| {
                    <Self as #into_eure_record<#target>>::write_to_record(value, rec)
                })
            },
            &flatten_bounds,
        )
    } else {
        context.impl_into_eure_with_where(
            quote! {
                c.record(|rec| {
                    <Self as #into_eure_record>::write_to_record(value, rec)
                })
            },
            &flatten_bounds,
        )
    };

    Ok(quote! {
        #record_impl
        #into_eure_impl
    })
}

fn flattened_type_param_ident(field_ty: &syn::Type) -> Option<&syn::Ident> {
    let syn::Type::Path(type_path) = field_ty else {
        return None;
    };
    if type_path.qself.is_some() {
        return None;
    }
    if type_path.path.segments.len() != 1 {
        return None;
    }
    let segment = type_path.path.segments.first().expect("checked len");
    if !matches!(segment.arguments, syn::PathArguments::None) {
        return None;
    }
    Some(&segment.ident)
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
    let mut field_names = Vec::new();
    let mut field_writes = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_name = format_ident!("field_{}", i);
        let field_ty = &f.ty;
        let attrs = FieldAttrs::from_field(f).expect("failed to parse field attributes");
        let write = if let Some(via_type) = attrs.via.as_ref() {
            quote_spanned! {via_type.span()=>
                t.next_via::<#via_type, _>(#field_name)?;
            }
        } else {
            quote_spanned! {field_ty.span()=>
                t.next(#field_name)?;
            }
        };
        field_names.push(field_name);
        field_writes.push(write);
    }

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

fn generate_newtype_struct(context: &MacroContext, field: &syn::Field) -> TokenStream {
    let document_crate = &context.config.document_crate;
    let field_ty = &field.ty;
    let span = field_ty.span();
    let target_type = respan(context.target_type(), span);
    let attrs = FieldAttrs::from_field(field).expect("failed to parse field attributes");
    let write = if let Some(via_type) = attrs.via.as_ref() {
        quote_spanned! {via_type.span()=>
            c.write_via::<#via_type, _>(inner)
        }
    } else {
        quote_spanned! {span=>
            <#field_ty as #document_crate::write::IntoEure>::write(inner, c)
        }
    };

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
            #write
        })
    } else {
        context.impl_into_eure(quote_spanned! {span=>
            let #target_type(inner) = value;
            #write
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
        quote_spanned! {via_type.span()=>
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
        quote_spanned! {via_type.span()=>
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
