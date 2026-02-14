//! RecordWriter for writing record types to Eure documents.

extern crate alloc;

use alloc::string::ToString;

use crate::document::constructor::DocumentConstructor;
use crate::path::PathSegment;
use crate::value::ObjectKey;

use super::{IntoEure, IntoEureRecord, WriteError};

/// Helper for writing record (map with string keys) to Eure documents.
///
/// Used within the closure passed to [`DocumentConstructor::record`].
///
/// When `ext_mode` is `true`, field writes are redirected to extension writes.
/// This is used by `flatten_ext` to write fields as extensions.
///
/// # Example
///
/// ```ignore
/// c.record(|rec| {
///     rec.field("name", "Alice")?;
///     rec.field_optional("age", Some(30))?;
///     Ok(())
/// })?;
/// ```
pub struct RecordWriter<'a> {
    constructor: &'a mut DocumentConstructor,
    ext_mode: bool,
}

impl<'a> RecordWriter<'a> {
    /// Create a new RecordWriter with `ext_mode` disabled.
    pub(crate) fn new(constructor: &'a mut DocumentConstructor) -> Self {
        Self {
            constructor,
            ext_mode: false,
        }
    }

    /// Create a new RecordWriter with the specified `ext_mode`.
    ///
    /// This is primarily used by generated derive code that needs to write
    /// flattened extension fields onto an existing node.
    pub fn new_with_ext_mode(constructor: &'a mut DocumentConstructor, ext_mode: bool) -> Self {
        Self {
            constructor,
            ext_mode,
        }
    }

