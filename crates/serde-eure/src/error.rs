use std::fmt;

use eure::document::InsertError;
use eure::document::constructor::ScopeError;
use eure::document::write::WriteError;
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
}

impl serde::ser::Error for SerError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}
