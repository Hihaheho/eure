mod span;

use ahash::HashMap;
use std::{collections::BTreeMap, convert::Infallible};
use thiserror::Error;

// Re-export common types from parol-walker
pub use parol_walker::{
    CstNodeData, CstNodeId, DynamicTokenId, InputSpan, NodeKind, NonTerminalData, TerminalData,
};

// Type alias for backwards compatibility with generated code
pub type ViewConstructionError<T, Nt, E = Infallible> = EureViewConstructionError<T, Nt, E>;

pub use span::{CharInfo, LineNumbers};

use crate::{
    CstConstructError,
    node_kind::{NonTerminalKind, TerminalKind},
    nodes::{BlockComment, LineComment, NewLine, RootHandle, Whitespace},
    visitor::{BuiltinTerminalVisitor, CstVisitor},
};

/// Extension trait for CstNodeData with Eure-specific validation methods
pub trait CstNodeDataExt<T, Nt> {
    /// Check if this node is a terminal of the expected kind, returning an error if not
    fn expected_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: T,
    ) -> Result<(T, TerminalData), EureViewConstructionError<T, Nt>>;

    /// Check if this node is a non-terminal of the expected kind, returning an error if not
    fn expected_non_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: Nt,
    ) -> Result<(Nt, NonTerminalData), EureViewConstructionError<T, Nt>>;
}

impl<T, Nt> CstNodeDataExt<T, Nt> for CstNodeData<T, Nt>
where
    T: PartialEq + Copy,
    Nt: PartialEq + Copy,
{
    fn expected_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: T,
    ) -> Result<(T, TerminalData), EureViewConstructionError<T, Nt>> {
        match self {
            CstNodeData::Terminal { kind, data } if *kind == expected => Ok((*kind, *data)),
            _ => Err(EureViewConstructionError::UnexpectedNode {
                node,
                data: *self,
                expected_kind: NodeKind::Terminal(expected),
            }),
        }
    }

    fn expected_non_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: Nt,
    ) -> Result<(Nt, NonTerminalData), EureViewConstructionError<T, Nt>> {
        match self {
            CstNodeData::NonTerminal { kind, data } if *kind == expected => Ok((*kind, *data)),
            _ => Err(EureViewConstructionError::UnexpectedNode {
                node,
                data: *self,
                expected_kind: NodeKind::NonTerminal(expected),
            }),
        }
    }
}

/// A generic concrete syntax tree with stable child ordering
#[derive(Debug, Clone)]
pub struct ConcreteSyntaxTree<T, Nt> {
    nodes: Vec<CstNodeData<T, Nt>>,
    children: HashMap<CstNodeId, Vec<CstNodeId>>,
    parent: HashMap<CstNodeId, CstNodeId>,
    dynamic_tokens: BTreeMap<DynamicTokenId, String>,
    next_dynamic_token_id: u32,
    root: CstNodeId,
}

