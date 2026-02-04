use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Type;

use crate::attrs::{ContainerAttrs, RenameAll};

/// Configuration for proxy/opaque type generation.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// The target type to implement `FromEure<'doc, Target>` for.
    pub target: Type,
    /// If true, uses `From` conversion; if false, uses struct literal directly.
    pub is_opaque: bool,
}

pub struct MacroConfig {
    pub document_crate: TokenStream,
    pub rename_all: Option<RenameAll>,
    pub rename_all_fields: Option<RenameAll>,
    /// Parse fields from extension namespace instead of record fields.
    pub parse_ext: bool,
    /// Allow unknown fields instead of denying them.
    pub allow_unknown_fields: bool,
    /// Allow unknown extensions instead of denying them.
    pub allow_unknown_extensions: bool,
    /// Custom error type for the FromEure impl.
    pub parse_error: Option<TokenStream>,
    /// Type name for BuildSchema registration.
    pub type_name: Option<String>,
    /// Proxy configuration for implementing `FromEure<'doc, Target>`.
    /// - `proxy = "T"` → `ProxyConfig { target: T, is_opaque: false }`
    /// - `opaque = "T"` → `ProxyConfig { target: T, is_opaque: true }`
    pub proxy: Option<ProxyConfig>,
}

impl MacroConfig {
    pub fn from_attrs(
        attrs: ContainerAttrs,
        attr_spans: std::collections::HashMap<String, Span>,
    ) -> syn::Result<Self> {
        use quote::ToTokens;
        let document_crate = attrs
            .crate_path
            .map(|path| path.into_token_stream())
            .unwrap_or_else(|| quote! { ::eure::document });
        let parse_error = attrs.parse_error.map(|path| path.into_token_stream());

        // Validate that proxy and opaque are mutually exclusive
        if attrs.proxy.is_some() && attrs.opaque.is_some() {
            let span = attr_spans
                .get("proxy")
                .copied()
                .unwrap_or_else(Span::call_site);
            return Err(syn::Error::new(
                span,
                "cannot use both #[eure(proxy = \"...\")] and #[eure(opaque = \"...\")] on the same type; they are mutually exclusive",
            ));
        }

        // Convert proxy/opaque attributes to unified ProxyConfig
        let proxy = attrs
            .proxy
            .map(|target| ProxyConfig {
                target,
                is_opaque: false,
            })
            .or_else(|| {
                attrs.opaque.map(|target| ProxyConfig {
                    target,
                    is_opaque: true,
                })
            });

        Ok(Self {
            document_crate,
            rename_all: attrs.rename_all,
            rename_all_fields: attrs.rename_all_fields,
            parse_ext: attrs.parse_ext,
            allow_unknown_fields: attrs.allow_unknown_fields,
            allow_unknown_extensions: attrs.allow_unknown_extensions,
            parse_error,
            type_name: attrs.type_name,
            proxy,
        })
    }
}
