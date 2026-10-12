//! IntoEure trait for writing Rust types to Eure documents.

extern crate alloc;

pub mod record;
pub mod tuple;

pub use record::RecordWriter;
pub use tuple::TupleWriter;

use crate::document::constructor::DocumentConstructor;

use alloc::borrow::{Cow, ToOwned};
use alloc::string::String;
use num_bigint::BigInt;

use indexmap::{IndexMap, IndexSet};

use crate::document::InsertError;
use crate::document::constructor::ScopeError;
use crate::identifier::IdentifierError;
use crate::parse::VariantPath;
use crate::path::{ArrayIndexKind, PathSegment};
use crate::prelude_internal::*;
use crate::text::Text;
use crate::value::ValueKind;
use core::any::type_name;

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

    /// Invalid `$variant` extension type (must be text).
    #[error("invalid $variant extension type: expected text, got {actual}")]
    InvalidVariantExtensionType { actual: ValueKind },

    /// Invalid `$variant` path syntax.
    #[error("invalid $variant path: {source}")]
    InvalidVariantPath { source: IdentifierError },

    /// Unknown variant when writing a non-exhaustive proxy enum.
    #[error("non-exhaustive enum variant for {type_name}")]
    NonExhaustiveVariant { type_name: &'static str },

    /// Flatten target cannot be written as record fields.
    #[error("flatten target is not record-like: {type_name}")]
    FlattenTargetNotRecordLike { type_name: &'static str },

    /// A set element must be an integer, a plaintext string, or a tuple of those.
    #[error("invalid set element: expected integer, plaintext string, or tuple, got {actual}")]
    InvalidSetElement { actual: ValueKind },

    /// A set element cannot carry extensions.
    #[error("set elements cannot have extensions")]
    SetElementWithExtensions,

    /// Two elements of a Rust set wrote the same Eure value.
    #[error("duplicate set element: {element}")]
    DuplicateSetElement { element: ObjectKey },
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
///     type Error = WriteError;
///
///     fn write(value: User, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
///         c.record(|rec| {
///             rec.field("name", value.name)?;
///             rec.field_optional("age", value.age)?;
///             Ok::<(), WriteError>(())
///         })
///     }
/// }
/// ```
///
/// Remote type support via marker:
/// ```ignore
/// impl IntoEure<std::time::Duration> for DurationDef {
///     type Error = WriteError;
///
///     fn write(value: std::time::Duration, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
///         c.record(|rec| {
///             rec.field("secs", value.as_secs())?;
///             rec.field("nanos", value.subsec_nanos())?;
///             Ok::<(), WriteError>(())
///         })
///     }
/// }
/// ```
pub trait IntoEure<T = Self>: Sized {
    /// The error type returned when writing.
    ///
    /// This must be able to represent `WriteError` so document-constructor
    /// failures can be propagated through custom user errors.
    type Error: From<WriteError>;

    /// Write a value to the current node in the document constructor.
    fn write(value: T, c: &mut DocumentConstructor) -> Result<(), Self::Error>;

    /// Write a value as flattened record fields.
    ///
    /// The default implementation returns a runtime error. Types that are
    /// record-like (e.g. named structs, map-like containers) should override
    /// this to emit fields into `rec`.
    fn write_flatten(value: T, rec: &mut RecordWriter<'_>) -> Result<(), Self::Error> {
        let _ = value;
        let _ = rec;
        Err(WriteError::FlattenTargetNotRecordLike {
            type_name: type_name::<T>(),
        }
        .into())
    }
}

impl IntoEure for EureDocument {
    type Error = WriteError;

    fn write(value: Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.write_subtree(&value, value.get_root_id())
    }
}

fn write_subtree_node(
    src: &EureDocument,
    node_id: NodeId,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    let node = src.node(node_id);

    match &node.content {
        NodeValue::Hole(label) => {
            c.bind_hole(label.clone())?;
        }
        NodeValue::Primitive(prim) => {
            c.bind_primitive(prim.clone())?;
        }
        NodeValue::Set(set) => {
            c.bind_empty_set()?;
            for element in set.keys() {
                c.add_set_element(element.clone())?;
            }
        }
        NodeValue::Array(array) => {
            c.bind_empty_array()?;
            for &child_id in array.iter() {
                let scope = c.begin_scope();
                c.navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))?;
                write_subtree_node(src, child_id, c)?;
                c.end_scope(scope)?;
            }
        }
        NodeValue::Tuple(tuple) => {
            c.bind_empty_tuple()?;
            for (index, &child_id) in tuple.iter().enumerate() {
                let scope = c.begin_scope();
                c.navigate(PathSegment::TupleIndex(index as u8))?;
                write_subtree_node(src, child_id, c)?;
                c.end_scope(scope)?;
            }
        }
        NodeValue::Map(map) => {
            c.bind_empty_map()?;
            for (key, &child_id) in map.iter() {
                let scope = c.begin_scope();
                c.navigate(PathSegment::Value(key.clone()))?;
                write_subtree_node(src, child_id, c)?;
                c.end_scope(scope)?;
            }
        }
        NodeValue::PartialMap(map) => {
            c.bind_empty_partial_map()?;
            for (key, &child_id) in map.iter() {
                let scope = c.begin_scope();
                c.navigate_partial_map_entry(key.clone())?;
                write_subtree_node(src, child_id, c)?;
                c.end_scope(scope)?;
            }
        }
    }

    for (ident, &ext_node_id) in node.extensions.iter() {
        let scope = c.begin_scope();
        c.navigate(PathSegment::Extension(ident.clone()))?;
        write_subtree_node(src, ext_node_id, c)?;
        c.end_scope(scope)?;
    }

    Ok(())
}

