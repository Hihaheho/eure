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
    Code(Code),
    CodeBlock(Code),
    Array(Array),
    Tuple(Tuple<Value>),
    Map(Map),
    Variant(Variant),
    Unit,
    Path(Path),
    Hole,
    /// Meta-extension (Ident with $$ grammar token)
    MetaExtension(Identifier),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Key-comparable value which implements `Eq` and `Hash`.
pub enum KeyCmpValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    String(String),
    Tuple(Tuple<KeyCmpValue>),
    Unit,
    Hole,
    /// Meta-extension key (Ident with $$ grammar token)
    MetaExtension(Identifier),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Plural)]
pub struct Path(pub Vec<PathSegment>);

impl Path {
    /// Create an empty path representing the document root
    pub fn root() -> Self {
        Path(Vec::new())
    }
    
    /// Check if this is the root path
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
    
    /// Create a Path from PathSegments
    pub fn from_segments(segments: &[PathSegment]) -> Self {
        Path(segments.to_vec())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// Regular identifiers like id, description
    Ident(Identifier),
    /// Extension namespace fields starting with $ like $eure, $variant
    Extension(Identifier),
    /// MetaExtKey uses $$ prefix, e.g., $$eure, $$variant
    MetaExt(Identifier),
    /// Arbitrary value used as key
    Value(KeyCmpValue),
    /// Tuple element index (0-255)
    TupleIndex(u8),
    /// Array element access
    ArrayIndex(Option<u8>),
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

// Helper trait implementations for KeyCmpValue

impl PartialEq<str> for KeyCmpValue {
    fn eq(&self, other: &str) -> bool {
        match self {
            KeyCmpValue::String(s) => s == other,
            _ => false,
        }
    }
}

impl PartialEq<&str> for KeyCmpValue {
    fn eq(&self, other: &&str) -> bool {
        match self {
            KeyCmpValue::String(s) => s == *other,
            _ => false,
        }
    }
}

impl From<String> for KeyCmpValue {
    fn from(s: String) -> Self {
        KeyCmpValue::String(s)
    }
}

impl From<&str> for KeyCmpValue {
    fn from(s: &str) -> Self {
        KeyCmpValue::String(s.to_string())
    }
}

impl From<u64> for KeyCmpValue {
    fn from(n: u64) -> Self {
        KeyCmpValue::U64(n)
    }
}

impl From<i64> for KeyCmpValue {
    fn from(n: i64) -> Self {
        KeyCmpValue::I64(n)
    }
}

impl From<bool> for KeyCmpValue {
    fn from(b: bool) -> Self {
        KeyCmpValue::Bool(b)
    }
}
