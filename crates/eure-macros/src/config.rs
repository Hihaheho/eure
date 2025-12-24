use proc_macro2::TokenStream;
use quote::quote;

use crate::attrs::{ContainerAttrs, RenameAll};

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
    /// Custom error type for the ParseDocument impl.
    pub parse_error: Option<TokenStream>,
}

impl MacroConfig {
    pub fn from_attrs(attrs: ContainerAttrs) -> Self {
        use quote::ToTokens;
        let document_crate = attrs
            .crate_path
            .map(|path| path.into_token_stream())
            .unwrap_or_else(|| quote! { ::eure::document });
        let parse_error = attrs.parse_error.map(|path| path.into_token_stream());
        Self {
            document_crate,
            rename_all: attrs.rename_all,
            rename_all_fields: attrs.rename_all_fields,
            parse_ext: attrs.parse_ext,
            allow_unknown_fields: attrs.allow_unknown_fields,
            allow_unknown_extensions: attrs.allow_unknown_extensions,
            parse_error,
        }
    }
}
