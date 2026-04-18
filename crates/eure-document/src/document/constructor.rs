use alloc::collections::BTreeMap;
use indexmap::IndexSet;

use crate::document::interpreter_sink::InterpreterSink;
use crate::map::PartialNodeMap;
use crate::prelude_internal::*;
use crate::value::PartialObjectKey;

/// Tracks, for the current block scope, which child was most recently pushed into
/// each array encountered in that scope. Used to resolve `[^]` back to that child.
///
/// Entries are keyed by the array's `NodeId` (the node whose value is `NodeValue::Array`).
#[derive(Debug, Default, Clone)]
struct BlockScope {
    last_pushes: BTreeMap<NodeId, NodeId>,
}

/// Represents a scope in the document constructor.
/// Must be passed to `end_scope` to restore the constructor to the state when the scope was created.
/// Scopes must be ended in LIFO order (most recent first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope {
    id: usize,
    stack_depth: usize,
    path_depth: usize,
}

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum ScopeError {
    #[error("Cannot end scope at root")]
    CannotEndAtRoot,
    #[error("Scope must be ended in LIFO order (most recent first)")]
    NotMostRecentScope,
}

pub struct DocumentConstructor {
    document: EureDocument,
    /// The path from the root to the current node.
    path: Vec<PathSegment>,
    /// Stack of NodeIds from root to current position.
    stack: Vec<NodeId>,
    /// Counter for generating unique scope IDs.
    scope_counter: usize,
    /// Stack of outstanding scope IDs for LIFO enforcement.
    outstanding_scopes: Vec<usize>,
    /// Whether hole has been bound to the node
    hole_bound: Vec<bool>,
    /// IDs of nodes that are unbound.
    unbound_nodes: IndexSet<NodeId>,
    /// Stack of block scopes used to resolve `[^]` (current-index) segments. The topmost
    /// scope records the most recent push into each array observed within the current
    /// block/section body. Always non-empty: the first entry represents the document root.
    block_scope_stack: Vec<BlockScope>,
}

impl Default for DocumentConstructor {
    fn default() -> Self {
        let document = EureDocument::default();
        let root = document.get_root_id();
        Self {
            document,
            path: vec![],
            stack: vec![root],
            hole_bound: vec![false],
            scope_counter: 0,
            outstanding_scopes: vec![],
            unbound_nodes: IndexSet::new(),
            block_scope_stack: vec![BlockScope::default()],
        }
    }
}

impl DocumentConstructor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_node_id(&self) -> NodeId {
        *self.stack.last().expect("Stack should never be empty")
    }

    pub fn current_node(&self) -> &Node {
        self.document.node(self.current_node_id())
    }

    pub fn current_node_mut(&mut self) -> &mut Node {
        self.document.node_mut(self.current_node_id())
    }

    pub fn current_path(&self) -> &[PathSegment] {
        &self.path
    }

    pub fn document(&self) -> &EureDocument {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut EureDocument {
        &mut self.document
    }

    pub fn finish(mut self) -> EureDocument {
        for node_id in self.unbound_nodes {
            let node = self.document.node_mut(node_id);
            if node.content.is_hole() {
                node.content = NodeValue::Map(Default::default());
            }
        }
        // If the root node is Hole, empty map
        let root_id = self.document.get_root_id();
        let root_node = self.document.node_mut(root_id);
        if root_node.content.is_hole() && !self.hole_bound[0] {
            root_node.content = NodeValue::Map(Default::default());
        }
        self.document
    }
}

impl DocumentConstructor {
    /// Begin a new scope. Returns a scope handle that must be passed to `end_scope`.
    /// Scopes must be ended in LIFO order (most recent first).
    pub fn begin_scope(&mut self) -> Scope {
        let id = self.scope_counter;
        self.scope_counter += 1;
        self.outstanding_scopes.push(id);
        Scope {
            id,
            stack_depth: self.stack.len(),
            path_depth: self.path.len(),
        }
    }

