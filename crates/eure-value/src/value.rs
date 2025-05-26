use alloc::boxed::Box;
use alloc::string::String;
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
}

#[derive(Debug, Clone, PartialEq, Plural)]
pub struct Path(pub Vec<PathSegment>);

#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Extension(Identifier),
    Value(Value),
    Array { key: Value, index: Option<Value> },
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
