use core::fmt::Display;

use crate::prelude_internal::*;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// Regular identifiers like id, description
    Ident(Identifier),
    /// Extension namespace fields starting with $ like $eure, $variant
    Extension(Identifier),
    /// Arbitrary value used as key
    Value(ObjectKey),
    /// Tuple element index (0-255)
    TupleIndex(u8),
    /// Array element access
    ArrayIndex(Option<usize>),
}

impl Display for EurePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for segment in &self.0 {
            match segment {
                PathSegment::Ident(id) => write!(f, ".{}", id)?,
                PathSegment::Extension(id) => write!(f, ".${}", id)?,
                PathSegment::Value(key) => write!(f, ".{}", key)?,
                PathSegment::TupleIndex(index) => write!(f, ".#{}", index)?,
                PathSegment::ArrayIndex(Some(index)) => write!(f, "[{}]", index)?,
                PathSegment::ArrayIndex(None) => write!(f, "[]")?,
            }
        }
        Ok(())
    }
}