    /// End a scope, restoring the constructor to the state when the scope was created.
    /// Returns an error if the scope is not the most recent outstanding scope.
    pub fn end_scope(&mut self, scope: Scope) -> Result<(), ScopeError> {
        // LIFO enforcement: scope must be the most recent outstanding scope
        if self.outstanding_scopes.last() != Some(&scope.id) {
            return Err(ScopeError::NotMostRecentScope);
        }
        if scope.stack_depth < 1 {
            return Err(ScopeError::CannotEndAtRoot);
        }
        self.outstanding_scopes.pop();
        for i in scope.stack_depth..self.stack.len() {
            let hole_bound = self.hole_bound[i];
            if !hole_bound && self.document.node(self.stack[i]).content.is_hole() {
                self.unbound_nodes.insert(self.stack[i]);
            }
        }
        self.stack.truncate(scope.stack_depth);
        self.hole_bound.truncate(scope.stack_depth);
        self.path.truncate(scope.path_depth);
        Ok(())
    }

    /// Navigate to a child node by path segment.
    /// Creates the node if it doesn't exist.
    ///
    /// Array-index semantics:
    /// - `ArrayIndexKind::Push` — always pushes a new array element, and records that
    ///   push in the current block scope so a later `[^]` can find it.
    /// - `ArrayIndexKind::Current` — resolves to the element most recently pushed into
    ///   the target array within the current block scope. Errors with
    ///   [`InsertErrorKind::ArrayCurrentOutOfScope`] if no such push exists. This never
    ///   creates a new element.
    /// - `ArrayIndexKind::Specific(n)` — reuses or creates the element at index `n`.
    pub fn navigate(&mut self, segment: PathSegment) -> Result<NodeId, InsertError> {
        let current = self.current_node_id();

        // `[^]` must resolve to a prior push within the current block scope.
        if let PathSegment::ArrayIndex(ArrayIndexKind::Current) = &segment {
            // Ensure the target is actually an array so we surface ExpectedArray first.
            self.document
                .node(current)
                .as_array()
                .ok_or_else(|| InsertError {
                    kind: InsertErrorKind::ExpectedArray,
                    path: EurePath::from_iter(self.path.iter().cloned()),
                })?;
            let child_id = self
                .block_scope_stack
                .last()
                .expect("block scope stack is never empty")
                .last_pushes
                .get(&current)
                .copied()
                .ok_or_else(|| InsertError {
                    kind: InsertErrorKind::ArrayCurrentOutOfScope {
                        array_node_id: current,
                    },
                    path: EurePath::from_iter(self.path.iter().cloned()),
                })?;
            self.stack.push(child_id);
            self.hole_bound.push(false);
            self.path.push(segment);
            return Ok(child_id);
        }

        let node_mut = self
            .document
            .resolve_child_by_segment(segment.clone(), current)
            .map_err(|e| InsertError {
                kind: e,
                path: EurePath::from_iter(self.path.iter().cloned()),
            })?;
        let node_id = node_mut.node_id;

        // Record pushes so later `[^]` segments in the same block scope can find them.
        if let PathSegment::ArrayIndex(ArrayIndexKind::Push) = &segment {
            self.block_scope_stack
                .last_mut()
                .expect("block scope stack is never empty")
                .last_pushes
                .insert(current, node_id);
        }

        self.stack.push(node_id);
        self.hole_bound.push(false);
        self.path.push(segment);
        Ok(node_id)
    }

    /// Navigate into a PartialMap entry.
    ///
    /// Find-or-create semantics:
    /// - labeled holes and resolved keys reuse an existing entry
    /// - anonymous holes (`Hole(None)`) always create a fresh entry
    pub fn navigate_partial_map_entry(
        &mut self,
        key: PartialObjectKey,
    ) -> Result<NodeId, InsertError> {
        let current = self.current_node_id();
        let existing = self
            .document
            .node(current)
            .as_partial_map()
            .and_then(|pm| pm.find(&key))
            .copied();

        let node_id = if let Some(node_id) = existing {
            node_id
        } else {
            self.document
                .add_partial_map_child(key.clone(), current)
                .map_err(|kind| InsertError {
                    kind,
                    path: EurePath::from_iter(self.path.iter().cloned()),
                })?
                .node_id
        };

        let segment = PathSegment::from_partial_object_key(key);

        self.stack.push(node_id);
        self.hole_bound.push(false);
        self.path.push(segment);
        Ok(node_id)
    }

