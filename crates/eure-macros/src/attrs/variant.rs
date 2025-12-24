use darling::FromVariant;

#[derive(Debug, Default, FromVariant)]
#[darling(default, attributes(eure))]
pub struct VariantAttrs {
    /// Explicit rename for this variant (overrides rename_all)
    pub rename: Option<String>,
    /// Allow unknown fields for this variant instead of denying them.
    /// By default (false), unknown fields cause a parse error.
    /// When true, uses `allow_unknown_fields()` instead of `deny_unknown_fields()`.
    pub allow_unknown_fields: bool,
}
