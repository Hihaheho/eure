use crate::prelude_internal::*;

use super::origins::NodeOrigins;
use super::segment::Segment;

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum PopError {
    #[error("Cannot pop from root (stack is empty)")]
    CannotPopRoot,
}

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum FinishError {
    #[error("Unconsumed deferred path remains: the last segment was never bound")]
    UnconsumedDeferredPath,
}

/// Deferred segment waiting to be consumed by a bind operation or the next push.
#[derive(Debug, Clone)]
struct DeferredSegment<O> {
    path: PathSegment,
    origin: Option<O>,
}

impl<O> DeferredSegment<O> {
    fn from_segment(seg: &Segment<O>) -> Self
    where
        O: Clone,
    {
        Self {
            path: seg.path.clone(),
            origin: seg.origin.clone(),
        }
    }
}

pub struct DocumentConstructor<O = ()> {
    document: EureDocument,
    /// The path from the root to the current node.
    path: Vec<PathSegment>,
    /// Stack tracking the current position in the document tree.
    stack: Vec<StackItem>,
    /// Deferred segment waiting to be consumed.
    deferred: Option<DeferredSegment<O>>,
    /// Origin tracking for nodes and keys.
    origins: NodeOrigins<O>,
}

struct StackItem {
    node_id: NodeId,
    path_range: usize,
}

impl<O> Default for DocumentConstructor<O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O> DocumentConstructor<O> {
    pub fn new() -> Self {
        let mut document = EureDocument::default();
        // Initialize root as Null instead of Uninitialized
        let root_id = document.get_root_id();
        document.node_mut(root_id).content = NodeValue::Primitive(PrimitiveValue::Null);

        Self {
            document,
            path: vec![],
            stack: vec![StackItem {
                node_id: root_id,
                path_range: 0,
            }],
            deferred: None,
            origins: NodeOrigins::<O>::new(),
        }
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

    pub fn origins(&self) -> &NodeOrigins<O> {
        &self.origins
    }

    /// Finish construction and return the document with origins.
    ///
    /// Returns an error if there's an unconsumed deferred segment.
    pub fn finish(self) -> Result<(EureDocument, NodeOrigins<O>), FinishError> {
        if self.deferred.is_some() {
            return Err(FinishError::UnconsumedDeferredPath);
        }
        Ok((self.document, self.origins))
    }

    /// Finish construction and return only the document.
    ///
    /// Returns an error if there's an unconsumed deferred segment.
    pub fn finish_document(self) -> Result<EureDocument, FinishError> {
        self.finish().map(|(doc, _)| doc)
    }
}

/// Infer the container type that should hold a child with the given segment type.
fn infer_container_from(segment: &PathSegment) -> NodeValue {
    match segment {
        PathSegment::Ident(_) | PathSegment::Extension(_) | PathSegment::Value(_) => {
            NodeValue::Map(Default::default())
        }
        PathSegment::ArrayIndex(_) => NodeValue::Array(Default::default()),
        PathSegment::TupleIndex(_) => NodeValue::Tuple(Default::default()),
    }
}

impl<O: Clone> DocumentConstructor<O> {
    /// Consume the deferred segment, creating a node with the container type
    /// inferred from the next segment.
    fn consume_deferred_with_next(
        &mut self,
        next_segment: &PathSegment,
    ) -> Result<(), InsertError> {
        if let Some(deferred) = self.deferred.take() {
            let container = infer_container_from(next_segment);
            self.create_and_push_child(deferred.path, container, deferred.origin)?;
        }
        Ok(())
    }

    /// Consume the deferred segment with a specific value.
    fn consume_deferred_with_value(&mut self, value: NodeValue) -> Result<(), InsertError> {
        if let Some(deferred) = self.deferred.take() {
            self.create_and_push_child(deferred.path, value, deferred.origin)?;
        }
        Ok(())
    }

    /// Create a child node and push it onto the stack.
    fn create_and_push_child(
        &mut self,
        segment: PathSegment,
        content: NodeValue,
        origin: Option<O>,
    ) -> Result<NodeId, InsertError> {
        let parent_id = self.current_node_id();
        let base_path = EurePath::from_iter(self.current_path().iter().cloned());

        // Create the node with the given content
        let node_id = self.document.create_node(content);

        // Add the child to the parent based on segment type
        self.add_child_to_parent(parent_id, &segment, node_id, &base_path)?;

        // Record origin if present
        if let Some(origin) = origin {
            self.origins.record_node_origin(node_id, origin);
        }

        // Update path and stack
        let current_range = self.stack.last().map(|item| item.path_range).unwrap_or(0);
        self.path.truncate(current_range);
        self.path.push(segment);
        self.stack.push(StackItem {
            node_id,
            path_range: self.path.len(),
        });

        Ok(node_id)
    }

