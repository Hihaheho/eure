use std::collections::HashMap;

use darling::FromField;
use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::attrs::{DefaultValue, FieldAttrs, extract_eure_attr_spans};
use crate::context::MacroContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldMode {
    Record,
    Ext,
    Flatten,
    FlattenExt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenameScope {
    Container,
    Field,
}

#[derive(Debug, Clone)]
pub(crate) struct CommonFieldIr {
    pub ident: syn::Ident,
    pub ty: syn::Type,
    pub mode: FieldMode,
    pub wire_name: Option<String>,
    pub via: Option<syn::Type>,
    pub default: DefaultValue,
    pub attr_spans: HashMap<String, Span>,
}

pub(crate) fn analyze_common_named_fields(
    context: &MacroContext,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    rename_scope: RenameScope,
) -> syn::Result<Vec<CommonFieldIr>> {
    let mut out = Vec::with_capacity(fields.len());

    for field in fields {
        let ident = field.ident.clone().expect("named fields must have names");
        let ty = field.ty.clone();
        let attrs = FieldAttrs::from_field(field).expect("failed to parse field attributes");
        let attr_spans = extract_eure_attr_spans(&field.attrs);

        if attrs.flatten && attrs.flatten_ext {
            let span = attr_spans
                .get("flatten")
                .copied()
                .unwrap_or_else(|| field.span());
            return Err(syn::Error::new(
                span,
                "cannot use both #[eure(flatten)] and #[eure(flatten_ext)] on the same field",
            ));
        }
        if attrs.flatten && attrs.ext {
            let span = attr_spans
                .get("flatten")
                .copied()
                .unwrap_or_else(|| field.span());
            return Err(syn::Error::new(
                span,
                "cannot use both #[eure(flatten)] and #[eure(ext)] on the same field",
            ));
        }
        if attrs.ext && attrs.flatten_ext {
            let span = attr_spans
                .get("ext")
                .copied()
                .unwrap_or_else(|| field.span());
            return Err(syn::Error::new(
                span,
                "cannot use both #[eure(ext)] and #[eure(flatten_ext)] on the same field",
            ));
        }
        if attrs.via.is_some() && (attrs.flatten || attrs.flatten_ext) {
            let span = attr_spans
                .get("via")
                .copied()
                .unwrap_or_else(|| field.span());
            return Err(syn::Error::new(
                span,
                format!(
                    "cannot use #[eure(via = \"...\")] with #[eure(flatten)] or #[eure(flatten_ext)] on field `{}`",
                    ident
                ),
            ));
        }

        let mode = if attrs.flatten {
            FieldMode::Flatten
        } else if attrs.flatten_ext {
            FieldMode::FlattenExt
        } else if attrs.ext {
            FieldMode::Ext
        } else {
            FieldMode::Record
        };

        let wire_name = Some(attrs.rename.clone().unwrap_or_else(|| match rename_scope {
            RenameScope::Container => context.apply_rename(&ident.to_string()),
            RenameScope::Field => context.apply_field_rename(&ident.to_string()),
        }));

        out.push(CommonFieldIr {
            ident,
            ty,
            mode,
            wire_name,
            via: attrs.via.clone(),
            default: attrs.default.clone(),
            attr_spans,
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use darling::FromDeriveInput;
    use syn::parse_quote;

    use super::*;
    use crate::attrs::{ContainerAttrs, extract_container_attr_spans};
    use crate::config::MacroConfig;

    fn context_for(input: syn::DeriveInput) -> MacroContext {
        let attrs = ContainerAttrs::from_derive_input(&input).expect("valid attrs");
        let spans = extract_container_attr_spans(&input);
        let config = MacroConfig::from_attrs(attrs, spans).expect("valid config");
        MacroContext::new(config, input)
    }

    #[test]
    fn rejects_flatten_ext_conflict() {
        let input: syn::DeriveInput = parse_quote! {
            struct Sample {
                #[eure(flatten, ext)]
                inner: Inner
            }
        };
        let context = context_for(input.clone());
        let fields = match &input.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(fields) => &fields.named,
                _ => panic!("expected named fields"),
            },
            _ => panic!("expected struct"),
        };

        let err = analyze_common_named_fields(&context, fields, RenameScope::Container)
            .expect_err("expected conflict error");
        assert!(
            err.to_string()
                .contains("cannot use both #[eure(flatten)] and #[eure(ext)]")
        );
    }

    #[test]
    fn resolves_container_rename() {
        let input: syn::DeriveInput = parse_quote! {
            #[eure(rename_all = "kebab-case")]
            struct Sample {
                user_name: String
            }
        };
        let context = context_for(input.clone());
        let fields = match &input.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(fields) => &fields.named,
                _ => panic!("expected named fields"),
            },
            _ => panic!("expected struct"),
        };

        let fields = analyze_common_named_fields(&context, fields, RenameScope::Container)
            .expect("valid ir");
        assert_eq!(fields[0].wire_name.as_deref(), Some("user-name"));
    }
}
