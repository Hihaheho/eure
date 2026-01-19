use indexmap::IndexSet;

use crate::document::interpreter_sink::InterpreterSink;
use crate::prelude_internal::*;

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
    pub fn navigate(&mut self, segment: PathSegment) -> Result<NodeId, InsertError> {
        let current = self.current_node_id();
        let node_mut = self
            .document
            .resolve_child_by_segment(segment.clone(), current)
            .map_err(|e| InsertError {
                kind: e,
                path: EurePath::from_iter(self.path.iter().cloned()),
            })?;
        let node_id = node_mut.node_id;
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

    // =========================================================================
    // Source Layout Markers (no-op for DocumentConstructor)
    //
    // These methods allow the eure! macro to call them without importing the
    // InterpreterSink trait. They use the trait's default no-op implementations.
    // =========================================================================

    /// Enter a new EureSource block. No-op for DocumentConstructor.
    pub fn begin_eure_block(&mut self) {}

    /// Set the value binding for current block. No-op for DocumentConstructor.
    pub fn set_block_value(&mut self) -> Result<(), InsertError> {
        Ok(())
    }

    /// End current EureSource block. No-op for DocumentConstructor.
    pub fn end_eure_block(&mut self) -> Result<(), InsertError> {
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

    /// Begin section #4: items follow. No-op for DocumentConstructor.
    pub fn begin_section_items(&mut self) {}

    /// End section #4: finalize section with items. No-op for DocumentConstructor.
    pub fn end_section_items(&mut self) -> Result<(), InsertError> {
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
}
