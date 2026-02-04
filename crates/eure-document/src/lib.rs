#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// Re-export commonly used types for eure! macro users
pub use text::Text;

/// A data structure for representing a Eure document without any span information.
pub mod tree;

/// Identifier type and parser.
pub mod identifier;

/// Unified text type for strings and code.
pub mod text;

/// A type-safe data-type of Eure data-model.
pub mod value;

/// A data structure for representing a Eure document including extensions.
pub mod document;

// Re-export constructor at the root for macro compatibility with `#[eure(crate = ::eure_document)]`
pub use document::constructor;

/// Data structure for representing a path in a Eure document.
pub mod path;

/// Data structure for representing a data-model of Eure.
pub mod data_model;

/// Trait for parsing Rust types from Eure documents.
pub mod parse;

/// Trait for writing Rust types to Eure documents.
pub mod write;

/// Source-level document representation with layout metadata.
///
/// Used for programmatic construction of Eure documents with preserved
/// formatting information (comments, section ordering, etc.).
pub mod source;

/// Macro for building Eure documents.
mod eure_macro;

pub mod map;

/// Zero-sized types for compile-time literal matching in `FromEure`.
pub mod must_be;

pub(crate) mod prelude_internal {
    #![allow(unused_imports)]
    #![allow(deprecated)]
    pub use crate::data_model::*;
    pub use crate::document::constructor::DocumentConstructor;
    pub use crate::document::node::{Node, NodeMap, NodeMut, NodeValue};
    pub use crate::document::{EureDocument, InsertError, InsertErrorKind, NodeId};
    pub use crate::eure;
    pub use crate::identifier::Identifier;
    pub use crate::map::Map;
    pub use crate::path::{EurePath, PathSegment};
    pub use crate::text::{Language, SyntaxHint, Text, TextParseError};
    pub use crate::value::{ObjectKey, PrimitiveValue};
    pub use alloc::boxed::Box;
    pub use alloc::{string::String, string::ToString, vec, vec::Vec};
    pub use thisisplural::Plural;
}
