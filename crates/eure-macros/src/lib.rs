use darling::FromDeriveInput;
use syn::parse_macro_input;

use crate::{attrs::ContainerAttrs, config::MacroConfig, context::MacroContext};

mod attrs;
mod build_schema;
pub(crate) mod config;
pub(crate) mod context;
mod from_eure;
mod into_eure;
mod must_be_text;

#[proc_macro_derive(IntoEure, attributes(eure))]
pub fn into_eure_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    into_eure::derive(create_context(input)).into()
}

#[proc_macro_derive(FromEure, attributes(eure))]
pub fn from_eure_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    from_eure::derive(create_context(input)).into()
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
