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
/// # Type Parameters
///
/// - `T`: The target type to write (defaults to `Self`)
///
/// When `T = Self` (the default), this is standard writing.
/// When `T != Self`, `Self` acts as a "strategy" type for writing remote types.
/// This follows the same pattern as `FromEure<'doc, T>`.
///
/// # Remote Type Support
///
/// The `T` parameter enables writing external crate types that can't implement
/// `IntoEure` directly (due to Rust's orphan rule). Define a marker type and
/// implement `IntoEure<RemoteType>` for it:
///
/// ```ignore
/// struct DurationDef;
///
/// impl IntoEure<std::time::Duration> for DurationDef {
///     type Error = WriteError;
///     fn write_to(from: &std::time::Duration, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
///         c.record(|rec| {
///             rec.field("secs", &from.as_secs())?;
///             rec.field("nanos", &from.subsec_nanos())?;
///             Ok(())
///         })
///     }
/// }
/// ```
///
/// Container types (`Option<M>`, `Vec<M>`, etc.) automatically support remote types:
/// if `M: IntoEure<T>`, then `Option<M>: IntoEure<Option<T>>`.
///
/// # Examples
///
/// ```ignore
/// impl IntoEure for User {
///     type Error = WriteError;
///     fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
///         c.record(|rec| {
///             rec.field("name", &from.name)?;
///             rec.field_optional("age", from.age.as_ref())?;
///             Ok(())
///         })
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be written to Eure document",
    label = "this type does not implement `IntoEure`",
    note = "consider adding `#[derive(IntoEure)]` to `{Self}`"
)]
pub trait IntoEure<T: ?Sized = Self> {
    /// The error type returned by writing.
    type Error;

    /// Write a value of type T to the document constructor.
    fn write_to(from: &T, c: &mut DocumentConstructor) -> Result<(), Self::Error>;
}

// ============================================================================
// Primitive implementations
// ============================================================================

impl IntoEure for bool {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Bool(*from))?;
        Ok(())
    }
}

impl IntoEure for i32 {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(*from)))?;
        Ok(())
    }
}

impl IntoEure for i64 {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(*from)))?;
        Ok(())
    }
}

impl IntoEure for u32 {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(*from)))?;
        Ok(())
    }
}

impl IntoEure for u64 {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(*from)))?;
        Ok(())
    }
}

impl IntoEure for usize {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(*from)))?;
        Ok(())
    }
}

impl IntoEure for f32 {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::F32(*from))?;
        Ok(())
    }
}

impl IntoEure for f64 {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::F64(*from))?;
        Ok(())
    }
}

impl IntoEure for BigInt {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Integer(from.clone()))?;
        Ok(())
    }
}

impl IntoEure for String {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(from.clone())))?;
        Ok(())
    }
}

impl IntoEure for str {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(from)))?;
        Ok(())
    }
}

impl IntoEure for &str {
    type Error = WriteError;
    fn write_to(from: &&str, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(*from)))?;
        Ok(())
    }
}

impl IntoEure for Text {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(from.clone()))?;
        Ok(())
    }
}

impl IntoEure for PrimitiveValue {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(from.clone())?;
        Ok(())
    }
}

impl IntoEure for Identifier {
    type Error = WriteError;
    fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(from.as_ref())))?;
        Ok(())
    }
}

// ============================================================================
// Collection implementations
// ============================================================================

/// `Vec<M>` writes `Vec<T>` using M's IntoEure implementation.
///
/// When `M = T`, this is standard `Vec<T>` writing.
/// When `M ≠ T`, M acts as a strategy type for writing remote type T.
#[diagnostic::do_not_recommend]
impl<M, T> IntoEure<Vec<T>> for Vec<M>
where
    M: IntoEure<T>,
    M::Error: From<WriteError>,
{
    type Error = M::Error;
    fn write_to(from: &Vec<T>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_empty_array()
            .map_err(|e| M::Error::from(WriteError::from(e)))?;
        for item in from {
            let scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(None))
                .map_err(|e| M::Error::from(WriteError::from(e)))?;
            M::write_to(item, c)?;
            c.end_scope(scope)
                .map_err(|e| M::Error::from(WriteError::from(e)))?;
        }
        Ok(())
    }
}