impl<T, Nt> ConcreteSyntaxTree<T, Nt>
where
    T: Clone,
    Nt: Clone,
{
    pub fn new(root_data: CstNodeData<T, Nt>) -> Self {
        let nodes = vec![root_data];
        let root = CstNodeId(0);

        Self {
            nodes,
            children: HashMap::default(),
            parent: HashMap::default(),
            dynamic_tokens: BTreeMap::new(),
            next_dynamic_token_id: 0,
            root,
        }
    }

    pub fn root(&self) -> CstNodeId {
        self.root
    }

    pub fn set_root(&mut self, new_root: CstNodeId) {
        self.root = new_root;
    }

    pub fn change_parent(&mut self, id: CstNodeId, new_parent: CstNodeId) {
        // Remove from old parent's children
        if let Some(old_parent) = self.parent.get(&id).copied()
            && let Some(children) = self.children.get_mut(&old_parent)
        {
            children.retain(|&child| child != id);
        }

        // Add to new parent's children
        self.children.entry(new_parent).or_default().push(id);
        self.parent.insert(id, new_parent);
    }

    pub fn add_node(&mut self, data: CstNodeData<T, Nt>) -> CstNodeId {
        let id = CstNodeId(self.nodes.len());
        self.nodes.push(data);
        id
    }

    pub fn add_node_with_parent(
        &mut self,
        data: CstNodeData<T, Nt>,
        parent: CstNodeId,
    ) -> CstNodeId {
        let node = self.add_node(data);
        self.add_child(parent, node);
        node
    }

    pub fn add_child(&mut self, parent: CstNodeId, child: CstNodeId) {
        self.children.entry(parent).or_default().push(child);
        self.parent.insert(child, parent);
    }

    pub fn has_no_children(&self, node: CstNodeId) -> bool {
        self.children
            .get(&node)
            .is_none_or(|children| children.is_empty())
    }

    pub fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId> + '_ {
        self.children
            .get(&node)
            .into_iter()
            .flat_map(|children| children.iter().copied())
    }

    pub fn parent(&self, node: CstNodeId) -> Option<CstNodeId> {
        self.parent.get(&node).copied()
    }

    pub fn get_str<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        terminal: TerminalData,
        input: &'b str,
    ) -> Option<&'c str> {
        match terminal {
            TerminalData::Input(span) => Some(span.as_str(input)),
            TerminalData::Dynamic(id) => self.dynamic_token(id),
        }
    }

    pub fn dynamic_token(&self, id: DynamicTokenId) -> Option<&str> {
        self.dynamic_tokens.get(&id).map(|s| s.as_str())
    }

    pub fn update_node(
        &mut self,
        id: CstNodeId,
        data: CstNodeData<T, Nt>,
    ) -> Option<CstNodeData<T, Nt>> {
        if id.0 < self.nodes.len() {
            Some(std::mem::replace(&mut self.nodes[id.0], data))
        } else {
            None
        }
    }

    pub fn update_children(
        &mut self,
        id: CstNodeId,
        new_children: impl IntoIterator<Item = CstNodeId>,
    ) {
        let new_children: Vec<_> = new_children.into_iter().collect();

        // Update parent pointers for old children (remove this parent)
        if let Some(old_children) = self.children.get(&id) {
            for &child in old_children {
                self.parent.remove(&child);
            }
        }

        // Update parent pointers for new children
        for &child in &new_children {
            self.parent.insert(child, id);
        }

        // Set new children
        if new_children.is_empty() {
            self.children.remove(&id);
        } else {
            self.children.insert(id, new_children);
        }
    }

    pub fn insert_dynamic_terminal(&mut self, data: impl Into<String>) -> DynamicTokenId {
        let id = DynamicTokenId(self.next_dynamic_token_id);
        self.dynamic_tokens.insert(id, data.into());
        self.next_dynamic_token_id += 1;
        id
    }
}

impl<T, Nt> ConcreteSyntaxTree<T, Nt>
where
    T: Copy,
    Nt: Copy,
{
    pub fn node_data(&self, node: CstNodeId) -> Option<CstNodeData<T, Nt>> {
        self.nodes.get(node.0).copied()
    }

    /// Remove a node from the tree by removing it from all parent-child relationships.
    /// The node data remains in the vector but becomes unreachable through tree traversal.
    pub fn remove_node(&mut self, id: CstNodeId) {
        // Remove from parent's children list
        if let Some(parent_id) = self.parent.remove(&id)
            && let Some(parent_children) = self.children.get_mut(&parent_id)
        {
            parent_children.retain(|&child| child != id);
        }

        // Remove children mapping (but don't delete child nodes recursively)
        self.children.remove(&id);
    }
}

impl TerminalKind {
    fn auto_ws_is_off(&self, _index: usize) -> bool {
        matches!(
            self,
            TerminalKind::Ws | TerminalKind::GrammarNewline | TerminalKind::Text
        )
    }
}

