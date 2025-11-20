use crate::prelude_internal::*;

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum PopError {
    #[error("Cannot pop from root (stack is empty)")]
    CannotPopRoot,
    #[error("Node ID mismatch: expected {expected:?}, but got {actual:?}")]
    NodeIdMismatch { expected: NodeId, actual: NodeId },
}

pub struct DocumentConstructor {
    document: EureDocument,
    /// The path from the root to the current node.
    /// It will contain unused parts after pop operation and those spaces will be used for future push operations.
    path: Vec<PathSegment>,
    /// The second element of the tuple indicates the current path range from the root.
    /// 0 means the root node.
    stack: Vec<StackItem>,
}

pub struct StackItem {
    node_id: NodeId,
    path_range: usize,
}

impl Default for DocumentConstructor {
    fn default() -> Self {
        let document = EureDocument::default();
        let root = document.get_root_id();
        Self {
            document,
            path: vec![],
            stack: vec![StackItem {
                node_id: root,
                path_range: 0,
            }],
        }
    }
}

impl DocumentConstructor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_node_id(&self) -> NodeId {
        self.stack
            .last()
            .expect("Stack should never be empty")
            .node_id
    }

    pub fn current_node(&self) -> &Node {
        self.document.node(self.current_node_id())
    }

    pub fn current_node_mut(&mut self) -> &mut Node {
        self.document.node_mut(self.current_node_id())
    }

    pub fn current_path(&self) -> &[PathSegment] {
        self.stack
            .last()
            .map(|item| &self.path[..item.path_range])
            .unwrap_or(&[])
    }

    pub fn document(&self) -> &EureDocument {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut EureDocument {
        &mut self.document
    }

    pub fn finish(self) -> EureDocument {
        self.document
    }
}

impl DocumentConstructor {
    pub fn push_path(&mut self, path: &[PathSegment]) -> Result<NodeId, InsertError> {
        let target = self.current_node_id();
        let base_path = EurePath::from_iter(self.current_path().iter().cloned());
        let node_id = self
            .document
            .prepare_node_from(target, base_path, path)?
            .node_id;
        self.path.extend(path.iter().cloned());
        self.stack.push(StackItem {
            node_id,
            path_range: self.path.len(),
        });
        Ok(node_id)
    }