    /// Add a child node to a parent based on the segment type.
    fn add_child_to_parent(
        &mut self,
        parent_id: NodeId,
        segment: &PathSegment,
        child_id: NodeId,
        base_path: &EurePath,
    ) -> Result<(), InsertError> {
        let parent = self.document.node_mut(parent_id);

        let map_err = |kind: InsertErrorKind| InsertError {
            kind,
            path: base_path.clone(),
        };

        let result = match segment {
            PathSegment::Ident(identifier) => {
                let map = parent.require_map().map_err(map_err)?;
                map.add(ObjectKey::String(identifier.clone().into_string()), child_id)
            }
            PathSegment::Value(object_key) => {
                let map = parent.require_map().map_err(map_err)?;
                map.add(object_key.clone(), child_id)
            }
            PathSegment::Extension(identifier) => {
                if parent.extensions.contains_key(identifier) {
                    return Err(InsertError {
                        kind: InsertErrorKind::AlreadyAssignedExtension {
                            identifier: identifier.clone(),
                        },
                        path: base_path.clone(),
                    });
                }
                parent.extensions.insert(identifier.clone(), child_id);
                Ok(())
            }
            PathSegment::TupleIndex(index) => {
                let tuple = parent.require_tuple().map_err(map_err)?;
                tuple.add_at(*index, child_id)
            }
            PathSegment::ArrayIndex(index) => {
                let array = parent.require_array().map_err(map_err)?;
                if let Some(idx) = index {
                    array.add_at(*idx, child_id)
                } else {
                    array.push(child_id)
                }
            }
        };

        result.map_err(|kind| InsertError {
            kind,
            path: base_path.clone(),
        })
    }

    /// Try to get an existing child node for the given segment.
    fn try_get_child(&self, segment: &PathSegment) -> Option<NodeId> {
        let node = self.current_node();
        match segment {
            PathSegment::Ident(identifier) => node
                .as_map()
                .and_then(|m| m.get(&ObjectKey::String(identifier.clone().into_string()))),
            PathSegment::Value(object_key) => node.as_map().and_then(|m| m.get(object_key)),
            PathSegment::Extension(identifier) => node.get_extension(identifier),
            PathSegment::TupleIndex(index) => node.as_tuple().and_then(|t| t.get(*index as usize)),
            PathSegment::ArrayIndex(Some(index)) => node.as_array().and_then(|a| a.get(*index)),
            PathSegment::ArrayIndex(None) => None, // push always creates new
        }
    }

    /// Move to an existing node (push it onto the stack).
    fn move_to_existing(&mut self, node_id: NodeId, segment: PathSegment, origin: Option<O>) {
        // Record origin if present
        if let Some(origin) = origin {
            self.origins.record_node_origin(node_id, origin);
        }

        // Update path and stack
        let current_range = self.stack.last().map(|item| item.path_range).unwrap_or(0);
        self.path.truncate(current_range);
        self.path.push(segment);
        self.stack.push(StackItem {
            node_id,
            path_range: self.path.len(),
        });
    }

    /// Push a path for navigation (e.g., section headers).
    ///
    /// - If a node exists at the segment, moves to it.
    /// - If no node exists, defers the segment creation until the next operation.
    pub fn push_path(&mut self, segments: &[Segment<O>]) -> Result<(), InsertError> {
        for (i, seg) in segments.iter().enumerate() {
            // First, consume any pending deferred segment
            // by inferring its type from the current segment
            self.consume_deferred_with_next(&seg.path)?;

            let is_last = i == segments.len() - 1;

            if let Some(existing) = self.try_get_child(&seg.path) {
                // Existing node found - move to it
                self.move_to_existing(existing, seg.path.clone(), seg.origin.clone());
            } else if is_last {
                // Last segment with no existing node - defer
                self.deferred = Some(DeferredSegment::from_segment(seg));
            } else {
                // Intermediate segment with no existing node - defer
                // It will be consumed by the next iteration
                self.deferred = Some(DeferredSegment::from_segment(seg));
            }
        }
        Ok(())
    }

