use num_bigint::BigInt;

use crate::{identifier::Identifier, prelude_internal::*, text::Text};

#[derive(Debug, Clone, PartialEq, Copy)]
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
    PartialMap,
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
            Self::PartialMap => write!(f, "partial-map"),
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
/// Eure restricts map keys to three types — `String`, `Number` (BigInt),
/// and `Tuple<ObjectKey>` — for practical and predictable behavior.
///
/// - **Deterministic equality:**
///   These types provide stable, well-defined equality and hashing.
///   Types like floats, null, or holes introduce ambiguous or
///   platform-dependent comparison rules.
///
/// - **Reliable round-tripping:**
///   Keys must serialize and deserialize without losing meaning.
///   Strings, integers, and tuples have canonical and unambiguous textual forms.
///
/// - **Tooling-friendly:**
///   This set balances expressiveness and simplicity, making keys easy
///   to validate, index, and reason about across implementations.
///
/// Note: In key position, `true`, `false`, and `null` are parsed as string
/// identifiers, not as boolean/null values. For example, `a.true = true`
/// creates a key `"true"` with boolean value `true`.
pub enum ObjectKey {
    Number(BigInt),
    String(String),
    Tuple(Tuple<ObjectKey>),
}

impl core::fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ObjectKey::Number(n) => write!(f, "{}", n),
            ObjectKey::String(s) => {
                write!(f, "\"")?;
                for c in s.chars() {
                    match c {
                        '"' => write!(f, "\\\"")?,
                        '\\' => write!(f, "\\\\")?,
                        _ => write!(f, "{}", c)?,
                    }
                }
                write!(f, "\"")
            }
            ObjectKey::Tuple(t) => write!(f, "{}", t),
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
        ObjectKey::String(if b { "true" } else { "false" }.to_string())
    }
}

macro_rules! impl_from_int_for_object_key {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for ObjectKey {
                fn from(n: $ty) -> Self {
                    ObjectKey::Number(BigInt::from(n))
                }
            }
        )*
    };
}

impl_from_int_for_object_key!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl From<BigInt> for ObjectKey {
    fn from(n: BigInt) -> Self {
        ObjectKey::Number(n)
    }
}

/// A key that may be a hole (unresolved placeholder) or a fully resolved key.
///
/// This is a superset of [`ObjectKey`] used exclusively in [`PartialMap`] nodes.
///
/// # Equality Semantics
/// Equality is label-based: `Hole(Some("a")) == Hole(Some("a"))` is true,
/// enabling structural document comparison (e.g., in `assert_eq!`).
/// This is **syntactic** equality only — it does not imply semantic interchangeability.
///
/// Anonymous holes (`Hole(None)`) still compare equal for structural equality,
/// but lookup operations treat them as unique placeholders that never deduplicate.
/// Labeled holes (`Hole(Some(label))`) are deduplicated by label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PartialObjectKey {
    Number(BigInt),
    String(String),
    /// An unresolved hole key: `!` (None) or `!label` (Some(label)).
    Hole(Option<Identifier>),
    /// A tuple key that may contain holes.
    Tuple(Tuple<PartialObjectKey>),
}

impl PartialObjectKey {
    /// Returns true when this key contains an anonymous hole anywhere within it.
    ///
    /// Anonymous holes are syntactically comparable, but they are never
    /// deduplicated when looking up PartialMap entries.
    pub fn contains_anonymous_hole(&self) -> bool {
        match self {
            PartialObjectKey::Hole(None) => true,
            PartialObjectKey::Hole(Some(_))
            | PartialObjectKey::Number(_)
            | PartialObjectKey::String(_) => false,
            PartialObjectKey::Tuple(items) => items.iter().any(Self::contains_anonymous_hole),
        }
    }
}

impl core::fmt::Display for PartialObjectKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PartialObjectKey::Number(n) => write!(f, "{}", n),
            PartialObjectKey::String(s) => {
                write!(f, "\"")?;
                for c in s.chars() {
                    match c {
                        '"' => write!(f, "\\\"")?,
                        '\\' => write!(f, "\\\\")?,
                        _ => write!(f, "{}", c)?,
                    }
                }
                write!(f, "\"")
            }
            PartialObjectKey::Hole(None) => write!(f, "!"),
            PartialObjectKey::Hole(Some(label)) => write!(f, "!{}", label),
            PartialObjectKey::Tuple(t) => write!(f, "{}", t),
        }
    }
}

impl From<ObjectKey> for PartialObjectKey {
    fn from(key: ObjectKey) -> Self {
        match key {
            ObjectKey::Number(n) => PartialObjectKey::Number(n),
            ObjectKey::String(s) => PartialObjectKey::String(s),
            ObjectKey::Tuple(t) => PartialObjectKey::Tuple(Tuple(
                t.0.into_iter().map(PartialObjectKey::from).collect(),
            )),
        }
    }
}

/// Error returned when a [`PartialObjectKey`] contains a hole and cannot be converted to [`ObjectKey`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("PartialObjectKey contains a hole and cannot be converted to ObjectKey")]
pub struct HoleKeyError;

impl TryFrom<PartialObjectKey> for ObjectKey {
    type Error = HoleKeyError;

    fn try_from(key: PartialObjectKey) -> Result<Self, Self::Error> {
        match key {
            PartialObjectKey::Number(n) => Ok(ObjectKey::Number(n)),
            PartialObjectKey::String(s) => Ok(ObjectKey::String(s)),
            PartialObjectKey::Hole(_) => Err(HoleKeyError),
            PartialObjectKey::Tuple(t) => {
                let keys: Result<Vec<ObjectKey>, _> =
                    t.0.into_iter().map(ObjectKey::try_from).collect();
                Ok(ObjectKey::Tuple(Tuple(keys?)))
            }
        }
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

impl core::fmt::Display for Tuple<PartialObjectKey> {
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

macro_rules! impl_from_int_for_primitive_value {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for PrimitiveValue {
                fn from(n: $ty) -> Self {
                    PrimitiveValue::Integer(BigInt::from(n))
                }
            }
        )*
    };
}

impl_from_int_for_primitive_value!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

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
