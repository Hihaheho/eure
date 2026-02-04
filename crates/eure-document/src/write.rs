//! IntoEure trait for writing Rust types to Eure documents.

extern crate alloc;

pub mod record;
pub mod tuple;

pub use record::RecordWriter;
pub use tuple::TupleWriter;

use alloc::string::String;
use num_bigint::BigInt;

use crate::document::InsertError;
use crate::document::constructor::{DocumentConstructor, ScopeError};
use crate::path::PathSegment;
use crate::prelude_internal::*;
use crate::text::Text;

/// Error type for write operations.
#[derive(Debug, thiserror::Error, Clone)]
pub enum WriteError {
    /// Error during document insertion.
    #[error("insert error: {0}")]
    Insert(#[from] InsertError),

    /// Error during scope management.
    #[error("scope error: {0}")]
    Scope(#[from] ScopeError),

    /// Invalid identifier provided.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
}

/// Trait for writing Rust types to Eure documents.
///
/// Types implementing this trait can be serialized into [`EureDocument`]
/// via [`DocumentConstructor`].
///
/// The generic parameter `T` defaults to `Self`, allowing remote type support
/// via marker types:
/// - `IntoEure` (same as `IntoEure<Self>`) - standard implementation
/// - `IntoEure<RemoteType>` - marker type implements writing for a remote type
///
/// # Examples
///
/// Standard implementation:
/// ```ignore
/// impl IntoEure for User {
///     fn write(value: User, c: &mut DocumentConstructor) -> Result<(), WriteError> {
///         c.record(|rec| {
///             rec.field("name", value.name)?;
///             rec.field_optional("age", value.age)?;
///             Ok(())
///         })
///     }
/// }
/// ```
///
/// Remote type support via marker:
/// ```ignore
/// impl IntoEure<std::time::Duration> for DurationDef {
///     fn write(value: std::time::Duration, c: &mut DocumentConstructor) -> Result<(), WriteError> {
///         c.record(|rec| {
///             rec.field("secs", value.as_secs())?;
///             rec.field("nanos", value.subsec_nanos())?;
///             Ok(())
///         })
///     }
/// }
/// ```
pub trait IntoEure<T = Self>: Sized {
    /// Write a value to the current node in the document constructor.
    fn write(value: T, c: &mut DocumentConstructor) -> Result<(), WriteError>;
}

// ============================================================================
// Primitive implementations
// ============================================================================

impl IntoEure for bool {
    fn write(value: bool, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Bool(value))?;
        Ok(())
    }
}

impl IntoEure for i32 {
    fn write(value: i32, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(value)))?;
        Ok(())
    }
}

impl IntoEure for i64 {
    fn write(value: i64, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(value)))?;
        Ok(())
    }
}

impl IntoEure for u32 {
    fn write(value: u32, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(value)))?;
        Ok(())
    }
}

impl IntoEure for u64 {
    fn write(value: u64, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(value)))?;
        Ok(())
    }
}

impl IntoEure for usize {
    fn write(value: usize, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(value)))?;
        Ok(())
    }
}

impl IntoEure for f32 {
    fn write(value: f32, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::F32(value))?;
        Ok(())
    }
}

impl IntoEure for f64 {
    fn write(value: f64, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::F64(value))?;
        Ok(())
    }
}

impl IntoEure for BigInt {
    fn write(value: BigInt, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(value))?;
        Ok(())
    }
}

impl IntoEure for String {
    fn write(value: String, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(value)))?;
        Ok(())
    }
}

impl<'a> IntoEure for &'a str {
    fn write(value: &'a str, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(value)))?;
        Ok(())
    }
}

impl IntoEure for Text {
    fn write(value: Text, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(value))?;
        Ok(())
    }
}

impl IntoEure for PrimitiveValue {
    fn write(value: PrimitiveValue, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(value)?;
        Ok(())
    }
}

impl IntoEure for Identifier {
    fn write(value: Identifier, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(value.into_string())))?;
        Ok(())
    }
}

// ============================================================================
// Collection implementations
// ============================================================================

impl<M, T> IntoEure<Vec<T>> for Vec<M>
where
    M: IntoEure<T>,
{
    fn write(value: Vec<T>, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_empty_array()?;
        for item in value {
            let scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(None))?;
            M::write(item, c)?;
            c.end_scope(scope)?;
        }
        Ok(())
    }
}