    /// Validate that the current node is a Hole (unbound).
    /// Use this before binding a value to ensure the node hasn't already been assigned.
    pub fn require_hole(&self) -> Result<(), InsertError> {
        let node = self.current_node();
        if !node.content.is_hole() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.path.iter().cloned()),
            });
        }
        Ok(())
    }

    /// Bind a hole (optionally labeled) to the current node.
    pub fn bind_hole(&mut self, label: Option<Identifier>) -> Result<(), InsertError> {
        if !self.current_node().content.is_hole() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        self.hole_bound[self.stack.len() - 1] = true;
        self.unbound_nodes.swap_remove(&self.current_node_id());
        self.current_node_mut().content = NodeValue::Hole(label);
        Ok(())
    }

    /// Bind a primitive value to the current node. Error if already bound.
    pub fn bind_primitive(&mut self, value: PrimitiveValue) -> Result<(), InsertError> {
        let node = self.current_node_mut();
        if !node.content.is_hole() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        node.content = NodeValue::Primitive(value);
        Ok(())
    }

    /// Bind a value to the current node using `Into<PrimitiveValue>`.
    ///
    /// This is a convenience method for use with the `eure!` macro.
    /// It accepts any type that implements `Into<PrimitiveValue>`.
    pub fn bind_from(&mut self, value: impl Into<PrimitiveValue>) -> Result<(), InsertError> {
        self.bind_primitive(value.into())
    }

    /// Bind an empty map to the current node. Error if already bound.
    pub fn bind_empty_map(&mut self) -> Result<(), InsertError> {
        let node = self.current_node_mut();
        if !node.content.is_hole() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        node.content = NodeValue::Map(Default::default());
        Ok(())
    }

    /// Bind an empty PartialMap to the current node. Error if already bound.
    pub fn bind_empty_partial_map(&mut self) -> Result<(), InsertError> {
        let node = self.current_node_mut();
        if !node.content.is_hole() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        node.content = NodeValue::PartialMap(PartialNodeMap::new());
        Ok(())
    }

    /// Bind an empty array to the current node. Error if already bound.
    pub fn bind_empty_array(&mut self) -> Result<(), InsertError> {
        let node = self.current_node_mut();
        if !node.content.is_hole() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        node.content = NodeValue::Array(Default::default());
        Ok(())
    }

    /// Bind an empty tuple to the current node. Error if already bound.
    pub fn bind_empty_tuple(&mut self) -> Result<(), InsertError> {
        let node = self.current_node_mut();
        if !node.content.is_hole() {
            return Err(InsertError {
                kind: InsertErrorKind::BindingTargetHasValue,
                path: EurePath::from_iter(self.current_path().iter().cloned()),
            });
        }
        node.content = NodeValue::Tuple(Default::default());
        Ok(())
    }

    /// Push a fresh block scope used to track `[^]` resolution.
    fn begin_block_scope(&mut self) {
        self.block_scope_stack.push(BlockScope::default());
    }

    /// Pop the topmost block scope. Safe to call as long as at least two scopes are on
    /// the stack; the root scope is preserved.
    fn end_block_scope(&mut self) {
        debug_assert!(
            self.block_scope_stack.len() > 1,
            "attempted to pop the root block scope"
        );
        if self.block_scope_stack.len() > 1 {
            self.block_scope_stack.pop();
        }
    }

    // =========================================================================
    // Source Layout Markers
    //
    // These methods allow the eure! macro and interpreter to delimit block scopes so
    // `[^]` can be resolved against the correct scope.
    // =========================================================================

    /// Enter a new EureSource block (e.g. `{ ... }` body of a binding or section).
    pub fn begin_eure_block(&mut self) {
        self.begin_block_scope();
    }

    /// Set the value binding for current block. No-op for DocumentConstructor.
    pub fn set_block_value(&mut self) -> Result<(), InsertError> {
        Ok(())
    }

    /// End current EureSource block.
    pub fn end_eure_block(&mut self) -> Result<(), InsertError> {
        self.end_block_scope();
        Ok(())
    }

    /// Mark the start of a binding statement. No-op for DocumentConstructor.
    pub fn begin_binding(&mut self) {}

    /// End binding #1: path = value. No-op for DocumentConstructor.
    pub fn end_binding_value(&mut self) -> Result<(), InsertError> {
        Ok(())
    }

    /// End binding #2/#3: path { eure }. No-op for DocumentConstructor.
    pub fn end_binding_block(&mut self) -> Result<(), InsertError> {
        Ok(())
    }

    /// Start a section header. No-op for DocumentConstructor.
    pub fn begin_section(&mut self) {}

    /// Begin section #4: items follow (flat section body).
    pub fn begin_section_items(&mut self) {
        self.begin_block_scope();
    }

    /// End section #4: finalize section with items.
    pub fn end_section_items(&mut self) -> Result<(), InsertError> {
        self.end_block_scope();
        Ok(())
    }

    /// End section #5/#6: block. No-op for DocumentConstructor.
    pub fn end_section_block(&mut self) -> Result<(), InsertError> {
        Ok(())
    }
}

