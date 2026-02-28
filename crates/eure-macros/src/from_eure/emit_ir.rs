use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use eure_codegen_ir::{
    DefaultValueIr, FieldModeIr, IrModule, RustFieldIr, RustTypeKindIr, RustVariantIr,
    VariantShapeIr,
};

use crate::emit_ir_common::{self, DeriveIrType, field_attr_span, field_span, field_ty_span};
use crate::ir_spans::DeriveSpanTable;

pub(super) fn derive(ir: &IrModule, spans: &DeriveSpanTable) -> syn::Result<TokenStream> {
    let emit = DeriveIrType::single_root(ir, spans)?;
    let parse_body = match emit.binding().kind() {
        RustTypeKindIr::Record => emit_named_struct_parser(&emit)?,
        RustTypeKindIr::Newtype => emit_newtype_struct_parser(&emit)?,
        RustTypeKindIr::Tuple => emit_tuple_struct_parser(&emit)?,
        RustTypeKindIr::Unit => emit_unit_struct_parser(&emit)?,
        RustTypeKindIr::Enum => emit_union_parser(&emit)?,
    };
    emit_ir_common::impl_from_eure(&emit, parse_body)
}

fn emit_named_struct_parser(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    if emit.container().parse_ext() {
        emit_named_struct_from_ext(emit)
    } else {
        emit_named_struct_from_record(emit)
    }
}