/// `Map<K, M>` writes `Map<K, T>` using M's IntoEure implementation.
#[diagnostic::do_not_recommend]
impl<K, M, T> IntoEure<Map<K, T>> for Map<K, M>
where
    K: Clone + Into<ObjectKey>,
    M: IntoEure<T>,
    M::Error: From<WriteError>,
{
    type Error = M::Error;
    fn write_to(from: &Map<K, T>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_empty_map()
            .map_err(|e| M::Error::from(WriteError::from(e)))?;
        for (key, value) in from {
            let scope = c.begin_scope();
            c.navigate(PathSegment::Value(key.clone().into()))
                .map_err(|e| M::Error::from(WriteError::from(e)))?;
            M::write_to(value, c)?;
            c.end_scope(scope)
                .map_err(|e| M::Error::from(WriteError::from(e)))?;
        }
        Ok(())
    }
}

/// `Option<M>` writes `Option<T>` using M's IntoEure implementation.
///
/// When `M = T`, this is standard `Option<T>` writing.
/// When `M ≠ T`, M acts as a strategy type for writing remote type T.
#[diagnostic::do_not_recommend]
impl<M, T> IntoEure<Option<T>> for Option<M>
where
    M: IntoEure<T>,
    M::Error: From<WriteError>,
{
    type Error = M::Error;
    fn write_to(from: &Option<T>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        match from {
            Some(value) => M::write_to(value, c),
            None => {
                c.bind_primitive(PrimitiveValue::Null)
                    .map_err(|e| M::Error::from(WriteError::from(e)))?;
                Ok(())
            }
        }
    }
}

// ============================================================================
// Tuple implementations
// ============================================================================

macro_rules! impl_into_document_tuple {
    ($n:expr, $($idx:tt: $var:ident),+) => {
        #[diagnostic::do_not_recommend]
        impl<$($var: IntoEure<Error = WriteError>),+> IntoEure for ($($var,)+) {
            type Error = WriteError;
            fn write_to(from: &Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
                c.bind_empty_tuple()?;
                $(
                    let scope = c.begin_scope();
                    c.navigate(PathSegment::TupleIndex($idx))?;
                    $var::write_to(&from.$idx, c)?;
                    c.end_scope(scope)?;
                )+
                Ok(())
            }
        }
    };
}