impl InterpreterSink for DocumentConstructor {
    type Error = InsertError;
    type Scope = Scope;

    fn begin_scope(&mut self) -> Self::Scope {
        DocumentConstructor::begin_scope(self)
    }

    fn end_scope(&mut self, scope: Self::Scope) -> Result<(), Self::Error> {
        DocumentConstructor::end_scope(self, scope).map_err(|e| InsertError {
            kind: InsertErrorKind::ScopeError(e),
            path: EurePath::from_iter(self.current_path().iter().cloned()),
        })
    }

    fn navigate(&mut self, segment: PathSegment) -> Result<NodeId, Self::Error> {
        DocumentConstructor::navigate(self, segment)
    }

    fn require_hole(&self) -> Result<(), Self::Error> {
        DocumentConstructor::require_hole(self)
    }

    fn bind_primitive(&mut self, value: PrimitiveValue) -> Result<(), Self::Error> {
        DocumentConstructor::bind_primitive(self, value)
    }

    fn bind_hole(&mut self, label: Option<Identifier>) -> Result<(), Self::Error> {
        DocumentConstructor::bind_hole(self, label)
    }

    fn bind_empty_map(&mut self) -> Result<(), Self::Error> {
        DocumentConstructor::bind_empty_map(self)
    }

    fn bind_empty_array(&mut self) -> Result<(), Self::Error> {
        DocumentConstructor::bind_empty_array(self)
    }

    fn bind_empty_tuple(&mut self) -> Result<(), Self::Error> {
        DocumentConstructor::bind_empty_tuple(self)
    }

    fn current_node_id(&self) -> NodeId {
        DocumentConstructor::current_node_id(self)
    }

    fn current_path(&self) -> &[PathSegment] {
        DocumentConstructor::current_path(self)
    }

    fn document(&self) -> &EureDocument {
        DocumentConstructor::document(self)
    }

    fn document_mut(&mut self) -> &mut EureDocument {
        DocumentConstructor::document_mut(self)
    }

    fn begin_eure_block(&mut self) {
        DocumentConstructor::begin_eure_block(self)
    }

    fn end_eure_block(&mut self) -> Result<(), Self::Error> {
        DocumentConstructor::end_eure_block(self)
    }

    fn begin_section_items(&mut self) {
        DocumentConstructor::begin_section_items(self)
    }

    fn end_section_items(&mut self) -> Result<(), Self::Error> {
        DocumentConstructor::end_section_items(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifier::IdentifierParser;
    use crate::value::{PartialObjectKey, Tuple};

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
        assert!(node.content.is_hole());
    }

    #[test]
    fn test_navigate_single_ident() {
        let mut constructor = DocumentConstructor::new();

        let identifier = create_identifier("field");
        let segment = PathSegment::Ident(identifier.clone());

        let node_id = constructor
            .navigate(segment.clone())
            .expect("Failed to navigate");

        assert_eq!(constructor.current_node_id(), node_id);
        assert_eq!(constructor.current_path(), &[segment]);
    }

    #[test]
    fn test_navigate_multiple_times() {
        let mut constructor = DocumentConstructor::new();

        let id1 = create_identifier("field1");
        let id2 = create_identifier("field2");

        constructor
            .navigate(PathSegment::Ident(id1.clone()))
            .expect("Failed to navigate first");

        let node_id2 = constructor
            .navigate(PathSegment::Extension(id2.clone()))
            .expect("Failed to navigate second");

        assert_eq!(constructor.current_node_id(), node_id2);
        assert_eq!(
            constructor.current_path(),
            &[PathSegment::Ident(id1), PathSegment::Extension(id2)]
        );
    }

    #[test]
    fn test_navigate_error_propagates() {
        // Try to add tuple index to primitive node (should fail)
        let mut constructor = DocumentConstructor::new();
        // First navigate to the field node
        let identifier = create_identifier("field");
        constructor
            .navigate(PathSegment::Ident(identifier))
            .expect("Failed to navigate");
        // Set it to Primitive
        let node_id = constructor.current_node_id();
        constructor.document_mut().node_mut(node_id).content =
            NodeValue::Primitive(PrimitiveValue::Null);

        let result = constructor.navigate(PathSegment::TupleIndex(0));

        assert_eq!(
            result.map_err(|e| e.kind),
            Err(InsertErrorKind::ExpectedTuple)
        );
    }

    #[test]
    fn test_scope_success() {
        let mut constructor = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        let identifier = create_identifier("field");
        let token = constructor.begin_scope();
        let _node_id = constructor
            .navigate(PathSegment::Ident(identifier.clone()))
            .expect("Failed to navigate");

        // End scope
        let result = constructor.end_scope(token);
        assert_eq!(result, Ok(()));

        // After end_scope, should be back at root
        assert_eq!(constructor.current_node_id(), root_id);
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_scope_lifo_enforcement() {
        let mut constructor = DocumentConstructor::new();

        let id1 = create_identifier("field1");
        let id2 = create_identifier("field2");

        let token1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(id1))
            .expect("Failed to navigate");

        let token2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Extension(id2))
            .expect("Failed to navigate");

        // Try to end token1 before token2 (should fail)
        let result = constructor.end_scope(token1);
        assert_eq!(result, Err(ScopeError::NotMostRecentScope));

        // End in correct order
        constructor
            .end_scope(token2)
            .expect("Failed to end scope 2");
        constructor
            .end_scope(token1)
            .expect("Failed to end scope 1");
    }

