//! TupleWriter for writing tuple types to Eure documents.

extern crate alloc;

use crate::document::constructor::DocumentConstructor;
use crate::path::PathSegment;

use super::{IntoEure, WriteError};

/// Helper for writing tuple types to Eure documents.
///
/// Used within the closure passed to [`DocumentConstructor::tuple`].
/// Automatically tracks position, no manual index management needed.
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
pub struct TupleWriter<'a> {
    constructor: &'a mut DocumentConstructor,
    position: u8,
}

impl<'a> TupleWriter<'a> {
    /// Create a new TupleWriter.
    pub(crate) fn new(constructor: &'a mut DocumentConstructor) -> Self {
        Self {
            constructor,
            position: 0,
        }
    }

    /// Write the next element, advancing position automatically.
    ///
    /// # Example
    ///
    /// ```ignore
    /// t.next("value")?;
    /// t.next(123)?;
    /// ```
    pub fn next<T: IntoEure>(&mut self, value: T) -> Result<(), WriteError> {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::TupleIndex(self.position))?;
        value.write_to(self.constructor)?;
        self.constructor.end_scope(scope)?;
        self.position += 1;
        Ok(())
    }

    /// Write the next element using a custom writer closure.
    ///
    /// Useful for nested structures that need custom handling.
    ///
    /// # Example
    ///
    /// ```ignore
    /// t.next_with(|c| {
    ///     c.record(|rec| {
    ///         rec.field("inner", "value")?;
    ///         Ok(())
    ///     })
    /// })?;
    /// ```
    pub fn next_with<F, R>(&mut self, f: F) -> Result<R, WriteError>
    where
        F: FnOnce(&mut DocumentConstructor) -> Result<R, WriteError>,
    {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::TupleIndex(self.position))?;
        let result = f(self.constructor)?;
        self.constructor.end_scope(scope)?;
        self.position += 1;
        Ok(result)
    }

    /// Get the current position (number of elements written).
    pub fn position(&self) -> u8 {
        self.position
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
    use alloc::string::ToString;

    use super::*;
    use crate::document::node::NodeValue;
    use crate::text::Text;
    use crate::value::{ObjectKey, PrimitiveValue};

    #[test]
    fn test_next_sequential() {
        let mut c = DocumentConstructor::new();
        c.tuple(|t| {
            t.next(1i32)?;
            t.next("two")?;
            t.next(true)?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let tuple = doc.root().as_tuple().unwrap();
        assert_eq!(tuple.len(), 3);
    }

    #[test]
    fn test_next_with_nested() {
        let mut c = DocumentConstructor::new();
        c.tuple(|t| {
            t.next("first")?;
            t.next_with(|c| {
                c.record(|rec| {
                    rec.field("inner", "value")?;
                    Ok(())
                })
            })?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let tuple = doc.root().as_tuple().unwrap();
        assert_eq!(tuple.len(), 2);

        // Check nested record
        let nested_id = tuple.get(1).unwrap();
        let nested = doc.node(nested_id).as_map().unwrap();
        assert!(
            nested
                .get(&ObjectKey::String("inner".to_string()))
                .is_some()
        );
    }

    #[test]
    fn test_position_tracking() {
        let mut c = DocumentConstructor::new();
        c.tuple(|t| {
            assert_eq!(t.position(), 0);
            t.next(1i32)?;
            assert_eq!(t.position(), 1);
            t.next(2i32)?;
            assert_eq!(t.position(), 2);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_empty_tuple() {
        let mut c = DocumentConstructor::new();
        c.tuple(|_t| Ok(())).unwrap();
        let doc = c.finish();
        let tuple = doc.root().as_tuple().unwrap();
        assert!(tuple.is_empty());
    }

    #[test]
    fn test_values_written_correctly() {
        let mut c = DocumentConstructor::new();
        c.tuple(|t| {
            t.next(42i32)?;
            t.next("hello")?;
            Ok(())
        })
        .unwrap();
        let doc = c.finish();
        let tuple = doc.root().as_tuple().unwrap();

        // Check first element
        let first_id = tuple.get(0).unwrap();
        assert_eq!(
            doc.node(first_id).content,
            NodeValue::Primitive(PrimitiveValue::Integer(42.into()))
        );

        // Check second element
        let second_id = tuple.get(1).unwrap();
        assert_eq!(
            doc.node(second_id).content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }
}