impl ConcreteSyntaxTree<TerminalKind, NonTerminalKind> {
    pub fn get_non_terminal(
        &self,
        id: CstNodeId,
        kind: NonTerminalKind,
    ) -> Result<NonTerminalData, CstConstructError> {
        let node_data = self
            .node_data(id)
            .ok_or(EureViewConstructionError::NodeIdNotFound { node: id })?;
        let (_, data) = node_data.expected_non_terminal_or_error(id, kind)?;
        Ok(data)
    }

    pub fn get_terminal(
        &self,
        id: CstNodeId,
        kind: TerminalKind,
    ) -> Result<TerminalData, CstConstructError> {
        let node_data = self
            .node_data(id)
            .ok_or(EureViewConstructionError::NodeIdNotFound { node: id })?;
        let (_, data) = node_data.expected_terminal_or_error(id, kind)?;
        Ok(data)
    }

    pub fn collect_nodes<
        'v,
        const N: usize,
        V: BuiltinTerminalVisitor<E, F>,
        O,
        E,
        F: CstFacade,
    >(
        &self,
        facade: &F,
        parent: CstNodeId,
        nodes: [NodeKind<TerminalKind, NonTerminalKind>; N],
        mut visitor: impl FnMut(
            [CstNodeId; N],
            &'v mut V,
        ) -> Result<(O, &'v mut V), CstConstructError<E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        let children = self.children(parent).collect::<Vec<_>>();
        let mut children = children.into_iter();
        let mut result = Vec::with_capacity(N);
        let mut ignored = Vec::with_capacity(N);
        'outer: for expected_kind in nodes {
            'inner: for (idx, child) in children.by_ref().enumerate() {
                let child_data = self
                    .node_data(child)
                    .ok_or(EureViewConstructionError::NodeIdNotFound { node: child })?;
                match child_data {
                    CstNodeData::Terminal { kind, data } => {
                        if NodeKind::Terminal(kind) == expected_kind {
                            result.push(child);
                            continue 'outer;
                        } else if kind.is_builtin_whitespace() || kind.is_builtin_new_line() {
                            if kind.auto_ws_is_off(idx) {
                                return Err(EureViewConstructionError::UnexpectedNode {
                                    node: child,
                                    data: child_data,
                                    expected_kind,
                                });
                            }
                            ignored.push((child, kind, data));
                            continue 'inner;
                        } else if kind.is_builtin_line_comment() || kind.is_builtin_block_comment()
                        {
                            ignored.push((child, kind, data));
                            continue 'inner;
                        } else {
                            return Err(EureViewConstructionError::UnexpectedNode {
                                node: child,
                                data: child_data,
                                expected_kind,
                            });
                        }
                    }
                    CstNodeData::NonTerminal { kind, .. } => {
                        if NodeKind::NonTerminal(kind) == expected_kind {
                            result.push(child);
                            continue 'outer;
                        } else {
                            return Err(EureViewConstructionError::UnexpectedNode {
                                node: child,
                                data: child_data,
                                expected_kind,
                            });
                        }
                    }
                }
            }
            return Err(EureViewConstructionError::UnexpectedEndOfChildren { parent });
        }
        for (child, kind, data) in ignored {
            match kind {
                TerminalKind::Whitespace => visit_ignored.visit_builtin_whitespace_terminal(
                    Whitespace(child),
                    data,
                    facade,
                )?,
                TerminalKind::NewLine => {
                    visit_ignored.visit_builtin_new_line_terminal(NewLine(child), data, facade)?
                }
                TerminalKind::LineComment => visit_ignored.visit_builtin_line_comment_terminal(
                    LineComment(child),
                    data,
                    facade,
                )?,
                TerminalKind::BlockComment => visit_ignored.visit_builtin_block_comment_terminal(
                    BlockComment(child),
                    data,
                    facade,
                )?,
                _ => unreachable!(),
            }
        }
        let (result, visit_ignored) = visitor(
            result
                .try_into()
                .expect("Result should have the same length as nodes"),
            visit_ignored,
        )?;
        for child in children.by_ref() {
            let child_data = self
                .node_data(child)
                .ok_or(EureViewConstructionError::NodeIdNotFound { node: child })?;
            match child_data {
                CstNodeData::Terminal { kind, data } => {
                    if kind.is_builtin_terminal() {
                        match kind {
                            TerminalKind::Whitespace => visit_ignored
                                .visit_builtin_whitespace_terminal(
                                    Whitespace(child),
                                    data,
                                    facade,
                                )?,
                            TerminalKind::NewLine => visit_ignored
                                .visit_builtin_new_line_terminal(NewLine(child), data, facade)?,
                            TerminalKind::LineComment => visit_ignored
                                .visit_builtin_line_comment_terminal(
                                    LineComment(child),
                                    data,
                                    facade,
                                )?,
                            TerminalKind::BlockComment => visit_ignored
                                .visit_builtin_block_comment_terminal(
                                    BlockComment(child),
                                    data,
                                    facade,
                                )?,
                            _ => unreachable!(),
                        }
                    } else {
                        return Err(EureViewConstructionError::UnexpectedNode {
                            node: child,
                            data: child_data,
                            expected_kind: NodeKind::Terminal(kind),
                        });
                    }
                }
                CstNodeData::NonTerminal { kind, .. } => {
                    return Err(EureViewConstructionError::UnexpectedNode {
                        node: child,
                        data: child_data,
                        expected_kind: NodeKind::NonTerminal(kind),
                    });
                }
            }
        }
        Ok(result)
    }

    pub fn root_handle(&self) -> RootHandle {
        RootHandle(self.root())
    }

    pub fn visit_from_root<V: CstVisitor<Self>>(&self, visitor: &mut V) -> Result<(), V::Error> {
        visitor.visit_root_handle(self.root_handle(), self)
    }
}

