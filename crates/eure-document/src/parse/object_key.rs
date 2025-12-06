//! ParseObjectKey trait and implementations for object key types.

use crate::{parse::ParseErrorKind, prelude_internal::*};
use num_bigint::BigInt;

/// Trait for types that can be used as object keys when parsing from Eure documents.
///
/// This trait abstracts over key types that can be used in `Map<K, V>`, supporting
/// owned (`ObjectKey`), borrowed (`&'doc ObjectKey`), and primitive types (`bool`,
/// `BigInt`, `String`) for type-constrained maps.
///
/// # Lifetime Parameter
///
/// The `'doc` lifetime ties the parsed key to the document's lifetime, allowing
/// zero-copy parsing for reference types.
///
/// # Type Constraints
///
/// Implementors must satisfy:
/// - `Eq + Hash` for use with `AHashMap` (std feature)
/// - `Ord` for use with `BTreeMap` (no_std)
///
/// # Examples
///
/// ```ignore
/// use eure_document::{EureDocument, Map, ObjectKey, ParseDocument};
///
/// // Parse map with borrowed keys (zero-copy)
/// let map: Map<&ObjectKey, String> = doc.parse(root_id)?;
///
/// // Parse map with owned keys
/// let map: Map<ObjectKey, String> = doc.parse(root_id)?;
///
/// // Parse map with type-constrained keys
/// let map: Map<String, i32> = doc.parse(root_id)?;
/// ```
pub trait ParseObjectKey<'doc>: Sized + Eq + core::hash::Hash + Ord {
    /// Parse an object key from the given ObjectKey reference in the document.
    ///
    /// # Arguments
    ///
    /// * `key` - Reference to the ObjectKey in the document's NodeMap
    ///
    /// # Returns
    ///
    /// Returns `Self` on success, or `ParseErrorKind` if the key cannot be
    /// converted to the target type (e.g., trying to parse a Bool as String).
    ///
    /// # Errors
    ///
    /// Returns `ParseErrorKind::TypeMismatch` when the ObjectKey variant doesn't
    /// match the expected type.
    fn from_object_key(key: &'doc ObjectKey) -> Result<Self, ParseErrorKind>;
}

// ============================================================================
// Implementation for &'doc ObjectKey (borrowed, zero-copy)
// ============================================================================

impl<'doc> ParseObjectKey<'doc> for &'doc ObjectKey {
    fn from_object_key(key: &'doc ObjectKey) -> Result<Self, ParseErrorKind> {
        Ok(key)
    }
}

// ============================================================================
// Implementation for ObjectKey (owned, cloned)
// ============================================================================

impl ParseObjectKey<'_> for ObjectKey {
    fn from_object_key(key: &ObjectKey) -> Result<Self, ParseErrorKind> {
        Ok(key.clone())
    }
}

// ============================================================================
// Implementation for bool (type-constrained)
// ============================================================================

impl ParseObjectKey<'_> for bool {
    fn from_object_key(key: &ObjectKey) -> Result<Self, ParseErrorKind> {
        match key {
            ObjectKey::Bool(b) => Ok(*b),
            _ => Err(ParseErrorKind::TypeMismatch {
                expected: crate::value::ValueKind::Bool,
                actual: key_to_value_kind(key),
            }),
        }
    }
}

// ============================================================================
// Implementation for BigInt (type-constrained)
// ============================================================================

impl ParseObjectKey<'_> for BigInt {
    fn from_object_key(key: &ObjectKey) -> Result<Self, ParseErrorKind> {
        match key {
            ObjectKey::Number(n) => Ok(n.clone()),
            _ => Err(ParseErrorKind::TypeMismatch {
                expected: crate::value::ValueKind::Integer,
                actual: key_to_value_kind(key),
            }),
        }
    }
}

// ============================================================================
// Implementation for String (type-constrained)
// ============================================================================

impl ParseObjectKey<'_> for String {
    fn from_object_key(key: &ObjectKey) -> Result<Self, ParseErrorKind> {
        match key {
            ObjectKey::String(s) => Ok(s.clone()),
            _ => Err(ParseErrorKind::TypeMismatch {
                expected: crate::value::ValueKind::Text,
                actual: key_to_value_kind(key),
            }),
        }
    }
}

// ============================================================================
// Helper function to convert ObjectKey to ValueKind for error messages
// ============================================================================

fn key_to_value_kind(key: &ObjectKey) -> crate::value::ValueKind {
    match key {
        ObjectKey::Bool(_) => crate::value::ValueKind::Bool,
        ObjectKey::Number(_) => crate::value::ValueKind::Integer,
        ObjectKey::String(_) => crate::value::ValueKind::Text,
        ObjectKey::Tuple(_) => crate::value::ValueKind::Tuple,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Tuple;

    #[test]
    fn test_parse_object_key_borrowed() {
        let key = ObjectKey::String("test".into());
        let borrowed: &ObjectKey = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(borrowed, &key);
    }

    #[test]
    fn test_parse_object_key_owned() {
        let key = ObjectKey::Bool(true);
        let owned: ObjectKey = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(owned, key);
    }

    #[test]
    fn test_borrowed_key_is_zero_copy() {
        let key = ObjectKey::String("test".into());
        let borrowed: &ObjectKey = ParseObjectKey::from_object_key(&key).unwrap();
        assert!(core::ptr::eq(&key, borrowed));
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn test_parse_bool_key() {
        let key = ObjectKey::Bool(true);
        let b: bool = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(b, true);
    }

    #[test]
    fn test_parse_bool_key_type_mismatch() {
        let key = ObjectKey::String("not a bool".into());
        let result: Result<bool, _> = ParseObjectKey::from_object_key(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_bigint_key() {
        let key = ObjectKey::Number(BigInt::from(42));
        let n: BigInt = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(n, BigInt::from(42));
    }

    #[test]
    fn test_parse_bigint_key_type_mismatch() {
        let key = ObjectKey::Bool(false);
        let result: Result<BigInt, _> = ParseObjectKey::from_object_key(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_string_key() {
        let key = ObjectKey::String("hello".into());
        let s: String = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_parse_string_key_type_mismatch() {
        let key = ObjectKey::Number(BigInt::from(123));
        let result: Result<String, _> = ParseObjectKey::from_object_key(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tuple_key() {
        let key = ObjectKey::Tuple(Tuple(vec![
            ObjectKey::String("a".into()),
            ObjectKey::Number(BigInt::from(1)),
        ]));
        let owned: ObjectKey = ParseObjectKey::from_object_key(&key).unwrap();
        assert_eq!(owned, key);
    }
}
