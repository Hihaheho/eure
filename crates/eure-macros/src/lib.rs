use darling::FromDeriveInput;
use syn::parse_macro_input;

use crate::attrs::{ContainerAttrs, extract_container_attr_spans};
use crate::config::MacroConfig;
use crate::context::MacroContext;

mod attrs;
mod build_schema;
pub(crate) mod config;
pub(crate) mod context;
mod from_eure;
mod into_eure;
mod must_be_text;
mod util;

#[proc_macro_derive(IntoEure, attributes(eure))]
pub fn into_eure_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match create_context(input) {
        Ok(context) => into_eure::derive(context).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(FromEure, attributes(eure))]
pub fn from_eure_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match create_context(input) {
        Ok(context) => from_eure::derive(context).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(BuildSchema, attributes(eure))]
pub fn build_schema_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match create_context(input) {
        Ok(context) => build_schema::derive(context).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn create_context(input: syn::DeriveInput) -> syn::Result<MacroContext> {
    let attrs = ContainerAttrs::from_derive_input(&input).expect("Failed to parse eure attributes");
    let attr_spans = extract_container_attr_spans(&input);
    let config = MacroConfig::from_attrs(attrs, attr_spans)?;
    Ok(MacroContext::new(config, input))
}

/// Creates a zero-sized type that only parses from a specific Text value.
///
/// # Syntax
///
/// ```ignore
/// MustBeText!("content")           // Implicit language: `content`
/// MustBeText!(plaintext, "content") // Plaintext language: "content"
/// MustBeText!(rust, "content")      // Other language: rust`content`
/// ```
///
/// # Example
///
/// ```ignore
/// use eure_macros::MustBeText;
///
/// // This type only successfully parses from the text value `any`
/// let marker = MustBeText!("any");
/// ```
#[proc_macro]
#[allow(non_snake_case)]
pub fn MustBeText(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as must_be_text::MustBeTextInput);
    must_be_text::expand(input).into()
}
