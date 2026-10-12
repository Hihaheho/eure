#![no_std]
extern crate alloc;

#[cfg(any(feature = "std", test))]
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

/// Arbitrary-precision decimal number.
pub mod decimal;

/// Binary data encodings for the reserved `base64` and `hex` languages.
pub mod bytes;

/// A data structure for representing a Eure document including extensions.
pub mod document;

// Re-export constructor at the root for macro compatibility with `#[eure(crate = ::eure_document)]`
pub use document::constructor;

use crate::write::IntoEure;

/// Data structure for representing a path in a Eure document.
pub mod path;

/// Data structure for representing a data-model of Eure.
pub mod data_model;

/// Trait for parsing Rust types from Eure documents.
pub mod parse;

/// Trait for writing Rust types to Eure documents.
pub mod write;

/// Total, verified layout plan: a coverage-checked projection from
/// [`document::EureDocument`] to [`source::SourceDocument`].
pub mod plan;
/// Source-level document representation with layout metadata.
///
/// Used for programmatic construction of Eure documents with preserved
/// formatting information (comments, section ordering, etc.).
pub mod source;

/// Macro for building Eure documents.
mod eure_macro;

#[doc(hidden)]
pub mod __private {
    use crate::{decimal::Decimal, value::PrimitiveValue};

    /// Converts a Rust literal used in `eure!` into a primitive value.
    ///
    /// Rust types unsuffixed float literals as `f64`, but in Eure they are exact decimals,
    /// so the literal's source token decides: `1.5` is a decimal and `1.5f64` stays a float.
    pub fn literal(token: &str, value: impl Into<PrimitiveValue>) -> PrimitiveValue {
        let value = value.into();
        if !matches!(value, PrimitiveValue::F64(_)) || token.ends_with("f64") {
            return value;
        }
        let digits: alloc::string::String = token.chars().filter(|&c| c != '_').collect();
        let decimal: Decimal = digits
            .parse()
            .expect("an unsuffixed Rust float literal is a valid decimal");
        PrimitiveValue::Decimal(decimal)
    }
}

pub mod map;

/// Hash-based collections used throughout the crate.
pub mod collections {
    /// Hash builder for hash-based collections.
    ///
    /// Randomized per process and `no_std`-compatible, so the collection types are identical
    /// whether or not the `std` feature is enabled.
    pub type HashBuilder = foldhash::fast::RandomState;

    /// Insertion-ordered map using [`HashBuilder`].
    pub type IndexMap<K, V> = indexmap::IndexMap<K, V, HashBuilder>;

    /// Insertion-ordered set using [`HashBuilder`].
    pub type IndexSet<T> = indexmap::IndexSet<T, HashBuilder>;
}

/// Zero-sized types for compile-time literal matching in `FromEure`.
pub mod must_be;

/// Proxy types for use with `#[eure(via = "...")]` attribute.
pub mod proxy;

pub(crate) mod prelude_internal {
    #![allow(unused_imports)]
    #![allow(deprecated)]
    pub use crate::data_model::*;
    pub use crate::document::constructor::DocumentConstructor;
    pub use crate::document::node::{Node, NodeMap, NodeMut, NodeSet, NodeValue};
    pub use crate::document::{EureDocument, InsertError, InsertErrorKind, NodeId};
    pub use crate::eure;
    pub use crate::identifier::Identifier;
    pub use crate::map::Map;
    pub use crate::path::{ArrayIndexKind, EurePath, PathSegment};
    pub use crate::text::{Language, SyntaxHint, Text, TextParseError};
    pub use crate::value::{ObjectKey, PrimitiveValue};
    pub use alloc::boxed::Box;
    pub use alloc::{string::String, string::ToString, vec, vec::Vec};
    pub use thisisplural::Plural;
}

impl IntoEure for regex::Regex {
    type Error = write::WriteError;

    fn write(value: Self, c: &mut constructor::DocumentConstructor) -> Result<(), Self::Error> {
        c.write(value.as_str())
    }
}
