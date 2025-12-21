use darling::FromDeriveInput;
use syn::Path;

use super::RenameAll;

#[derive(Debug, Default, FromDeriveInput)]
#[darling(attributes(eure), default)]
pub struct ContainerAttrs {
    #[darling(rename = "crate")]
    pub crate_path: Option<Path>,
    pub rename_all: Option<RenameAll>,
    /// Renames all struct variant fields in an enum.
    /// Unlike `rename_all`, this only applies to fields within struct variants.
    pub rename_all_fields: Option<RenameAll>,
}
