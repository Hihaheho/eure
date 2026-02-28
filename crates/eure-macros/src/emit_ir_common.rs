use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};

use crate::ir_spans::DeriveSpanTable;
use eure_codegen_ir::{
    ConstParamIr, ContainerAttrsIr, IrModule, LifetimeParamIr, MapImplTypeIr, PrimitiveRustTypeIr,
    ProxyModeIr, RustBindingIr, RustPathIr, RustTypeExprIr, TypeDefIr, TypeParamIr, WrapperKindIr,
};

pub(crate) struct DeriveIrType<'a> {
    ty: &'a TypeDefIr,
    spans: &'a DeriveSpanTable,
}

impl<'a> DeriveIrType<'a> {
    pub(crate) fn single_root(
        module: &'a IrModule,
        spans: &'a DeriveSpanTable,
    ) -> syn::Result<Self> {
        if module.roots().len() != 1 {
            return Err(syn::Error::new(
                spans.derive_span,
                format!(
                    "derive IR must have exactly one root type, found {}",
                    module.roots().len()
                ),
            ));
        }

        let root_id = &module.roots()[0];
        let ty = module.types().get(root_id).ok_or_else(|| {
            syn::Error::new(
                spans.derive_span,
                format!(
                    "derive IR root type `{}` is missing from module.types()",
                    root_id.0
                ),
            )
        })?;

        Ok(Self { ty, spans })
    }

    pub(crate) fn ty(&self) -> &TypeDefIr {
        self.ty
    }

    pub(crate) fn binding(&self) -> &RustBindingIr {
        self.ty.rust_binding()
    }

    pub(crate) fn container(&self) -> &ContainerAttrsIr {
        self.ty.rust_binding().container()
    }

    pub(crate) fn ident(&self) -> syn::Result<syn::Ident> {
        parse_ident_with_span(
            self.ty.names().rust_name(),
            "type name",
            self.spans.derive_span,
        )
    }

    pub(crate) fn document_crate(&self) -> syn::Result<TokenStream> {
        match &self.container().crate_path() {
            Some(path) => parse_tokens_with_span(
                &path.path,
                "crate path",
                self.spans
                    .container_attr_span("crate")
                    .unwrap_or(self.spans.derive_span),
            ),
            None => Ok(quote! { ::eure::document }),
        }
    }

    pub(crate) fn schema_crate(&self) -> TokenStream {
        quote! { ::eure_schema }
    }