/// Trait for accessing CST structure with Eure-specific methods
pub trait CstFacade: Sized {
    /// Get the string slice for a terminal node
    fn get_str<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        terminal: TerminalData,
        input: &'b str,
    ) -> Option<&'c str>;

    /// Get node data for a given node ID
    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData<TerminalKind, NonTerminalKind>>;

    /// Check if a node has no children
    fn has_no_children(&self, node: CstNodeId) -> bool;

    /// Get an iterator over a node's children
    fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId>;

    /// Get data for a terminal node of specific kind
    fn get_terminal(
        &self,
        node: CstNodeId,
        kind: TerminalKind,
    ) -> Result<TerminalData, CstConstructError>;

    /// Get data for a non-terminal node of specific kind
    fn get_non_terminal(
        &self,
        node: CstNodeId,
        kind: NonTerminalKind,
    ) -> Result<NonTerminalData, CstConstructError>;

    /// Get parent of a node
    fn parent(&self, node: CstNodeId) -> Option<CstNodeId>;

    /// Collect child nodes matching expected kinds, skipping builtin terminals
    fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        nodes: [NodeKind<TerminalKind, NonTerminalKind>; N],
        visitor: impl FnMut([CstNodeId; N], &'v mut V) -> Result<(O, &'v mut V), CstConstructError<E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>>;

    /// Get the content of a dynamic token
    fn dynamic_token(&self, id: DynamicTokenId) -> Option<&str>;

    /// Get the root handle
    fn root_handle(&self) -> RootHandle;

    /// Returns the string representation of a terminal. Returns None if the terminal is a dynamic token and not found.
    fn get_terminal_str<'a: 'c, 'b: 'c, 'c, T: TerminalHandle>(
        &'a self,
        input: &'b str,
        handle: T,
    ) -> Result<Result<&'c str, DynamicTokenId>, CstConstructError> {
        let data = self.get_terminal(handle.node_id(), handle.kind())?;
        match data {
            TerminalData::Input(input_span) => Ok(Ok(
                &input[input_span.start as usize..input_span.end as usize]
            )),
            TerminalData::Dynamic(id) => Ok(self.dynamic_token(id).ok_or(id)),
        }
    }

    /// Returns a span that excludes leading and trailing trivia (whitespace, newlines, comments).
    ///
    /// For terminal nodes:
    /// - Always returns the input span (even for trivia terminals like whitespace/comments)
    /// - Returns None only for dynamic terminals
    ///
    /// For non-terminal nodes:
    /// - Recursively finds the first and last non-trivia descendant spans
    /// - Returns the merged span of those two endpoints
    /// - Falls back to the original span if all descendants are trivia
    ///
    /// This is useful when you want to get the "meaningful" span of a syntax node,
    /// excluding any surrounding whitespace or comments.
    fn span(&self, node_id: CstNodeId) -> Option<InputSpan> {
        match self.node_data(node_id)? {
            CstNodeData::Terminal { data, .. } => {
                // Always return the span for terminals (including trivia)
                match data {
                    TerminalData::Input(span) => Some(span),
                    TerminalData::Dynamic(_) => None,
                }
            }
            CstNodeData::NonTerminal { data, .. } => {
                // Find first non-trivia span
                let first_span = self
                    .children(node_id)
                    .find_map(|child| self.find_first_non_trivia_span(child));

                // Find last non-trivia span
                let last_span = self
                    .children(node_id)
                    .rev()
                    .find_map(|child| self.find_last_non_trivia_span(child));

                match (first_span, last_span) {
                    (Some(first), Some(last)) => Some(first.merge(last)),
                    (Some(span), None) | (None, Some(span)) => Some(span),
                    (None, None) => {
                        // All children are trivia or no children - fall back to original span
                        match data {
                            NonTerminalData::Input(span) if span != InputSpan::EMPTY => Some(span),
                            _ => None,
                        }
                    }
                }
            }
        }
    }

    /// Returns the span of a node, including the trivia.
    fn concrete_span(&self, node_id: CstNodeId) -> Option<InputSpan> {
        match self.node_data(node_id)? {
            CstNodeData::NonTerminal {
                data: NonTerminalData::Input(span),
                ..
            } => Some(span),
            CstNodeData::Terminal { data, .. } => match data {
                TerminalData::Input(span) => Some(span),
                TerminalData::Dynamic(_) => None,
            },
            _ => None,
        }
    }

    /// Helper: finds the first non-trivia span by depth-first search from the start.
    fn find_first_non_trivia_span(&self, node_id: CstNodeId) -> Option<InputSpan> {
        match self.node_data(node_id)? {
            CstNodeData::Terminal { kind, data } => {
                if kind.is_builtin_terminal() {
                    None
                } else {
                    match data {
                        TerminalData::Input(span) => Some(span),
                        TerminalData::Dynamic(_) => None,
                    }
                }
            }
            CstNodeData::NonTerminal { .. } => self
                .children(node_id)
                .find_map(|child| self.find_first_non_trivia_span(child)),
        }
    }

    /// Helper: finds the last non-trivia span by depth-first search from the end.
    fn find_last_non_trivia_span(&self, node_id: CstNodeId) -> Option<InputSpan> {
        match self.node_data(node_id)? {
            CstNodeData::Terminal { kind, data } => {
                if kind.is_builtin_terminal() {
                    None
                } else {
                    match data {
                        TerminalData::Input(span) => Some(span),
                        TerminalData::Dynamic(_) => None,
                    }
                }
            }
            CstNodeData::NonTerminal { .. } => self
                .children(node_id)
                .rev()
                .find_map(|child| self.find_last_non_trivia_span(child)),
        }
    }
}