    #[test]
    fn test_scope_with_multiple_navigations() {
        let mut constructor = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        let id1 = create_identifier("level1");
        let id2 = create_identifier("level2");
        let id3 = create_identifier("level3");

        let token = constructor.begin_scope();

        // Navigate three levels
        let node_id1 = constructor
            .navigate(PathSegment::Ident(id1.clone()))
            .expect("Failed to navigate level1");

        let node_id2 = constructor
            .navigate(PathSegment::Extension(id2.clone()))
            .expect("Failed to navigate level2");

        let node_id3 = constructor
            .navigate(PathSegment::Extension(id3.clone()))
            .expect("Failed to navigate level3");

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

        // End scope - should restore to root
        constructor.end_scope(token).expect("Failed to end scope");
        assert_eq!(constructor.current_node_id(), root_id);
        assert_eq!(constructor.current_path(), &[]);

        // Verify nodes still exist in document (node() panics if not found)
        let _ = constructor.document().node(node_id1);
        let _ = constructor.document().node(node_id2);
        let _ = constructor.document().node(node_id3);
    }

    #[test]
    fn test_nested_scopes() {
        let mut constructor = DocumentConstructor::new();
        let root_id = constructor.document().get_root_id();

        let id1 = create_identifier("a");
        let id2 = create_identifier("b");
        let id3 = create_identifier("c");

        // Outer scope: navigate to a
        let token_outer = constructor.begin_scope();
        let node_a = constructor
            .navigate(PathSegment::Ident(id1.clone()))
            .expect("Failed to navigate a");

        // Inner scope: navigate to b.c
        let token_inner = constructor.begin_scope();
        let _node_b = constructor
            .navigate(PathSegment::Extension(id2.clone()))
            .expect("Failed to navigate b");
        let _node_c = constructor
            .navigate(PathSegment::Extension(id3.clone()))
            .expect("Failed to navigate c");

        // End inner scope - should be back at a
        constructor
            .end_scope(token_inner)
            .expect("Failed to end inner scope");
        assert_eq!(constructor.current_node_id(), node_a);
        assert_eq!(constructor.current_path(), &[PathSegment::Ident(id1)]);

        // End outer scope - should be back at root
        constructor
            .end_scope(token_outer)
            .expect("Failed to end outer scope");
        assert_eq!(constructor.current_node_id(), root_id);
        assert_eq!(constructor.current_path(), &[]);
    }

