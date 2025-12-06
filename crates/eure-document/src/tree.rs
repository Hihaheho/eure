use alloc::string::String;
use alloc::vec::Vec;
use thisisplural::Plural;

/// A data structure for representing a Eure document without any span information.
#[derive(Debug, Clone, PartialEq)]
pub struct EureTree {
    pub sections: Vec<TreeSection>,
    pub bindings: Vec<EureBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeSection {
    pub keys: EureKeys,
    pub body: SectionBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SectionBody {
    Nested(EureTree),
    Bindings(Vec<EureBinding>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EureBinding {
    pub keys: Vec<TreeKey>,
    pub rhs: BindingRhs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingRhs {
    Value(TreeValue),
    Text(String),
    Eure(EureTree),
}

#[derive(Debug, Clone, PartialEq, Eq, Plural)]
pub struct EureKeys(Vec<TreeKey>);

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum TreeKey {
    Ident(String),
    String(String),
    Extension(String),
    ArrayIndex(u32),
    Array,
    TupleIndex(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<TreeValue>),
    Tuple(Vec<TreeValue>),
    Map(Vec<(TreeValue, TreeValue)>),
    Eure(EureTree),
}
