use eure_value::value::VariantRepr;

/// Configuration for YAML conversion
#[derive(Debug, Clone)]
pub struct Config {
    /// How to represent variants in YAML
    pub variant_repr: VariantRepr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            variant_repr: VariantRepr::External,
        }
    }
}
