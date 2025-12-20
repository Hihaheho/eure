#![expect(unused)]

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Generics, Ident};

use crate::{config::MacroConfig, context::MacroContext};

pub fn generate_record_parser(context: &MacroContext, input: &DataStruct) -> TokenStream {
    let ident = context.ident();
    let MacroConfig { document_crate, .. } = &context.config;
    quote! {}
}
