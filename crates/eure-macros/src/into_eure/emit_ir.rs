use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use eure_codegen_ir::{
    FieldModeIr, IrModule, RustFieldIr, RustTypeKindIr, RustVariantIr, VariantShapeIr,
};

use crate::emit_ir_common::{self, DeriveIrType, field_attr_span, field_ty_span};
use crate::ir_spans::DeriveSpanTable;

pub(super) fn derive(ir: &IrModule, spans: &DeriveSpanTable) -> syn::Result<TokenStream> {
    let emit = DeriveIrType::single_root(ir, spans)?;
    let (write_body, flatten_body) = match emit.binding().kind() {
        RustTypeKindIr::Record => emit_named_struct_writer(&emit)?,
        RustTypeKindIr::Newtype => (emit_newtype_struct_writer(&emit)?, None),
        RustTypeKindIr::Tuple => (emit_tuple_struct_writer(&emit)?, None),
        RustTypeKindIr::Unit => (emit_unit_struct_writer(&emit)?, None),
        RustTypeKindIr::Enum => (emit_union_writer(&emit)?, None),
    };

    emit_ir_common::impl_into_eure(&emit, write_body, flatten_body, &[])
}

fn emit_named_struct_writer(
    emit: &DeriveIrType<'_>,
) -> syn::Result<(TokenStream, Option<TokenStream>)> {
    let document_crate = emit.document_crate()?;
    let into_eure = quote! { #document_crate::write::IntoEure };
    let fields = &emit.binding().fields();
    let has_record = fields
        .iter()
        .any(|field| matches!(field.mode(), FieldModeIr::Record));
    let flatten_count = fields
        .iter()
        .filter(|field| matches!(field.mode(), FieldModeIr::Flatten))
        .count();
    let content_mode = !has_record && flatten_count == 1;
    let needs_type_asserts = emit.container().proxy_mode().is_some();

    let field_writes = fields
        .iter()
        .map(|field| named_struct_field_write(field, emit, content_mode))
        .collect::<syn::Result<Vec<_>>>()?;
    let field_asserts = if needs_type_asserts {
        fields
            .iter()
            .map(|field| {
                let span = emit.field_ty_span(field.rust_name());
                let field_name = emit_ir_common::field_ident_with_span(
                    field.rust_name(),
                    emit.field_span(field.rust_name()),
                )?;
                let field_ty = emit_ir_common::rust_type_tokens_with_span(
                    field.ty(),
                    &document_crate,
                    emit.field_ty_span(field.rust_name()),
                )?;
                Ok(quote_spanned! {span=> let _: &#field_ty = &value.#field_name; })
            })
            .collect::<syn::Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    if content_mode {
        let body = quote! {
            #(#field_asserts)*
            #(#field_writes)*
            Ok(())
        };
        let write_body = if let Some(self_ty) = self_type_for_opaque(emit)? {
            let opaque_span = emit.container_attr_span("opaque");
            let into_stmt = quote_spanned! {opaque_span=> let value: #self_ty = value.into(); };
            quote! {
                #into_stmt
                #body
            }
        } else {
            body
        };
        Ok((write_body, None))
    } else {
        let flatten_body = quote! {
            #(#field_asserts)*
            #(#field_writes)*
            Ok(())
        };
        let flatten_body = if let Some(self_ty) = self_type_for_opaque(emit)? {
            let opaque_span = emit.container_attr_span("opaque");
            let into_stmt = quote_spanned! {opaque_span=> let value: #self_ty = value.into(); };
            quote! {
                #into_stmt
                #flatten_body
            }
        } else {
            flatten_body
        };

        let write_body = if let Some(target) = emit.proxy_target_ty()? {
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
        Ok((write_body, Some(flatten_body)))
    }
}

fn emit_unit_struct_writer(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let document_crate = emit.document_crate()?;
    if let Some(self_ty) = self_type_for_opaque(emit)? {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt = quote_spanned! {opaque_span=> let _: #self_ty = value.into(); };
        Ok(quote! {
            #into_stmt
            c.bind_primitive(#document_crate::value::PrimitiveValue::Null)
                .map_err(#document_crate::write::WriteError::from)?;
            Ok(())
        })
    } else {
        Ok(quote! {
            let _ = value;
            c.bind_primitive(#document_crate::value::PrimitiveValue::Null)
                .map_err(#document_crate::write::WriteError::from)?;
            Ok(())
        })
    }
}

fn emit_tuple_struct_writer(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let fields = &emit.binding().fields();
    let tuple_span = fields
        .first()
        .map(|field| emit.field_ty_span(field.rust_name()))
        .unwrap_or_else(proc_macro2::Span::call_site);
    let field_names = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            emit_ir_common::tuple_binding_ident_with_span(idx, emit.field_span(field.rust_name()))
        })
        .collect::<Vec<_>>();
    let field_writes = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            let field_name = &field_names[idx];
            if let Some(via) = &field.via() {
                let via_ty = emit_ir_common::path_to_type_tokens_with_span(
                    via,
                    emit.field_attr_span(field.rust_name(), "via_value"),
                )?;
                Ok(quote! { t.next_via::<#via_ty, _>(#field_name)?; })
            } else {
                Ok(quote! { t.next(#field_name)?; })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    if let Some(self_ty) = self_type_for_opaque(emit)? {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt = quote_spanned! {opaque_span=> let value: #self_ty = value.into(); };
        let destructure = quote_spanned! {tuple_span=> let #self_ty(#(#field_names),*) = value; };
        Ok(quote! {
            #into_stmt
            #destructure
            c.tuple(|t| {
                #(#field_writes)*
                Ok(())
            })
        })
    } else {
        let target_type = emit_ir_common::with_span(emit.target_constructor_ty()?, tuple_span);
        let destructure =
            quote_spanned! {tuple_span=> let #target_type(#(#field_names),*) = value; };
        Ok(quote! {
            #destructure
            c.tuple(|t| {
                #(#field_writes)*
                Ok(())
            })
        })
    }
}

fn emit_newtype_struct_writer(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let document_crate = emit.document_crate()?;
    let field =
        emit.binding().fields().first().ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "newtype has no field")
        })?;
    let field_span = emit.field_ty_span(field.rust_name());
    let field_ty =
        emit_ir_common::rust_type_tokens_with_span(field.ty(), &document_crate, field_span)?;
    let write = if let Some(via) = &field.via() {
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(
            via,
            emit.field_attr_span(field.rust_name(), "via_value"),
        )?;
        quote! { c.write_via::<#via_ty, _>(inner) }
    } else {
        quote! { <#field_ty as #document_crate::write::IntoEure>::write(inner, c) }
    };

    if let Some(self_ty) = self_type_for_opaque(emit)? {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt = quote_spanned! {opaque_span=> let value: #self_ty = value.into(); };
        Ok(quote! {
            #into_stmt
            let #self_ty(inner) = value;
            #write
        })
    } else {
        let target_type = emit_ir_common::with_span(emit.target_constructor_ty()?, field_span);
        let destructure = quote_spanned! {field_span=> let #target_type(inner) = value; };
        Ok(quote! {
            #destructure
            #write
        })
    }
}

fn emit_union_writer(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let document_crate = emit.document_crate()?;
    let arms = emit
        .binding()
        .variants()
        .iter()
        .map(|variant| emit_variant_arm(emit, variant))
        .collect::<syn::Result<Vec<_>>>()?;

    let mut arms = arms;
    if emit.container().proxy_mode().is_some()
        && emit.container().non_exhaustive()
        && emit.opaque_target_ty()?.is_none()
    {
        let write_error = quote! { #document_crate::write::WriteError };
        let proxy_target = emit.proxy_target_ty()?.ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "missing proxy target")
        })?;
        arms.push(quote! {
            _ => Err(#write_error::NonExhaustiveVariant {
                type_name: ::core::any::type_name::<#proxy_target>(),
            }.into())
        });
    }

    if let Some(self_ty) = self_type_for_opaque(emit)? {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt = quote_spanned! {opaque_span=> let value: #self_ty = value.into(); };
        Ok(quote! {
            #into_stmt
            match value {
                #(#arms)*
            }
        })
    } else {
        Ok(quote! {
            match value {
                #(#arms)*
            }
        })
    }
}

fn emit_variant_arm(emit: &DeriveIrType<'_>, variant: &RustVariantIr) -> syn::Result<TokenStream> {
    let document_crate = emit.document_crate()?;
    let enum_type = if emit.opaque_target_ty()?.is_some() {
        self_type_for_opaque(emit)?
            .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing self type"))?
    } else {
        emit_ir_common::with_span(
            emit.target_constructor_ty()?,
            emit.variant_span(variant.rust_name()),
        )
    };
    let variant_ident = emit_ir_common::parse_ident_with_span(
        variant.rust_name(),
        "variant name",
        emit.variant_span(variant.rust_name()),
    )?;
    let variant_name = &variant.wire_name();
    let needs_type_asserts = emit.container().proxy_mode().is_some();

    match &variant.shape() {
        VariantShapeIr::Unit => Ok(quote! {
            #enum_type::#variant_ident => {
                c.set_variant(#variant_name)?;
                c.bind_primitive(#document_crate::value::PrimitiveValue::Text(
                    #document_crate::text::Text::plaintext(#variant_name)
                ))
                .map_err(#document_crate::write::WriteError::from)?;
                Ok(())
            }
        }),
        VariantShapeIr::Newtype { ty, via } => {
            let field_span = emit.variant_field_ty_span(variant.rust_name(), "0");
            let field_ty =
                emit_ir_common::rust_type_tokens_with_span(ty, &document_crate, field_span)?;
            let write = if let Some(via) = via {
                let via_span = emit.variant_field_attr_span(variant.rust_name(), "0", "via_value");
                let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
                quote_spanned! {via_span=> c.write_via::<#via_ty, _>(inner) }
            } else {
                quote_spanned! {field_span=> <#field_ty as #document_crate::write::IntoEure>::write(inner, c) }
            };
            Ok(quote_spanned! {field_span=>
                #enum_type::#variant_ident(inner) => {
                    c.set_variant(#variant_name)?;
                    #write
                }
            })
        }
        VariantShapeIr::Tuple(elements) => {
            let field_names = elements
                .iter()
                .enumerate()
                .map(|(idx, _)| {
                    emit_ir_common::tuple_binding_ident_with_span(
                        idx,
                        emit.variant_field_span(variant.rust_name(), &idx.to_string()),
                    )
                })
                .collect::<Vec<_>>();
            let type_asserts = if needs_type_asserts {
                elements
                    .iter()
                    .enumerate()
                    .map(|(idx, element)| {
                        let span =
                            emit.variant_field_ty_span(variant.rust_name(), &idx.to_string());
                        let field_name = &field_names[idx];
                        let field_ty = emit_ir_common::rust_type_tokens_with_span(
                            &element.ty,
                            &document_crate,
                            emit.variant_field_ty_span(variant.rust_name(), &idx.to_string()),
                        )?;
                        Ok(quote_spanned! {span=> let _: &#field_ty = &#field_name; })
                    })
                    .collect::<syn::Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let writes = elements
                .iter()
                .enumerate()
                .map(|(idx, element)| {
                    let field_name = &field_names[idx];
                    let field_span =
                        emit.variant_field_ty_span(variant.rust_name(), &idx.to_string());
                    if let Some(via) = &element.via {
                        let via_span = emit.variant_field_attr_span(
                            variant.rust_name(),
                            &idx.to_string(),
                            "via_value",
                        );
                        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
                        Ok(quote_spanned! {via_span=> t.next_via::<#via_ty, _>(#field_name)?; })
                    } else {
                        Ok(quote_spanned! {field_span=> t.next(#field_name)?; })
                    }
                })
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(quote! {
                #enum_type::#variant_ident(#(#field_names),*) => {
                    #(#type_asserts)*
                    c.set_variant(#variant_name)?;
                    c.tuple(|t| {
                        #(#writes)*
                        Ok(())
                    })
                }
            })
        }
        VariantShapeIr::Record(fields) => {
            let names = fields
                .iter()
                .map(|field| {
                    emit_ir_common::field_ident_with_span(
                        field.rust_name(),
                        emit.variant_field_span(variant.rust_name(), field.rust_name()),
                    )
                })
                .collect::<syn::Result<Vec<_>>>()?;
            let type_asserts = if needs_type_asserts {
                fields
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| {
                        let span =
                            emit.variant_field_ty_span(variant.rust_name(), field.rust_name());
                        let field_name = &names[idx];
                        let field_ty = emit_ir_common::rust_type_tokens_with_span(
                            field.ty(),
                            &document_crate,
                            emit.variant_field_ty_span(variant.rust_name(), field.rust_name()),
                        )?;
                        Ok(quote_spanned! {span=> let _: &#field_ty = &#field_name; })
                    })
                    .collect::<syn::Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            let writes = fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    record_variant_field_write(field, &names[idx], emit, Some(variant.rust_name()))
                })
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(quote! {
                #enum_type::#variant_ident { #(#names),* } => {
                    #(#type_asserts)*
                    c.set_variant(#variant_name)?;
                    c.record(|rec| {
                        #(#writes)*
                        Ok(())
                    })
                }
            })
        }
    }
}

fn named_struct_field_write(
    field: &RustFieldIr,
    emit: &DeriveIrType<'_>,
    content_mode: bool,
) -> syn::Result<TokenStream> {
    let span = emit.field_span(field.rust_name());
    let document_crate = emit.document_crate()?;
    let field_name = emit_ir_common::field_ident_with_span(
        field.rust_name(),
        emit.field_span(field.rust_name()),
    )?;
    let field_ty = emit_ir_common::rust_type_tokens_with_span(
        field.ty(),
        &document_crate,
        emit.field_ty_span(field.rust_name()),
    )?;
    let wire = &field.wire_name();
    Ok(match field.mode() {
        FieldModeIr::Flatten => {
            if content_mode {
                quote_spanned! {span=> <#field_ty as #document_crate::write::IntoEure>::write(value.#field_name, c)?; }
            } else {
                quote_spanned! {span=> rec.flatten::<#field_ty, _>(value.#field_name)?; }
            }
        }
        FieldModeIr::FlattenExt => {
            if content_mode {
                quote_spanned! {span=>
                    {
                        let mut ext_rec = #document_crate::write::RecordWriter::new_with_ext_mode(c, true);
                        <#field_ty as #document_crate::write::IntoEure>::write_flatten(value.#field_name, &mut ext_rec)?;
                    }
                }
            } else {
                quote_spanned! {span=> rec.flatten_ext::<#field_ty, _>(value.#field_name)?; }
            }
        }
        FieldModeIr::Ext => {
            if content_mode {
                ext_write_constructor(
                    &field_name,
                    &field_ty,
                    wire,
                    field.via(),
                    emit.field_attr_span(field.rust_name(), "via_value"),
                    &document_crate,
                )?
            } else {
                ext_write_named(
                    &field_name,
                    &field_ty,
                    wire,
                    field.via(),
                    emit.field_attr_span(field.rust_name(), "via_value"),
                    &document_crate,
                )?
            }
        }
        FieldModeIr::Record => record_field_write_named(
            &field_name,
            &field_ty,
            wire,
            field.via(),
            emit.field_attr_span(field.rust_name(), "via_value"),
            &document_crate,
        )?,
    })
}

fn record_variant_field_write(
    field: &RustFieldIr,
    binding: &syn::Ident,
    emit: &DeriveIrType<'_>,
    variant_name: Option<&str>,
) -> syn::Result<TokenStream> {
    let span = variant_name
        .map(|variant_name| emit.variant_field_span(variant_name, field.rust_name()))
        .unwrap_or_else(|| emit.field_span(field.rust_name()));
    let document_crate = emit.document_crate()?;
    let field_ty = emit_ir_common::rust_type_tokens_with_span(
        field.ty(),
        &document_crate,
        field_ty_span(emit, field, variant_name),
    )?;
    let wire = &field.wire_name();
    Ok(match field.mode() {
        FieldModeIr::Flatten => quote_spanned! {span=> rec.flatten::<#field_ty, _>(#binding)?; },
        FieldModeIr::FlattenExt => {
            quote_spanned! {span=> rec.flatten_ext::<#field_ty, _>(#binding)?; }
        }
        FieldModeIr::Ext => ext_write_variant(
            binding,
            &field_ty,
            wire,
            field.via(),
            field_attr_span(emit, field, variant_name, "via_value"),
            &document_crate,
        )?,
        FieldModeIr::Record => record_field_write_variant(
            binding,
            &field_ty,
            wire,
            field.via(),
            field_attr_span(emit, field, variant_name, "via_value"),
            &document_crate,
        )?,
    })
}

fn record_field_write_named(
    field_name: &syn::Ident,
    _field_ty: &TokenStream,
    wire_name: &str,
    via: Option<&eure_codegen_ir::RustPathIr>,
    via_span: proc_macro2::Span,
    _document_crate: &TokenStream,
) -> syn::Result<TokenStream> {
    if let Some(via) = via {
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
        Ok(quote_spanned! {via_span=>
            rec.field_via::<#via_ty, _>(#wire_name, value.#field_name)?;
        })
    } else {
        Ok(quote! {
            rec.field(#wire_name, value.#field_name)?;
        })
    }
}

fn ext_write_named(
    field_name: &syn::Ident,
    _field_ty: &TokenStream,
    wire_name: &str,
    via: Option<&eure_codegen_ir::RustPathIr>,
    via_span: proc_macro2::Span,
    document_crate: &TokenStream,
) -> syn::Result<TokenStream> {
    if let Some(via) = via {
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
        Ok(quote_spanned! {via_span=>
            {
                let scope = rec.constructor().begin_scope();
                let ident: #document_crate::identifier::Identifier = #wire_name.parse()
                    .map_err(|_| #document_crate::write::WriteError::InvalidIdentifier(#wire_name.into()))?;
                rec.constructor()
                    .navigate(#document_crate::path::PathSegment::Extension(ident))
                    .map_err(#document_crate::write::WriteError::from)?;
                rec.constructor().write_via::<#via_ty, _>(value.#field_name)?;
                rec.constructor()
                    .end_scope(scope)
                    .map_err(#document_crate::write::WriteError::from)?;
            }
        })
    } else {
        Ok(quote! {
            rec.constructor().set_extension(#wire_name, value.#field_name)?;
        })
    }
}

fn ext_write_constructor(
    field_name: &syn::Ident,
    _field_ty: &TokenStream,
    wire_name: &str,
    via: Option<&eure_codegen_ir::RustPathIr>,
    via_span: proc_macro2::Span,
    document_crate: &TokenStream,
) -> syn::Result<TokenStream> {
    if let Some(via) = via {
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
        Ok(quote_spanned! {via_span=>
            {
                let scope = c.begin_scope();
                let ident: #document_crate::identifier::Identifier = #wire_name.parse()
                    .map_err(|_| #document_crate::write::WriteError::InvalidIdentifier(#wire_name.into()))?;
                c.navigate(#document_crate::path::PathSegment::Extension(ident))
                    .map_err(#document_crate::write::WriteError::from)?;
                c.write_via::<#via_ty, _>(value.#field_name)?;
                c.end_scope(scope).map_err(#document_crate::write::WriteError::from)?;
            }
        })
    } else {
        Ok(quote! {
            c.set_extension(#wire_name, value.#field_name)?;
        })
    }
}

fn record_field_write_variant(
    binding: &syn::Ident,
    _field_ty: &TokenStream,
    wire_name: &str,
    via: Option<&eure_codegen_ir::RustPathIr>,
    via_span: proc_macro2::Span,
    _document_crate: &TokenStream,
) -> syn::Result<TokenStream> {
    if let Some(via) = via {
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
        Ok(quote_spanned! {via_span=>
            rec.field_via::<#via_ty, _>(#wire_name, #binding)?;
        })
    } else {
        Ok(quote! {
            rec.field(#wire_name, #binding)?;
        })
    }
}

fn ext_write_variant(
    binding: &syn::Ident,
    _field_ty: &TokenStream,
    wire_name: &str,
    via: Option<&eure_codegen_ir::RustPathIr>,
    via_span: proc_macro2::Span,
    document_crate: &TokenStream,
) -> syn::Result<TokenStream> {
    if let Some(via) = via {
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
        Ok(quote_spanned! {via_span=>
            {
                let scope = rec.constructor().begin_scope();
                let ident: #document_crate::identifier::Identifier = #wire_name.parse()
                    .map_err(|_| #document_crate::write::WriteError::InvalidIdentifier(#wire_name.into()))?;
                rec.constructor()
                    .navigate(#document_crate::path::PathSegment::Extension(ident))
                    .map_err(#document_crate::write::WriteError::from)?;
                rec.constructor().write_via::<#via_ty, _>(#binding)?;
                rec.constructor()
                    .end_scope(scope)
                    .map_err(#document_crate::write::WriteError::from)?;
            }
        })
    } else {
        Ok(quote! {
            rec.constructor().set_extension(#wire_name, #binding)?;
        })
    }
}

fn self_type_for_opaque(emit: &DeriveIrType<'_>) -> syn::Result<Option<TokenStream>> {
    if emit.opaque_target_ty()?.is_some() {
        let ident = emit.ident()?;
        Ok(Some(quote! { #ident }))
    } else {
        Ok(None)
    }
}
