use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::identifier::Identifier;
use thisisplural::Plural;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    TypedString(TypedString),
    Code(Code),
    Array(Array),
    Tuple(Tuple<Value>),
    Map(Map),
    Variant(Variant),
    Unit,
    Path(Path),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Key-comparable value which implements `Eq` and `Hash`.
pub enum KeyCmpValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    String(String),
    /// Extension identifier (e.g., $type, $serde)
    Extension(String),
    /// Meta-extension identifier (e.g., $$meta)
    MetaExtension(String),
    Tuple(Tuple<KeyCmpValue>),
    Unit,
}

#[derive(Debug, Clone, PartialEq, Plural)]
pub struct Path(pub Vec<PathSegment>);

#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    /// Regular identifiers like id, description
    Ident(Identifier),
    /// Extension namespace fields starting with $ like $eure, $variant
    Extension(Identifier),
    /// MetaExtKey uses $$ prefix, e.g., $$eure, $$variant
    MetaExt(Identifier),
    Value(KeyCmpValue),
    Array {
        key: Value,
        index: Option<Value>,
    },
}

// A simplified path representation that can be used as a HashMap key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathKey(pub Vec<PathKeySegment>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathKeySegment {
    Ident(String),
    Extension(String),
    Array { key: String, index: Option<usize> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedString {
    pub type_name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Code {
    pub language: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Plural, Default)]
pub struct Array(pub Vec<Value>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Plural, Default)]
pub struct Tuple<T>(pub Vec<T>);

#[derive(Debug, Clone, PartialEq, Plural, Default)]
#[cfg_attr(
    not(feature = "std"),
    plural(len, iter, into_iter, into_iter_ref, from_iter, new)
)]
pub struct Map(pub crate::Map<KeyCmpValue, Value>);

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
            Value::String(s) if s == "untagged" => Some(VariantRepr::Untagged),
            Value::Map(Map(map)) => {
                let tag = map
                    .get(&KeyCmpValue::String("tag".to_string()))
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    });

                let content = map
                    .get(&KeyCmpValue::String("content".to_string()))
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
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