    pub(crate) fn parse_error_ty(&self, document_crate: &TokenStream) -> syn::Result<TokenStream> {
        match &self.container().parse_error() {
            Some(path) => parse_tokens_with_span(
                &path.path,
                "parse_error type",
                self.spans
                    .container_attr_span("parse_error")
                    .unwrap_or(self.spans.derive_span),
            ),
            None => Ok(quote! { #document_crate::parse::ParseError }),
        }
    }

    pub(crate) fn write_error_ty(&self, document_crate: &TokenStream) -> syn::Result<TokenStream> {
        match &self.container().write_error() {
            Some(path) => parse_tokens_with_span(
                &path.path,
                "write_error type",
                self.spans
                    .container_attr_span("write_error")
                    .unwrap_or(self.spans.derive_span),
            ),
            None => Ok(quote! { #document_crate::write::WriteError }),
        }
    }

    pub(crate) fn proxy_target_ty(&self) -> syn::Result<Option<TokenStream>> {
        match self.container().proxy_mode() {
            Some(ProxyModeIr::Transparent(path)) => Ok(Some(path_to_type_tokens(&path)?)),
            Some(ProxyModeIr::Opaque(path)) => Ok(Some(path_to_type_tokens_with_span(
                &path,
                self.spans
                    .container_attr_span("opaque")
                    .unwrap_or(self.spans.derive_span),
            )?)),
            None => Ok(None),
        }
    }

    pub(crate) fn opaque_target_ty(&self) -> syn::Result<Option<TokenStream>> {
        match self.container().proxy_mode() {
            Some(ProxyModeIr::Opaque(path)) => Ok(Some(path_to_type_tokens_with_span(
                &path,
                self.spans
                    .container_attr_span("opaque")
                    .unwrap_or(self.spans.derive_span),
            )?)),
            _ => Ok(None),
        }
    }

    pub(crate) fn target_constructor_ty(&self) -> syn::Result<TokenStream> {
        match self.container().proxy_mode() {
            Some(ProxyModeIr::Transparent(path)) => path_to_type_tokens(&path),
            _ => {
                let ident = self.ident()?;
                Ok(quote! { #ident })
            }
        }
    }

    pub(crate) fn field_span(&self, field_name: &str) -> Span {
        self.spans
            .field_span(field_name)
            .unwrap_or(self.spans.derive_span)
    }

    pub(crate) fn container_attr_span(&self, attr_name: &str) -> Span {
        self.spans
            .container_attr_span(attr_name)
            .unwrap_or(self.spans.derive_span)
    }

    pub(crate) fn field_ty_span(&self, field_name: &str) -> Span {
        self.spans
            .field_ty_span(field_name)
            .unwrap_or_else(|| self.field_span(field_name))
    }

    pub(crate) fn field_attr_span(&self, field_name: &str, attr_name: &str) -> Span {
        self.spans
            .field_attr_span(field_name, attr_name)
            .unwrap_or_else(|| self.field_span(field_name))
    }

    pub(crate) fn variant_span(&self, variant_name: &str) -> Span {
        self.spans
            .variant_span(variant_name)
            .unwrap_or(self.spans.derive_span)
    }

    pub(crate) fn variant_field_span(&self, variant_name: &str, field_name: &str) -> Span {
        self.spans
            .variant_field_span(variant_name, field_name)
            .unwrap_or_else(|| self.variant_span(variant_name))
    }

    pub(crate) fn variant_field_ty_span(&self, variant_name: &str, field_name: &str) -> Span {
        self.spans
            .variant_field_ty_span(variant_name, field_name)
            .unwrap_or_else(|| self.variant_field_span(variant_name, field_name))
    }

    pub(crate) fn variant_field_attr_span(
        &self,
        variant_name: &str,
        field_name: &str,
        attr_name: &str,
    ) -> Span {
        self.spans
            .variant_field_attr_span(variant_name, field_name, attr_name)
            .unwrap_or_else(|| self.variant_field_span(variant_name, field_name))
    }
}

pub(crate) fn path_to_type_tokens(path: &RustPathIr) -> syn::Result<TokenStream> {
    parse_tokens(&path.path, "RustPathIr")
}

pub(crate) fn path_to_type_tokens_with_span(
    path: &RustPathIr,
    span: Span,
) -> syn::Result<TokenStream> {
    parse_tokens_with_span(&path.path, "RustPathIr", span)
}

pub(crate) fn parse_ident(name: &str, context: &str) -> syn::Result<syn::Ident> {
    syn::parse_str::<syn::Ident>(name).map_err(|_| {
        syn::Error::new(
            Span::call_site(),
            format!("invalid identifier `{name}` in {context}"),
        )
    })
}

pub(crate) fn parse_ident_with_span(
    name: &str,
    context: &str,
    span: Span,
) -> syn::Result<syn::Ident> {
    let mut ident = parse_ident(name, context)?;
    ident.set_span(span);
    Ok(ident)
}

pub(crate) fn parse_tokens(source: &str, context: &str) -> syn::Result<TokenStream> {
    syn::parse_str::<TokenStream>(source).map_err(|err| {
        syn::Error::new(
            Span::call_site(),
            format!("failed to parse `{source}` as {context}: {err}"),
        )
    })
}

pub(crate) fn parse_tokens_with_span(
    source: &str,
    context: &str,
    span: Span,
) -> syn::Result<TokenStream> {
    parse_tokens(source, context).map(|tokens| with_span(tokens, span))
}

pub(crate) fn with_span(tokens: TokenStream, span: Span) -> TokenStream {
    tokens
        .into_iter()
        .map(|token| with_span_tree(token, span))
        .collect()
}

fn with_span_tree(token: TokenTree, span: Span) -> TokenTree {
    match token {
        TokenTree::Group(group) => {
            let stream = group
                .stream()
                .into_iter()
                .map(|child| with_span_tree(child, span))
                .collect();
            let mut new_group = Group::new(group.delimiter(), stream);
            new_group.set_span(span);
            TokenTree::Group(new_group)
        }
        mut other => {
            other.set_span(span);
            other
        }
    }
}

fn type_param_tokens(param: &TypeParamIr) -> syn::Result<TokenStream> {
    let ident = parse_ident(&param.name, "type parameter")?;
    let bounds = param
        .bounds
        .iter()
        .map(|bound| parse_tokens(bound, "type parameter bound"))
        .collect::<syn::Result<Vec<_>>>()?;

    if bounds.is_empty() {
        Ok(quote! { #ident })
    } else {
        Ok(quote! { #ident: #(#bounds)+* })
    }
}

fn lifetime_param_tokens(param: &LifetimeParamIr) -> syn::Result<TokenStream> {
    let lifetime = syn::parse_str::<syn::Lifetime>(&param.name).map_err(|_| {
        syn::Error::new(
            Span::call_site(),
            format!("invalid lifetime parameter `{}`", param.name),
        )
    })?;
    let bounds = param
        .bounds
        .iter()
        .map(|bound| {
            syn::parse_str::<syn::Lifetime>(bound).map_err(|_| {
                syn::Error::new(
                    Span::call_site(),
                    format!("invalid lifetime bound `{bound}` on `{}`", param.name),
                )
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    if bounds.is_empty() {
        Ok(quote! { #lifetime })
    } else {
        Ok(quote! { #lifetime: #(#bounds)+* })
    }
}

fn const_param_tokens(param: &ConstParamIr) -> syn::Result<TokenStream> {
    let ident = parse_ident(&param.name, "const parameter")?;
    let ty = parse_tokens(&param.ty, "const parameter type")?;
    Ok(quote! { const #ident: #ty })
}

fn impl_generics(binding: &RustBindingIr) -> syn::Result<Vec<TokenStream>> {
    let lifetimes = binding
        .generics()
        .lifetime_params
        .iter()
        .map(lifetime_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let consts = binding
        .generics()
        .const_params
        .iter()
        .map(const_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let types = binding
        .generics()
        .type_params
        .iter()
        .map(type_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(lifetimes
        .into_iter()
        .chain(consts)
        .chain(types)
        .collect::<Vec<_>>())
}

fn for_generics(binding: &RustBindingIr) -> syn::Result<Vec<TokenStream>> {
    let lifetimes = binding
        .generics()
        .lifetime_params
        .iter()
        .map(|param| {
            syn::parse_str::<syn::Lifetime>(&param.name)
                .map(|lt| quote! { #lt })
                .map_err(|_| {
                    syn::Error::new(
                        Span::call_site(),
                        format!("invalid lifetime parameter `{}`", param.name),
                    )
                })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let consts = binding
        .generics()
        .const_params
        .iter()
        .map(|param| {
            parse_ident(&param.name, "const parameter name").map(|ident| quote! { #ident })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let types = binding
        .generics()
        .type_params
        .iter()
        .map(|param| parse_ident(&param.name, "type parameter name").map(|ident| quote! { #ident }))
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(lifetimes
        .into_iter()
        .chain(consts)
        .chain(types)
        .collect::<Vec<_>>())
}

fn type_params(binding: &RustBindingIr) -> syn::Result<Vec<syn::Ident>> {
    binding
        .generics()
        .type_params
        .iter()
        .map(|param| parse_ident(&param.name, "type parameter"))
        .collect()
}

fn impl_generics_with_parse_document_bounds(
    binding: &RustBindingIr,
    parse_document: &TokenStream,
) -> syn::Result<Vec<TokenStream>> {
    let lifetimes = binding
        .generics()
        .lifetime_params
        .iter()
        .map(lifetime_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let consts = binding
        .generics()
        .const_params
        .iter()
        .map(const_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let types = binding
        .generics()
        .type_params
        .iter()
        .map(|param| {
            let ident = parse_ident(&param.name, "type parameter")?;
            let bounds = param
                .bounds
                .iter()
                .map(|bound| parse_tokens(bound, "type parameter bound"))
                .collect::<syn::Result<Vec<_>>>()?;
            if bounds.is_empty() {
                Ok(quote! { #ident: #parse_document<'doc> })
            } else {
                Ok(quote! { #ident: #(#bounds)+* + #parse_document<'doc> })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(lifetimes
        .into_iter()
        .chain(consts)
        .chain(types)
        .collect::<Vec<_>>())
}

fn impl_generics_with_unified_error_bounds(
    binding: &RustBindingIr,
    parse_document: &TokenStream,
    error_ty: &TokenStream,
) -> syn::Result<Vec<TokenStream>> {
    let lifetimes = binding
        .generics()
        .lifetime_params
        .iter()
        .map(lifetime_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let consts = binding
        .generics()
        .const_params
        .iter()
        .map(const_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let types = binding
        .generics()
        .type_params
        .iter()
        .map(|param| {
            let ident = parse_ident(&param.name, "type parameter")?;
            let bounds = param
                .bounds
                .iter()
                .map(|bound| parse_tokens(bound, "type parameter bound"))
                .collect::<syn::Result<Vec<_>>>()?;
            if bounds.is_empty() {
                Ok(quote! { #ident: #parse_document<'doc, Error = #error_ty> })
            } else {
                Ok(quote! { #ident: #(#bounds)+* + #parse_document<'doc, Error = #error_ty> })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(lifetimes
        .into_iter()
        .chain(consts)
        .chain(types)
        .collect::<Vec<_>>())
}

fn impl_generics_with_into_eure_bounds(
    binding: &RustBindingIr,
    into_eure: &TokenStream,
) -> syn::Result<Vec<TokenStream>> {
    let lifetimes = binding
        .generics()
        .lifetime_params
        .iter()
        .map(lifetime_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let consts = binding
        .generics()
        .const_params
        .iter()
        .map(const_param_tokens)
        .collect::<syn::Result<Vec<_>>>()?;
    let types = binding
        .generics()
        .type_params
        .iter()
        .map(|param| {
            let ident = parse_ident(&param.name, "type parameter")?;
            let bounds = param
                .bounds
                .iter()
                .map(|bound| parse_tokens(bound, "type parameter bound"))
                .collect::<syn::Result<Vec<_>>>()?;
            if bounds.is_empty() {
                Ok(quote! { #ident: #into_eure })
            } else {
                Ok(quote! { #ident: #(#bounds)+* + #into_eure })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(lifetimes
        .into_iter()
        .chain(consts)
        .chain(types)
        .collect::<Vec<_>>())
}

fn type_with_for_generics(ident: &syn::Ident, for_generics: &[TokenStream]) -> TokenStream {
    if for_generics.is_empty() {
        quote! { #ident }
    } else {
        quote! { #ident<#(#for_generics),*> }
    }
}

pub(crate) fn impl_build_schema(
    emit: &DeriveIrType<'_>,
    build_body: TokenStream,
) -> syn::Result<TokenStream> {
    let ident = emit.ident()?;
    let binding = emit.binding();
    let for_generics = for_generics(binding)?;
    let schema_crate = emit.schema_crate();

    let impl_generics_with_bounds = binding
        .generics()
        .lifetime_params
        .iter()
        .map(lifetime_param_tokens)
        .chain(
            binding
                .generics()
                .const_params
                .iter()
                .map(const_param_tokens),
        )
        .chain(binding.generics().type_params.iter().map(|param| {
            let ident = parse_ident(&param.name, "type parameter")?;
            let bounds = param
                .bounds
                .iter()
                .map(|bound| parse_tokens(bound, "type parameter bound"))
                .collect::<syn::Result<Vec<_>>>()?;
            if bounds.is_empty() {
                Ok(quote! { #ident: #schema_crate::BuildSchema + 'static })
            } else {
                Ok(quote! { #ident: #(#bounds)+* + #schema_crate::BuildSchema + 'static })
            }
        }))
        .collect::<syn::Result<Vec<_>>>()?;

    let type_name_impl = if let Some(name) = &emit.container().type_name() {
        quote! {
            fn type_name() -> Option<&'static str> {
                Some(#name)
            }
        }
    } else {
        quote! {}
    };

    if impl_generics_with_bounds.is_empty() {
        Ok(quote! {
            impl #schema_crate::BuildSchema for #ident {
                #type_name_impl

                fn build_schema(ctx: &mut #schema_crate::SchemaBuilder) -> #schema_crate::SchemaNodeContent {
                    use #schema_crate::BuildSchema;
                    #build_body
                }
            }
        })
    } else {
        let ty = type_with_for_generics(&ident, &for_generics);
        Ok(quote! {
            impl<#(#impl_generics_with_bounds),*> #schema_crate::BuildSchema for #ty {
                #type_name_impl

                fn build_schema(ctx: &mut #schema_crate::SchemaBuilder) -> #schema_crate::SchemaNodeContent {
                    use #schema_crate::BuildSchema;
                    #build_body
                }
            }
        })
    }
}

pub(crate) fn impl_from_eure(
    emit: &DeriveIrType<'_>,
    parse_body: TokenStream,
) -> syn::Result<TokenStream> {
    let ident = emit.ident()?;
    let binding = emit.binding();
    let for_generics = for_generics(binding)?;
    let document_crate = emit.document_crate()?;
    let parse_document = quote! { #document_crate::parse::FromEure };
    let parse_context = quote! { #document_crate::parse::ParseContext };
    let parse_error = emit.parse_error_ty(&document_crate)?;
    let type_params = type_params(binding)?;
    let has_custom_error = emit.container().parse_error().is_some();
    let target = emit.proxy_target_ty()?;

    let trait_sig = match &target {
        Some(remote) => quote! { #parse_document<'doc, #remote> },
        None => quote! { #parse_document<'doc> },
    };
    let return_type = match &target {
        Some(remote) => quote! { #remote },
        None => quote! { Self },
    };
    let ty = type_with_for_generics(&ident, &for_generics);

    if type_params.is_empty() {
        let impl_generics = impl_generics(binding)?;
        if impl_generics.is_empty() {
            Ok(quote! {
                impl<'doc> #trait_sig for #ty {
                    type Error = #parse_error;

                    fn parse(ctx: &#parse_context<'doc>) -> Result<#return_type, Self::Error> {
                        #parse_body
                    }
                }
            })
        } else {
            Ok(quote! {
                impl<'doc, #(#impl_generics),*> #trait_sig for #ty {
                    type Error = #parse_error;

                    fn parse(ctx: &#parse_context<'doc>) -> Result<#return_type, Self::Error> {
                        #parse_body
                    }
                }
            })
        }
    } else if has_custom_error {
        let base_generics = impl_generics_with_parse_document_bounds(binding, &parse_document)?;
        let from_bounds = type_params
            .iter()
            .map(|ident| quote! { #parse_error: From<<#ident as #parse_document<'doc>>::Error> })
            .collect::<Vec<_>>();
        Ok(quote! {
            impl<'doc, #(#base_generics),*> #trait_sig for #ty
            where
                #(#from_bounds),*
            {
                type Error = #parse_error;

                fn parse(ctx: &#parse_context<'doc>) -> Result<#return_type, Self::Error> {
                    #parse_body
                }
            }
        })
    } else {
        let base_generics =
            impl_generics_with_unified_error_bounds(binding, &parse_document, &parse_error)?;
        Ok(quote! {
            impl<'doc, #(#base_generics),*> #trait_sig for #ty {
                type Error = #parse_error;

                fn parse(ctx: &#parse_context<'doc>) -> Result<#return_type, Self::Error> {
                    #parse_body
                }
            }
        })
    }
}

pub(crate) fn impl_into_eure(
    emit: &DeriveIrType<'_>,
    write_body: TokenStream,
    flatten_body: Option<TokenStream>,
    extra_where: &[TokenStream],
) -> syn::Result<TokenStream> {
    let ident = emit.ident()?;
    let binding = emit.binding();
    let for_generics = for_generics(binding)?;
    let document_crate = emit.document_crate()?;
    let into_eure = quote! { #document_crate::write::IntoEure };
    let write_error = emit.write_error_ty(&document_crate)?;
    let document_constructor = quote! { #document_crate::constructor::DocumentConstructor };
    let record_writer = quote! { #document_crate::write::RecordWriter };
    let target = emit.proxy_target_ty()?;
    let type_params = type_params(binding)?;
    let ty = type_with_for_generics(&ident, &for_generics);

    let mut where_preds = extra_where.to_vec();
    where_preds.extend(type_params.iter().map(
        |ident| quote! { #write_error: ::core::convert::From<<#ident as #into_eure>::Error> },
    ));
    let where_clause = if where_preds.is_empty() {
        quote! {}
    } else {
        quote! { where #(#where_preds),* }
    };

    let trait_sig = match &target {
        Some(remote) => quote! { #into_eure<#remote> },
        None => quote! { #into_eure },
    };
    let value_ty = match &target {
        Some(remote) => quote! { #remote },
        None => quote! { Self },
    };

    let flatten_method = if let Some(flatten_body) = flatten_body {
        quote! {
            fn write_flatten(value: #value_ty, rec: &mut #record_writer<'_>) -> ::core::result::Result<(), Self::Error> {
                #flatten_body
            }
        }
    } else {
        quote! {}
    };

    if type_params.is_empty() {
        let impl_generics = impl_generics(binding)?;
        if impl_generics.is_empty() {
            Ok(quote! {
                impl #trait_sig for #ty #where_clause {
                    type Error = #write_error;

                    fn write(value: #value_ty, c: &mut #document_constructor) -> ::core::result::Result<(), Self::Error> {
                        #write_body
                    }

                    #flatten_method
                }
            })
        } else {
            Ok(quote! {
                impl<#(#impl_generics),*> #trait_sig for #ty #where_clause {
                    type Error = #write_error;

                    fn write(value: #value_ty, c: &mut #document_constructor) -> ::core::result::Result<(), Self::Error> {
                        #write_body
                    }

                    #flatten_method
                }
            })
        }
    } else {
        let base_generics = impl_generics_with_into_eure_bounds(binding, &into_eure)?;
        Ok(quote! {
            impl<#(#base_generics),*> #trait_sig for #ty #where_clause {
                type Error = #write_error;

                fn write(value: #value_ty, c: &mut #document_constructor) -> ::core::result::Result<(), Self::Error> {
                    #write_body
                }

                #flatten_method
            }
        })
    }
}

pub(crate) fn rust_type_tokens(
    ty: &RustTypeExprIr,
    document_crate: &TokenStream,
) -> syn::Result<TokenStream> {
    match ty {
        RustTypeExprIr::Primitive(primitive) => match primitive {
            PrimitiveRustTypeIr::String => Ok(quote! { String }),
            PrimitiveRustTypeIr::Bool => Ok(quote! { bool }),
            PrimitiveRustTypeIr::Unit => Ok(quote! { () }),
            PrimitiveRustTypeIr::Text => Ok(quote! { #document_crate::text::Text }),
            PrimitiveRustTypeIr::I8 => Ok(quote! { i8 }),
            PrimitiveRustTypeIr::I16 => Ok(quote! { i16 }),
            PrimitiveRustTypeIr::I32 => Ok(quote! { i32 }),
            PrimitiveRustTypeIr::I64 => Ok(quote! { i64 }),
            PrimitiveRustTypeIr::I128 => Ok(quote! { i128 }),
            PrimitiveRustTypeIr::Isize => Ok(quote! { isize }),
            PrimitiveRustTypeIr::U8 => Ok(quote! { u8 }),
            PrimitiveRustTypeIr::U16 => Ok(quote! { u16 }),
            PrimitiveRustTypeIr::U32 => Ok(quote! { u32 }),
            PrimitiveRustTypeIr::U64 => Ok(quote! { u64 }),
            PrimitiveRustTypeIr::U128 => Ok(quote! { u128 }),
            PrimitiveRustTypeIr::Usize => Ok(quote! { usize }),
            PrimitiveRustTypeIr::F32 => Ok(quote! { f32 }),
            PrimitiveRustTypeIr::F64 => Ok(quote! { f64 }),
            PrimitiveRustTypeIr::Any => Err(syn::Error::new(
                Span::call_site(),
                "cannot emit derive code for RustTypeExprIr::Primitive(Any)",
            )),
        },
        RustTypeExprIr::Named(name) => {
            let path = if let Some(namespace) = &name.namespace {
                format!("{namespace}::{}", name.name)
            } else {
                name.name.clone()
            };
            parse_tokens(&path, "named Rust type")
        }
        RustTypeExprIr::Path(path) => parse_tokens(&path.path, "Rust path type"),
        RustTypeExprIr::GenericParam(name) => {
            let ident = parse_ident(name, "generic parameter usage")?;
            Ok(quote! { #ident })
        }
        RustTypeExprIr::Option(inner) => {
            let inner = rust_type_tokens(inner, document_crate)?;
            Ok(quote! { ::core::option::Option<#inner> })
        }
        RustTypeExprIr::Vec(inner) => {
            let inner = rust_type_tokens(inner, document_crate)?;
            Ok(quote! { ::std::vec::Vec<#inner> })
        }
        RustTypeExprIr::Map {
            key,
            value,
            impl_type,
        } => {
            let key = rust_type_tokens(key, document_crate)?;
            let value = rust_type_tokens(value, document_crate)?;
            match impl_type {
                MapImplTypeIr::HashMap => Ok(quote! { ::std::collections::HashMap<#key, #value> }),
                MapImplTypeIr::BTreeMap => {
                    Ok(quote! { ::std::collections::BTreeMap<#key, #value> })
                }
                MapImplTypeIr::IndexMap => Ok(quote! { ::indexmap::IndexMap<#key, #value> }),
            }
        }
        RustTypeExprIr::Tuple(elements) => {
            let elements = elements
                .iter()
                .map(|element| rust_type_tokens(element, document_crate))
                .collect::<syn::Result<Vec<_>>>()?;
            if elements.is_empty() {
                Ok(quote! { () })
            } else if elements.len() == 1 {
                let element = &elements[0];
                Ok(quote! { (#element,) })
            } else {
                Ok(quote! { (#(#elements),*) })
            }
        }
        RustTypeExprIr::Result { ok, err } => {
            let ok = rust_type_tokens(ok, document_crate)?;
            let err = rust_type_tokens(err, document_crate)?;
            Ok(quote! { ::core::result::Result<#ok, #err> })
        }
        RustTypeExprIr::Wrapper { inner, wrapper } => {
            let inner = rust_type_tokens(inner, document_crate)?;
            match wrapper {
                WrapperKindIr::Box => Ok(quote! { ::std::boxed::Box<#inner> }),
                WrapperKindIr::Rc => Ok(quote! { ::std::rc::Rc<#inner> }),
                WrapperKindIr::Arc => Ok(quote! { ::std::sync::Arc<#inner> }),
            }
        }
    }
}

pub(crate) fn rust_type_tokens_with_span(
    ty: &RustTypeExprIr,
    document_crate: &TokenStream,
    span: Span,
) -> syn::Result<TokenStream> {
    rust_type_tokens(ty, document_crate).map(|tokens| with_span(tokens, span))
}

pub(crate) fn field_ident_with_span(name: &str, span: Span) -> syn::Result<syn::Ident> {
    parse_ident_with_span(name, "field name", span)
}

pub(crate) fn tuple_binding_ident(index: usize) -> syn::Ident {
    format_ident!("field_{}", index)
}

pub(crate) fn tuple_binding_ident_with_span(index: usize, span: Span) -> syn::Ident {
    let mut ident = tuple_binding_ident(index);
    ident.set_span(span);
    ident
}

pub(crate) fn field_span(
    emit: &DeriveIrType<'_>,
    field: &eure_codegen_ir::RustFieldIr,
    variant_name: Option<&str>,
) -> Span {
    match variant_name {
        Some(variant_name) => emit.variant_field_span(variant_name, field.rust_name()),
        None => emit.field_span(field.rust_name()),
    }
}

pub(crate) fn field_ty_span(
    emit: &DeriveIrType<'_>,
    field: &eure_codegen_ir::RustFieldIr,
    variant_name: Option<&str>,
) -> Span {
    match variant_name {
        Some(variant_name) => emit.variant_field_ty_span(variant_name, field.rust_name()),
        None => emit.field_ty_span(field.rust_name()),
    }
}

pub(crate) fn field_attr_span(
    emit: &DeriveIrType<'_>,
    field: &eure_codegen_ir::RustFieldIr,
    variant_name: Option<&str>,
    attr_name: &str,
) -> Span {
    match variant_name {
        Some(variant_name) => {
            emit.variant_field_attr_span(variant_name, field.rust_name(), attr_name)
        }
        None => emit.field_attr_span(field.rust_name(), attr_name),
    }
}
