use serde::{de, ser};
use std::fmt::Display;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("unexpected end of input")]
    Eof,

    #[error("syntax error at position {position}: {message}")]
    Syntax {
        position: usize,
        message: String,
    },

    #[error("expected {expected}")]
    Expected {
        expected: &'static str,
    },

    #[error("trailing characters")]
    TrailingCharacters,

    #[error("invalid type: {0}")]
    InvalidType(String),

    #[error("invalid value: {0}")]
    InvalidValue(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("tree error: {0}")]
    TreeError(String),

    #[error("value visitor error: {0}")]
    ValueVisitorError(String),

    #[error("unsupported variant representation")]
    UnsupportedVariantRepr,

    #[error("cannot serialize {type_name} as EURE")]
    CannotSerialize { type_name: &'static str },

    #[error("cannot deserialize {type_name} from EURE")]
    CannotDeserialize { type_name: &'static str },
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}