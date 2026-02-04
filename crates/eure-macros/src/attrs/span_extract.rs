use proc_macro2::Span;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{Attribute, DeriveInput};

/// Extracts spans of individual attribute keys from `#[eure(...)]` attributes.
///
/// Returns a map from attribute key names (e.g., "flatten", "ext") to their spans.
/// This allows error messages to point to specific attributes instead of the derive macro.
///
/// For example, given `#[eure(flatten, ext)]`:
/// - `spans.get("flatten")` returns the span of "flatten"
/// - `spans.get("ext")` returns the span of "ext"
pub fn extract_eure_attr_spans(attrs: &[Attribute]) -> HashMap<String, Span> {
    let mut spans = HashMap::new();

    for attr in attrs {
        if !attr.path().is_ident("eure") {
            continue;
        }

        // Parse the nested meta items from the attribute
        let _ = attr.parse_nested_meta(|meta| {
            if let Some(ident) = meta.path.get_ident() {
                spans.insert(ident.to_string(), meta.path.span());
            }
            // Skip any value (like `via = "..."`)
            if meta.input.peek(syn::Token![=]) {
                let _: syn::Token![=] = meta.input.parse()?;
                // Skip the value
                let _: syn::Expr = meta.input.parse()?;
            }
            Ok(())
        });
    }

    spans
}

/// Extracts spans from container-level `#[eure(...)]` attributes on a derive input.
///
/// This is a convenience wrapper for extracting spans from the attributes
/// on a struct/enum definition.
pub fn extract_container_attr_spans(input: &DeriveInput) -> HashMap<String, Span> {
    extract_eure_attr_spans(&input.attrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn extracts_simple_attributes() {
        let field: syn::Field = parse_quote! {
            #[eure(flatten, ext)]
            field: String
        };

        let spans = extract_eure_attr_spans(&field.attrs);

        assert!(spans.contains_key("flatten"));
        assert!(spans.contains_key("ext"));
    }

    #[test]
    fn extracts_name_value_attributes() {
        let field: syn::Field = parse_quote! {
            #[eure(via = "SomeType", rename = "other_name")]
            field: String
        };

        let spans = extract_eure_attr_spans(&field.attrs);

        assert!(spans.contains_key("via"));
        assert!(spans.contains_key("rename"));
    }

    #[test]
    fn handles_multiple_eure_attributes() {
        let field: syn::Field = parse_quote! {
            #[eure(flatten)]
            #[eure(ext)]
            field: String
        };

        let spans = extract_eure_attr_spans(&field.attrs);

        assert!(spans.contains_key("flatten"));
        assert!(spans.contains_key("ext"));
    }

    #[test]
    fn ignores_non_eure_attributes() {
        let field: syn::Field = parse_quote! {
            #[serde(rename = "foo")]
            #[eure(flatten)]
            field: String
        };

        let spans = extract_eure_attr_spans(&field.attrs);

        assert!(spans.contains_key("flatten"));
        assert!(!spans.contains_key("rename")); // from serde, not eure
    }
}
