use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unsupported value type: {0}")]
    UnsupportedValue(String),

    #[error("Invalid variant structure: {0}")]
    InvalidVariant(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("Invalid number: cannot represent {0} as JSON number")]
    InvalidNumber(String),

    #[error("Missing variant tag in {0} representation")]
    MissingVariantTag(String),
}
