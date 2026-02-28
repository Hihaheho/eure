use indexmap::IndexMap;
use num_bigint::BigInt;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DecimalInt(pub BigInt);

impl DecimalInt {
    pub fn new(value: impl AsRef<str>) -> Self {
        let raw = value.as_ref();
        let trimmed = raw.trim();
        let parsed = BigInt::from_str(trimmed)
            .unwrap_or_else(|_| panic!("invalid decimal integer literal: `{raw}`"));
        Self(parsed)
    }

    pub fn as_bigint(&self) -> &BigInt {
        &self.0
    }

    pub fn canonicalized(&self) -> Self {
        canonicalize_decimal(self)
    }
}

impl fmt::Display for DecimalInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for DecimalInt {
    type Err = num_bigint::ParseBigIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(BigInt::from_str(s.trim())?))
    }
}

impl From<BigInt> for DecimalInt {
    fn from(value: BigInt) -> Self {
        Self(value)
    }
}

impl From<DecimalInt> for BigInt {
    fn from(value: DecimalInt) -> Self {
        value.0
    }
}

pub fn canonicalize_decimal(value: &DecimalInt) -> DecimalInt {
    value.clone()
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueIr {
    Null,
    Bool(bool),
    Integer(DecimalInt),
    Float(f64),
    Text(TextValueIr),
    Array(Vec<ValueIr>),
    Tuple(Vec<ValueIr>),
    Map(IndexMap<ObjectKeyIr, ValueIr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextValueIr {
    pub value: String,
    pub language: TextLanguageIr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextLanguageIr {
    Plain,
    Implicit,
    Tagged(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectKeyIr {
    String(String),
    Integer(DecimalInt),
    Tuple(Vec<ObjectKeyIr>),
}
