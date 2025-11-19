#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

/// A data structure for representing a Eure document without any span information.
pub mod tree;

/// Identifier type and parser.
pub mod identifier;

/// String type.
pub mod string;

/// A type-safe data-type of EURE data-model.
mod value;

pub use value::*;

/// A data structure for representing a Eure document including extensions.
pub mod document;

/// Data structure for representing a path in a Eure document.
pub mod path;

/// Code type.
pub mod code;

#[cfg(feature = "std")]
pub use ahash::AHashMap as Map;
#[cfg(not(feature = "std"))]
pub type Map<K, V> = alloc::collections::BTreeMap<K, V>;

pub(crate) mod prelude_internal {
    #![allow(unused_imports)]
    pub use crate::Map;
    pub use crate::code::Code;
    pub use crate::document::constructor::DocumentConstructor;
    pub use crate::document::node::{Node, NodeMut, NodeValue};
    pub use crate::document::{DocumentKey, EureDocument, InsertError, InsertErrorKind, NodeId};
    pub use crate::identifier::Identifier;
    pub use crate::path::{EurePath, PathSegment};
    pub use crate::string::EureString;
    pub use crate::value::PrimitiveValue;
    pub use crate::value::{ObjectKey, Value};
    pub use alloc::boxed::Box;
    pub use alloc::{string::String, string::ToString, vec, vec::Vec};
    pub use thisisplural::Plural;
}