impl<M, K, V> IntoEure<Map<K, V>> for Map<K, M>
where
    M: IntoEure<V>,
    K: Into<ObjectKey>,
{
    fn write(value: Map<K, V>, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_empty_map()?;
        for (key, v) in value {
            let scope = c.begin_scope();
            c.navigate(PathSegment::Value(key.into()))?;
            M::write(v, c)?;
            c.end_scope(scope)?;
        }
        Ok(())
    }
}

impl<M, T> IntoEure<Option<T>> for Option<M>
where
    M: IntoEure<T>,
{
    fn write(value: Option<T>, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        match value {
            Some(v) => M::write(v, c),
            None => {
                c.bind_primitive(PrimitiveValue::Null)?;
                Ok(())
            }
        }
    }
}

// ============================================================================
// Tuple implementations
// ============================================================================

macro_rules! impl_into_document_tuple {
    ($n:expr, $($idx:tt: $marker:ident : $ty:ident),+) => {
        impl<$($marker, $ty),+> IntoEure<($($ty,)+)> for ($($marker,)+)
        where
            $($marker: IntoEure<$ty>),+
        {
            fn write(value: ($($ty,)+), c: &mut DocumentConstructor) -> Result<(), WriteError> {
                c.bind_empty_tuple()?;
                $(
                    let scope = c.begin_scope();
                    c.navigate(PathSegment::TupleIndex($idx))?;
                    $marker::write(value.$idx, c)?;
                    c.end_scope(scope)?;
                )+
                Ok(())
            }
        }
    };
}

impl_into_document_tuple!(1, 0: MA: A);
impl_into_document_tuple!(2, 0: MA: A, 1: MB: B);
impl_into_document_tuple!(3, 0: MA: A, 1: MB: B, 2: MC: C);
impl_into_document_tuple!(4, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D);
impl_into_document_tuple!(5, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E);
impl_into_document_tuple!(6, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F);
impl_into_document_tuple!(7, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G);
impl_into_document_tuple!(8, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H);
impl_into_document_tuple!(9, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I);
impl_into_document_tuple!(10, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I, 9: MJ: J);
impl_into_document_tuple!(11, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I, 9: MJ: J, 10: MK: K);
impl_into_document_tuple!(12, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I, 9: MJ: J, 10: MK: K, 11: ML: L);
impl_into_document_tuple!(13, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I, 9: MJ: J, 10: MK: K, 11: ML: L, 12: MM: M);
impl_into_document_tuple!(14, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I, 9: MJ: J, 10: MK: K, 11: ML: L, 12: MM: M, 13: MN: N);
impl_into_document_tuple!(15, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I, 9: MJ: J, 10: MK: K, 11: ML: L, 12: MM: M, 13: MN: N, 14: MO: O);
impl_into_document_tuple!(16, 0: MA: A, 1: MB: B, 2: MC: C, 3: MD: D, 4: ME: E, 5: MF: F, 6: MG: G, 7: MH: H, 8: MI: I, 9: MJ: J, 10: MK: K, 11: ML: L, 12: MM: M, 13: MN: N, 14: MO: O, 15: MP: P);

// ============================================================================
// DocumentConstructor extensions
// ============================================================================

