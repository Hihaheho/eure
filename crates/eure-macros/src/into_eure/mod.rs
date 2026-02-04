mod write_record;
mod write_union;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;

use crate::context::MacroContext;

pub fn derive(context: MacroContext) -> TokenStream {
    match &context.input.data {
        Data::Struct(data) => write_record::generate_record_writer(&context, data),
        Data::Union(_) => quote! { compile_error!("Union is not supported yet") },
        Data::Enum(data) => write_union::generate_union_writer(&context, data),
    }
}
