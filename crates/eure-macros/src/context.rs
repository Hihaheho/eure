use convert_case::Casing as _;
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

    /// Returns the type to construct when parsing.
    ///
    /// If `remote` is set, returns the remote type; otherwise returns `Self`.
    pub fn target_type(&self) -> TokenStream {
        if let Some(ref remote) = self.config.remote {
            quote! { #remote }
        } else {
            let ident = self.ident();
            quote! { #ident }
        }
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
    pub fn FromEure(&self) -> TokenStream {
        let document_crate = &self.config.document_crate;
        quote!(#document_crate::parse::FromEure)
    }

    #[allow(non_snake_case)]
    pub fn ParseError(&self) -> TokenStream {
        if let Some(ref custom_error) = self.config.parse_error {
            custom_error.clone()
        } else {
            let document_crate = &self.config.document_crate;
            quote!(#document_crate::parse::ParseError)
        }
    }

    #[allow(non_snake_case)]
    pub fn ParseContext(&self) -> TokenStream {
        let document_crate = &self.config.document_crate;
        quote!(#document_crate::parse::ParseContext)
    }

    #[allow(non_snake_case)]
    pub fn VariantLiteralParser(&self, value: TokenStream, mapper: TokenStream) -> TokenStream {
        let document_crate = &self.config.document_crate;
        quote!(#document_crate::parse::DocumentParserExt::map(#document_crate::parse::VariantLiteralParser(#value), #mapper))
    }

    /// Applies container-level `rename_all` to a name.
    /// For structs: renames fields. For enums: renames variants.
    pub fn apply_rename(&self, name: &str) -> String {
        match self.config.rename_all {
            Some(rename_all) => name.to_case(rename_all.to_case()),
            None => name.to_string(),
        }
    }

    /// Applies `rename_all_fields` to a field name in an enum struct variant.
    /// This is separate from `rename_all` which only affects variant names in enums.
    pub fn apply_field_rename(&self, name: &str) -> String {
        match self.config.rename_all_fields {
            Some(rename_all_fields) => name.to_case(rename_all_fields.to_case()),
            None => name.to_string(),
        }
    }

    pub fn impl_from_eure(&self, parse_body: TokenStream) -> TokenStream {
        // Delegate to impl_from_eure_for with appropriate target type
        if let Some(ref remote) = self.config.remote {
            self.impl_from_eure_for(parse_body, Some(quote! { #remote }))
        } else {
            // For non-remote types, target defaults to Self (omit second type param)
            self.impl_from_eure_for(parse_body, None)
        }
    }

    /// Generate FromEure implementation with specified target type.
    ///
    /// When `target_type` is `None`, this generates standard `FromEure<'doc>`.
    /// When `target_type` is `Some(T)`, this generates `FromEure<'doc, T>`.
    fn impl_from_eure_for(
        &self,
        parse_body: TokenStream,
        target_type: Option<TokenStream>,
    ) -> TokenStream {
        let ident = self.ident();
        let for_generics = self.for_generics();
        let parse_document = self.FromEure();
        let parse_context = self.ParseContext();
        let parse_error = self.ParseError();

        let type_params: Vec<_> = self.generics().type_params().collect();
        let has_custom_error = self.config.parse_error.is_some();

        // Trait signature: FromEure<'doc> or FromEure<'doc, RemoteType>
        let trait_sig = match &target_type {
            Some(remote) => quote! { #parse_document<'doc, #remote> },
            None => quote! { #parse_document<'doc> },
        };

        // Return type: Self or RemoteType
        let return_type = match &target_type {
            Some(remote) => quote! { #remote },
            None => quote! { Self },
        };

        // Build impl generics based on the number of type parameters and error configuration
        if type_params.is_empty() {
            // No type parameters: use default or custom error
            let impl_generics = self.impl_generics();
            quote! {
                impl<'doc, #(#impl_generics),*> #trait_sig for #ident<#(#for_generics),*> {
                    type Error = #parse_error;

                    fn parse(ctx: &#parse_context<'doc>) -> Result<#return_type, Self::Error> {
                        #parse_body
                    }
                }
            }
        } else if has_custom_error {
            // Custom error specified: add FromEure bounds and CustomErr: From<T::Error> bounds
            let base_generics = self.impl_generics_with_parse_document_bounds();
            let from_bounds: Vec<_> = type_params
                .iter()
                .map(|tp| {
                    let ident = &tp.ident;
                    quote! { #parse_error: From<<#ident as #parse_document<'doc>>::Error> }
                })
                .collect();
            quote! {
                impl<'doc, #(#base_generics),*> #trait_sig for #ident<#(#for_generics),*>
                where
                    #(#from_bounds),*
                {
                    type Error = #parse_error;

                    fn parse(ctx: &#parse_context<'doc>) -> Result<#return_type, Self::Error> {
                        #parse_body
                    }
                }
            }
        } else {
            // Generic type parameters: require all to have Error = ParseError
            // This ensures compatibility with the existing eure-document API constraints
            let base_generics = self.impl_generics_with_unified_error_bounds(parse_error.clone());
            quote! {
                impl<'doc, #(#base_generics),*> #trait_sig for #ident<#(#for_generics),*> {
                    type Error = #parse_error;

                    fn parse(ctx: &#parse_context<'doc>) -> Result<#return_type, Self::Error> {
                        #parse_body
                    }
                }
            }
        }
    }

    /// Returns impl generics with FromEure<'doc> bounds added to type parameters.
    fn impl_generics_with_parse_document_bounds(&self) -> Vec<TokenStream> {
        let parse_document = self.FromEure();
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
                    if bounds.is_empty() {
                        quote! { #ident: #parse_document<'doc> }
                    } else {
                        quote! { #ident #colon_token #bounds + #parse_document<'doc> }
                    }
                },
            ))
            .collect()
    }

    /// Returns impl generics with unified error type bounds for multiple type parameters.
    fn impl_generics_with_unified_error_bounds(&self, error_type: TokenStream) -> Vec<TokenStream> {
        let parse_document = self.FromEure();
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
                    if bounds.is_empty() {
                        quote! { #ident: #parse_document<'doc, Error = #error_type> }
                    } else {
                        quote! { #ident #colon_token #bounds + #parse_document<'doc, Error = #error_type> }
                    }
                },
            ))
            .collect()
    }

    // ========================================================================
    // BuildSchema helpers
    // ========================================================================

    /// Returns the path to the schema crate (eure_schema)
    pub fn schema_crate(&self) -> TokenStream {
        // For now, always use ::eure_schema
        // In the future, this could be configurable via an attribute
        quote!(::eure_schema)
    }

    /// Generates the BuildSchema impl block
    pub fn impl_build_schema(&self, build_body: TokenStream) -> TokenStream {
        let ident = self.ident();
        let impl_generics = self.impl_generics();
        let for_generics = self.for_generics();
        let schema_crate = self.schema_crate();

        // Add BuildSchema + 'static bounds to type parameters
        let impl_generics_with_bounds: Vec<_> = self
            .generics()
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
                    if bounds.is_empty() {
                        quote! { #ident: #schema_crate::BuildSchema + 'static }
                    } else {
                        quote! { #ident #colon_token #bounds + #schema_crate::BuildSchema + 'static }
                    }
                },
            ))
            .collect();

        // Generate type_name() if configured
        let type_name_impl = if let Some(ref name) = self.config.type_name {
            quote! {
                fn type_name() -> Option<&'static str> {
                    Some(#name)
                }
            }
        } else {
            quote! {}
        };

        // Handle empty generics case
        if impl_generics.is_empty() {
            quote! {
                impl #schema_crate::BuildSchema for #ident {
                    #type_name_impl

                    fn build_schema(ctx: &mut #schema_crate::SchemaBuilder) -> #schema_crate::SchemaNodeContent {
                        use #schema_crate::BuildSchema;
                        #build_body
                    }
                }
            }
        } else {
            quote! {
                impl<#(#impl_generics_with_bounds),*> #schema_crate::BuildSchema for #ident<#(#for_generics),*> {
                    #type_name_impl

                    fn build_schema(ctx: &mut #schema_crate::SchemaBuilder) -> #schema_crate::SchemaNodeContent {
                        use #schema_crate::BuildSchema;
                        #build_body
                    }
                }
            }
        }
    }
}
