use proc_macro2::TokenStream;
use quote::quote;
use syn::{ConstParam, DeriveInput, Generics, Ident, LifetimeParam, TypeParam};

use crate::config::MacroConfig;

pub struct MacroContext {
    pub config: MacroConfig,
    pub input: DeriveInput,
}

impl MacroContext {
    pub fn new(config: MacroConfig, input: DeriveInput) -> Self {
        Self { config, input }
    }

    pub fn ident(&self) -> &Ident {
        &self.input.ident
    }

    pub fn generics(&self) -> &Generics {
        &self.input.generics
    }

    /// Returns generics for the impl<...>
    pub fn impl_generics(&self) -> Vec<TokenStream> {
        self.generics()
            .lifetimes()
            .map(
                |LifetimeParam {
                     lifetime,
                     colon_token,
                     bounds,
                     ..
                 }| {
                    quote! { #lifetime #colon_token #bounds }
                },
            )
            .chain(self.generics().const_params().map(
                |ConstParam {
                     const_token,
                     colon_token,
                     ty,
                     ..
                 }| {
                    quote! { #const_token #colon_token #ty }
                },
            ))
            .chain(self.generics().type_params().map(
                |TypeParam {
                     ident,
                     colon_token,
                     bounds,
                     ..
                 }| {
                    quote! { #ident #colon_token #bounds }
                },
            ))
            .collect()
    }

    /// Returns generics for the for #ident<...>
    pub fn for_generics(&self) -> Vec<TokenStream> {
        self.generics()
            .lifetimes()
            .map(|LifetimeParam { lifetime, .. }| {
                quote! { #lifetime }
            })
            .chain(
                self.generics()
                    .const_params()
                    .map(|ConstParam { const_token, .. }| {
                        quote! { #const_token }
                    }),
            )
            .chain(
                self.generics()
                    .type_params()
                    .map(|TypeParam { ident, .. }| {
                        quote! { #ident }
                    }),
            )
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn ParseDocument(&self) -> TokenStream {
        let document_crate = &self.config.document_crate;
        quote!(#document_crate::parse::ParseDocument)
    }

    #[allow(non_snake_case)]
    pub fn ParseError(&self) -> TokenStream {
        let document_crate = &self.config.document_crate;
        quote!(#document_crate::parse::ParseError)
    }

    #[allow(non_snake_case)]
    pub fn ParseContext(&self) -> TokenStream {
        let document_crate = &self.config.document_crate;
        quote!(#document_crate::parse::ParseContext)
    }

    pub fn impl_parse_document(&self, parse_body: TokenStream) -> TokenStream {
        let ident = self.ident();
        let impl_generics = self.impl_generics();
        let for_generics = self.for_generics();
        let parse_document = self.ParseDocument();
        let parse_error = self.ParseError();
        let parse_context = self.ParseContext();
        quote! {
            impl<'doc, #(#impl_generics),*> #parse_document<'doc> for #ident<#(#for_generics),*> {
                type Error = #parse_error;

                fn parse(ctx: &#parse_context<'doc>) -> Result<Self, Self::Error> {
                    #parse_body
                }
            }
        }
    }
}
