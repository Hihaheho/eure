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
        }
    }
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

impl From<&str> for ObjectKey {
    fn from(s: &str) -> Self {
        ObjectKey::String(s.to_string())
    }
}

impl From<String> for ObjectKey {
    fn from(s: String) -> Self {
        ObjectKey::String(s)
    }
}

impl From<bool> for ObjectKey {
    fn from(b: bool) -> Self {
        ObjectKey::Bool(b)
    }
}

impl From<i32> for ObjectKey {
    fn from(n: i32) -> Self {
        ObjectKey::Number(BigInt::from(n))
    }
}

impl From<i64> for ObjectKey {
    fn from(n: i64) -> Self {
        ObjectKey::Number(BigInt::from(n))
    }
}

impl From<BigInt> for ObjectKey {
    fn from(n: BigInt) -> Self {
        ObjectKey::Number(n)
    }
}

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

// ============================================================================
// From implementations for PrimitiveValue
// ============================================================================

impl From<bool> for PrimitiveValue {
    fn from(b: bool) -> Self {
        PrimitiveValue::Bool(b)
    }
}

impl From<i32> for PrimitiveValue {
    fn from(n: i32) -> Self {
        PrimitiveValue::Integer(BigInt::from(n))
    }
}

impl From<i64> for PrimitiveValue {
    fn from(n: i64) -> Self {
        PrimitiveValue::Integer(BigInt::from(n))
    }
}

impl From<f32> for PrimitiveValue {
    fn from(n: f32) -> Self {
        PrimitiveValue::F32(n)
    }
}

impl From<f64> for PrimitiveValue {
    fn from(n: f64) -> Self {
        PrimitiveValue::F64(n)
    }
}

impl From<&str> for PrimitiveValue {
    fn from(s: &str) -> Self {
        PrimitiveValue::Text(Text::plaintext(s))
    }
}

impl From<String> for PrimitiveValue {
    fn from(s: String) -> Self {
        PrimitiveValue::Text(Text::plaintext(s))
    }
}

impl From<Text> for PrimitiveValue {
    fn from(t: Text) -> Self {
        PrimitiveValue::Text(t)
    }
}