impl CstFacade for ConcreteSyntaxTree<TerminalKind, NonTerminalKind> {
    fn get_str<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        terminal: TerminalData,
        input: &'b str,
    ) -> Option<&'c str> {
        ConcreteSyntaxTree::get_str(self, terminal, input)
    }

    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData<TerminalKind, NonTerminalKind>> {
        ConcreteSyntaxTree::node_data(self, node)
    }

    fn has_no_children(&self, node: CstNodeId) -> bool {
        ConcreteSyntaxTree::has_no_children(self, node)
    }

    fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId> {
        ConcreteSyntaxTree::children(self, node)
    }

    fn get_terminal(
        &self,
        node: CstNodeId,
        kind: TerminalKind,
    ) -> Result<TerminalData, CstConstructError> {
        ConcreteSyntaxTree::get_terminal(self, node, kind)
    }

    fn get_non_terminal(
        &self,
        node: CstNodeId,
        kind: NonTerminalKind,
    ) -> Result<NonTerminalData, CstConstructError> {
        ConcreteSyntaxTree::get_non_terminal(self, node, kind)
    }

    fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        nodes: [NodeKind<TerminalKind, NonTerminalKind>; N],
        visitor: impl FnMut([CstNodeId; N], &'v mut V) -> Result<(O, &'v mut V), CstConstructError<E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        ConcreteSyntaxTree::collect_nodes(self, self, parent, nodes, visitor, visit_ignored)
    }

    fn dynamic_token(&self, id: DynamicTokenId) -> Option<&str> {
        ConcreteSyntaxTree::dynamic_token(self, id)
    }

    fn parent(&self, node: CstNodeId) -> Option<CstNodeId> {
        ConcreteSyntaxTree::parent(self, node)
    }

    fn root_handle(&self) -> RootHandle {
        ConcreteSyntaxTree::root_handle(self)
    }
}

