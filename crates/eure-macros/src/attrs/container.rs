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
    /// Parse fields from extension namespace ($ext-type) instead of record fields.
    /// When true, uses `ctx.parse_extension()` and `ext.parse_ext()` instead of
    /// `ctx.parse_record()` and `rec.parse_field()`.
    pub parse_ext: bool,
    /// Allow unknown fields instead of denying them.
    /// By default (false), unknown fields cause a parse error.
    /// When true, uses `allow_unknown_fields()` instead of `deny_unknown_fields()`.
    pub allow_unknown_fields: bool,
    /// Allow unknown extensions instead of denying them.
    /// By default (false), unknown extensions cause a parse error.
    /// When true, skips the `deny_unknown_extensions()` check.
    pub allow_unknown_extensions: bool,
}
