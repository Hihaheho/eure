use syn::parse_macro_input;

pub(crate) mod config;
pub(crate) mod context;
mod into_document;
mod parse_document;

#[proc_macro_derive(IntoDocument, attributes(eure))]
pub fn into_document_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let context = context::MacroContext::new(config::MacroConfig::default(), input);
    into_document::derive(context).into()
}

#[proc_macro_derive(ParseDocument, attributes(eure))]
pub fn parse_document_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let context = context::MacroContext::new(config::MacroConfig::default(), input);
    parse_document::derive(context).into()
}
