use darling::FromDeriveInput;
use syn::parse_macro_input;

use crate::{attrs::ContainerAttrs, config::MacroConfig, context::MacroContext};

mod attrs;
mod build_schema;
pub(crate) mod config;
pub(crate) mod context;
mod into_document;
mod parse_document;

#[proc_macro_derive(IntoDocument, attributes(eure))]
pub fn into_document_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    into_document::derive(create_context(input)).into()
}

#[proc_macro_derive(ParseDocument, attributes(eure))]
pub fn parse_document_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    parse_document::derive(create_context(input)).into()
}

#[proc_macro_derive(BuildSchema, attributes(eure))]
pub fn build_schema_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    build_schema::derive(create_context(input)).into()
}

fn create_context(input: syn::DeriveInput) -> MacroContext {
    let attrs = ContainerAttrs::from_derive_input(&input).expect("Failed to parse eure attributes");
    MacroContext::new(MacroConfig::from_attrs(attrs), input)
}