#[derive(Debug, Clone, Error)]
/// Error that occurs when constructing a view from a [NonTerminalHandle].
pub enum EureViewConstructionError<T, Nt, E = Infallible> {
    /// Expected a specific kind of terminal node, but got an invalid node
    #[error("Unexpected node for expected kind: {expected_kind:?} but got {data:?}")]
    UnexpectedNode {
        /// The index of the node.
        node: CstNodeId,
        /// The data of the node.
        data: CstNodeData<T, Nt>,
        /// The expected kind.
        expected_kind: NodeKind<T, Nt>,
    },
    /// Expected an extra node, but got an invalid node
    #[error("Unexpected extra node")]
    UnexpectedExtraNode {
        /// The index of the node.
        node: CstNodeId,
    },
    /// Unexpected end of children
    #[error("Unexpected end of children")]
    UnexpectedEndOfChildren {
        /// The index of the node.
        parent: CstNodeId,
    },
    /// Unexpected empty children for a non-terminal
    #[error("Unexpected empty children for a non-terminal: {node}")]
    UnexpectedEmptyChildren {
        /// The index of the node.
        node: CstNodeId,
    },
    /// The node ID not found in the tree
    #[error("Node ID not found in the tree: {node}")]
    NodeIdNotFound {
        /// The index of the node.
        node: CstNodeId,
    },
    /// Error that occurs when constructing a view from a [NonTerminalHandle].
    #[error(transparent)]
    Error(#[from] E),
}

impl<T, Nt, E> EureViewConstructionError<T, Nt, E> {
    pub fn extract_error(self) -> Result<E, EureViewConstructionError<T, Nt, Infallible>> {
        match self {
            EureViewConstructionError::Error(e) => Ok(e),
            EureViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            } => Err(EureViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            }),
            EureViewConstructionError::UnexpectedExtraNode { node } => {
                Err(EureViewConstructionError::UnexpectedExtraNode { node })
            }
            EureViewConstructionError::UnexpectedEndOfChildren { parent } => {
                Err(EureViewConstructionError::UnexpectedEndOfChildren { parent })
            }
            EureViewConstructionError::UnexpectedEmptyChildren { node } => {
                Err(EureViewConstructionError::UnexpectedEmptyChildren { node })
            }
            EureViewConstructionError::NodeIdNotFound { node } => {
                Err(EureViewConstructionError::NodeIdNotFound { node })
            }
        }
    }
}

