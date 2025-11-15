use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::identifier::Identifier;
use crate::string::EureString;
use thisisplural::Plural;

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
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

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Primitive(PrimitiveValue),
    Array(Array),
    Tuple(Tuple<Value>),
    Map(Map),
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
pub struct EurePath(pub Vec<PathSegment>);

impl EurePath {
    /// Create an empty path representing the document root
    pub fn root() -> Self {
        EurePath(Vec::new())
    }

    /// Check if this is the root path
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Create a Path from PathSegments
    pub fn from_segments(segments: &[PathSegment]) -> Self {
        EurePath(segments.to_vec())
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
#[plural(len, is_empty, iter, into_iter, into_iter_ref, from_iter, new)]
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
            Value::Primitive(PrimitiveValue::String(s)) if s == "untagged" => {
                Some(VariantRepr::Untagged)
            }
            Value::Map(Map(map)) => {
                let tag = map
                    .get(&KeyCmpValue::String("tag".to_string()))
                    .and_then(|v| match v {
                        Value::Primitive(PrimitiveValue::String(s)) => Some(s.as_str().to_string()),
                        _ => None,
                    });

                let content = map
                    .get(&KeyCmpValue::String("content".to_string()))
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
