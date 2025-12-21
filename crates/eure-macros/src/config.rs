use proc_macro2::TokenStream;
use quote::quote;

use crate::attrs::ContainerAttrs;

pub struct MacroConfig {
    pub document_crate: TokenStream,
}

impl MacroConfig {
    pub fn from_attrs(attrs: ContainerAttrs) -> Self {
        use quote::ToTokens;
        let document_crate = attrs
            .crate_path
            .map(|path| path.into_token_stream())
            .unwrap_or_else(|| quote! { ::eure::document });
        Self { document_crate }
    }

    #[allow(non_snake_case)]
    pub fn ParseDocument(&self) -> TokenStream {
        let document_crate = &self.document_crate;
        quote!(#document_crate::parse::ParseDocument)
    }

    #[allow(non_snake_case)]
    pub fn ParseError(&self) -> TokenStream {
        let document_crate = &self.document_crate;
        quote!(#document_crate::parse::ParseError)
    }

    #[allow(non_snake_case)]
    pub fn ParseContext(&self) -> TokenStream {
        let document_crate = &self.document_crate;
        quote!(#document_crate::parse::ParseContext)
    }
}
