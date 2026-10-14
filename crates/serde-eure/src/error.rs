use std::fmt;

use eure_document::bytes::BytesDecodeError;
use eure_document::decimal::DecimalParseError;
use eure_document::document::InsertError;
use eure_document::document::constructor::ScopeError;
use eure_document::write::WriteError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeError {
    #[error("{0}")]
    Custom(String),
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("missing field: {0}")]
    MissingField(String),
    #[error("unexpected end of sequence")]
    EndOfSequence,
    #[error("integer out of range")]
    IntOutOfRange,
    #[error("no variant matched")]
    NoVariantMatched,
    #[error("invalid variant name: {0}")]
    InvalidVariantName(String),
    #[error("PartialMap unsupported in serde-eure v1")]
    PartialMapUnsupported,
    #[error("non-finite float {0} cannot be a decimal")]
    NonFiniteDecimal(f64),
    #[error("invalid decimal: {0}")]
    InvalidDecimal(#[from] DecimalParseError),
    #[error("invalid base64 bytes: {0}")]
    InvalidBytes(#[from] BytesDecodeError),
    #[error(transparent)]
    Write(#[from] WriteError),
    #[error(transparent)]
    Insert(#[from] InsertError),
    #[error(transparent)]
    Scope(#[from] ScopeError),
}

impl serde::de::Error for DeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

#[derive(Debug, Error)]
pub enum SerError {
    #[error("{0}")]
    Custom(String),
    #[error("missing field: {0}")]
    MissingField(String),
    #[error("hole node cannot be serialized")]
    UnexpectedHole,
    #[error("PartialMap unsupported")]
    PartialMapUnsupported,
    #[error("non-string map key")]
    NonStringKey,
    #[error("BigInt out of range for serialization")]
    BigIntOutOfRange,
    #[error("non-finite float")]
    NonFiniteFloat,
    #[error("decimal cannot be serialized as a float without losing precision")]
    DecimalPrecisionLoss,
}

impl serde::ser::Error for SerError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}
