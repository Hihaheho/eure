use num_bigint::BigInt;

use crate::{code::Code, prelude_internal::*};

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveValue {
    Null,
    Bool(bool),
    BigInt(BigInt),
    F32(f32),
    F64(f64),
    String(EureString),
    Code(Code),
    CodeBlock(Code),
    Unit,
    Hole,
    Variant(Variant),
    Path(EurePath),
}
impl PrimitiveValue {
    pub fn as_code(&self) -> Option<&Code> {
        if let Self::Code(code) = self {
            Some(code)
        } else {
            None
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
/// EURE restricts map keys to four types — `String`, `Bool`, `Integer`,
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

#[derive(Debug, Clone, PartialEq)]
pub enum VariantRepr {
    /// Default representation: {"variant-name": {...}}
    External,

    /// Internal tagging: {"type": "variant-name", ...fields...}
    Internal { tag: String },

    /// Adjacent tagging: {"type": "variant-name", "content": {...}}
    Adjacent { tag: String, content: String },

    /// Untagged: just the content without variant information
    Untagged,
}

impl VariantRepr {
    /// Create a VariantRepr from $variant.repr annotation value
    pub fn from_annotation(value: &Value) -> Option<Self> {
        match value {
            Value::Primitive(PrimitiveValue::String(s)) if s == "untagged" => {
                Some(VariantRepr::Untagged)
            }
            Value::Map(Map(map)) => {
                let tag = map
                    .get(&ObjectKey::String("tag".to_string()))
                    .and_then(|v| match v {
                        Value::Primitive(PrimitiveValue::String(s)) => Some(s.as_str().to_string()),
                        _ => None,
                    });

                let content = map
                    .get(&ObjectKey::String("content".to_string()))
                    .and_then(|v| match v {
                        Value::Primitive(PrimitiveValue::String(s)) => Some(s.as_str().to_string()),
                        _ => None,
                    });

                match (tag, content) {
                    (Some(tag), Some(content)) => Some(VariantRepr::Adjacent { tag, content }),
                    (Some(tag), None) => Some(VariantRepr::Internal { tag }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
