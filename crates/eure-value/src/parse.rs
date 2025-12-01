//! ParseDocument trait for parsing Rust types from Eure documents.

pub mod object_key;
pub mod record;

pub use object_key::ParseObjectKey;
pub use record::{ExtParser, RecordParser};

use num_bigint::BigInt;

use crate::{
    document::node::NodeArray, identifier::IdentifierError, prelude_internal::*, value::ValueKind,
};

/// Trait for parsing Rust types from Eure documents.
///
/// Types implementing this trait can be constructed from [`EureDocument`].
/// Used for type-safe extraction of structures from documents during conversion.
///
/// # Lifetime Parameter
///
/// The `'doc` lifetime ties the parsed output to the document's lifetime,
/// allowing zero-copy parsing for reference types like `&'doc str`.
///
/// # Examples
///
/// ```ignore
/// // Reference type - borrows from document
/// impl<'doc> ParseDocument<'doc> for &'doc str { ... }
///
/// // Owned type - no lifetime dependency
/// impl ParseDocument<'_> for String { ... }
/// ```
pub trait ParseDocument<'doc>: Sized {
    /// Parse a value of this type from an Eure document at the given node.
    fn parse(doc: &'doc EureDocument, node_id: NodeId) -> Result<Self, ParseError>;
}

fn handle_unexpected_node_value(node_value: &NodeValue) -> ParseErrorKind {
    match node_value {
        NodeValue::Uninitialized => ParseErrorKind::UnexpectedUninitialized,
        value => value
            .value_kind()
            .map(|actual| ParseErrorKind::TypeMismatch {
                expected: ValueKind::Text,
                actual,
            })
            .unwrap_or_else(|| ParseErrorKind::UnexpectedUninitialized),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("parse error: {kind}")]
pub struct ParseError {
    pub node_id: NodeId,
    pub kind: ParseErrorKind,
}

/// Error type for parsing failures.
#[derive(Debug, thiserror::Error)]
pub enum ParseErrorKind {
    /// Unexpected uninitialized value.
    #[error("unexpected uninitialized value")]
    UnexpectedUninitialized,

    /// Type mismatch between expected and actual value.
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: ValueKind,
        actual: ValueKind,
    },

    /// Required field is missing.
    #[error("missing field: {0}")]
    MissingField(String),

    /// Unknown variant in a union type.
    #[error("unknown variant: {0}")]
    UnknownVariant(String),

    /// Value is out of valid range.
    #[error("value out of range: {0}")]
    OutOfRange(String),

    /// Invalid string pattern.
    #[error("invalid pattern: expected {pattern}, got {value}")]
    InvalidPattern { pattern: String, value: String },

    /// Nested parse error with path context.
    #[error("at {path}: {source}")]
    Nested {
        path: String,
        #[source]
        source: Box<ParseErrorKind>,
    },

    /// Invalid identifier.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(#[from] IdentifierError),

    /// Unexpected tuple length.
    #[error("unexpected tuple length: expected {expected}, got {actual}")]
    UnexpectedTupleLength { expected: usize, actual: usize },

    /// Unknown field in record.
    #[error("unknown field: {0}")]
    UnknownField(String),
}

impl ParseErrorKind {
    /// Wrap this error with path context.
    pub fn at(self, path: impl Into<String>) -> Self {
        ParseErrorKind::Nested {
            path: path.into(),
            source: Box::new(self),
        }
    }
}

impl<'doc> EureDocument {
    pub fn parse<T: ParseDocument<'doc>>(&'doc self, node_id: NodeId) -> Result<T, ParseError> {
        T::parse(self, node_id)
    }
}

