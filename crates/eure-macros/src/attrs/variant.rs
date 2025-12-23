use darling::FromVariant;

#[derive(Debug, Default, FromVariant)]
#[darling(default, attributes(eure))]
pub struct VariantAttrs {
    /// Explicit rename for this variant (overrides rename_all)
    pub rename: Option<String>,
}
