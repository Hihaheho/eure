use std::collections::HashSet;

use crate::prelude_internal::*;

use super::origins::NodeOrigins;
use super::segment::Segment;

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum PopError {
    #[error("Cannot pop from root (stack is empty)")]
    CannotPopRoot,
    #[error("Stack depth mismatch: expected {expected}, but got {actual}")]
    DepthMismatch { expected: usize, actual: usize },
}

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum FinishError {
    #[error("Unconsumed deferred path remains: the last segment was never bound")]
    UnconsumedDeferredPath,
}

/// Token returned by push operations to validate symmetric pop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushToken {
    /// The stack depth before the push operation.
    depth_before: usize,
}

impl PushToken {
    fn new(depth_before: usize) -> Self {
        Self { depth_before }
    }
}

/// Deferred segment waiting to be consumed by a bind operation or the next push.
#[derive(Debug, Clone)]
struct DeferredSegment<O> {
    path: PathSegment,
    origin: Option<O>,
}

impl<O> DeferredSegment<O> {
    fn new(path: PathSegment, origin: Option<O>) -> Self {
        Self { path, origin }
    }

    fn from_segment(seg: &Segment<O>) -> Self
    where
        O: Clone,
    {
        Self::new(seg.path.clone(), seg.origin.clone())
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
    /// Nodes created only as intermediates for extensions (can be rebound).
    extension_only_nodes: HashSet<NodeId>,
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
            extension_only_nodes: HashSet::new(),
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

    /// Get the current stack depth.
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
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
        PathSegment::Ident(_) | PathSegment::Value(_) => NodeValue::Map(Default::default()),
        PathSegment::Extension(_) => {
            // Extensions are added to parent's extensions map, not as map entries.
            // The parent's content should remain Null until explicitly bound.
            NodeValue::Primitive(PrimitiveValue::Null)
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
            let node_id = self.create_and_push_child(deferred.path, container, deferred.origin)?;

            // Mark as extension-only if created for extensions
            if matches!(next_segment, PathSegment::Extension(_)) {
                self.extension_only_nodes.insert(node_id);
            }
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
        let origins: Vec<O> = origin.into_iter().collect();
        self.create_and_push_child_with_origins(segment, content, origins)
    }

    /// Create a child node and push it onto the stack, with multiple origins.
    fn create_and_push_child_with_origins(
        &mut self,
        segment: PathSegment,
        content: NodeValue,
        origins: Vec<O>,
    ) -> Result<NodeId, InsertError> {
        let parent_id = self.current_node_id();
        let base_path = EurePath::from_iter(self.current_path().iter().cloned());

        // If parent is Null (default value), convert it to the appropriate container type
        {
            let parent = self.document.node(parent_id);
            if matches!(parent.content, NodeValue::Primitive(PrimitiveValue::Null)) {
                let container = infer_container_from(&segment);
                self.document.node_mut(parent_id).content = container;
            }
        }

        // Create the node with the given content
        let node_id = self.document.create_node(content);

        // Add the child to the parent based on segment type
        self.add_child_to_parent(parent_id, &segment, node_id, &base_path)?;

        // Record all origins
        for origin in origins {
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
                map.add(
                    ObjectKey::String(identifier.clone().into_string()),
                    child_id,
                )
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
    /// - The last segment is always deferred; bind_value will consume it.
    /// - Returns a PushToken for validating symmetric pop.
    pub fn push_binding_path(&mut self, segments: &[Segment<O>]) -> Result<PushToken, InsertError> {
        let depth_before = self.stack.len();

        if segments.is_empty() {
            return Ok(PushToken::new(depth_before));
        }

        // Handle all but the last segment like push_path
        let (init, last) = segments.split_at(segments.len() - 1);
        self.push_path(init)?;

        let last = &last[0];

        // Consume any pending deferred segment
        self.consume_deferred_with_next(&last.path)?;

        // Check if the last segment already exists
        if let Some(existing_id) = self.try_get_child(&last.path) {
            // Only allow rebinding if node is extension-only
            if !self.extension_only_nodes.contains(&existing_id) {
                return Err(InsertError {
                    kind: InsertErrorKind::BindingTargetHasValue,
                    path: EurePath::from_iter(self.current_path().iter().cloned()),
                });
            }
            // Extension-only node - move to it for rebinding
            self.move_to_existing(existing_id, last.path.clone(), last.origin.clone());
        } else {
            // Defer the last segment for creation
            self.deferred = Some(DeferredSegment::from_segment(last));
        }
        Ok(PushToken::new(depth_before))
    }

    /// Pop the current position from the stack.
    pub fn pop(&mut self) -> Result<(), PopError> {
        if self.stack.len() <= 1 {
            return Err(PopError::CannotPopRoot);
        }
        self.stack.pop();
        Ok(())
    }

    /// Pop all entries until we reach the specified depth.
    pub fn pop_to_depth(&mut self, target_depth: usize) -> Result<(), PopError> {
        while self.stack.len() > target_depth {
            self.pop()?;
        }
        Ok(())
    }

    /// Pop using a PushToken to validate symmetric push/pop.
    pub fn pop_to_token(&mut self, token: PushToken) -> Result<(), PopError> {
        let current_depth = self.stack.len();
        if current_depth < token.depth_before {
            return Err(PopError::DepthMismatch {
                expected: token.depth_before,
                actual: current_depth,
            });
        }
        self.pop_to_depth(token.depth_before)
    }

    /// Consume any pending deferred segment, creating a Map node as default.
    ///
    /// This is useful for sections that need to create their target node immediately
    /// (e.g., when the section body has multiple bindings that should all be inside
    /// the same node).
    pub fn consume_deferred_as_map(&mut self) -> Result<(), InsertError> {
        if self.deferred.is_some() {
            self.consume_deferred_with_value(NodeValue::Map(Default::default()))?;
        }
        Ok(())
    }

    /// Bind a primitive value to the current position.
    ///
    /// If there's a deferred segment, it's consumed and a new node is created.
    /// Otherwise, the current node's value is set (if it's still the default Null).
    pub fn bind_primitive(
        &mut self,
        value: PrimitiveValue,
        origin: Option<O>,
    ) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Primitive(value), origin)
    }

    /// Bind an empty map to the current position.
    pub fn bind_empty_map(&mut self, origin: Option<O>) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Map(Default::default()), origin)
    }

    /// Bind an empty array to the current position.
    pub fn bind_empty_array(&mut self, origin: Option<O>) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Array(Default::default()), origin)
    }

    /// Bind an empty tuple to the current position.
    pub fn bind_empty_tuple(&mut self, origin: Option<O>) -> Result<NodeId, InsertError> {
        self.bind_value(NodeValue::Tuple(Default::default()), origin)
    }

    /// Internal: bind a value, consuming deferred if present.
    fn bind_value(&mut self, value: NodeValue, origin: Option<O>) -> Result<NodeId, InsertError> {
        if let Some(deferred) = self.deferred.take() {
            // Create new node with the value
            let origins: Vec<O> = [deferred.origin, origin].into_iter().flatten().collect();
            let node_id = self.create_and_push_child_with_origins(deferred.path, value, origins)?;
            Ok(node_id)
        } else {
            // No deferred - set the current node's value
            // This is for binding to the root or after moving to an existing node
            let node_id = self.current_node_id();
            let root_id = self.document.get_root_id();

            // Check if this is an extension-only node being rebound
            let is_extension_only = self.extension_only_nodes.remove(&node_id);

            // Check if we can bind
            let node = self.document.node(node_id);
            let can_bind = is_extension_only
                || node_id == root_id
                    && matches!(node.content, NodeValue::Primitive(PrimitiveValue::Null));

            if !can_bind {
                return Err(InsertError {
                    kind: InsertErrorKind::BindingTargetHasValue,
                    path: EurePath::from_iter(self.current_path().iter().cloned()),
                });
            }

            self.document.node_mut(node_id).content = value;

            // Record origin if provided
            if let Some(origin) = origin {
                self.origins.record_node_origin(node_id, origin);
            }

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
            .bind_primitive(PrimitiveValue::Bool(true), None)
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
            .bind_primitive(PrimitiveValue::Bool(true), None)
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
    fn test_push_binding_path_already_exists_with_value() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let identifier = create_identifier("field");

        // First binding with a real value
        constructor
            .push_binding_path(&[seg(PathSegment::Ident(identifier.clone()))])
            .expect("Failed to push binding");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true), None)
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        // Second binding to same location should fail (node has non-Null value)
        let result = constructor.push_binding_path(&[seg(PathSegment::Ident(identifier))]);

        assert_eq!(
            result.unwrap_err().kind,
            InsertErrorKind::BindingTargetHasValue
        );
    }

    #[test]
    fn test_push_binding_path_to_extension_only_node() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let field = create_identifier("field");
        let ext = create_identifier("optional");

        // First: field.$optional = true (creates field with Null, adds extension)
        constructor
            .push_binding_path(&[
                seg(PathSegment::Ident(field.clone())),
                seg(PathSegment::Extension(ext)),
            ])
            .expect("Failed to push binding");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true), None)
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");
        constructor.pop().expect("Failed to pop");

        // Second: field = "hello" should succeed (node has Null value)
        constructor
            .push_binding_path(&[seg(PathSegment::Ident(field))])
            .expect("Should allow binding to extension-only node");
        constructor
            .bind_primitive(
                PrimitiveValue::Text(crate::text::Text::plaintext("hello".to_string())),
                None,
            )
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        let (doc, _) = constructor.finish().expect("Failed to finish");

        // Verify the field has the text value
        let root = doc.root();
        let field_id = root
            .as_map()
            .unwrap()
            .get(&ObjectKey::String("field".to_string()))
            .unwrap();
        let field_node = doc.node(field_id);
        assert!(matches!(
            &field_node.content,
            NodeValue::Primitive(PrimitiveValue::Text(t)) if t.content == "hello"
        ));

        // Verify the extension is still there
        let ext_id = field_node
            .get_extension(&create_identifier("optional"))
            .unwrap();
        let ext_node = doc.node(ext_id);
        assert!(matches!(
            ext_node.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        ));
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
            .bind_primitive(PrimitiveValue::Bool(true), None)
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
            .bind_primitive(PrimitiveValue::Bool(true), None)
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
            .bind_primitive(PrimitiveValue::Bool(true), None)
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
        constructor
            .bind_empty_array(None)
            .expect("Failed to bind array");

        constructor
            .push_binding_path(&[seg(PathSegment::ArrayIndex(Some(0)))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true), None)
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        constructor
            .push_binding_path(&[seg(PathSegment::ArrayIndex(Some(1)))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(false), None)
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

        constructor
            .bind_empty_tuple(None)
            .expect("Failed to bind tuple");

        constructor
            .push_binding_path(&[seg(PathSegment::TupleIndex(0))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true), None)
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        constructor
            .push_binding_path(&[seg(PathSegment::TupleIndex(1))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(false), None)
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
            .bind_primitive(PrimitiveValue::Bool(true), None)
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        let (_, origins) = constructor.finish().expect("Failed to finish");

        let node_origins = origins.get_node_origins(node_id).unwrap();
        assert_eq!(node_origins.len(), 1);
        assert_eq!(node_origins[0], TestOrigin(42));
    }

    #[test]
    fn test_origin_from_bind() {
        #[derive(Debug, Clone, PartialEq)]
        struct TestOrigin(u32);

        let mut constructor: DocumentConstructor<TestOrigin> = DocumentConstructor::new();

        let identifier = create_identifier("field");
        constructor
            .push_binding_path(&[Segment::new(PathSegment::Ident(identifier))])
            .expect("Failed to push");
        let node_id = constructor
            .bind_primitive(PrimitiveValue::Bool(true), Some(TestOrigin(99)))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        let (_, origins) = constructor.finish().expect("Failed to finish");

        let node_origins = origins.get_node_origins(node_id).unwrap();
        assert_eq!(node_origins.len(), 1);
        assert_eq!(node_origins[0], TestOrigin(99));
    }

    #[test]
    fn test_origin_from_both_segment_and_bind() {
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
            .bind_primitive(PrimitiveValue::Bool(true), Some(TestOrigin(99)))
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        let (_, origins) = constructor.finish().expect("Failed to finish");

        // Should have both origins
        let node_origins = origins.get_node_origins(node_id).unwrap();
        assert_eq!(node_origins.len(), 2);
        assert_eq!(node_origins[0], TestOrigin(42));
        assert_eq!(node_origins[1], TestOrigin(99));
    }

    #[test]
    fn test_null_rebinding_should_error() {
        // a = null; a = 1 should fail
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let identifier = create_identifier("a");

        // First: a = null
        constructor
            .push_binding_path(&[seg(PathSegment::Ident(identifier.clone()))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Null, None)
            .expect("Failed to bind");
        constructor.pop().expect("Failed to pop");

        // Second: a = 1 should fail (null was explicitly bound, not extension-only)
        let result = constructor.push_binding_path(&[seg(PathSegment::Ident(identifier))]);

        assert_eq!(
            result.unwrap_err().kind,
            InsertErrorKind::BindingTargetHasValue
        );
    }

    #[test]
    fn test_pop_to_token() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let id1 = create_identifier("a");
        let id2 = create_identifier("b");

        // Push a.b with token
        let token = constructor
            .push_binding_path(&[seg(PathSegment::Ident(id1)), seg(PathSegment::Ident(id2))])
            .expect("Failed to push");

        constructor
            .bind_primitive(PrimitiveValue::Bool(true), None)
            .expect("Failed to bind");

        // Pop back to token depth
        constructor
            .pop_to_token(token)
            .expect("Failed to pop to token");

        // Should be back at root
        let root_id = constructor.document().get_root_id();
        assert_eq!(constructor.current_node_id(), root_id);
    }

    #[test]
    fn test_push_token_validates_depth() {
        let mut constructor: DocumentConstructor<()> = DocumentConstructor::new();

        let id1 = create_identifier("a");
        let id2 = create_identifier("b");

        // First push
        let token1 = constructor
            .push_binding_path(&[seg(PathSegment::Ident(id1))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(true), None)
            .expect("Failed to bind");

        // Pop back to root before second push
        constructor
            .pop_to_token(token1)
            .expect("Failed to pop to token1");

        // Second push (from root)
        let token2 = constructor
            .push_binding_path(&[seg(PathSegment::Ident(id2))])
            .expect("Failed to push");
        constructor
            .bind_primitive(PrimitiveValue::Bool(false), None)
            .expect("Failed to bind");

        // Pop to token2 (back to root)
        constructor
            .pop_to_token(token2)
            .expect("Failed to pop to token2");

        // Should be at root
        let root_id = constructor.document().get_root_id();
        assert_eq!(constructor.current_node_id(), root_id);

        // Verify both keys were created
        let (doc, _) = constructor.finish().expect("Failed to finish");
        let root = doc.root();
        let map = root.as_map().expect("Root should be a map");
        assert!(map.get(&ObjectKey::String("a".to_string())).is_some());
        assert!(map.get(&ObjectKey::String("b".to_string())).is_some());
    }
}
