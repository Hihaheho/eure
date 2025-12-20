use proc_macro2::TokenStream;
use quote::quote;

pub struct MacroConfig {
    pub document_crate: TokenStream,
}

impl Default for MacroConfig {
    fn default() -> Self {
        Self {
            document_crate: quote! { ::eure::document },
        }
    }
}