impl<T, Nt> EureViewConstructionError<T, Nt, Infallible> {
    pub fn into_any_error<E>(self) -> EureViewConstructionError<T, Nt, E> {
        match self {
            EureViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            } => EureViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            },
            EureViewConstructionError::UnexpectedExtraNode { node } => {
                EureViewConstructionError::UnexpectedExtraNode { node }
            }
            EureViewConstructionError::UnexpectedEndOfChildren { parent } => {
                EureViewConstructionError::UnexpectedEndOfChildren { parent }
            }
            EureViewConstructionError::UnexpectedEmptyChildren { node } => {
                EureViewConstructionError::UnexpectedEmptyChildren { node }
            }
            EureViewConstructionError::NodeIdNotFound { node } => {
                EureViewConstructionError::NodeIdNotFound { node }
            }
        }
    }
}

impl<T, Nt> EureViewConstructionError<T, Nt, Infallible>
where
    T: Copy,
    Nt: Copy,
{
    pub fn unexpected_node(&self) -> Option<UnexpectedNode<T, Nt>> {
        match self {
            EureViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            } => Some(UnexpectedNode {
                node: *node,
                data: *data,
                expected_kind: *expected_kind,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnexpectedNode<T, Nt> {
    node: CstNodeId,
    data: CstNodeData<T, Nt>,
    expected_kind: NodeKind<T, Nt>,
}

struct DummyTerminalVisitor;

impl<F: CstFacade> CstVisitor<F> for DummyTerminalVisitor {
    type Error = Infallible;
    fn visit_new_line_terminal(
        &mut self,
        _terminal: crate::nodes::NewLine,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_whitespace_terminal(
        &mut self,
        _terminal: crate::nodes::Whitespace,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_line_comment_terminal(
        &mut self,
        _terminal: crate::nodes::LineComment,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_block_comment_terminal(
        &mut self,
        _terminal: crate::nodes::BlockComment,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub trait TerminalHandle {
    /// Node ID of the terminal.
    fn node_id(&self) -> CstNodeId;
    /// Kind of the terminal.
    fn kind(&self) -> TerminalKind;
    /// Data of the terminal.
    fn get_data<F: CstFacade>(&self, tree: &F) -> Result<TerminalData, CstConstructError> {
        tree.get_terminal(self.node_id(), self.kind())
    }
}

/// A trait that all generated non-terminal handles implements.
pub trait NonTerminalHandle: Sized {
    /// The type of the view for this non-terminal.
    type View;

    /// Node ID of the non-terminal.
    fn node_id(&self) -> CstNodeId;

    /// Create a new non-terminal handle from a node.
    fn new<F: CstFacade>(index: CstNodeId, tree: &F) -> Result<Self, CstConstructError> {
        Self::new_with_visit(index, tree, &mut DummyTerminalVisitor)
    }

    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>>;

    /// Get the view of the non-terminal.
    fn get_view<C: CstFacade>(&self, tree: &C) -> Result<Self::View, CstConstructError> {
        self.get_view_with_visit(
            tree,
            |view, visit_ignored| (view, visit_ignored),
            &mut DummyTerminalVisitor,
        )
    }

    /// Get the view of the non-terminal.
    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>>;

    /// Get the kind of the non-terminal.
    fn kind(&self) -> NonTerminalKind;
}

/// A trait that generated recursive views implements.
pub trait RecursiveView<F: CstFacade> {
    /// The type of the item in the view.
    type Item;
    fn get_all(&self, tree: &F) -> Result<Vec<Self::Item>, CstConstructError> {
        self.get_all_with_visit(tree, &mut DummyTerminalVisitor)
    }

    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>>;
}