fn emit_named_struct_from_record(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let fields = &emit.binding().fields();
    let has_record = fields
        .iter()
        .any(|field| matches!(field.mode(), FieldModeIr::Record));
    let assignments = fields
        .iter()
        .map(|field| record_mode_assignment(field, has_record, emit, None))
        .collect::<syn::Result<Vec<_>>>()?;
    let target_type_raw = emit.target_constructor_ty()?;
    let opaque_target = emit.opaque_target_ty()?;

    let unknown_fields_check = if has_record {
        if emit.container().allow_unknown_fields() {
            quote! { rec.allow_unknown_fields()?; }
        } else {
            quote! { rec.deny_unknown_fields()?; }
        }
    } else if emit.container().allow_unknown_fields() {
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

    let unknown_extensions_check = if emit.container().allow_unknown_extensions() {
        quote! {}
    } else {
        quote! { ctx.deny_unknown_extensions()?; }
    };

    let init_span = fields
        .first()
        .map(|field| emit.field_span(field.rust_name()))
        .unwrap_or_else(proc_macro2::Span::call_site);
    let target_type = emit_ir_common::with_span(target_type_raw, init_span);
    let value_expr = quote_spanned! {init_span=>
        let value = #target_type {
            #(#assignments),*
        };
    };

    let body = if let Some(opaque_target) = opaque_target {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt = quote_spanned! {opaque_span=> let value: #opaque_target = value.into(); };
        let into_value = quote! {
            #into_stmt
            Ok(value)
        };
        if has_record {
            quote! {
                let rec = ctx.parse_record()?;
                #value_expr
                #unknown_fields_check
                #unknown_extensions_check
                #into_value
            }
        } else {
            quote! {
                #value_expr
                #unknown_fields_check
                #unknown_extensions_check
                #into_value
            }
        }
    } else if has_record {
        quote! {
            let rec = ctx.parse_record()?;
            #value_expr
            #unknown_fields_check
            #unknown_extensions_check
            Ok(value)
        }
    } else {
        quote! {
            #value_expr
            #unknown_fields_check
            #unknown_extensions_check
            Ok(value)
        }
    };

    Ok(body)
}

fn emit_named_struct_from_ext(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let assignments = emit
        .binding()
        .fields()
        .iter()
        .map(|field| ext_mode_assignment(field, emit))
        .collect::<syn::Result<Vec<_>>>()?;
    let target_type_raw = emit.target_constructor_ty()?;
    let opaque_target = emit.opaque_target_ty()?;
    let init_span = emit
        .binding()
        .fields()
        .first()
        .map(|field| emit.field_span(field.rust_name()))
        .unwrap_or_else(proc_macro2::Span::call_site);
    let target_type = emit_ir_common::with_span(target_type_raw, init_span);
    let ctor = quote_spanned! {init_span=>
        let value = #target_type {
            #(#assignments),*
        };
    };

    if let Some(opaque_target) = opaque_target {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt = quote_spanned! {opaque_span=> let value: #opaque_target = value.into(); };
        Ok(quote! {
            #ctor
            #into_stmt
            Ok(value)
        })
    } else {
        Ok(quote! {
            #ctor
            Ok(value)
        })
    }
}

fn emit_unit_struct_parser(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let target_type = emit.target_constructor_ty()?;
    if let Some(opaque_target) = emit.opaque_target_ty()? {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt =
            quote_spanned! {opaque_span=> let value: #opaque_target = #target_type.into(); };
        Ok(quote! {
            ctx.parse::<()>()?;
            #into_stmt
            Ok(value)
        })
    } else {
        Ok(quote! {
            ctx.parse::<()>()?;
            Ok(#target_type)
        })
    }
}

fn emit_tuple_struct_parser(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let fields = &emit.binding().fields();
    let tuple_span = fields
        .first()
        .map(|field| emit.field_ty_span(field.rust_name()))
        .unwrap_or_else(proc_macro2::Span::call_site);
    let target_type = emit.target_constructor_ty()?;
    let document_crate = emit.document_crate()?;
    let field_types = fields
        .iter()
        .map(|field| {
            emit_ir_common::rust_type_tokens_with_span(
                field.ty(),
                &document_crate,
                emit.field_ty_span(field.rust_name()),
            )
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let field_names = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            emit_ir_common::tuple_binding_ident_with_span(idx, emit.field_span(field.rust_name()))
        })
        .collect::<Vec<_>>();

    let has_via = fields.iter().any(|field| field.via().is_some());
    let field_len = fields.len();
    if has_via {
        let field_parsers = fields
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let field_name = &field_names[idx];
                let field_ty = &field_types[idx];
                if let Some(via) = &field.via() {
                    let via_ty = emit_ir_common::path_to_type_tokens_with_span(
                        via,
                        emit.field_attr_span(field.rust_name(), "via_value"),
                    )?;
                    Ok(quote! {
                        let #field_name = tuple.next_via::<#via_ty, #field_ty>()?;
                    })
                } else {
                    Ok(quote! {
                        let #field_name = tuple.next::<#field_ty>()?;
                    })
                }
            })
            .collect::<syn::Result<Vec<_>>>()?;

        if let Some(opaque_target) = emit.opaque_target_ty()? {
            let opaque_span = emit.container_attr_span("opaque");
            let ctor_stmt =
                quote_spanned! {tuple_span=> let value = #target_type(#(#field_names),*); };
            let into_stmt =
                quote_spanned! {opaque_span=> let value: #opaque_target = value.into(); };
            Ok(quote! {
                let mut tuple = ctx.parse_tuple()?;
                tuple.expect_len(#field_len)?;
                #(#field_parsers)*
                #ctor_stmt
                #into_stmt
                Ok(value)
            })
        } else {
            let ctor_stmt =
                quote_spanned! {tuple_span=> let value = #target_type(#(#field_names),*); };
            Ok(quote! {
                let mut tuple = ctx.parse_tuple()?;
                tuple.expect_len(#field_len)?;
                #(#field_parsers)*
                #ctor_stmt
                Ok(value)
            })
        }
    } else if let Some(opaque_target) = emit.opaque_target_ty()? {
        let opaque_span = emit.container_attr_span("opaque");
        let ctor_stmt = quote_spanned! {tuple_span=> let value = #target_type(#(#field_names),*); };
        let into_stmt = quote_spanned! {opaque_span=> let value: #opaque_target = value.into(); };
        Ok(quote! {
            let (#(#field_names,)*) = ctx.parse::<(#(#field_types,)*)>()?;
            #ctor_stmt
            #into_stmt
            Ok(value)
        })
    } else {
        let ctor_stmt = quote_spanned! {tuple_span=> let value = #target_type(#(#field_names),*); };
        Ok(quote! {
            let (#(#field_names,)*) = ctx.parse::<(#(#field_types,)*)>()?;
            #ctor_stmt
            Ok(value)
        })
    }
}

fn emit_newtype_struct_parser(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let field =
        emit.binding().fields().first().ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "newtype has no field")
        })?;
    let document_crate = emit.document_crate()?;
    let field_span = emit.field_ty_span(field.rust_name());
    let field_ty =
        emit_ir_common::rust_type_tokens_with_span(field.ty(), &document_crate, field_span)?;
    let parse = if let Some(via) = &field.via() {
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(
            via,
            emit.field_attr_span(field.rust_name(), "via_value"),
        )?;
        quote! { let field_0 = ctx.parse_via::<#via_ty, #field_ty>()?; }
    } else {
        quote! { let field_0 = ctx.parse::<#field_ty>()?; }
    };
    let target_type = emit_ir_common::with_span(emit.target_constructor_ty()?, field_span);
    let ctor_stmt = quote_spanned! {field_span=> let value = #target_type(field_0); };

    if let Some(opaque_target) = emit.opaque_target_ty()? {
        let opaque_span = emit.container_attr_span("opaque");
        let into_stmt = quote_spanned! {opaque_span=> let value: #opaque_target = value.into(); };
        Ok(quote! {
            #parse
            #ctor_stmt
            #into_stmt
            Ok(value)
        })
    } else {
        Ok(quote! {
            #parse
            #ctor_stmt
            Ok(value)
        })
    }
}

fn emit_union_parser(emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let variants = emit
        .binding()
        .variants()
        .iter()
        .map(|variant| emit_variant_parser(emit, variant))
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {
        ctx.parse_union()?
            #(#variants)*
            .parse()
    })
}

fn emit_variant_parser(
    emit: &DeriveIrType<'_>,
    variant: &RustVariantIr,
) -> syn::Result<TokenStream> {
    let document_crate = emit.document_crate()?;
    let target_type = emit.target_constructor_ty()?;
    let variant_ident = emit_ir_common::parse_ident_with_span(
        variant.rust_name(),
        "variant name",
        emit.variant_span(variant.rust_name()),
    )?;
    let variant_name = &variant.wire_name();
    let opaque_span = emit.container_attr_span("opaque");

    match &variant.shape() {
        VariantShapeIr::Unit => {
            let target_type = emit_ir_common::with_span(
                target_type.clone(),
                emit.variant_span(variant.rust_name()),
            );
            let mapper = if emit.opaque_target_ty()?.is_some() {
                quote_spanned! {opaque_span=> |_| #target_type::#variant_ident.into() }
            } else {
                quote! { |_| #target_type::#variant_ident }
            };
            let parser = quote! {
                #document_crate::parse::DocumentParserExt::map(
                    #document_crate::parse::VariantLiteralParser(#variant_name),
                    #mapper
                )
            };
            Ok(quote! { .variant(#variant_name, #parser) })
        }
        VariantShapeIr::Newtype { ty, via } => {
            let field_span = emit.variant_field_ty_span(variant.rust_name(), "0");
            // Keep constructor target type on variant span (legacy behavior),
            // while parsing/type errors still point at the field type span.
            let target_type = emit_ir_common::with_span(
                target_type.clone(),
                emit.variant_span(variant.rust_name()),
            );
            let field_ty =
                emit_ir_common::rust_type_tokens_with_span(ty, &document_crate, field_span)?;
            let ctor = if emit.opaque_target_ty()?.is_some() {
                quote_spanned! {opaque_span=> Ok(#target_type::#variant_ident(field_0).into()) }
            } else {
                quote_spanned! {field_span=> Ok(#target_type::#variant_ident(field_0)) }
            };
            if let Some(via) = via {
                let via_span = emit.variant_field_attr_span(variant.rust_name(), "0", "via_value");
                let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
                Ok(quote_spanned! {via_span=>
                    .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                        let field_0 = ctx.parse_via::<#via_ty, #field_ty>()?;
                        #ctor
                    })
                })
            } else {
                Ok(quote_spanned! {field_span=>
                    .parse_variant::<#field_ty>(#variant_name, |field_0| #ctor)
                })
            }
        }
        VariantShapeIr::Tuple(elements) => {
            let field_types = elements
                .iter()
                .enumerate()
                .map(|(idx, element)| {
                    emit_ir_common::rust_type_tokens_with_span(
                        &element.ty,
                        &document_crate,
                        emit.variant_field_ty_span(variant.rust_name(), &idx.to_string()),
                    )
                })
                .collect::<syn::Result<Vec<_>>>()?;
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
            let has_via = elements.iter().any(|element| element.via.is_some());
            let tuple_len = elements.len();
            let tuple_span = emit.variant_span(variant.rust_name());
            let target_type = emit_ir_common::with_span(target_type.clone(), tuple_span);
            let value = if emit.opaque_target_ty()?.is_some() {
                quote_spanned! {opaque_span=>
                    let value = #target_type::#variant_ident(#(#field_names),*);
                    Ok(value.into())
                }
            } else {
                quote! {
                    let value = #target_type::#variant_ident(#(#field_names),*);
                    Ok(value)
                }
            };

            if has_via {
                let parsers = elements
                    .iter()
                    .enumerate()
                    .map(|(idx, element)| {
                        let field_name = &field_names[idx];
                        let field_ty = &field_types[idx];
                        let field_span =
                            emit.variant_field_ty_span(variant.rust_name(), &idx.to_string());
                        if let Some(via) = &element.via {
                            let via_span = emit.variant_field_attr_span(
                                variant.rust_name(),
                                &idx.to_string(),
                                "via_value",
                            );
                            let via_ty =
                                emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
                            Ok(quote_spanned! {via_span=>
                                let #field_name = tuple.next_via::<#via_ty, #field_ty>()?;
                            })
                        } else {
                            Ok(quote_spanned! {field_span=>
                                let #field_name = tuple.next::<#field_ty>()?;
                            })
                        }
                    })
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(quote_spanned! {tuple_span=>
                    .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                        let mut tuple = ctx.parse_tuple()?;
                        tuple.expect_len(#tuple_len)?;
                        #(#parsers)*
                        #value
                    })
                })
            } else {
                Ok(quote_spanned! {tuple_span=>
                    .parse_variant::<(#(#field_types,)*)>(#variant_name, |(#(#field_names,)*)| {
                        #value
                    })
                })
            }
        }
        VariantShapeIr::Record(fields) => {
            let has_record = fields
                .iter()
                .any(|field| matches!(field.mode(), FieldModeIr::Record));
            let target_span = fields
                .first()
                .map(|field| emit.variant_field_span(variant.rust_name(), field.rust_name()))
                .unwrap_or_else(|| emit.variant_span(variant.rust_name()));
            let target_type = emit_ir_common::with_span(target_type.clone(), target_span);
            let assignments = fields
                .iter()
                .map(|field| {
                    record_mode_assignment(field, has_record, emit, Some(variant.rust_name()))
                })
                .collect::<syn::Result<Vec<_>>>()?;

            let return_value = if emit.opaque_target_ty()?.is_some() {
                quote_spanned! {opaque_span=> Ok(value.into()) }
            } else {
                quote! { Ok(value) }
            };

            if has_record {
                let unknown_fields_check = if variant.allow_unknown_fields() {
                    quote! { rec.allow_unknown_fields()?; }
                } else {
                    quote! { rec.deny_unknown_fields()?; }
                };
                Ok(quote! {
                    .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                        let mut rec = ctx.parse_record()?;
                        let value = #target_type::#variant_ident {
                            #(#assignments),*
                        };
                        #unknown_fields_check
                        #return_value
                    })
                })
            } else {
                Ok(quote! {
                    .variant(#variant_name, |ctx: &#document_crate::parse::ParseContext<'doc>| {
                        let value = #target_type::#variant_ident {
                            #(#assignments),*
                        };
                        ctx.deny_unknown_extensions()?;
                        #return_value
                    })
                })
            }
        }
    }
}

fn record_mode_assignment(
    field: &RustFieldIr,
    has_record: bool,
    emit: &DeriveIrType<'_>,
    variant_name: Option<&str>,
) -> syn::Result<TokenStream> {
    let span = field_ty_span(emit, field, variant_name);
    let name = emit_ir_common::field_ident_with_span(field.rust_name(), span)?;
    let document_crate = emit.document_crate()?;
    let field_ty = emit_ir_common::rust_type_tokens_with_span(
        field.ty(),
        &document_crate,
        field_ty_span(emit, field, variant_name),
    )?;
    Ok(match field.mode() {
        FieldModeIr::Flatten => {
            if has_record {
                quote_spanned! {span=> #name: <#field_ty>::parse(&rec.flatten())? }
            } else {
                quote_spanned! {span=> #name: <#field_ty>::parse(&ctx.flatten())? }
            }
        }
        FieldModeIr::FlattenExt => {
            quote_spanned! {span=> #name: <#field_ty>::parse(&ctx.flatten_ext())? }
        }
        FieldModeIr::Ext => parse_ext_field(field, emit, variant_name)?,
        FieldModeIr::Record => parse_record_field(field, emit, variant_name)?,
    })
}

fn ext_mode_assignment(field: &RustFieldIr, emit: &DeriveIrType<'_>) -> syn::Result<TokenStream> {
    let span = emit.field_span(field.rust_name());
    let name = emit_ir_common::field_ident_with_span(
        field.rust_name(),
        emit.field_span(field.rust_name()),
    )?;
    let document_crate = emit.document_crate()?;
    let field_ty = emit_ir_common::rust_type_tokens_with_span(
        field.ty(),
        &document_crate,
        emit.field_ty_span(field.rust_name()),
    )?;
    Ok(if matches!(field.mode(), FieldModeIr::FlattenExt) {
        quote_spanned! {span=> #name: <#field_ty>::parse(&ctx.flatten_ext())? }
    } else {
        parse_ext_field(field, emit, None)?
    })
}

fn parse_record_field(
    field: &RustFieldIr,
    emit: &DeriveIrType<'_>,
    variant_name: Option<&str>,
) -> syn::Result<TokenStream> {
    let span = field_ty_span(emit, field, variant_name);
    let name = emit_ir_common::field_ident_with_span(field.rust_name(), span)?;
    let document_crate = emit.document_crate()?;
    let field_ty = emit_ir_common::rust_type_tokens_with_span(
        field.ty(),
        &document_crate,
        field_ty_span(emit, field, variant_name),
    )?;
    let wire = &field.wire_name();
    if let Some(via) = &field.via() {
        let via_span = field_attr_span(emit, field, variant_name, "via_value");
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
        return Ok(match &field.default() {
            DefaultValueIr::None => quote_spanned! {via_span=>
                #name: rec.parse_field_with(#wire, <#via_ty as #document_crate::parse::FromEure<'doc, #field_ty>>::parse)?
            },
            DefaultValueIr::DefaultTrait => quote_spanned! {via_span=>
                #name: rec.parse_field_optional_with(#wire, <#via_ty as #document_crate::parse::FromEure<'doc, #field_ty>>::parse)?
                    .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
            },
            DefaultValueIr::Function(path) => {
                let default_span = field_attr_span(emit, field, variant_name, "default");
                let path = emit_ir_common::path_to_type_tokens_with_span(path, default_span)?;
                quote_spanned! {default_span=>
                    #name: rec.parse_field_optional_with(#wire, <#via_ty as #document_crate::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(#path)
                }
            }
        });
    }

    Ok(match &field.default() {
        DefaultValueIr::None => {
            quote_spanned! {span=> #name: rec.parse_field::<#field_ty>(#wire)? }
        }
        DefaultValueIr::DefaultTrait => quote_spanned! {span=>
            #name: rec.parse_field_optional::<#field_ty>(#wire)?
                .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
        },
        DefaultValueIr::Function(path) => {
            let default_span = field_attr_span(emit, field, variant_name, "default");
            let path = emit_ir_common::path_to_type_tokens_with_span(path, default_span)?;
            quote_spanned! {default_span=>
                #name: rec.parse_field_optional::<#field_ty>(#wire)?
                    .unwrap_or_else(#path)
            }
        }
    })
}

fn parse_ext_field(
    field: &RustFieldIr,
    emit: &DeriveIrType<'_>,
    variant_name: Option<&str>,
) -> syn::Result<TokenStream> {
    let span = field_span(emit, field, variant_name);
    let name = emit_ir_common::field_ident_with_span(field.rust_name(), span)?;
    let document_crate = emit.document_crate()?;
    let field_ty = emit_ir_common::rust_type_tokens_with_span(
        field.ty(),
        &document_crate,
        field_ty_span(emit, field, variant_name),
    )?;
    let wire = &field.wire_name();
    if let Some(via) = &field.via() {
        let via_span = field_attr_span(emit, field, variant_name, "via_value");
        let via_ty = emit_ir_common::path_to_type_tokens_with_span(via, via_span)?;
        return Ok(match &field.default() {
            DefaultValueIr::None => quote_spanned! {via_span=>
                #name: ctx.parse_ext_with(#wire, <#via_ty as #document_crate::parse::FromEure<'doc, #field_ty>>::parse)?
            },
            DefaultValueIr::DefaultTrait => quote_spanned! {via_span=>
                #name: ctx.parse_ext_optional_with(#wire, <#via_ty as #document_crate::parse::FromEure<'doc, #field_ty>>::parse)?
                    .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
            },
            DefaultValueIr::Function(path) => {
                let default_span = field_attr_span(emit, field, variant_name, "default");
                let path = emit_ir_common::path_to_type_tokens_with_span(path, default_span)?;
                quote_spanned! {default_span=>
                    #name: ctx.parse_ext_optional_with(#wire, <#via_ty as #document_crate::parse::FromEure<'doc, #field_ty>>::parse)?
                        .unwrap_or_else(#path)
                }
            }
        });
    }

    Ok(match &field.default() {
        DefaultValueIr::None => quote_spanned! {span=> #name: ctx.parse_ext::<#field_ty>(#wire)? },
        DefaultValueIr::DefaultTrait => quote_spanned! {span=>
            #name: ctx.parse_ext_optional::<#field_ty>(#wire)?
                .unwrap_or_else(<#field_ty as ::core::default::Default>::default)
        },
        DefaultValueIr::Function(path) => {
            let default_span = field_attr_span(emit, field, variant_name, "default");
            let path = emit_ir_common::path_to_type_tokens_with_span(path, default_span)?;
            quote_spanned! {default_span=>
                #name: ctx.parse_ext_optional::<#field_ty>(#wire)?
                    .unwrap_or_else(#path)
            }
        }
    })
}