    /// Push a binding path for assignment operations (e.g., `key = value`).
    ///
    /// - Intermediate segments are handled like `push_path`.
    /// - The last segment is always deferred and must not already exist.
    pub fn push_binding_path(&mut self, segments: &[Segment<O>]) -> Result<(), InsertError> {
        if segments.is_empty() {
            return Ok(());
        }

        // Handle all but the last segment like push_path
        let (init, last) = segments.split_at(segments.len() - 1);
        self.push_path(init)?;

        let last = &last[0];

        // Consume any pending deferred segment
        self.consume_deferred_with_next(&last.path)?;

        // Check if the last segment already exists
        if self.try_get_child(&last.path).is_some() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }

        // Defer the last segment
        self.deferred = Some(DeferredSegment::from_segment(last));
        Ok(())
    }

    /// Pop the current position from the stack.
    pub fn pop(&mut self) -> Result<(), PopError> {
        if self.stack.len() <= 1 {
            return Err(PopError::CannotPopRoot);
        }
        self.stack.pop();
        Ok(())
    }

    /// Bind a primitive value to the current position.
    ///
    /// If there's a deferred segment, it's consumed and a new node is created.
    /// Otherwise, the current node's value is set (if it's still the default Null).
    pub fn bind_primitive(&mut self, value: PrimitiveValue) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Primitive(value))
    }

    /// Bind an empty map to the current position.
    pub fn bind_empty_map(&mut self) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Map(Default::default()))
    }

    /// Bind an empty array to the current position.
    pub fn bind_empty_array(&mut self) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Array(Default::default()))
    }

    /// Bind an empty tuple to the current position.
    pub fn bind_empty_tuple(&mut self) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Tuple(Default::default()))
    }

    /// Internal: bind a value, consuming deferred if present.
    fn bind_value(&mut self, value: NodeValue) -> Result<NodeId, InsertError> {
        if self.deferred.is_some() {
            // Consume deferred and create the node with the value
            let deferred = self.deferred.take().unwrap();
            let node_id =
                self.create_and_push_child(deferred.path, value, deferred.origin)?;
            Ok(node_id)
        } else {
            // No deferred - set the current node's value
            // This is for binding to the root or after push_path to an existing node
            let node_id = self.current_node_id();
            let root_id = self.document.get_root_id();
            let node = self.document.node_mut(node_id);

            // Check if we can bind (only if current value is the default Null for root)
            // For non-root nodes that were navigated to, they might have content already
            if !matches!(node.content, NodeValue::Primitive(PrimitiveValue::Null))
                && node_id == root_id
            {
                // Root was already bound to something else
                return Err(InsertError {
                    kind: InsertErrorKind::BindingTargetHasValue,
                    path: EurePath::from_iter(self.current_path().iter().cloned()),
                });
            }

            node.content = value;
            Ok(node_id)
        }
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

    fn seg(path: PathSegment) -> Segment<()> {
        Segment::new(path)
    }

    #[test]
    fn test_new_initializes_at_root() {
        let constructor: DocumentConstructor<()> = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        assert_eq!(constructor.current_node_id(), root_id);
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_current_node_returns_root_initially() {
        let constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let node = constructor.current_node();
        // Root is now initialized to Null instead of Uninitialized
        assert!(matches!(
            node.content,
            NodeValue::Primitive(PrimitiveValue::Null)
        ));
    }

    #[test]
    fn test_push_binding_and_bind() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let identifier = create_identifier("field");
        let segments = &[seg(PathSegment::Ident(identifier.clone()))];

        constructor
            .push_binding_path(segments)
            .expect("Failed to push");

        // Bind a value
        let node_id = constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");

        assert_eq!(constructor.current_node_id(), node_id);

        let node = constructor.document().node(node_id);
        assert!(matches!(
            node.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        ));
    }

    #[test]
    fn test_push_path_to_existing() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let identifier = create_identifier("field");

        // First, create the path with a binding
        constructor
            .push_binding_path(&[seg(PathSegment::Ident(identifier.clone()))])
            .expect("Failed to push binding");
        let node_id1 = constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        // Now push_path to the same location - should find existing
        constructor
            .push_path(&[seg(PathSegment::Ident(identifier.clone()))])
            .expect("Failed to push path");

        // Should be at the same node
        assert_eq!(constructor.current_node_id(), node_id1);
    }

    #[test]
    fn test_push_binding_path_already_exists() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let identifier = create_identifier("field");

        // First binding
        constructor
            .push_binding_path(&[seg(PathSegment::Ident(identifier.clone()))])
            .expect("Failed to push binding");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        // Second binding to same location should fail
        let result = constructor.push_binding_path(&[seg(PathSegment::Ident(identifier))]);

        assert_eq!(result.unwrap_err().kind, InsertErrorKind::BindingTargetHasValue);
    }

    #[test]
    fn test_pop_success() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        let identifier = create_identifier("field");
        constructor
            .push_binding_path(&[seg(PathSegment::Ident(identifier))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");

        let result = constructor.pop();
        assert_eq!(result, Ok(()));

        assert_eq!(constructor.current_node_id(), root_id);
    }

    #[test]
    fn test_pop_root_fails() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let result = constructor.pop();
        assert_eq!(result, Err(PopError::CannotPopRoot));
    }

    #[test]
    fn test_finish_success() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");

        let result = constructor.finish();
        assert!(result.is_ok());

        let (doc, _origins) = result.unwrap();
        let root = doc.root();
        assert!(matches!(
            root.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        ));
    }

    #[test]
    fn test_finish_with_unconsumed_deferred() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let identifier = create_identifier("field");
        constructor
            .push_binding_path(&[seg(PathSegment::Ident(identifier))])
            .expect("Failed to push");

        // Don't bind - leave deferred unconsumed
        let result = constructor.finish();
        assert_eq!(result.unwrap_err(), FinishError::UnconsumedDeferredPath);
    }

    #[test]
    fn test_nested_binding() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let id1 = create_identifier("a");
        let id2 = create_identifier("b");

        // a.b = true
        constructor
            .push_binding_path(&[
                seg(PathSegment::Ident(id1.clone())),
                seg(PathSegment::Ident(id2.clone())),
            ])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");
        constructor.pop().expect("Failed to pop");

        let (doc, _) = constructor.finish().expect("Failed to finish");

        // Verify structure: root.a.b = true
        let root = doc.root();
        let a_id = root
            .as_map()
            .unwrap()
            .get(&ObjectKey::String("a".to_string()))
            .unwrap();
        let a_node = doc.node(a_id);
        let b_id = a_node
            .as_map()
            .unwrap()
            .get(&ObjectKey::String("b".to_string()))
            .unwrap();
        let b_node = doc.node(b_id);
        assert!(matches!(
            b_node.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        ));
    }

    #[test]
    fn test_array_elements() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        // Create array with two elements
        constructor.bind_empty_array().expect("Failed to bind array");

        constructor
            .push_binding_path(&[seg(PathSegment::ArrayIndex(Some(0)))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        constructor
            .push_binding_path(&[seg(PathSegment::ArrayIndex(Some(1)))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(false))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        let (doc, _) = constructor.finish().expect("Failed to finish");

        let root = doc.root();
        let array = root.as_array().unwrap();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_tuple_elements() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        constructor.bind_empty_tuple().expect("Failed to bind tuple");

        constructor
            .push_binding_path(&[seg(PathSegment::TupleIndex(0))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        constructor
            .push_binding_path(&[seg(PathSegment::TupleIndex(1))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(false))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        let (doc, _) = constructor.finish().expect("Failed to finish");

        let root = doc.root();
        let tuple = root.as_tuple().unwrap();
        assert_eq!(tuple.len(), 2);
    }

    #[test]
    fn test_origin_tracking() {
        #[derive(Debug, Clone, PartialEq)]
        struct TestOrigin(u32);

        let mut constructor: DocumentConstructor<TestOrigin> = DocumentConstructor::new();

        let identifier = create_identifier("field");
        constructor
            .push_binding_path(&[Segment::with_origin(
                PathSegment::Ident(identifier),
                TestOrigin(42),
            )])
            .expect("Failed to push");
        let node_id = constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        let (_, origins) = constructor.finish().expect("Failed to finish");

        let node_origins = origins.get_node_origins(node_id).unwrap();
        assert_eq!(node_origins.len(), 1);
        assert_eq!(node_origins[0], TestOrigin(42));
    }
}
