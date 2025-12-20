use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::context::MacroContext;

pub fn derive(context: MacroContext) -> TokenStream {
    TokenStream::new()
}
