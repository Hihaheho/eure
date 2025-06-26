use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unsupported value type for YAML conversion: {0}")]
    UnsupportedValue(String),
    
    #[error("Invalid variant structure: {0}")]
    InvalidVariant(String),
    
    #[error("Conversion error: {0}")]
    ConversionError(String),
    
    #[error("YAML parsing error: {0}")]
    YamlParseError(#[from] serde_yaml::Error),
    
    #[error("Missing variant tag in {0} representation")]
    MissingVariantTag(String),
}