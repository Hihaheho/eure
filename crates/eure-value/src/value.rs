use num_bigint::BigInt;

use crate::{prelude_internal::*, text::Text};

#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind {
    Hole,
    Null,
    Bool,
    Integer,
    F32,
    F64,
    Text,
    Variant,
    Array,
    Tuple,
    Map,
}

impl core::fmt::Display for ValueKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Hole => write!(f, "hole"),
            Self::Null => write!(f, "null"),
            Self::Bool => write!(f, "bool"),
            Self::Integer => write!(f, "integer"),
            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
            Self::Text => write!(f, "text"),
            Self::Variant => write!(f, "variant"),
            Self::Array => write!(f, "array"),
            Self::Tuple => write!(f, "tuple"),
            Self::Map => write!(f, "map"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveValue {
    Null,
    Bool(bool),
    Integer(BigInt),
    F32(f32),
    F64(f64),
    /// Unified text type for strings and code.
    ///
    /// - `"..."` syntax produces `Text` with `Language::Plaintext`
    /// - `` `...` `` syntax produces `Text` with `Language::Implicit`
    /// - `` lang`...` `` syntax produces `Text` with `Language::Other(lang)`
    Text(Text),
    Hole,
    Variant(Variant),
}

impl PrimitiveValue {
    /// Returns the text if this is a `Text` variant.
    pub fn as_text(&self) -> Option<&Text> {
        if let Self::Text(text) = self {
            Some(text)
        } else {
            None
        }
    }

    /// Returns the text content as a string slice if this is a `Text` variant.
    pub fn as_str(&self) -> Option<&str> {
        self.as_text().map(|t| t.as_str())
    }

    pub(crate) fn kind(&self) -> ValueKind {
        match self {
            Self::Null => ValueKind::Null,
            Self::Bool(_) => ValueKind::Bool,
            Self::Integer(_) => ValueKind::Integer,
            Self::F32(_) => ValueKind::F32,
            Self::F64(_) => ValueKind::F64,
            Self::Text(_) => ValueKind::Text,
            Self::Hole => ValueKind::Hole,
            Self::Variant(_) => ValueKind::Variant,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Primitive(PrimitiveValue),
    Array(Array),
    Tuple(Tuple<Value>),
    Map(Map),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Key-comparable value which implements `Eq` and `Hash`.
///
/// Eure restricts map keys to four types — `String`, `Bool`, `Integer`,
/// and `Tuple<Key...>` — for practical and predictable behavior.
///
/// - **Deterministic equality:**
///   These types provide stable, well-defined equality and hashing.
///   Types like floats, null, or holes introduce ambiguous or
///   platform-dependent comparison rules.
///
/// - **Reliable round-tripping:**
///   Keys must serialize and deserialize without losing meaning.
///   Strings, booleans, integers, and tuples have canonical and
///   unambiguous textual forms.
///
/// - **Tooling-friendly:**
///   This set balances expressiveness and simplicity, making keys easy
///   to validate, index, and reason about across implementations.
pub enum ObjectKey {
    Bool(bool),
    Number(BigInt),
    String(String),
    Tuple(Tuple<ObjectKey>),
}

impl core::fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ObjectKey::Bool(bool) => write!(f, "{}", bool),
            ObjectKey::Number(big_int) => write!(f, "{}", big_int),
            ObjectKey::String(string) => write!(f, "{}", string),
            ObjectKey::Tuple(tuple) => write!(f, "{}", tuple),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Plural, Default)]
pub struct Array(pub Vec<Value>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Plural, Default)]
pub struct Tuple<T>(pub Vec<T>);

impl core::fmt::Display for Tuple<ObjectKey> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(")?;
        for (i, item) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Plural, Default)]
#[plural(len, is_empty, iter, into_iter, into_iter_ref, from_iter, new)]
pub struct Map(pub crate::Map<ObjectKey, Value>);

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub tag: String,
    pub content: Box<Value>,
}
