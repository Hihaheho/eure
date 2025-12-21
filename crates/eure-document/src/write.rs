//! IntoDocument trait for writing Rust types to Eure documents.

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
/// # Examples
///
/// ```ignore
/// impl IntoDocument for User {
///     fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
///         c.record(|rec| {
///             rec.field("name", self.name)?;
///             rec.field_optional("age", self.age)?;
///             Ok(())
///         })
///     }
/// }
/// ```
pub trait IntoDocument {
    /// Write this value to the current node in the document constructor.
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError>;
}

// ============================================================================
// Primitive implementations
// ============================================================================

impl IntoDocument for bool {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Bool(self))?;
        Ok(())
    }
}

impl IntoDocument for i32 {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(self)))?;
        Ok(())
    }
}

impl IntoDocument for i64 {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(self)))?;
        Ok(())
    }
}

impl IntoDocument for u32 {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(self)))?;
        Ok(())
    }
}

impl IntoDocument for u64 {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(self)))?;
        Ok(())
    }
}

impl IntoDocument for usize {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(BigInt::from(self)))?;
        Ok(())
    }
}

impl IntoDocument for f32 {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::F32(self))?;
        Ok(())
    }
}

impl IntoDocument for f64 {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::F64(self))?;
        Ok(())
    }
}

impl IntoDocument for BigInt {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Integer(self))?;
        Ok(())
    }
}

impl IntoDocument for String {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(self)))?;
        Ok(())
    }
}

impl IntoDocument for &str {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(self)))?;
        Ok(())
    }
}

impl IntoDocument for Text {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(self))?;
        Ok(())
    }
}

impl IntoDocument for PrimitiveValue {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(self)?;
        Ok(())
    }
}

impl IntoDocument for Identifier {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(self.into_string())))?;
        Ok(())
    }
}

// ============================================================================
// Collection implementations
// ============================================================================

impl<T: IntoDocument> IntoDocument for Vec<T> {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_empty_array()?;
        for item in self {
            let scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(None))?;
            item.write_to(c)?;
            c.end_scope(scope)?;
        }
        Ok(())
    }
}

impl<K, V> IntoDocument for Map<K, V>
where
    K: Into<ObjectKey>,
    V: IntoDocument,
{
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        c.bind_empty_map()?;
        for (key, value) in self {
            let scope = c.begin_scope();
            c.navigate(PathSegment::Value(key.into()))?;
            value.write_to(c)?;
            c.end_scope(scope)?;
        }
        Ok(())
    }
}

impl<T: IntoDocument> IntoDocument for Option<T> {
    fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        match self {
            Some(value) => value.write_to(c),
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
    ($n:expr, $($idx:tt: $var:ident),+) => {
        impl<$($var: IntoDocument),+> IntoDocument for ($($var,)+) {
            fn write_to(self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
                c.bind_empty_tuple()?;
                $(
                    let scope = c.begin_scope();
                    c.navigate(PathSegment::TupleIndex($idx))?;
                    self.$idx.write_to(c)?;
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
    pub fn set_extension<T: IntoDocument>(
        &mut self,
        name: &str,
        value: T,
    ) -> Result<(), WriteError> {
        let ident: Identifier = name
            .parse()
            .map_err(|_| WriteError::InvalidIdentifier(name.into()))?;
        let scope = self.begin_scope();
        self.navigate(PathSegment::Extension(ident))?;
        value.write_to(self)?;
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
    pub fn set_extension_optional<T: IntoDocument>(
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
    ///         inner.write_to(c)?;
    ///     }
    /// }
    /// ```
    pub fn set_variant(&mut self, variant: &str) -> Result<(), WriteError> {
        self.set_extension("variant", variant)
    }

    /// Write a value implementing `IntoDocument` to the current node.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.write(my_value)?;
    /// ```
    pub fn write<T: IntoDocument>(&mut self, value: T) -> Result<(), WriteError> {
        value.write_to(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_bool() {
        let mut c = DocumentConstructor::new();
        true.write_to(&mut c).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        );
    }

    #[test]
    fn test_primitive_string() {
        let mut c = DocumentConstructor::new();
        "hello".write_to(&mut c).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }

    #[test]
    fn test_vec() {
        let mut c = DocumentConstructor::new();
        vec![1i32, 2, 3].write_to(&mut c).unwrap();
        let doc = c.finish();
        let arr = doc.root().as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_tuple() {
        let mut c = DocumentConstructor::new();
        (1i32, "two", true).write_to(&mut c).unwrap();
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
}
