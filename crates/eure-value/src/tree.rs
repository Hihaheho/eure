use alloc::string::String;
use alloc::vec::Vec;
use thisisplural::Plural;

/// A data structure for representing a Eure document without any span information.
pub struct EureDocument {
    pub sections: Vec<EureSection>,
    pub bindings: Vec<EureBinding>,
}

pub struct EureSection {
    pub keys: EureKeys,
    pub body: SectionBody,
}

pub enum SectionBody {
    Nested(EureDocument),
    Bindings(Vec<EureBinding>),
}

pub struct EureBinding {
    pub keys: Vec<EureKey>,
    pub rhs: BindingRhs,
}

pub enum BindingRhs {
    Value(EureValue),
    Text(String),
    Eure(EureDocument),
}

#[derive(Debug, Clone, PartialEq, Eq, Plural)]
pub struct EureKeys(Vec<EureKey>);

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum EureKey {
    Ident(String),
    String(String),
    Extension(String),
    ArrayIndex(u32),
    Array,
    TupleIndex(u8),
}

pub enum EureValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<EureValue>),
    Tuple(Vec<EureValue>),
    Map(Vec<(EureValue, EureValue)>),
    Eure(EureDocument),
}
