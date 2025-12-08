//! TupleParser for parsing tuple types from Eure documents.

extern crate alloc;

use alloc::format;

use crate::document::node::NodeTuple;
use crate::prelude_internal::*;

use super::{ParseContext, ParseDocument, ParseError, ParseErrorKind};

/// Helper for parsing tuple types from Eure documents.
///
/// Provides both sequential access via `next()` and random access via `get()`.
/// Use `finish()` to verify all elements were consumed.
///
/// # Example
///
/// ```ignore
/// let mut tuple = ctx.parse_tuple()?;
/// let first: String = tuple.next()?;
/// let second: i32 = tuple.next()?;
/// tuple.finish()?; // Ensures no extra elements
/// ```
#[derive(Debug)]
pub struct TupleParser<'doc> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    tuple: &'doc NodeTuple,
    position: usize,
}

impl<'doc> TupleParser<'doc> {
    /// Create a new TupleParser for the given context.
    pub(crate) fn new(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        Self::from_doc_and_node(ctx.doc(), ctx.node_id())
    }

    /// Create a new TupleParser from document and node ID directly.
    pub(crate) fn from_doc_and_node(
        doc: &'doc EureDocument,
        node_id: NodeId,
    ) -> Result<Self, ParseError> {
        let node = doc.node(node_id);
        match &node.content {
            NodeValue::Tuple(tuple) => Ok(Self {
                doc,
                node_id,
                tuple,
                position: 0,
            }),
            NodeValue::Hole(_) => Err(ParseError {
                node_id,
                kind: ParseErrorKind::UnexpectedHole,
            }),
            value => Err(ParseError {
                node_id,
                kind: value
                    .value_kind()
                    .map(|actual| ParseErrorKind::TypeMismatch {
                        expected: crate::value::ValueKind::Tuple,
                        actual,
                    })
                    .unwrap_or(ParseErrorKind::UnexpectedHole),
            }),
        }
    }

