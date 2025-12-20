mod parse_record;
mod parse_union;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;

use crate::context::MacroContext;

pub fn derive(context: MacroContext) -> TokenStream {
    match &context.input.data {
        Data::Struct(data) => parse_record::generate_record_parser(&context, data),
        Data::Union(_) => quote! { compile_error!("Union is not supported yet") },
        Data::Enum(data) => parse_union::generate_union_parser(&context, data),
    }
}