    #[test]
    fn test_require_hole_success() {
        let mut constructor = DocumentConstructor::new();

        let identifier = create_identifier("field");
        constructor
            .navigate(PathSegment::Ident(identifier))
            .expect("Failed to navigate");

        // New node should be a Hole
        let result = constructor.require_hole();
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_require_hole_fails_when_bound() {
        let mut constructor = DocumentConstructor::new();

        let identifier = create_identifier("field");
        let node_id = constructor
            .navigate(PathSegment::Ident(identifier))
            .expect("Failed to navigate");

        // Set the node to have a value
        constructor.document_mut().node_mut(node_id).content =
            NodeValue::Primitive(PrimitiveValue::Bool(true));

        // require_hole should fail
        let result = constructor.require_hole();
        assert_eq!(
            result.unwrap_err().kind,
            InsertErrorKind::BindingTargetHasValue
        );
    }

    #[test]
    fn test_bind_primitive_success() {
        let mut constructor = DocumentConstructor::new();
        let identifier = create_identifier("field");

        // Navigate to a field node
        let node_id = constructor
            .navigate(PathSegment::Ident(identifier))
            .expect("Failed to navigate");

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

        // Navigate to a field node
        let node_id = constructor
            .navigate(PathSegment::Ident(identifier.clone()))
            .expect("Failed to navigate");

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

    #[test]
    fn test_finish_replaces_uninitialized_root_with_null() {
        let constructor = DocumentConstructor::new();

        // Root should be Hole before finish
        let root_id = constructor.document().get_root_id();
        assert!(constructor.document().node(root_id).content.is_hole());

        // After finish, root should be empty map
        let document = constructor.finish();
        let root_node = document.node(document.get_root_id());
        assert_eq!(root_node.content, NodeValue::Map(Default::default()));
    }

    #[test]
    fn test_finish_preserves_initialized_root() {
        let mut constructor = DocumentConstructor::new();

        // Bind a value to the root
        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .expect("Failed to bind");

        // After finish, root should still have the bound value
        let document = constructor.finish();
        let root_node = document.node(document.get_root_id());
        assert!(matches!(
            root_node.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        ));
    }

    #[test]
    fn test_finish_preserves_partial_map_root() {
        let mut constructor = DocumentConstructor::new();

        constructor
            .navigate_partial_map_entry(PartialObjectKey::Hole(Some(create_identifier("x"))))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(1.into()))
            .unwrap();

        let document = constructor.finish();
        assert!(matches!(
            document.node(document.get_root_id()).content,
            NodeValue::PartialMap(_)
        ));
    }

    #[test]
    fn test_navigate_partial_map_entry_does_not_reuse_tuple_with_anonymous_hole() {
        let mut constructor = DocumentConstructor::new();
        let scope = constructor.begin_scope();

        let key = PartialObjectKey::Tuple(Tuple(vec![
            PartialObjectKey::Number(1.into()),
            PartialObjectKey::Hole(None),
        ]));

        let first = constructor.navigate_partial_map_entry(key.clone()).unwrap();
        constructor.end_scope(scope).unwrap();

        let second_scope = constructor.begin_scope();
        let second = constructor.navigate_partial_map_entry(key).unwrap();

        assert_ne!(first, second);
        constructor.end_scope(second_scope).unwrap();
    }

    #[test]
    fn test_navigate_reuses_labeled_hole_key_segment() {
        let mut constructor = DocumentConstructor::new();
        let label = create_identifier("x");

        let scope = constructor.begin_scope();
        let first = constructor
            .navigate(PathSegment::HoleKey(Some(label.clone())))
            .unwrap();
        constructor.end_scope(scope).unwrap();

        let scope = constructor.begin_scope();
        let second = constructor
            .navigate(PathSegment::HoleKey(Some(label)))
            .unwrap();

        assert_eq!(first, second);
        constructor.end_scope(scope).unwrap();
    }

    #[test]
    fn test_typical_binding_pattern() {
        // Test the typical pattern: a.b.c = true
        let mut constructor = DocumentConstructor::new();

        let id_a = create_identifier("a");
        let id_b = create_identifier("b");
        let id_c = create_identifier("c");

        let token = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(id_a.clone()))
            .unwrap();
        constructor
            .navigate(PathSegment::Extension(id_b.clone()))
            .unwrap();
        let node_c = constructor
            .navigate(PathSegment::Extension(id_c.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Bool(true))
            .unwrap();
        constructor.end_scope(token).unwrap();

        // Verify the value was bound
        let node = constructor.document().node(node_c);
        assert!(matches!(
            node.content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        ));
    }

    // ==========================================================================
    // `[^]` (ArrayIndexKind::Current) semantics
    //
    // Each of these tests mirrors one of the cases outlined in the design doc:
    // - Push records the target element in the active block scope.
    // - Current resolves to that element, merging further navigation into it.
    // - A Current with no matching prior Push in the same block scope errors.
    // - Block scope boundaries (begin_eure_block / begin_section_items) isolate
    //   pushes so that `[^]` cannot reach across into a parent scope.
    // - Current respects the usual type checks: if the target is not an array,
    //   the ExpectedArray error is surfaced.
    // ==========================================================================

    #[test]
    fn test_array_current_merges_with_preceding_push_root_scope() {
        // users[].x = 1; users[^].y = 2 → one element with both fields.
        let mut constructor = DocumentConstructor::new();
        let users = create_identifier("users");
        let x = create_identifier("x");
        let y = create_identifier("y");

        // users[].x = 1
        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let pushed = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        let x_node = constructor.navigate(PathSegment::Ident(x.clone())).unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(1.into()))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        // users[^].y = 2
        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let current = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap();
        let y_node = constructor.navigate(PathSegment::Ident(y.clone())).unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(2.into()))
            .unwrap();
        constructor.end_scope(t2).unwrap();

