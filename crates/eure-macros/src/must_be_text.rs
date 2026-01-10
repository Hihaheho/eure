//! Implementation of the `MustBeText!` macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

/// Parsed input for the `MustBeText!` macro.
///
/// Supports two forms:
/// - `MustBeText!("content")` - Implicit language
/// - `MustBeText!(language, "content")` - Explicit language
pub struct MustBeTextInput {
    pub language: LanguageSpec,
    pub content: LitStr,
}

/// Language specification for the macro.
pub enum LanguageSpec {
    /// Implicit language (no language tag, from `` `...` `` syntax)
    Implicit,
    /// Plaintext language (from `"..."` syntax)
    Plaintext,
    /// Other language with explicit tag
    Other(Ident),
}

impl Parse for MustBeTextInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Check if there's an identifier followed by a comma (language specifier)
        if input.peek(Ident) && input.peek2(Token![,]) {
            let lang_ident: Ident = input.parse()?;
            let _comma: Token![,] = input.parse()?;
            let content: LitStr = input.parse()?;

            let language = match lang_ident.to_string().as_str() {
                "plaintext" => LanguageSpec::Plaintext,
                _ => LanguageSpec::Other(lang_ident),
            };

            Ok(MustBeTextInput { language, content })
        } else {
            // Just content string - implicit language
            let content: LitStr = input.parse()?;
            Ok(MustBeTextInput {
                language: LanguageSpec::Implicit,
                content,
            })
        }
    }
}

/// Generate a unique marker struct name based on language and content.
fn generate_marker_name(language: &LanguageSpec, content: &str) -> Ident {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    match language {
        LanguageSpec::Implicit => "implicit".hash(&mut hasher),
        LanguageSpec::Plaintext => "plaintext".hash(&mut hasher),
        LanguageSpec::Other(ident) => ident.to_string().hash(&mut hasher),
    }
    content.hash(&mut hasher);
    let hash = hasher.finish();

    Ident::new(
        &format!("__MustBeText_{:016x}", hash),
        proc_macro2::Span::call_site(),
    )
}

/// Generate the token stream for the `MustBeText!` macro.
pub fn expand(input: MustBeTextInput) -> TokenStream {
    let content = &input.content;
    let marker_name = generate_marker_name(&input.language, &content.value());

    let language_expr = match &input.language {
        LanguageSpec::Implicit => quote! {
            ::eure_document::text::Language::Implicit
        },
        LanguageSpec::Plaintext => quote! {
            ::eure_document::text::Language::Plaintext
        },
        LanguageSpec::Other(ident) => {
            let lang_str = ident.to_string();
            quote! {
                ::eure_document::text::Language::Other(::std::borrow::Cow::Borrowed(#lang_str))
            }
        }
    };

    quote! {
        {
            #[derive(Clone, Copy)]
            struct #marker_name;

            impl ::eure_document::must_be::MustBeTextMarker for #marker_name {
                const CONTENT: &'static str = #content;
                const LANGUAGE: ::eure_document::text::Language = #language_expr;
            }

            ::eure_document::must_be::MustBeText::<#marker_name>::new()
        }
    }
}
