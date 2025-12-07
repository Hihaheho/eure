//! RecordWriter for writing record types to Eure documents.

extern crate alloc;

use alloc::string::ToString;

use crate::document::constructor::DocumentConstructor;
use crate::path::PathSegment;
use crate::value::ObjectKey;

use super::{IntoDocument, WriteError};

/// Helper for writing record (map with string keys) to Eure documents.
///
/// Used within the closure passed to [`DocumentConstructor::record`].
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
}

impl<'a> RecordWriter<'a> {
    /// Create a new RecordWriter.
    pub(crate) fn new(constructor: &'a mut DocumentConstructor) -> Self {
        Self { constructor }
    }

    /// Write a required field.
    ///
    /// # Example
    ///
    /// ```ignore
    /// rec.field("name", "Alice")?;
    /// ```
    pub fn field<T: IntoDocument>(&mut self, name: &str, value: T) -> Result<(), WriteError> {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::Value(ObjectKey::String(name.to_string())))?;
        value.write_to(self.constructor)?;
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
    pub fn field_optional<T: IntoDocument>(
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
    ///     meta.write_to(c)
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

    /// Get a mutable reference to the underlying DocumentConstructor.
    ///
    /// Useful for advanced use cases that need direct access.
    pub fn constructor(&mut self) -> &mut DocumentConstructor {
        self.constructor
    }
}

#[cfg(test)]
mod tests {
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
        let node = doc.node(name_id);
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
        let nested = doc.node(nested_id).as_map().unwrap();
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
}
