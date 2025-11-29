#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

/// A data structure for representing a Eure document without any span information.
pub mod tree;

/// Identifier type and parser.
pub mod identifier;

/// Unified text type for strings and code.
pub mod text;

/// A type-safe data-type of EURE data-model.
pub mod value;

/// A data structure for representing a Eure document including extensions.
pub mod document;

/// Data structure for representing a path in a Eure document.
pub mod path;

/// Data structure for representing a data-model of EURE.
pub mod data_model;

#[cfg(feature = "std")]
pub use ahash::AHashMap as Map;
#[cfg(not(feature = "std"))]
pub type Map<K, V> = alloc::collections::BTreeMap<K, V>;

pub(crate) mod prelude_internal {
    #![allow(unused_imports)]
    #![allow(deprecated)]
    pub use crate::Map;
    pub use crate::data_model::*;
    pub use crate::document::constructor::DocumentConstructor;
    pub use crate::document::node::{Node, NodeMut, NodeValue};
    pub use crate::document::{EureDocument, InsertError, InsertErrorKind, NodeId};
    pub use crate::identifier::Identifier;
    pub use crate::path::{EurePath, PathSegment};
    pub use crate::text::{Language, SyntaxHint, Text, TextParseError};
    pub use crate::value::PrimitiveValue;
    pub use crate::value::{ObjectKey, Value};
    pub use alloc::boxed::Box;
    pub use alloc::{string::String, string::ToString, vec, vec::Vec};
    pub use thisisplural::Plural;
}
