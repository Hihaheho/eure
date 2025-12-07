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

/// Errors that can occur when converting JSON to Eure.
/// Currently this is infallible, but the error type is provided for future extensibility
/// and API consistency.
#[derive(Debug, Error, PartialEq)]
pub enum JsonToEureError {
    // Currently no error cases - JSON to Eure conversion is infallible.
    // This enum is provided for:
    // 1. API consistency with EureToJsonError
    // 2. Future extensibility (e.g., schema-guided conversion constraints)
}
