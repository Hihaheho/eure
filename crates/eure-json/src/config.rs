use eure_schema::interop::VariantRepr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
