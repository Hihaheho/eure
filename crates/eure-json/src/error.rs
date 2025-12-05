use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum EureToJsonError {
    #[error("Hole (uninitialized value) is not supported in JSON")]
    HoleNotSupported,

    #[error("BigInt value is out of range for JSON number")]
    BigIntOutOfRange,

    #[error("Non-finite floating point value (NaN or Infinity) is not supported in JSON")]
    NonFiniteFloat,

    #[error("Variant content already contains tag field '{tag}' in Internal representation")]
    VariantTagConflict { tag: String },

    #[error("Variant content already contains field '{field}' in Adjacent representation")]
    VariantAdjacentConflict { field: String },
}