        assert_eq!(pushed, current, "[^] must resolve to the last [] push");

        let element = constructor.document().node(pushed);
        let map = element.as_map().expect("element should be a map");
        let resolved_x = map
            .get_node_id(&ObjectKey::String(x.to_string()))
            .expect("x should exist");
        let resolved_y = map
            .get_node_id(&ObjectKey::String(y.to_string()))
            .expect("y should exist");
        assert_eq!(resolved_x, x_node);
        assert_eq!(resolved_y, y_node);
    }

    #[test]
    fn test_array_current_merges_inside_block_scope() {
        // posts[].title = "a"; posts[^].body = "b" inside a single block scope.
        let mut constructor = DocumentConstructor::new();
        let posts = create_identifier("posts");
        let title = create_identifier("title");
        let body = create_identifier("body");

        // Simulate `{ posts[].title = "a"; posts[^].body = "b" }` by wrapping in
        // a block scope.
        constructor.begin_eure_block();

        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(posts.clone()))
            .unwrap();
        let pushed = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(title.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("a")))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(posts.clone()))
            .unwrap();
        let current = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(body.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("b")))
            .unwrap();
        constructor.end_scope(t2).unwrap();

        constructor.end_eure_block().unwrap();

        assert_eq!(pushed, current);
        let map = constructor.document().node(pushed).as_map().unwrap();
        assert!(
            map.get_node_id(&ObjectKey::String(title.to_string()))
                .is_some()
        );
        assert!(
            map.get_node_id(&ObjectKey::String(body.to_string()))
                .is_some()
        );
    }

    #[test]
    fn test_array_current_cannot_reach_across_block_scope() {
        // users[] at root; inside a nested block, users[^] must error — the inner
        // scope saw no push of its own.
        let mut constructor = DocumentConstructor::new();
        let users = create_identifier("users");
        let x = create_identifier("x");

        // Root: users[].x = 1
        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor.navigate(PathSegment::Ident(x.clone())).unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(1.into()))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        // Inside a nested block scope: users[^].x must fail.
        constructor.begin_eure_block();
        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let err = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap_err();
        assert!(matches!(
            err.kind,
            InsertErrorKind::ArrayCurrentOutOfScope { .. }
        ));
        // Rewind the failed scope so we can finish the test cleanly.
        constructor.end_scope(t2).unwrap();
        constructor.end_eure_block().unwrap();
    }

    #[test]
    fn test_array_current_after_inner_block_refers_to_outer_last_push() {
        // Two root-level sibling pushes; a nested block inside the first must
        // not leak into the outer scope's `last_pushes`. After both, root-level
        // users[^] should resolve to the second root-level push.
        let mut constructor = DocumentConstructor::new();
        let users = create_identifier("users");
        let nested = create_identifier("nested");
        let x = create_identifier("x");
        let y = create_identifier("y");

        // users[].x = 1 with a nested-block-local array touched in between.
        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let first = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor.navigate(PathSegment::Ident(x.clone())).unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(1.into()))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        // A nested block at root that happens to push into `nested` — this
        // must not be visible to the outer scope.
        constructor.begin_eure_block();
        let inner_scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(nested.clone()))
            .unwrap();
        constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor.bind_primitive(PrimitiveValue::Null).unwrap();
        constructor.end_scope(inner_scope).unwrap();
        constructor.end_eure_block().unwrap();

        // A second root-level push.
        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let second = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor.navigate(PathSegment::Ident(x.clone())).unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(2.into()))
            .unwrap();
        constructor.end_scope(t2).unwrap();

        // users[^].y at root should attach to `second`, not `first`.
        let t3 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let current = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap();
        constructor.navigate(PathSegment::Ident(y.clone())).unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(3.into()))
            .unwrap();
        constructor.end_scope(t3).unwrap();

        assert_ne!(first, second);
        assert_eq!(
            current, second,
            "[^] must refer to the second root-level push"
        );
    }

    #[test]
    fn test_array_current_without_prior_push_errors() {
        // An array seeded by an explicit index (no Push recorded) should also
        // surface ArrayCurrentOutOfScope when followed by `[^]` in the same
        // scope.
        let mut constructor = DocumentConstructor::new();
        let users = create_identifier("users");
        let x = create_identifier("x");

        // Seed: users[0].x = 1 — this makes `users` an array without recording
        // a push in the block scope.
        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Specific(0)))
            .unwrap();
        constructor.navigate(PathSegment::Ident(x.clone())).unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(1.into()))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        // users[^] should now error — no Push has been recorded for this array.
        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let err = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap_err();
        assert!(
            matches!(err.kind, InsertErrorKind::ArrayCurrentOutOfScope { .. }),
            "expected ArrayCurrentOutOfScope, got {:?}",
            err.kind
        );
        constructor.end_scope(t2).unwrap();
    }

    #[test]
    fn test_array_current_nested_arrays() {
        // orgs[].teams[].members[].name = ...; orgs[^].teams[^].members[].age = ...
        // Inner-inner `[]` creates a second member in the same team.
        let mut constructor = DocumentConstructor::new();
        let orgs = create_identifier("orgs");
        let teams = create_identifier("teams");
        let members = create_identifier("members");
        let name = create_identifier("name");
        let age = create_identifier("age");

        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(orgs.clone()))
            .unwrap();
        let org = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(teams.clone()))
            .unwrap();
        let team = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(members.clone()))
            .unwrap();
        let member1 = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(name.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("Ada")))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(orgs.clone()))
            .unwrap();
        let org_current = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(teams.clone()))
            .unwrap();
        let team_current = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(members.clone()))
            .unwrap();
        let member2 = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(age.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(30.into()))
            .unwrap();
        constructor.end_scope(t2).unwrap();

        assert_eq!(org_current, org);
        assert_eq!(team_current, team);
        assert_ne!(member1, member2, "the inner [] should create a new member");

        // Team should now have two members.
        let members_node = {
            let team_map = constructor.document().node(team).as_map().unwrap();
            team_map
                .get_node_id(&ObjectKey::String(members.to_string()))
                .expect("members array")
        };
        let arr = constructor
            .document()
            .node(members_node)
            .as_array()
            .unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_array_current_followed_by_push_into_child() {
        // users[].name = "Ada"; users[^].posts[].title = "hello"
        // — reuse the same user, add a new nested post.
        let mut constructor = DocumentConstructor::new();
        let users = create_identifier("users");
        let name = create_identifier("name");
        let posts = create_identifier("posts");
        let title = create_identifier("title");

        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let user = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(name.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("Ada")))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(users.clone()))
            .unwrap();
        let same_user = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(posts.clone()))
            .unwrap();
        constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(title.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("hello")))
            .unwrap();
        constructor.end_scope(t2).unwrap();

        assert_eq!(same_user, user);
    }

    #[test]
    fn test_array_current_on_non_array_errors_as_expected_array() {
        // Set a field to a primitive first, then try `[^]` on it.
        // The ExpectedArray error should take precedence over ArrayCurrentOutOfScope.
        let mut constructor = DocumentConstructor::new();
        let items = create_identifier("items");

        let t1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(items.clone()))
            .unwrap();
        constructor.require_hole().unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(42.into()))
            .unwrap();
        constructor.end_scope(t1).unwrap();

        let t2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(items.clone()))
            .unwrap();
        let err = constructor
            .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Current))
            .unwrap_err();
        assert!(
            matches!(err.kind, InsertErrorKind::ExpectedArray),
            "expected ExpectedArray, got {:?}",
            err.kind
        );
        constructor.end_scope(t2).unwrap();
    }
}