impl_into_document_tuple!(1, 0: A);
impl_into_document_tuple!(2, 0: A, 1: B);
impl_into_document_tuple!(3, 0: A, 1: B, 2: C);
impl_into_document_tuple!(4, 0: A, 1: B, 2: C, 3: D);
impl_into_document_tuple!(5, 0: A, 1: B, 2: C, 3: D, 4: E);
impl_into_document_tuple!(6, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_into_document_tuple!(7, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_into_document_tuple!(8, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
impl_into_document_tuple!(9, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
impl_into_document_tuple!(10, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
impl_into_document_tuple!(11, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K);
impl_into_document_tuple!(12, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L);
impl_into_document_tuple!(13, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M);
impl_into_document_tuple!(14, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M, 13: N);
impl_into_document_tuple!(15, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M, 13: N, 14: O);
impl_into_document_tuple!(16, 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L, 12: M, 13: N, 14: O, 15: P);

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
    ///     rec.field("name", &"Alice")?;
    ///     rec.field("age", &30)?;
    ///     Ok(())
    /// })?;
    /// ```
    pub fn record<F, R>(&mut self, f: F) -> Result<R, WriteError>
    where
        F: FnOnce(&mut RecordWriter<'_>) -> Result<R, WriteError>,
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
    ///     t.next(&"first")?;
    ///     t.next(&42)?;
    ///     t.next(&true)?;
    ///     Ok(())
    /// })?;
    /// ```
    pub fn tuple<F, R>(&mut self, f: F) -> Result<R, WriteError>
    where
        F: FnOnce(&mut TupleWriter<'_>) -> Result<R, WriteError>,
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
    /// c.set_extension("optional", &true)?;
    /// ```
    pub fn set_extension<T: IntoEure<Error = WriteError> + ?Sized>(
        &mut self,
        name: &str,
        value: &T,
    ) -> Result<(), WriteError> {
        let ident: Identifier = name
            .parse()
            .map_err(|_| WriteError::InvalidIdentifier(name.into()))?;
        let scope = self.begin_scope();
        self.navigate(PathSegment::Extension(ident))?;
        T::write_to(value, self)?;
        self.end_scope(scope)?;
        Ok(())
    }

    /// Set an extension value on the current node using a marker/strategy type.
    ///
    /// This is used for writing remote types where `M` implements
    /// `IntoEure<T>` but `T` doesn't implement `IntoEure` itself.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.set_extension_via::<DurationDef, _>("timeout", &duration)?;
    /// ```
    pub fn set_extension_via<M, T>(&mut self, name: &str, value: &T) -> Result<(), M::Error>
    where
        M: IntoEure<T>,
        M::Error: From<WriteError>,
    {
        let ident: Identifier = name
            .parse()
            .map_err(|_| M::Error::from(WriteError::InvalidIdentifier(name.into())))?;
        let scope = self.begin_scope();
        self.navigate(PathSegment::Extension(ident))
            .map_err(|e| M::Error::from(WriteError::from(e)))?;
        M::write_to(value, self)?;
        self.end_scope(scope)
            .map_err(|e| M::Error::from(WriteError::from(e)))?;
        Ok(())
    }

    /// Set an optional extension value on the current node.
    /// Does nothing if the value is `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.set_extension_optional("default", self.default.as_ref())?;
    /// ```
    pub fn set_extension_optional<T: IntoEure<Error = WriteError>>(
        &mut self,
        name: &str,
        value: Option<&T>,
    ) -> Result<(), WriteError> {
        if let Some(v) = value {
            self.set_extension(name, v)?;
        }
        Ok(())
    }

    /// Set an optional extension value on the current node using a marker/strategy type.
    /// Does nothing if the value is `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.set_extension_optional_via::<DurationDef, _>("timeout", self.timeout.as_ref())?;
    /// ```
    pub fn set_extension_optional_via<M, T>(
        &mut self,
        name: &str,
        value: Option<&T>,
    ) -> Result<(), M::Error>
    where
        M: IntoEure<T>,
        M::Error: From<WriteError>,
    {
        if let Some(v) = value {
            self.set_extension_via::<M, T>(name, v)?;
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
    ///         str::write_to(inner, c)?;
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
    /// c.write(&my_value)?;
    /// ```
    pub fn write<T: IntoEure<Error = WriteError>>(&mut self, value: &T) -> Result<(), WriteError> {
        T::write_to(value, self)
    }

    /// Write a value to the current node using a marker/strategy type.
    ///
    /// This is used for writing remote types where `M` implements
    /// `IntoEure<T>` but `T` doesn't implement `IntoEure` itself.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.write_via::<DurationDef, _>(&duration)?;
    /// ```
    pub fn write_via<M, T>(&mut self, value: &T) -> Result<(), M::Error>
    where
        M: IntoEure<T>,
    {
        M::write_to(value, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_bool() {
        let mut c = DocumentConstructor::new();
        bool::write_to(&true, &mut c).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        );
    }

    #[test]
    fn test_primitive_string() {
        let mut c = DocumentConstructor::new();
        str::write_to("hello", &mut c).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }

    #[test]
    fn test_vec() {
        let mut c = DocumentConstructor::new();
        <Vec<i32>>::write_to(&vec![1i32, 2, 3], &mut c).unwrap();
        let doc = c.finish();
        let arr = doc.root().as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_tuple() {
        let mut c = DocumentConstructor::new();
        <(i32, &str, bool)>::write_to(&(1i32, "two", true), &mut c).unwrap();
        let doc = c.finish();
        let tuple = doc.root().as_tuple().unwrap();
        assert_eq!(tuple.len(), 3);
    }

    #[test]
    fn test_record() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", &"Alice")?;
            rec.field("age", &30i32)?;
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
            rec.field("type", &"string")?;
            Ok(())
        })
        .unwrap();
        c.set_extension("optional", &true).unwrap();
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
            rec.field("value", &42i32)?;
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
            rec.field("name", &"Alice")?;
            rec.field_with("address", |c| {
                c.record(|rec| {
                    rec.field("city", &"Tokyo")?;
                    rec.field("zip", &"100-0001")?;
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

    // =========================================================================
    // Remote type support tests
    // =========================================================================

    /// A "remote" type that we can't implement IntoEure for directly.
    #[derive(Debug, PartialEq)]
    struct RemoteDuration {
        secs: u64,
        nanos: u32,
    }

    /// Marker type that implements IntoEure<RemoteDuration>.
    struct RemoteDurationDef;

    impl IntoEure<RemoteDuration> for RemoteDurationDef {
        type Error = WriteError;

        fn write_to(from: &RemoteDuration, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
            c.record(|rec| {
                rec.field("secs", &from.secs)?;
                rec.field("nanos", &from.nanos)?;
                Ok(())
            })
        }
    }

    #[test]
    fn test_remote_type_basic_writing() {
        let duration = RemoteDuration {
            secs: 10,
            nanos: 500,
        };
        let mut c = DocumentConstructor::new();
        c.write_via::<RemoteDurationDef, _>(&duration).unwrap();
        let doc = c.finish();

        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_remote_type_in_vec() {
        let durations = vec![
            RemoteDuration { secs: 1, nanos: 0 },
            RemoteDuration {
                secs: 2,
                nanos: 100,
            },
        ];
        let mut c = DocumentConstructor::new();
        c.write_via::<Vec<RemoteDurationDef>, _>(&durations)
            .unwrap();
        let doc = c.finish();

        let arr = doc.root().as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_remote_type_in_option_some() {
        let duration = Some(RemoteDuration { secs: 5, nanos: 0 });
        let mut c = DocumentConstructor::new();
        c.write_via::<Option<RemoteDurationDef>, _>(&duration)
            .unwrap();
        let doc = c.finish();

        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_remote_type_in_option_none() {
        let duration: Option<RemoteDuration> = None;
        let mut c = DocumentConstructor::new();
        c.write_via::<Option<RemoteDurationDef>, _>(&duration)
            .unwrap();
        let doc = c.finish();

        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Null)
        );
    }

    #[test]
    fn test_field_via() {
        let duration = RemoteDuration { secs: 30, nanos: 0 };
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", &"test")?;
            rec.field_via::<RemoteDurationDef, _>("timeout", &duration)?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();

        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_field_optional_via_some() {
        let duration = Some(RemoteDuration { secs: 60, nanos: 0 });
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", &"test")?;
            rec.field_optional_via::<RemoteDurationDef, _>("timeout", duration.as_ref())?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();

        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_field_optional_via_none() {
        let duration: Option<RemoteDuration> = None;
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", &"test")?;
            rec.field_optional_via::<RemoteDurationDef, _>("timeout", duration.as_ref())?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();

        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 1); // Only "name", no "timeout"
    }

    #[test]
    fn test_set_extension_via() {
        let duration = RemoteDuration {
            secs: 120,
            nanos: 0,
        };
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("value", &42i32)?;
            Ok(())
        })
        .unwrap();
        c.set_extension_via::<RemoteDurationDef, _>("timeout", &duration)
            .unwrap();
        let doc = c.finish();

        let root = doc.root();
        assert!(
            root.extensions
                .contains_key(&"timeout".parse::<Identifier>().unwrap())
        );
    }
}
