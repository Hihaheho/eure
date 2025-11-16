use crate::prelude_internal::*;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum PopError {
    #[error("Cannot pop from root (stack is empty)")]
    CannotPopRoot,
    #[error("Node ID mismatch: expected {expected:?}, but got {actual:?}")]
    NodeIdMismatch { expected: NodeId, actual: NodeId },
}

pub struct DocumentConstructor<'d> {
    document: &'d mut EureDocument,
    /// The path from the root to the current node.
    /// It will contain unused parts after pop operation and those spaces will be used for future push operations.
    path: Vec<PathSegment>,
    /// The second element of the tuple indicates the current path range from the root.
    /// 0 means the root node.
    stack: Vec<(NodeId, usize)>,
}

impl<'d> DocumentConstructor<'d> {
    pub fn new(document: &'d mut EureDocument) -> Self {
        let root = document.get_root_id();
        Self {
            document,
            path: vec![],
            stack: vec![(root, 0)],
        }
    }

    pub fn current_node_id(&self) -> Option<NodeId> {
        self.stack.last().map(|(node_id, _)| *node_id)
    }

    pub fn current_node(&self) -> Option<&Node> {
        self.current_node_id()
            .map(|node_id| self.document.get_node(node_id))
    }

    pub fn current_path(&self) -> &[PathSegment] {
        self.stack
            .last()
            .map(|(_, path_range)| &self.path[..*path_range])
            .unwrap_or(&[])
    }
}

impl<'d> DocumentConstructor<'d> {
    pub fn push_segments(&mut self, segments: &[PathSegment]) -> Result<NodeId, InsertError> {
        let target = self
            .current_node_id()
            .unwrap_or_else(|| self.document.get_root_id());
        let base_path = EurePath::from_iter(self.current_path().iter().cloned());
        let node_id = self
            .document
            .prepare_node_from(target, base_path, segments)?;
        self.path.extend(segments.iter().cloned());
        self.stack.push((node_id, self.path.len()));
        Ok(node_id)
    }

    /// Pop the current segment. the node_id is used to assert the item is intended to be popped.
    pub fn pop(&mut self, node_id: NodeId) -> Result<(), PopError> {
        // Check if we can pop (must have more than just root)
        if self.stack.len() <= 1 {
            return Err(PopError::CannotPopRoot);
        }

        // Check node_id before popping to avoid mutating state on error
        let (current_node_id, _) = self.stack.last().unwrap(); // Safe: checked len > 1
        if *current_node_id != node_id {
            return Err(PopError::NodeIdMismatch {
                expected: *current_node_id,
                actual: node_id,
            });
        }

        self.stack.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifier::IdentifierParser;

    fn create_identifier(s: &str) -> Identifier {
        let parser = IdentifierParser::init();
        parser.parse(s).unwrap()
    }

    #[test]
    fn test_new_initializes_at_root() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let constructor = DocumentConstructor::new(&mut doc);

        assert_eq!(constructor.current_node_id(), Some(root_id));
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_current_node_returns_root_initially() {
        let mut doc = EureDocument::new();
        let constructor = DocumentConstructor::new(&mut doc);

        let node = constructor
            .current_node()
            .expect("Should have current node");
        assert!(node.as_map().is_some());
    }

    #[test]
    fn test_push_segments_single_ident() {
        let mut doc = EureDocument::new();
        let mut constructor = DocumentConstructor::new(&mut doc);

        let identifier = create_identifier("field");
        let segments = &[PathSegment::Ident(identifier.clone())];

        let node_id = constructor.push_segments(segments).expect("Failed to push");

        assert_eq!(constructor.current_node_id(), Some(node_id));
        assert_eq!(constructor.current_path(), segments);
    }

    #[test]
    fn test_push_segments_multiple_times() {
        let mut doc = EureDocument::new();
        let mut constructor = DocumentConstructor::new(&mut doc);

        let id1 = create_identifier("field1");
        let id2 = create_identifier("field2");

        constructor
            .push_segments(&[PathSegment::Ident(id1.clone())])
            .expect("Failed to push first");

        let node_id2 = constructor
            .push_segments(&[PathSegment::Extension(id2.clone())])
            .expect("Failed to push second");

        assert_eq!(constructor.current_node_id(), Some(node_id2));
        assert_eq!(
            constructor.current_path(),
            &[PathSegment::Ident(id1), PathSegment::Extension(id2)]
        );
    }

    #[test]
    fn test_push_segments_error_propagates() {
        let mut doc = EureDocument::new();

        // Push an ident which creates a map child with uninitialized node
        let identifier = create_identifier("field");
        let node_id = {
            let mut constructor = DocumentConstructor::new(&mut doc);
            constructor
                .push_segments(&[PathSegment::Ident(identifier)])
                .expect("Failed to push")
        };

        // Set the node to Primitive to force error
        doc.get_node_mut(node_id).content = NodeValue::Primitive(PrimitiveValue::Null);

        // Try to add tuple index to primitive node (should fail)
        let mut constructor = DocumentConstructor::new(&mut doc);
        let result = constructor.push_segments(&[PathSegment::TupleIndex(0)]);

        assert_eq!(
            result.map_err(|e| e.kind),
            Err(InsertErrorKind::ExpectedTuple)
        );
    }

    #[test]
    fn test_pop_success() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let mut constructor = DocumentConstructor::new(&mut doc);

        let identifier = create_identifier("field");
        let node_id = constructor
            .push_segments(&[PathSegment::Ident(identifier.clone())])
            .expect("Failed to push");

        // Pop with correct node_id
        let result = constructor.pop(node_id);
        assert_eq!(result, Ok(()));

        // After pop, should be back at root
        assert_eq!(constructor.current_node_id(), Some(root_id));
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_pop_wrong_node_id_fails() {
        let mut doc = EureDocument::new();
        let mut constructor = DocumentConstructor::new(&mut doc);

        let identifier = create_identifier("field");
        let node_id = constructor
            .push_segments(&[PathSegment::Ident(identifier)])
            .expect("Failed to push");

        // Try to pop with wrong node_id
        let wrong_id = NodeId(999);
        let result = constructor.pop(wrong_id);

        assert_eq!(
            result,
            Err(PopError::NodeIdMismatch {
                expected: node_id,
                actual: wrong_id
            })
        );

        // State should remain unchanged
        assert_eq!(constructor.current_node_id(), Some(node_id));
    }

    #[test]
    fn test_pop_root_fails() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let mut constructor = DocumentConstructor::new(&mut doc);

        // Try to pop root (should fail)
        let result = constructor.pop(root_id);

        assert_eq!(result, Err(PopError::CannotPopRoot));

        // State should remain unchanged
        assert_eq!(constructor.current_node_id(), Some(root_id));
    }

