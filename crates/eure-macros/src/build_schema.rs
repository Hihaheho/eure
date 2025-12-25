//! BuildSchema derive macro implementation

mod build_record;
mod build_union;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;

use crate::context::MacroContext;

pub fn derive(context: MacroContext) -> TokenStream {
    match &context.input.data {
        Data::Struct(data) => build_record::generate_record_schema(&context, data),
        Data::Union(_) => quote! { compile_error!("Union is not supported for BuildSchema") },
        Data::Enum(data) => build_union::generate_union_schema(&context, data),
    }
}
