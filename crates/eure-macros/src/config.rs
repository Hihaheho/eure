use proc_macro2::TokenStream;
use quote::quote;

use crate::attrs::{ContainerAttrs, RenameAll};

pub struct MacroConfig {
    pub document_crate: TokenStream,
    pub rename_all: Option<RenameAll>,
    pub rename_all_fields: Option<RenameAll>,
    /// Parse fields from extension namespace instead of record fields.
    pub parse_ext: bool,
}

impl MacroConfig {
    pub fn from_attrs(attrs: ContainerAttrs) -> Self {
        use quote::ToTokens;
        let document_crate = attrs
            .crate_path
            .map(|path| path.into_token_stream())
            .unwrap_or_else(|| quote! { ::eure::document });
        Self {
            document_crate,
            rename_all: attrs.rename_all,
            rename_all_fields: attrs.rename_all_fields,
            parse_ext: attrs.parse_ext,
        }
    }
}