    #[test]
    fn test_push_pop_multiple_levels() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let mut constructor = DocumentConstructor::new(&mut doc);

        let id1 = create_identifier("level1");
        let id2 = create_identifier("level2");
        let id3 = create_identifier("level3");

        // Push three levels
        let node_id1 = constructor
            .push_segments(&[PathSegment::Ident(id1.clone())])
            .expect("Failed to push level1");

        let node_id2 = constructor
            .push_segments(&[PathSegment::Extension(id2.clone())])
            .expect("Failed to push level2");

        let node_id3 = constructor
            .push_segments(&[PathSegment::Extension(id3.clone())])
            .expect("Failed to push level3");

        // Verify at deepest level
        assert_eq!(constructor.current_node_id(), Some(node_id3));
        assert_eq!(
            constructor.current_path(),
            &[
                PathSegment::Ident(id1.clone()),
                PathSegment::Extension(id2.clone()),
                PathSegment::Extension(id3)
            ]
        );

        // Pop level 3
        constructor.pop(node_id3).expect("Failed to pop level3");
        assert_eq!(constructor.current_node_id(), Some(node_id2));
        assert_eq!(
            constructor.current_path(),
            &[PathSegment::Ident(id1.clone()), PathSegment::Extension(id2)]
        );

        // Pop level 2
        constructor.pop(node_id2).expect("Failed to pop level2");
        assert_eq!(constructor.current_node_id(), Some(node_id1));
        assert_eq!(constructor.current_path(), &[PathSegment::Ident(id1)]);

        // Pop level 1
        constructor.pop(node_id1).expect("Failed to pop level1");
        assert_eq!(constructor.current_node_id(), Some(root_id));
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_push_multiple_segments_at_once() {
        let mut doc = EureDocument::new();
        let mut constructor = DocumentConstructor::new(&mut doc);

        let id1 = create_identifier("ext1");
        let id2 = create_identifier("ext2");

        // Push multiple segments at once
        let segments = &[
            PathSegment::Extension(id1.clone()),
            PathSegment::Extension(id2.clone()),
        ];

        let node_id = constructor.push_segments(segments).expect("Failed to push");

        assert_eq!(constructor.current_node_id(), Some(node_id));
        assert_eq!(constructor.current_path(), segments.as_slice());
    }
}
