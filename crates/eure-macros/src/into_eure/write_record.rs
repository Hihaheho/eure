#[cfg(test)]
mod tests;

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataStruct, Fields};

use crate::attrs::FieldAttrs;
use crate::context::MacroContext;
use crate::ir::{FieldMode, RenameScope, analyze_common_named_fields};
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
    let into_eure = context.IntoEure();
    let common_fields = analyze_common_named_fields(context, fields, RenameScope::Container)?;
    let has_record = common_fields
        .iter()
        .any(|field| matches!(field.mode, FieldMode::Record));
    let flatten_count = common_fields
        .iter()
        .filter(|field| matches!(field.mode, FieldMode::Flatten))
        .count();
    let content_mode = !has_record && flatten_count == 1;

    let needs_type_asserts = context.config.proxy.is_some();

    let mut field_writes = Vec::new();
    let mut field_asserts = Vec::new();
    for field in &common_fields {
        let field_name = &field.ident;
        let field_ty = &field.ty;

        let write = if matches!(field.mode, FieldMode::Flatten) {
            let span = field.ty.span();
            if content_mode {
                quote_spanned! {span=>
                    <#field_ty as #document_crate::write::IntoEure>::write(value.#field_name, c)?;
                }
            } else {
                quote_spanned! {span=>
                    rec.flatten::<#field_ty, _>(value.#field_name)?;
                }
            }
        } else if matches!(field.mode, FieldMode::FlattenExt) {
            let span = field.ty.span();
            if content_mode {
                quote_spanned! {span=>
                    {
                        let mut ext_rec = #document_crate::write::RecordWriter::new_with_ext_mode(c, true);
                        <#field_ty as #document_crate::write::IntoEure>::write_flatten(value.#field_name, &mut ext_rec)?;
                    }
                }
            } else {
                quote_spanned! {span=>
                    rec.flatten_ext::<#field_ty, _>(value.#field_name)?;
                }
            }
        } else if matches!(field.mode, FieldMode::Ext) {
            let field_name_str = field
                .wire_name
                .as_deref()
                .expect("wire name required for ext field");
            if content_mode {
                generate_ext_write_constructor(
                    document_crate,
                    field_name,
                    field_ty,
                    field_name_str,
                    field.via.as_ref(),
                )
            } else {
                generate_ext_write(
                    document_crate,
                    field_name,
                    field_ty,
                    field_name_str,
                    field.via.as_ref(),
                )
            }
        } else {
            let field_name_str = field
                .wire_name
                .as_deref()
                .expect("wire name required for record field");
            generate_record_field_write(
                document_crate,
                field_name,
                field_ty,
                field_name_str,
                field.via.as_ref(),
            )
        };
        field_writes.push(write);
        if needs_type_asserts {
            field_asserts.push(quote_spanned! {field.ty.span()=>
                let _: &#field_ty = &value.#field_name;
            });
        }
    }

    if content_mode {
        let write_body = quote! {
            #(#field_asserts)*
            #(#field_writes)*
            Ok(())
        };

        // For opaque proxy, we need to convert via .into() first
        let into_eure_impl = if context.opaque_target().is_some() {
            let ident = context.ident();
            let opaque_span = context.opaque_error_span();
            let into_value = quote_spanned! {opaque_span=>
                let value: #ident = value.into();
            };
            context.impl_into_eure_with_where_and_flatten(
                quote! {
                    #into_value
                    #write_body
                },
                None,
                &[],
            )
        } else {
            context.impl_into_eure_with_where_and_flatten(write_body, None, &[])
        };

        return Ok(into_eure_impl);
    }

    let flatten_record_body = quote! {
        #(#field_asserts)*
        #(#field_writes)*
        Ok(())
    };
    let flatten_body = if context.opaque_target().is_some() {
        let ident = context.ident();
        let opaque_span = context.opaque_error_span();
        let into_value = quote_spanned! {opaque_span=>
            let value: #ident = value.into();
        };
        quote! {
            #into_value
            #flatten_record_body
        }
    } else {
        flatten_record_body
    };

    let write_body = if let Some(ref proxy) = context.config.proxy {
        let target = &proxy.target;
        quote! {
            c.record(|rec| {
                <Self as #into_eure<#target>>::write_flatten(value, rec)
            })
        }
    } else {
        quote! {
            c.record(|rec| {
                <Self as #into_eure>::write_flatten(value, rec)
            })
        }
    };

    Ok(context.impl_into_eure_with_where_and_flatten(write_body, Some(flatten_body), &[]))
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
            c.bind_primitive(#document_crate::value::PrimitiveValue::Null)
                .map_err(#document_crate::write::WriteError::from)?;
            Ok(())
        })
    } else {
        context.impl_into_eure(quote_spanned! {span=>
            let _ = value;
            c.bind_primitive(#document_crate::value::PrimitiveValue::Null)
                .map_err(#document_crate::write::WriteError::from)?;
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

fn generate_ext_write_constructor(
    document_crate: &TokenStream,
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    field_name_str: &str,
    via: Option<&syn::Type>,
) -> TokenStream {
    let span = field_ty.span();

    if let Some(via_type) = via {
        quote_spanned! {via_type.span()=>
            {
                let scope = c.begin_scope();
                let ident: #document_crate::identifier::Identifier = #field_name_str.parse()
                    .map_err(|_| #document_crate::write::WriteError::InvalidIdentifier(#field_name_str.into()))?;
                c.navigate(#document_crate::path::PathSegment::Extension(ident))
                    .map_err(#document_crate::write::WriteError::from)?;
                c.write_via::<#via_type, _>(value.#field_name)?;
                c.end_scope(scope).map_err(#document_crate::write::WriteError::from)?;
            }
        }
    } else {
        quote_spanned! {span=>
            c.set_extension(#field_name_str, value.#field_name)?;
        }
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
                rec.constructor()
                    .navigate(#document_crate::path::PathSegment::Extension(ident))
                    .map_err(#document_crate::write::WriteError::from)?;
                rec.constructor().write_via::<#via_type, _>(value.#field_name)?;
                rec.constructor()
                    .end_scope(scope)
                    .map_err(#document_crate::write::WriteError::from)?;
            }
        }
    } else {
        quote_spanned! {span=>
            rec.constructor().set_extension(#field_name_str, value.#field_name)?;
        }
    }
}