    /// Write a required field.
    ///
    /// In `ext_mode`, this redirects to an extension write.
    ///
    /// # Example
    ///
    /// ```ignore
    /// rec.field("name", "Alice")?;
    /// ```
    pub fn field<T: IntoEure>(&mut self, name: &str, value: T) -> Result<(), WriteError> {
        if self.ext_mode {
            return self.constructor.set_extension(name, value);
        }
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::Value(ObjectKey::String(name.to_string())))?;
        T::write(value, self.constructor)?;
        self.constructor.end_scope(scope)?;
        Ok(())
    }

    /// Write a required field using a marker type.
    ///
    /// This enables writing types from external crates that can't implement
    /// `IntoEure` directly due to Rust's orphan rule.
    ///
    /// In `ext_mode`, this redirects to an extension write via the marker type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // DurationDef implements IntoEure<std::time::Duration>
    /// rec.field_via::<DurationDef, _>("timeout", duration)?;
    /// ```
    pub fn field_via<M, T>(&mut self, name: &str, value: T) -> Result<(), WriteError>
    where
        M: IntoEure<T>,
    {
        if self.ext_mode {
            let ident: crate::identifier::Identifier = name
                .parse()
                .map_err(|_| WriteError::InvalidIdentifier(name.into()))?;
            let scope = self.constructor.begin_scope();
            self.constructor.navigate(PathSegment::Extension(ident))?;
            M::write(value, self.constructor)?;
            self.constructor.end_scope(scope)?;
            return Ok(());
        }
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::Value(ObjectKey::String(name.to_string())))?;
        M::write(value, self.constructor)?;
        self.constructor.end_scope(scope)?;
        Ok(())
    }

    /// Write an optional field.
    /// Does nothing if the value is `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// rec.field_optional("age", self.age)?;
    /// ```
    pub fn field_optional<T: IntoEure>(
        &mut self,
        name: &str,
        value: Option<T>,
    ) -> Result<(), WriteError> {
        if let Some(v) = value {
            self.field(name, v)?;
        }
        Ok(())
    }

    /// Write a field using a custom writer closure.
    ///
    /// Useful for nested structures that need custom handling.
    ///
    /// # Example
    ///
    /// ```ignore
    /// rec.field_with("address", |c| {
    ///     c.record(|rec| {
    ///         rec.field("city", "Tokyo")?;
    ///         Ok(())
    ///     })
    /// })?;
    /// ```
    pub fn field_with<F, T>(&mut self, name: &str, f: F) -> Result<T, WriteError>
    where
        F: FnOnce(&mut DocumentConstructor) -> Result<T, WriteError>,
    {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::Value(ObjectKey::String(name.to_string())))?;
        let result = f(self.constructor)?;
        self.constructor.end_scope(scope)?;
        Ok(result)
    }

    /// Write an optional field using a custom writer closure.
    /// Does nothing if the value is `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// rec.field_with_optional("metadata", self.metadata.as_ref(), |c, meta| {
    ///     c.write(meta)
    /// })?;
    /// ```
    pub fn field_with_optional<T, F, R>(
        &mut self,
        name: &str,
        value: Option<T>,
        f: F,
    ) -> Result<Option<R>, WriteError>
    where
        F: FnOnce(&mut DocumentConstructor, T) -> Result<R, WriteError>,
    {
        if let Some(v) = value {
            let result = self.field_with(name, |c| f(c, v))?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Flatten a value's fields into this record writer.
    ///
    /// The flattened type's fields are written as if they were direct fields
    /// of this record. The current `ext_mode` is inherited.
    ///
    /// # Example
    ///
    /// ```ignore
    /// rec.flatten(value.address)?;
    /// ```
    pub fn flatten<M, T>(&mut self, value: T) -> Result<(), WriteError>
    where
        M: IntoEureRecord<T>,
    {
        M::write_to_record(value, self)
    }

    /// Flatten a value's fields as extensions into this record.
    ///
    /// Creates a temporary `RecordWriter` with `ext_mode: true`, so that
    /// all field writes from the flattened type become extension writes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// rec.flatten_ext(value.ext)?;
    /// ```
    pub fn flatten_ext<M, T>(&mut self, value: T) -> Result<(), WriteError>
    where
        M: IntoEureRecord<T>,
    {
        let mut ext_rec = RecordWriter::new_with_ext_mode(self.constructor, true);
        M::write_to_record(value, &mut ext_rec)
    }

    /// Get a mutable reference to the underlying DocumentConstructor.
    ///
    /// Useful for advanced use cases that need direct access.
    pub fn constructor(&mut self) -> &mut DocumentConstructor {
        self.constructor
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::string::{String, ToString};

    use super::*;
    use crate::document::node::NodeValue;
    use crate::text::Text;
    use crate::value::PrimitiveValue;

    #[test]
    fn test_field() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "Alice")?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        let name_id = map.get(&ObjectKey::String("name".to_string())).unwrap();
        let node = doc.node(*name_id);
        assert_eq!(
            node.content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("Alice")))
        );
    }

    #[test]
    fn test_field_optional_some() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field_optional("age", Some(30i32))?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        assert!(map.get(&ObjectKey::String("age".to_string())).is_some());
    }

    #[test]
    fn test_field_optional_none() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field_optional::<i32>("age", None)?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        assert!(map.get(&ObjectKey::String("age".to_string())).is_none());
    }

    #[test]
    fn test_field_with() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field_with("nested", |c| {
                c.record(|rec| {
                    rec.field("inner", "value")?;
                    Ok(())
                })
            })?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        let nested_id = map.get(&ObjectKey::String("nested".to_string())).unwrap();
        let nested = doc.node(*nested_id).as_map().unwrap();
        assert!(
            nested
                .get(&ObjectKey::String("inner".to_string()))
                .is_some()
        );
    }

    #[test]
    fn test_multiple_fields() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "Bob")?;
            rec.field("age", 25i32)?;
            rec.field("active", true)?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 3);
    }

    // Manually implement IntoEureRecord for testing
    struct TestAddress {
        city: String,
        country: String,
    }

    impl IntoEureRecord for TestAddress {
        fn write_to_record(
            value: TestAddress,
            rec: &mut super::RecordWriter<'_>,
        ) -> Result<(), WriteError> {
            rec.field("city", value.city)?;
            rec.field("country", value.country)?;
            Ok(())
        }
    }

    #[test]
    fn test_flatten() {
        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "Alice")?;
            rec.flatten::<TestAddress, _>(TestAddress {
                city: "Tokyo".to_string(),
                country: "Japan".to_string(),
            })?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let map = doc.root().as_map().unwrap();
        assert_eq!(map.len(), 3);
        assert!(map.get(&ObjectKey::String("name".to_string())).is_some());
        assert!(map.get(&ObjectKey::String("city".to_string())).is_some());
        assert!(map.get(&ObjectKey::String("country".to_string())).is_some());
    }

    struct TestMeta {
        version: i32,
        deprecated: bool,
    }

    impl IntoEureRecord for TestMeta {
        fn write_to_record(
            value: TestMeta,
            rec: &mut super::RecordWriter<'_>,
        ) -> Result<(), WriteError> {
            rec.field("version", value.version)?;
            rec.field("deprecated", value.deprecated)?;
            Ok(())
        }
    }

    #[test]
    fn test_flatten_ext() {
        use crate::identifier::Identifier;

        let mut c = DocumentConstructor::new();
        c.record(|rec| {
            rec.field("name", "test")?;
            rec.flatten_ext::<TestMeta, _>(TestMeta {
                version: 2,
                deprecated: true,
            })?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let root = doc.root();
        // "name" should be a record field
        let map = root.as_map().unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.get(&ObjectKey::String("name".to_string())).is_some());
        // "version" and "deprecated" should be extensions
        assert!(
            root.extensions
                .contains_key(&"version".parse::<Identifier>().unwrap())
        );
        assert!(
            root.extensions
                .contains_key(&"deprecated".parse::<Identifier>().unwrap())
        );
    }
}