    /// Push a binding path for assignment operations (e.g., `a.b.c = value`).
    ///
    /// Creates the path if it doesn't exist. Returns an error if the target node
    /// already has a value assigned to it.
    ///
    /// # Example
    ///
    /// For `a.b.c = { x = 1 }`:
    /// - Call `push_binding_path(&[a, b, c])` to set up the binding target
    /// - Then call `push_path(&[x])` to build the value structure
    pub fn push_binding_path(&mut self, path: &[PathSegment]) -> Result<NodeId, InsertError> {
        let node_id = self.push_path(path)?;

        // Check if the target node already has a value
        let node = self.document.node(node_id);
        if !matches!(node.content, NodeValue::Uninitialized) {
            self.stack.pop(); // Cancel the push_path
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        Ok(node_id)
    }

    /// Pop the current segment. the node_id is used to assert the item is intended to be popped.
    pub fn pop(&mut self, node_id: NodeId) -> Result<(), PopError> {
        // Check if we can pop (must have more than just root)
        if self.stack.len() <= 1 {
            return Err(PopError::CannotPopRoot);
        }

        // Check node_id before popping to avoid mutating state on error
        let current_node_id = self.current_node_id();
        if current_node_id != node_id {
            return Err(PopError::NodeIdMismatch {
                expected: current_node_id,
                actual: node_id,
            });
        }

        self.stack.pop();
        Ok(())
    }

    /// Bind a primitive value to the current node. Error if already bound.
    pub fn bind_primitive(&mut self, value: PrimitiveValue) -> Result<(), InsertError> {
        let node = self.current_node_mut();
        if !matches!(node.content, NodeValue::Uninitialized) {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        node.content = NodeValue::Primitive(value);
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
        let constructor = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        assert_eq!(constructor.current_node_id(), root_id);
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_current_node_returns_root_initially() {
        let constructor = DocumentConstructor::new();

        let node = constructor.current_node();
        assert!(matches!(node.content, NodeValue::Uninitialized));
    }

    #[test]
    fn test_push_segments_single_ident() {
        let mut constructor = DocumentConstructor::new();

        let identifier = create_identifier("field");
        let segments = &[PathSegment::Ident(identifier.clone())];

        let node_id = constructor.push_path(segments).expect("Failed to push");

        assert_eq!(constructor.current_node_id(), node_id);
        assert_eq!(constructor.current_path(), segments);
    }

    #[test]
    fn test_push_segments_multiple_times() {
        let mut constructor = DocumentConstructor::new();

        let id1 = create_identifier("field1");
        let id2 = create_identifier("field2");

        constructor
            .push_path(&[PathSegment::Ident(id1.clone())])
            .expect("Failed to push first");

        let node_id2 = constructor
            .push_path(&[PathSegment::Extension(id2.clone())])
            .expect("Failed to push second");

        assert_eq!(constructor.current_node_id(), node_id2);
        assert_eq!(
            constructor.current_path(),
            &[PathSegment::Ident(id1), PathSegment::Extension(id2)]
        );
    }

    #[test]
    fn test_push_segments_error_propagates() {
        // Try to add tuple index to primitive node (should fail)
        let mut constructor = DocumentConstructor::new();
        // First push to the field node
        let identifier = create_identifier("field");
        constructor
            .push_path(&[PathSegment::Ident(identifier)])
            .expect("Failed to push");
        // Set it to Primitive
        let node_id = constructor.current_node_id();
        constructor.document_mut().node_mut(node_id).content =
            NodeValue::Primitive(PrimitiveValue::Null);

        let result = constructor.push_path(&[PathSegment::TupleIndex(0)]);

        assert_eq!(
            result.map_err(|e| e.kind),
            Err(InsertErrorKind::ExpectedTuple)
        );
    }

    #[test]
    fn test_pop_success() {
        let mut constructor = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        let identifier = create_identifier("field");
        let node_id = constructor
            .push_path(&[PathSegment::Ident(identifier.clone())])
            .expect("Failed to push");

        // Pop with correct node_id
        let result = constructor.pop(node_id);
        assert_eq!(result, Ok(()));

        // After pop, should be back at root
        assert_eq!(constructor.current_node_id(), root_id);
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_pop_wrong_node_id_fails() {
        let mut constructor = DocumentConstructor::new();

        let identifier = create_identifier("field");
        let node_id = constructor
            .push_path(&[PathSegment::Ident(identifier)])
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
        assert_eq!(constructor.current_node_id(), node_id);
    }

    #[test]
    fn test_pop_root_fails() {
        let mut constructor = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        // Try to pop root (should fail)
        let result = constructor.pop(root_id);

        assert_eq!(result, Err(PopError::CannotPopRoot));

        // State should remain unchanged
        assert_eq!(constructor.current_node_id(), root_id);
    }

    #[test]
    fn test_push_pop_multiple_levels() {
        let mut constructor = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        let id1 = create_identifier("level1");
        let id2 = create_identifier("level2");
        let id3 = create_identifier("level3");

        // Push three levels
        let node_id1 = constructor
            .push_path(&[PathSegment::Ident(id1.clone())])
            .expect("Failed to push level1");

        let node_id2 = constructor
            .push_path(&[PathSegment::Extension(id2.clone())])
            .expect("Failed to push level2");

        let node_id3 = constructor
            .push_path(&[PathSegment::Extension(id3.clone())])
            .expect("Failed to push level3");

        // Verify at deepest level
        assert_eq!(constructor.current_node_id(), node_id3);
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
        assert_eq!(constructor.current_node_id(), node_id2);
        assert_eq!(
            constructor.current_path(),
            &[PathSegment::Ident(id1.clone()), PathSegment::Extension(id2)]
        );

        // Pop level 2
        constructor.pop(node_id2).expect("Failed to pop level2");
        assert_eq!(constructor.current_node_id(), node_id1);
        assert_eq!(constructor.current_path(), &[PathSegment::Ident(id1)]);

        // Pop level 1
        constructor.pop(node_id1).expect("Failed to pop level1");
        assert_eq!(constructor.current_node_id(), root_id);
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_push_multiple_segments_at_once() {
        let mut constructor = DocumentConstructor::new();

        let id1 = create_identifier("ext1");
        let id2 = create_identifier("ext2");

        // Push multiple segments at once
        let segments = &[
            PathSegment::Extension(id1.clone()),
            PathSegment::Extension(id2.clone()),
        ];

        let node_id = constructor.push_path(segments).expect("Failed to push");

        assert_eq!(constructor.current_node_id(), node_id);
        assert_eq!(constructor.current_path(), segments.as_slice());
    }

    #[test]
    fn test_push_binding_path_success() {
        let mut constructor = DocumentConstructor::new();

        let id1 = create_identifier("field1");
        let id2 = create_identifier("field2");
        let path = &[
            PathSegment::Ident(id1.clone()),
            PathSegment::Extension(id2.clone()),
        ];

        // Push a new binding path should succeed
        let node_id = constructor
            .push_binding_path(path)
            .expect("Failed to push binding path");

        assert_eq!(constructor.current_node_id(), node_id);
        assert_eq!(constructor.current_path(), path.as_slice());

        // The node should be uninitialized
        let node = constructor.document().node(node_id);
        assert!(matches!(node.content, NodeValue::Uninitialized));
    }

    #[test]
    fn test_push_binding_path_already_bound() {
        let mut constructor = DocumentConstructor::new();
        let id1 = create_identifier("parent");
        let id2 = create_identifier("child");

        // Create parent.$child and assign a value to it
        let parent_id = constructor
            .document_mut()
            .prepare_node(&[PathSegment::Ident(id1.clone())])
            .expect("Failed to prepare parent")
            .node_id;
        let child_id = constructor
            .document_mut()
            .prepare_node(&[
                PathSegment::Ident(id1.clone()),
                PathSegment::Extension(id2.clone()),
            ])
            .expect("Failed to prepare child")
            .node_id;
        constructor.document_mut().node_mut(child_id).content =
            NodeValue::Primitive(PrimitiveValue::Bool(true));

        // Try to bind to the same location from parent (should fail)
        constructor
            .push_path(&[PathSegment::Ident(id1.clone())])
            .unwrap();

        let result = constructor.push_binding_path(&[PathSegment::Extension(id2.clone())]);

        assert_eq!(
            result.unwrap_err().kind,
            InsertErrorKind::BindingTargetHasValue
        );

        // After the error, constructor should still be at parent (stack was popped)
        assert_eq!(constructor.current_node_id(), parent_id);
        assert_eq!(constructor.current_path(), &[PathSegment::Ident(id1)]);
    }

    #[test]
    fn test_bind_primitive_success() {
        let mut constructor = DocumentConstructor::new();
        let identifier = create_identifier("field");

        // Push to a field node
        let node_id = constructor
            .push_path(&[PathSegment::Ident(identifier)])
            .expect("Failed to push");

        // Bind a primitive value to the node
        let result = constructor.bind_primitive(PrimitiveValue::Bool(true));
        assert_eq!(result, Ok(()));

        // Verify the node content is set to Primitive
        let node = constructor.document().node(node_id);
        assert!(matches!(
            node.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        ));
    }

    #[test]
    fn test_bind_primitive_already_bound() {
        let mut constructor = DocumentConstructor::new();
        let identifier = create_identifier("field");

        // Push to a field node
        let node_id = constructor
            .push_path(&[PathSegment::Ident(identifier.clone())])
            .expect("Failed to push");

        // Set the node to already have a value
        constructor.document_mut().node_mut(node_id).content =
            NodeValue::Primitive(PrimitiveValue::Null);

        // Try to bind a primitive value (should fail)
        let result = constructor.bind_primitive(PrimitiveValue::Bool(false));

        assert_eq!(
            result.unwrap_err().kind,
            InsertErrorKind::BindingTargetHasValue
        );

        // Verify the node content remains unchanged
        let node = constructor.document().node(node_id);
        assert!(matches!(
            node.content,
            NodeValue::Primitive(PrimitiveValue::Null)
        ));
    }
}