impl DocumentConstructor {
    /// Write a record (map with string keys) using a closure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.record(|rec| {
    ///     rec.field("name", "Alice")?;
    ///     rec.field("age", 30)?;
    ///     Ok(())
    /// })?;
    /// ```
    pub fn record<F, T>(&mut self, f: F) -> Result<T, WriteError>
    where
        F: FnOnce(&mut RecordWriter<'_>) -> Result<T, WriteError>,
    {
        self.bind_empty_map()?;
        let mut writer = RecordWriter::new(self);
        f(&mut writer)
    }

    /// Write a tuple using a closure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.tuple(|t| {
    ///     t.next("first")?;
    ///     t.next(42)?;
    ///     t.next(true)?;
    ///     Ok(())
    /// })?;
    /// ```
    pub fn tuple<F, T>(&mut self, f: F) -> Result<T, WriteError>
    where
        F: FnOnce(&mut TupleWriter<'_>) -> Result<T, WriteError>,
    {
        self.bind_empty_tuple()?;
        let mut writer = TupleWriter::new(self);
        f(&mut writer)
    }

    /// Set an extension value on the current node.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.set_extension("optional", true)?;
    /// ```
    pub fn set_extension<T: IntoEure>(&mut self, name: &str, value: T) -> Result<(), WriteError> {
        let ident: Identifier = name
            .parse()
            .map_err(|_| WriteError::InvalidIdentifier(name.into()))?;
        let scope = self.begin_scope();
        self.navigate(PathSegment::Extension(ident))?;
        T::write(value, self)?;
        self.end_scope(scope)?;
        Ok(())
    }

    /// Set an optional extension value on the current node.
    /// Does nothing if the value is `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.set_extension_optional("default", self.default)?;
    /// ```
    pub fn set_extension_optional<T: IntoEure>(
        &mut self,
        name: &str,
        value: Option<T>,
    ) -> Result<(), WriteError> {
        if let Some(v) = value {
            self.set_extension(name, v)?;
        }
        Ok(())
    }

    /// Set the `$variant` extension for union types.
    ///
    /// # Example
    ///
    /// ```ignore
    /// match self {
    ///     MyEnum::Foo(inner) => {
    ///         c.set_variant("foo")?;
    ///         c.write(inner)?;
    ///     }
    /// }
    /// ```
    pub fn set_variant(&mut self, variant: &str) -> Result<(), WriteError> {
        self.set_extension("variant", variant)
    }

    /// Write a value implementing `IntoEure` to the current node.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.write(my_value)?;
    /// ```
    pub fn write<T: IntoEure>(&mut self, value: T) -> Result<(), WriteError> {
        T::write(value, self)
    }

    /// Write a remote type using a marker type.
    ///
    /// This enables writing types from external crates that can't implement
    /// `IntoEure` directly due to Rust's orphan rule.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // DurationDef implements IntoEure<std::time::Duration>
    /// c.write_via::<DurationDef, _>(duration)?;
    /// ```
    pub fn write_via<M, T>(&mut self, value: T) -> Result<(), WriteError>
    where
        M: IntoEure<T>,
    {
        M::write(value, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_bool() {
        let mut c = DocumentConstructor::new();
        c.write(true).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        );
    }

    #[test]
    fn test_primitive_string() {
        let mut c = DocumentConstructor::new();
        c.write("hello").unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }

    #[test]
    fn test_vec() {
        let mut c = DocumentConstructor::new();
        c.write(vec![1i32, 2, 3]).unwrap();
        let doc = c.finish();
        let arr = doc.root().as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_tuple() {
        let mut c = DocumentConstructor::new();
        c.write((1i32, "two", true)).unwrap();
        let doc = c.finish();
        let tuple = doc.root().as_tuple().unwrap();
        assert_eq!(tuple.len(), 3);
    }

    #[test]
    fn test_record() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "Alice")?;
            rec.field("age", 30i32)?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_set_extension() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("type", "string")?;
            Ok(())
        })
        .unwrap();
        c.set_extension("optional", true).unwrap();
        let doc = c.finish();

        let root = doc.root();
        assert!(
            root.extensions
                .contains_key(&"optional".parse::<Identifier>().unwrap())
        );
    }

    #[test]
    fn test_set_variant() {
        let mut c = DocumentConstructor::new();
        c.set_variant("foo").unwrap();
        c.record(|rec| {
            rec.field("value", 42i32)?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();

        let root = doc.root();
        assert!(
            root.extensions
                .contains_key(&"variant".parse::<Identifier>().unwrap())
        );
    }

    #[test]
    fn test_nested_record() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "Alice")?;
            rec.field_with("address", |c| {
                c.record(|rec| {
                    rec.field("city", "Tokyo")?;
                    rec.field("zip", "100-0001")?;
                    Ok(())
                })
            })?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_write_via() {
        // Marker type for Duration-like struct
        struct DurationMarker;
        struct DurationLike {
            secs: u64,
            nanos: u32,
        }

        impl IntoEure<DurationLike> for DurationMarker {
            fn write(value: DurationLike, c: &mut DocumentConstructor) -> Result<(), WriteError> {
                c.record(|rec| {
                    rec.field("secs", value.secs)?;
                    rec.field("nanos", value.nanos)?;
                    Ok(())
                })
            }
        }

        let mut c = DocumentConstructor::new();
        c.write_via::<DurationMarker, _>(DurationLike {
            secs: 60,
            nanos: 123,
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 2);
    }
}