    /// Get the node ID being parsed.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the next element, advancing the position.
    ///
    /// Returns `ParseErrorKind::MissingField` if no more elements.
    #[allow(clippy::should_implement_trait)]
    pub fn next<T: ParseDocument<'doc>>(&mut self) -> Result<T, ParseError> {
        let index = self.position;
        let element_node_id = self.tuple.get(index).ok_or_else(|| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::MissingField(format!("#{}", index)),
        })?;
        self.position += 1;
        let ctx = ParseContext::new(self.doc, element_node_id);
        T::parse(&ctx)
    }

    /// Get the element at a specific index without advancing position.
    ///
    /// Returns `ParseErrorKind::MissingField` if the index is out of bounds.
    pub fn get<T: ParseDocument<'doc>>(&self, index: usize) -> Result<T, ParseError> {
        let element_node_id = self.tuple.get(index).ok_or_else(|| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::MissingField(format!("#{}", index)),
        })?;
        let ctx = ParseContext::new(self.doc, element_node_id);
        T::parse(&ctx)
    }

    /// Get the number of remaining elements.
    pub fn remaining(&self) -> usize {
        self.tuple.len().saturating_sub(self.position)
    }

    /// Verify all elements were consumed.
    ///
    /// Returns `ParseErrorKind::UnexpectedTupleLength` if elements remain.
    pub fn finish(self) -> Result<(), ParseError> {
        if self.position != self.tuple.len() {
            return Err(ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::UnexpectedTupleLength {
                    expected: self.position,
                    actual: self.tuple.len(),
                },
            });
        }
        Ok(())
    }

    /// Verify the tuple has the expected length.
    ///
    /// Returns `ParseErrorKind::UnexpectedTupleLength` if length doesn't match.
    pub fn expect_len(&self, expected: usize) -> Result<(), ParseError> {
        if self.tuple.len() != expected {
            return Err(ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::UnexpectedTupleLength {
                    expected,
                    actual: self.tuple.len(),
                },
            });
        }
        Ok(())
    }

    /// Get the total number of elements in the tuple.
    pub fn len(&self) -> usize {
        self.tuple.len()
    }

    /// Check if the tuple is empty.
    pub fn is_empty(&self) -> bool {
        self.tuple.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::constructor::DocumentConstructor;
    use crate::path::PathSegment;
    use crate::value::PrimitiveValue;

    fn create_tuple_doc(elements: Vec<PrimitiveValue>) -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_tuple().unwrap();
        for (i, elem) in elements.into_iter().enumerate() {
            let scope = c.begin_scope();
            c.navigate(PathSegment::TupleIndex(i as u8)).unwrap();
            c.bind_primitive(elem).unwrap();
            c.end_scope(scope).unwrap();
        }
        c.finish()
    }

    #[test]
    fn test_next_sequential() {
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Integer(1.into()),
            PrimitiveValue::Integer(2.into()),
            PrimitiveValue::Integer(3.into()),
        ]);

        let mut tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        assert_eq!(tuple.next::<i32>().unwrap(), 1);
        assert_eq!(tuple.next::<i32>().unwrap(), 2);
        assert_eq!(tuple.next::<i32>().unwrap(), 3);
        tuple.finish().unwrap();
    }

    #[test]
    fn test_next_past_end() {
        let doc = create_tuple_doc(vec![PrimitiveValue::Integer(1.into())]);

        let mut tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        tuple.next::<i32>().unwrap();
        let result = tuple.next::<i32>();
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::MissingField(_)
        ));
    }

    #[test]
    fn test_get_random_access() {
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Integer(10.into()),
            PrimitiveValue::Integer(20.into()),
            PrimitiveValue::Integer(30.into()),
        ]);

        let tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        assert_eq!(tuple.get::<i32>(2).unwrap(), 30);
        assert_eq!(tuple.get::<i32>(0).unwrap(), 10);
        assert_eq!(tuple.get::<i32>(1).unwrap(), 20);
    }

    #[test]
    fn test_get_out_of_bounds() {
        let doc = create_tuple_doc(vec![PrimitiveValue::Integer(1.into())]);

        let tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        let result = tuple.get::<i32>(5);
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::MissingField(_)
        ));
    }

    #[test]
    fn test_remaining() {
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Integer(1.into()),
            PrimitiveValue::Integer(2.into()),
            PrimitiveValue::Integer(3.into()),
        ]);

        let mut tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        assert_eq!(tuple.remaining(), 3);
        tuple.next::<i32>().unwrap();
        assert_eq!(tuple.remaining(), 2);
        tuple.next::<i32>().unwrap();
        assert_eq!(tuple.remaining(), 1);
        tuple.next::<i32>().unwrap();
        assert_eq!(tuple.remaining(), 0);
    }

    #[test]
    fn test_finish_with_remaining_elements() {
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Integer(1.into()),
            PrimitiveValue::Integer(2.into()),
        ]);

        let mut tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        tuple.next::<i32>().unwrap();
        // Only consumed 1 of 2 elements
        let result = tuple.finish();
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::UnexpectedTupleLength {
                expected: 1,
                actual: 2
            }
        ));
    }

    #[test]
    fn test_expect_len_correct() {
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Integer(1.into()),
            PrimitiveValue::Integer(2.into()),
        ]);

        let tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        tuple.expect_len(2).unwrap();
    }

    #[test]
    fn test_expect_len_incorrect() {
        let doc = create_tuple_doc(vec![PrimitiveValue::Integer(1.into())]);

        let tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        let result = tuple.expect_len(3);
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::UnexpectedTupleLength {
                expected: 3,
                actual: 1
            }
        ));
    }

    #[test]
    fn test_empty_tuple() {
        let doc = create_tuple_doc(vec![]);

        let tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        assert!(tuple.is_empty());
        assert_eq!(tuple.len(), 0);
        assert_eq!(tuple.remaining(), 0);
        tuple.finish().unwrap();
    }

    #[test]
    fn test_len_and_is_empty() {
        let doc = create_tuple_doc(vec![
            PrimitiveValue::Integer(1.into()),
            PrimitiveValue::Integer(2.into()),
        ]);

        let tuple = doc.parse_tuple(doc.get_root_id()).unwrap();
        assert!(!tuple.is_empty());
        assert_eq!(tuple.len(), 2);
    }

    #[test]
    fn test_parse_non_tuple_fails() {
        let mut c = DocumentConstructor::new();
        c.bind_empty_array().unwrap();
        let doc = c.finish();

        let result = doc.parse_tuple(doc.get_root_id());
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::TypeMismatch { .. }
        ));
    }
}
