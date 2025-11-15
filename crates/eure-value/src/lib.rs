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
pub mod value;

/// A data structure for representing a Eure document including extensions.
pub mod document;

#[cfg(feature = "std")]
use ahash::AHashMap as Map;
#[cfg(not(feature = "std"))]
type Map<K, V> = alloc::collections::BTreeMap<K, V>;
