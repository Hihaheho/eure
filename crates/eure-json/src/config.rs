use eure::data_model::VariantRepr;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub variant_repr: VariantRepr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            variant_repr: VariantRepr::External,
        }
    }
}