// ============================================================================
// Primitive implementations
// ============================================================================

impl IntoEure for bool {
    type Error = WriteError;

    fn write(value: bool, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Bool(value))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

macro_rules! impl_into_eure_int {
    ($($ty:ty),*) => {
        $(
            impl IntoEure for $ty {
                type Error = WriteError;

                fn write(value: $ty, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
                    c.bind_primitive(PrimitiveValue::Integer(BigInt::from(value)))
                        .map_err(WriteError::from)?;
                    Ok(())
                }
            }
        )*
    };
}

impl_into_eure_int!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl IntoEure for f32 {
    type Error = WriteError;

    fn write(value: f32, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::F32(value))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl IntoEure for f64 {
    type Error = WriteError;

    fn write(value: f64, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::F64(value))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl IntoEure for crate::decimal::Decimal {
    type Error = WriteError;

    fn write(value: Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Decimal(value))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl IntoEure for crate::bytes::Bytes {
    type Error = WriteError;

    fn write(value: Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Bytes(value.0))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl IntoEure for BigInt {
    type Error = WriteError;

    fn write(value: BigInt, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Integer(value))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl IntoEure for String {
    type Error = WriteError;

    fn write(value: String, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(value)))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl<'a> IntoEure for &'a str {
    type Error = WriteError;

    fn write(value: &'a str, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(value)))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl<'a, T> IntoEure<Cow<'a, T>> for Cow<'a, T>
where
    T: ToOwned + ?Sized,
    T::Owned: IntoEure,
{
    type Error = <T::Owned as IntoEure>::Error;

    fn write(value: Cow<'a, T>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        <T::Owned as IntoEure>::write(value.into_owned(), c)
    }
}

impl IntoEure for Text {
    type Error = WriteError;

    fn write(value: Text, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(value))
            .map_err(WriteError::from)?;
        Ok(())
    }
}

impl IntoEure for PrimitiveValue {
    type Error = WriteError;

    fn write(value: PrimitiveValue, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(value).map_err(WriteError::from)?;
        Ok(())
    }
}

impl IntoEure for Identifier {
    type Error = WriteError;

    fn write(value: Identifier, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_primitive(PrimitiveValue::Text(Text::plaintext(value.into_string())))
            .map_err(WriteError::from)?;
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
    type Error = M::Error;

    fn write(value: Vec<T>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_empty_array().map_err(WriteError::from)?;
        for item in value {
            let scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
                .map_err(WriteError::from)?;
            M::write(item, c)?;
            c.end_scope(scope).map_err(WriteError::from)?;
        }
        Ok(())
    }
}

/// Writes a Rust set as an Eure set (`#{...}`).
///
/// Each element is written on its own and must produce a value from the set element domain
/// (integer, plaintext string, or tuple of those).
fn write_set<M, T>(
    elements: impl IntoIterator<Item = T>,
    c: &mut DocumentConstructor,
) -> Result<(), M::Error>
where
    M: IntoEure<T>,
{
    c.bind_empty_set().map_err(WriteError::from)?;
    for element in elements {
        let mut element_constructor = DocumentConstructor::new();
        M::write(element, &mut element_constructor)?;
        let element_doc = element_constructor.finish();
        let key = set_element_key(&element_doc, element_doc.get_root_id())?;
        c.add_set_element(key).map_err(|error| match error.kind {
            InsertErrorKind::DuplicateSetElement { element } => {
                WriteError::DuplicateSetElement { element }
            }
            _ => WriteError::from(error),
        })?;
    }
    Ok(())
}

fn set_element_key(doc: &EureDocument, node_id: NodeId) -> Result<ObjectKey, WriteError> {
    let node = doc.node(node_id);
    if !node.extensions.is_empty() {
        return Err(WriteError::SetElementWithExtensions);
    }
    match &node.content {
        NodeValue::Primitive(PrimitiveValue::Integer(n)) => Ok(ObjectKey::Number(n.clone())),
        NodeValue::Primitive(PrimitiveValue::Text(text))
            if text.language == Language::Plaintext =>
        {
            Ok(ObjectKey::String(text.content.clone()))
        }
        NodeValue::Tuple(tuple) => tuple
            .iter()
            .map(|&child| set_element_key(doc, child))
            .collect::<Result<Vec<_>, _>>()
            .map(|keys| ObjectKey::Tuple(crate::value::Tuple(keys))),
        other => Err(WriteError::InvalidSetElement {
            actual: other.value_kind(),
        }),
    }
}

impl<M, T, S> IntoEure<IndexSet<T, S>> for IndexSet<M, S>
where
    M: IntoEure<T>,
{
    type Error = M::Error;

    fn write(value: IndexSet<T, S>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        write_set::<M, T>(value, c)
    }
}

#[cfg(feature = "std")]
impl<M, T, S> IntoEure<std::collections::HashSet<T, S>> for std::collections::HashSet<M, S>
where
    M: IntoEure<T>,
{
    type Error = M::Error;

    fn write(
        value: std::collections::HashSet<T, S>,
        c: &mut DocumentConstructor,
    ) -> Result<(), Self::Error> {
        write_set::<M, T>(value, c)
    }
}

impl<M, T> IntoEure<alloc::collections::BTreeSet<T>> for alloc::collections::BTreeSet<M>
where
    M: IntoEure<T>,
{
    type Error = M::Error;

    fn write(
        value: alloc::collections::BTreeSet<T>,
        c: &mut DocumentConstructor,
    ) -> Result<(), Self::Error> {
        write_set::<M, T>(value, c)
    }
}

impl<M, T, const N: usize> IntoEure<[T; N]> for [M; N]
where
    M: IntoEure<T>,
{
    type Error = M::Error;

    fn write(value: [T; N], c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_empty_array().map_err(WriteError::from)?;
        for item in value {
            let scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
                .map_err(WriteError::from)?;
            M::write(item, c)?;
            c.end_scope(scope).map_err(WriteError::from)?;
        }
        Ok(())
    }
}

impl<M, K, V> IntoEure<Map<K, V>> for Map<K, M>
where
    M: IntoEure<V>,
    K: Into<ObjectKey>,
{
    type Error = M::Error;

    fn write(value: Map<K, V>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_empty_map().map_err(WriteError::from)?;
        for (key, v) in value {
            let scope = c.begin_scope();
            c.navigate(PathSegment::Value(key.into()))
                .map_err(WriteError::from)?;
            M::write(v, c)?;
            c.end_scope(scope).map_err(WriteError::from)?;
        }
        Ok(())
    }

    fn write_flatten(value: Map<K, V>, rec: &mut RecordWriter<'_>) -> Result<(), Self::Error> {
        for (key, v) in value {
            let key = match key.into() {
                ObjectKey::String(name) => name,
                _ => {
                    return Err(WriteError::FlattenTargetNotRecordLike {
                        type_name: type_name::<Map<K, V>>(),
                    }
                    .into());
                }
            };
            rec.field_via::<M, _>(&key, v)?;
        }
        Ok(())
    }
}

impl<M, K, V, S> IntoEure<IndexMap<K, V, S>> for IndexMap<K, M, S>
where
    M: IntoEure<V>,
    K: Into<ObjectKey> + Eq + core::hash::Hash,
{
    type Error = M::Error;

    fn write(value: IndexMap<K, V, S>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        c.bind_empty_map().map_err(WriteError::from)?;
        for (key, v) in value {
            let scope = c.begin_scope();
            c.navigate(PathSegment::Value(key.into()))
                .map_err(WriteError::from)?;
            M::write(v, c)?;
            c.end_scope(scope).map_err(WriteError::from)?;
        }
        Ok(())
    }

    fn write_flatten(
        value: IndexMap<K, V, S>,
        rec: &mut RecordWriter<'_>,
    ) -> Result<(), Self::Error> {
        for (key, v) in value {
            let key = match key.into() {
                ObjectKey::String(name) => name,
                _ => {
                    return Err(WriteError::FlattenTargetNotRecordLike {
                        type_name: type_name::<IndexMap<K, V, S>>(),
                    }
                    .into());
                }
            };
            rec.field_via::<M, _>(&key, v)?;
        }
        Ok(())
    }
}

impl<M, T> IntoEure<Option<T>> for Option<M>
where
    M: IntoEure<T>,
{
    type Error = M::Error;

    fn write(value: Option<T>, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        match value {
            Some(v) => M::write(v, c),
            None => {
                c.bind_primitive(PrimitiveValue::Null)
                    .map_err(WriteError::from)?;
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
        impl<$($marker, $ty),+, __E> IntoEure<($($ty,)+)> for ($($marker,)+)
        where
            $($marker: IntoEure<$ty, Error = __E>,)+
            __E: From<WriteError>,
        {
            type Error = __E;

            fn write(value: ($($ty,)+), c: &mut DocumentConstructor) -> Result<(), Self::Error> {
                c.bind_empty_tuple().map_err(WriteError::from)?;
                $(
                    let scope = c.begin_scope();
                    c.navigate(PathSegment::TupleIndex($idx)).map_err(WriteError::from)?;
                    $marker::write(value.$idx, c)?;
                    c.end_scope(scope).map_err(WriteError::from)?;
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
    pub fn record<F, T, E>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut RecordWriter<'_>) -> Result<T, E>,
        E: From<WriteError>,
    {
        self.bind_empty_map().map_err(WriteError::from)?;
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
    pub fn tuple<F, T, E>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut TupleWriter<'_>) -> Result<T, E>,
        E: From<WriteError>,
    {
        self.bind_empty_tuple().map_err(WriteError::from)?;
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
    pub fn set_extension<T: IntoEure>(&mut self, name: &str, value: T) -> Result<(), T::Error> {
        let ident: Identifier = name
            .parse()
            .map_err(|_| WriteError::InvalidIdentifier(name.into()))?;
        let scope = self.begin_scope();
        self.navigate(PathSegment::Extension(ident))
            .map_err(WriteError::from)?;
        T::write(value, self)?;
        self.end_scope(scope).map_err(WriteError::from)?;
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
    ) -> Result<(), T::Error> {
        if let Some(v) = value {
            self.set_extension(name, v)?;
        }
        Ok(())
    }

    /// Set the `$variant` extension for union types.
    ///
    /// If called multiple times on the same node (nested unions), the variant
    /// path is appended using `.` (e.g., `outer.inner.leaf`).
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
        VariantPath::parse(variant)
            .map_err(|err| WriteError::InvalidVariantPath { source: err })?;
        let current_id = self.current_node_id();
        if let Some(variant_node_id) = self
            .document()
            .node(current_id)
            .get_extension(&Identifier::VARIANT)
        {
            let node = self.document().node(variant_node_id);
            let existing = match node.as_primitive().and_then(|value| value.as_str()) {
                Some(existing) => existing,
                None => {
                    let actual = node.content.value_kind();
                    return Err(WriteError::InvalidVariantExtensionType { actual });
                }
            };
            VariantPath::parse(existing)
                .map_err(|err| WriteError::InvalidVariantPath { source: err })?;
            let mut combined = String::with_capacity(existing.len() + 1 + variant.len());
            combined.push_str(existing);
            combined.push('.');
            combined.push_str(variant);
            self.document_mut().node_mut(variant_node_id).content =
                NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(combined)));
            Ok(())
        } else {
            self.set_extension("variant", variant)
        }
    }

    /// Copy a subtree from another `EureDocument` into the current position.
    ///
    /// This recursively copies the node at `node_id` from `src`, including all
    /// children and extensions, into the current node of this constructor.
    pub fn write_subtree(&mut self, src: &EureDocument, node_id: NodeId) -> Result<(), WriteError> {
        write_subtree_node(src, node_id, self)
    }

    /// Write a value implementing `IntoEure` to the current node.
    ///
    /// # Example
    ///
    /// ```ignore
    /// c.write(my_value)?;
    /// ```
    pub fn write<T: IntoEure>(&mut self, value: T) -> Result<(), T::Error> {
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
    pub fn write_via<M, T>(&mut self, value: T) -> Result<(), M::Error>
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
    fn test_btree_set() {
        let mut c = DocumentConstructor::new();
        c.write(alloc::collections::BTreeSet::from([3i32, 1, 2]))
            .unwrap();
        assert_eq!(c.finish(), eure!({ = #{1, 2, 3} }));
    }

    #[test]
    fn test_set_of_strings_and_tuples() {
        let mut c = DocumentConstructor::new();
        c.write(
            [("a".to_string(), 1i32), ("b".to_string(), 2)]
                .into_iter()
                .collect::<crate::collections::IndexSet<_>>(),
        )
        .unwrap();
        let a = ObjectKey::Tuple(crate::value::Tuple(vec!["a".into(), 1.into()]));
        let b = ObjectKey::Tuple(crate::value::Tuple(vec!["b".into(), 2.into()]));
        assert_eq!(c.finish(), eure!({ = #{a, b} }));
    }

    #[test]
    fn test_set_round_trip() {
        let original = alloc::collections::BTreeSet::from(["x".to_string(), "y".to_string()]);
        let mut c = DocumentConstructor::new();
        c.write(original.clone()).unwrap();
        let doc = c.finish();
        let parsed: alloc::collections::BTreeSet<String> = doc.parse(doc.get_root_id()).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_set_rejects_non_key_elements() {
        let mut c = DocumentConstructor::new();
        let result = c.write(alloc::collections::BTreeSet::from([true]));
        assert!(matches!(
            result,
            Err(WriteError::InvalidSetElement {
                actual: ValueKind::Bool
            })
        ));
    }

    #[test]
    fn test_decimal_and_bytes() {
        let price: crate::decimal::Decimal = "1.10".parse().unwrap();
        let data = crate::bytes::Bytes(b"Hi".to_vec());
        let mut c = DocumentConstructor::new();
        c.write(price.clone()).unwrap();
        assert_eq!(c.finish(), eure!({ = 1.10 }));
        let mut c = DocumentConstructor::new();
        c.write(data.clone()).unwrap();
        let doc = c.finish();
        assert_eq!(doc, eure!({ = data }));
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
    fn test_tuple_roundtrip() {
        let original = (42i32, "hello".to_string(), true);

        // Write
        let mut c = DocumentConstructor::new();
        c.write(original.clone()).unwrap();
        let doc = c.finish();

        assert_eq!(eure!({=(42i32, "hello", true)}), doc);

        // Parse back
        let root_id = doc.get_root_id();
        let parsed: (i32, String, bool) = doc.parse(root_id).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_record() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "Alice")?;
            rec.field("age", 30i32)?;
            Ok::<(), WriteError>(())
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
            Ok::<(), WriteError>(())
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
            Ok::<(), WriteError>(())
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
    fn test_set_variant_invalid_extension_type() {
        let mut c = DocumentConstructor::new();
        c.set_extension("variant", 1i32).unwrap();
        let err = c.set_variant("foo").unwrap_err();
        assert!(matches!(
            err,
            WriteError::InvalidVariantExtensionType {
                actual: ValueKind::Integer
            }
        ));
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
                    Ok::<(), WriteError>(())
                })
            })?;
            Ok::<(), WriteError>(())
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
            type Error = WriteError;

            fn write(value: DurationLike, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
                c.record(|rec| {
                    rec.field("secs", value.secs)?;
                    rec.field("nanos", value.nanos)?;
                    Ok::<(), WriteError>(())
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

    #[test]
    fn test_array_write() {
        let mut c = DocumentConstructor::new();
        c.write([1i32, 2, 3]).unwrap();
        let doc = c.finish();
        let arr = doc.root().as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_array_empty_write() {
        let mut c = DocumentConstructor::new();
        let empty: [i32; 0] = [];
        c.write(empty).unwrap();
        let doc = c.finish();
        let arr = doc.root().as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn test_array_roundtrip() {
        let original: [i32; 3] = [10, 20, 30];

        // Write
        let mut c = DocumentConstructor::new();
        c.write(original).unwrap();
        let doc = c.finish();

        // Parse back
        let root_id = doc.get_root_id();
        let parsed: [i32; 3] = doc.parse(root_id).unwrap();

        assert_eq!(parsed, original);
    }

    // =========================================================================
    // Regex tests
    // =========================================================================

    #[test]
    fn test_regex_roundtrip() {
        let original = regex::Regex::new(r"^[a-z]+\d{2,4}$").unwrap();

        // Write
        let mut c = DocumentConstructor::new();
        c.write(original.clone()).unwrap();
        let doc = c.finish();

        // Parse back
        let root_id = doc.get_root_id();
        let parsed: regex::Regex = doc.parse(root_id).unwrap();

        assert_eq!(parsed.as_str(), original.as_str());
    }

    // =========================================================================
    // Cow tests
    // =========================================================================

    #[test]
    fn test_cow_borrowed_str() {
        let mut c = DocumentConstructor::new();
        let value: Cow<'_, str> = Cow::Borrowed("hello");
        c.write(value).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }

    #[test]
    fn test_cow_owned_str() {
        let mut c = DocumentConstructor::new();
        let value: Cow<'static, str> = Cow::Owned("hello".to_string());
        c.write(value).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }

    // =========================================================================
    // IntoEure for EureDocument tests
    // =========================================================================

    #[test]
    fn test_document_write_primitive() {
        let src = eure!({ = "hello" });
        let mut c = DocumentConstructor::new();
        c.write(src.clone()).unwrap();
        let result = c.finish();
        assert_eq!(result, src);
    }

    #[test]
    fn test_document_write_map() {
        let src = eure!({
            name = "Alice"
            age = 30
        });
        let mut c = DocumentConstructor::new();
        c.write(src.clone()).unwrap();
        let result = c.finish();
        assert_eq!(result, src);
    }

    #[test]
    fn test_document_write_array() {
        let src = eure!({ = [1, 2, 3] });
        let mut c = DocumentConstructor::new();
        c.write(src.clone()).unwrap();
        let result = c.finish();
        assert_eq!(result, src);
    }

    #[test]
    fn test_document_write_nested() {
        let src = eure!({
            name = "Alice"
            address {
                city = "Tokyo"
                zip = "100-0001"
            }
            tags = ["a", "b"]
        });
        let mut c = DocumentConstructor::new();
        c.write(src.clone()).unwrap();
        let result = c.finish();
        assert_eq!(result, src);
    }

    #[test]
    fn test_document_write_as_field() {
        let inner = eure!({ city = "Tokyo" });
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "Alice")?;
            rec.field("address", inner)?;
            Ok::<(), WriteError>(())
        })
        .unwrap();
        let result = c.finish();
        let expected = eure!({
            name = "Alice"
            address { city = "Tokyo" }
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_document_write_subtree() {
        let src = eure!({
            name = "Alice"
            active = true
        });
        let mut c = DocumentConstructor::new();
        c.write_subtree(&src, src.get_root_id()).unwrap();
        let result = c.finish();
        assert_eq!(result, src);
    }
}