impl<'doc> ParseDocument<'doc> for &'doc str {
    fn parse(doc: &'doc EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Text(text)) => Ok(text.as_str()),
            value => Err(ParseError {
                node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

impl ParseDocument<'_> for String {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        doc.parse::<&str>(node_id).map(String::from)
    }
}

impl ParseDocument<'_> for bool {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Bool(b)) => Ok(*b),
            value => Err(ParseError {
                node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

impl ParseDocument<'_> for BigInt {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Integer(i)) => Ok(i.clone()),
            value => Err(ParseError {
                node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

impl ParseDocument<'_> for f32 {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::F32(f)) => Ok(*f),
            value => Err(ParseError {
                node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

impl<'doc> ParseDocument<'doc> for &'doc PrimitiveValue {
    fn parse(doc: &'doc EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Primitive(primitive) => Ok(primitive),
            value => Err(ParseError {
                node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

impl ParseDocument<'_> for PrimitiveValue {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        doc.parse::<&PrimitiveValue>(node_id).cloned()
    }
}

impl ParseDocument<'_> for Identifier {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Primitive(PrimitiveValue::Text(text)) => Ok(text
                .content
                .parse()
                .map_err(ParseErrorKind::InvalidIdentifier)
                .map_err(|kind| ParseError { node_id, kind })?),
            value => Err(ParseError {
                node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

impl<'doc> ParseDocument<'doc> for &'doc NodeArray {
    fn parse(doc: &'doc EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Array(array) => Ok(array),
            value => Err(handle_unexpected_node_value(value)),
        }
        .map_err(|kind| ParseError { node_id, kind })
    }
}

impl<'doc, T> ParseDocument<'doc> for Vec<T>
where
    T: ParseDocument<'doc>,
{
    fn parse(doc: &'doc EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        match &doc.node(node_id).content {
            NodeValue::Array(array) => array
                .iter()
                .map(|item| T::parse(doc, *item))
                .collect::<Result<Vec<_>, _>>(),
            value => Err(ParseError {
                node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

macro_rules! parse_tuple {
    ($n:expr, $($var:ident),*) => {
        impl<'doc, $($var: ParseDocument<'doc>),*> ParseDocument<'doc> for ($($var),*,) {
            fn parse(doc: &'doc EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
                let tuple = match &doc.node(node_id).content {
                    NodeValue::Tuple(tuple) => tuple,
                    value => return Err(ParseError { node_id, kind: handle_unexpected_node_value(value) }),
                };
                if tuple.len() != $n {
                    return Err(ParseError { node_id, kind: ParseErrorKind::UnexpectedTupleLength { expected: $n, actual: tuple.len() } });
                }
                let mut iter = tuple.iter();
                Ok(($($var::parse(doc, *iter.next().unwrap())?),*,))
            }
        }
    }
}

parse_tuple!(1, A);
parse_tuple!(2, A, B);
parse_tuple!(3, A, B, C);
parse_tuple!(4, A, B, C, D);
parse_tuple!(5, A, B, C, D, E);
parse_tuple!(6, A, B, C, D, E, F);
parse_tuple!(7, A, B, C, D, E, F, G);
parse_tuple!(8, A, B, C, D, E, F, G, H);
parse_tuple!(9, A, B, C, D, E, F, G, H, I);
parse_tuple!(10, A, B, C, D, E, F, G, H, I, J);
parse_tuple!(11, A, B, C, D, E, F, G, H, I, J, K);
parse_tuple!(12, A, B, C, D, E, F, G, H, I, J, K, L);
parse_tuple!(13, A, B, C, D, E, F, G, H, I, J, K, L, M);
parse_tuple!(14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
parse_tuple!(15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
parse_tuple!(16, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

impl<'doc, K, T> ParseDocument<'doc> for Map<K, T>
where
    K: ParseObjectKey<'doc>,
    T: ParseDocument<'doc>,
{
    fn parse(doc: &'doc EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let map = match &doc.node(node_id).content {
            NodeValue::Map(map) => map,
            value => {
                return Err(ParseError {
                    node_id,
                    kind: handle_unexpected_node_value(value),
                });
            }
        };
        map.iter()
            .map(|(key, value)| {
                Ok((
                    K::from_object_key(key).map_err(|kind| ParseError { node_id, kind })?,
                    T::parse(doc, *value)?,
                ))
            })
            .collect::<Result<Map<_, _>, _>>()
    }
}
