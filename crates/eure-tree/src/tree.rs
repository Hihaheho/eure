mod span;

use ahash::HashMap;
use std::{collections::BTreeMap, convert::Infallible};
use thiserror::Error;

pub use span::*;

use crate::{
    CstConstructError,
    common_visitors::{FormatVisitor, FormatVisitorError, InspectVisitor},
    node_kind::{NodeKind, NonTerminalKind, TerminalKind},
    nodes::{BlockComment, LineComment, NewLine, RootHandle, Whitespace},
    visitor::{BuiltinTerminalVisitor, CstVisitor, CstVisitorSuper as _},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A dynamic token id that provided by the user land.
pub struct DynamicTokenId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CstNodeData<T, Nt> {
    /// A terminal node with its kind and span
    Terminal { kind: T, data: TerminalData },
    /// A non-terminal node with its kind
    NonTerminal { kind: Nt, data: NonTerminalData },
}

impl<T, Nt> CstNodeData<T, Nt> {
    pub fn new_terminal(kind: T, data: TerminalData) -> Self {
        Self::Terminal { kind, data }
    }

    pub fn new_non_terminal(kind: Nt, data: NonTerminalData) -> Self {
        Self::NonTerminal { kind, data }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, CstNodeData::Terminal { .. })
    }

    pub fn is_non_terminal(&self) -> bool {
        matches!(self, CstNodeData::NonTerminal { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalData {
    Input(InputSpan),
    Dynamic(DynamicTokenId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonTerminalData {
    Input(InputSpan),
    Dynamic,
}

impl<T, Nt> CstNodeData<T, Nt>
where
    T: Copy,
    Nt: Copy,
{
    pub fn node_kind(&self) -> NodeKind<T, Nt> {
        match self {
            CstNodeData::Terminal { kind, .. } => NodeKind::Terminal(*kind),
            CstNodeData::NonTerminal { kind, .. } => NodeKind::NonTerminal(*kind),
        }
    }
}

impl<T, Nt> CstNodeData<T, Nt>
where
    T: PartialEq + Copy,
    Nt: PartialEq + Copy,
{
    pub fn expected_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: T,
    ) -> Result<(T, TerminalData), ViewConstructionError<T, Nt>> {
        match self {
            CstNodeData::Terminal { kind, data } if *kind == expected => Ok((*kind, *data)),
            _ => Err(ViewConstructionError::UnexpectedNode {
                node,
                data: *self,
                expected_kind: NodeKind::Terminal(expected),
            }),
        }
    }

    pub fn expected_non_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: Nt,
    ) -> Result<(Nt, NonTerminalData), ViewConstructionError<T, Nt>> {
        match self {
            CstNodeData::NonTerminal { kind, data } if *kind == expected => Ok((*kind, *data)),
            _ => Err(ViewConstructionError::UnexpectedNode {
                node,
                data: *self,
                expected_kind: NodeKind::NonTerminal(expected),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct CstNodeId(pub usize);

impl std::fmt::Display for CstNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

    pub fn children(&self, node: CstNodeId) -> impl Iterator<Item = CstNodeId> + '_ {
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
            TerminalKind::Ws
                | TerminalKind::GrammarNewline
                | TerminalKind::Text
                | TerminalKind::Code
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
            .ok_or(ViewConstructionError::NodeIdNotFound { node: id })?;
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
            .ok_or(ViewConstructionError::NodeIdNotFound { node: id })?;
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
                    .ok_or(ViewConstructionError::NodeIdNotFound { node: child })?;
                match child_data {
                    CstNodeData::Terminal { kind, data } => {
                        if NodeKind::Terminal(kind) == expected_kind {
                            result.push(child);
                            continue 'outer;
                        } else if kind.is_builtin_whitespace() || kind.is_builtin_new_line() {
                            if kind.auto_ws_is_off(idx) {
                                return Err(ViewConstructionError::UnexpectedNode {
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
                            return Err(ViewConstructionError::UnexpectedNode {
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
                            return Err(ViewConstructionError::UnexpectedNode {
                                node: child,
                                data: child_data,
                                expected_kind,
                            });
                        }
                    }
                }
            }
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent });
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
                .ok_or(ViewConstructionError::NodeIdNotFound { node: child })?;
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
                        return Err(ViewConstructionError::UnexpectedNode {
                            node: child,
                            data: child_data,
                            expected_kind: NodeKind::Terminal(kind),
                        });
                    }
                }
                CstNodeData::NonTerminal { kind, .. } => {
                    return Err(ViewConstructionError::UnexpectedNode {
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

    pub fn write(
        &self,
        input: &str,
        w: &mut impl std::fmt::Write,
    ) -> Result<(), FormatVisitorError> {
        let mut visitor = FormatVisitor::new(input, w);
        visitor.visit_root_handle(self.root_handle(), self)?;
        Ok(())
    }

    pub fn inspect(
        &self,
        input: &str,
        w: &mut impl std::fmt::Write,
    ) -> Result<(), FormatVisitorError> {
        let mut visitor = InspectVisitor::new(input, w);
        visitor.visit_root_handle(self.root_handle(), self)?;
        Ok(())
    }

    pub fn visit_from_root<V: CstVisitor<Self>>(&self, visitor: &mut V) -> Result<(), V::Error> {
        visitor.visit_root_handle(self.root_handle(), self)
    }
}

pub trait CstFacade: Sized {
    fn get_str<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        terminal: TerminalData,
        input: &'b str,
    ) -> Option<&'c str>;

    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData<TerminalKind, NonTerminalKind>>;

    fn has_no_children(&self, node: CstNodeId) -> bool;

    fn children(&self, node: CstNodeId) -> impl Iterator<Item = CstNodeId>;

    fn get_terminal(
        &self,
        node: CstNodeId,
        kind: TerminalKind,
    ) -> Result<TerminalData, CstConstructError>;

    fn get_non_terminal(
        &self,
        node: CstNodeId,
        kind: NonTerminalKind,
    ) -> Result<NonTerminalData, CstConstructError>;

    fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        nodes: [NodeKind<TerminalKind, NonTerminalKind>; N],
        visitor: impl FnMut([CstNodeId; N], &'v mut V) -> Result<(O, &'v mut V), CstConstructError<E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>>;

    fn dynamic_token(&self, id: DynamicTokenId) -> Option<&str>;

    fn parent(&self, node: CstNodeId) -> Option<CstNodeId>;

    fn root_handle(&self) -> RootHandle;
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

    fn children(&self, node: CstNodeId) -> impl Iterator<Item = CstNodeId> {
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
pub enum ViewConstructionError<T, Nt, E = Infallible> {
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

impl<T, Nt, E> ViewConstructionError<T, Nt, E> {
    pub fn extract_error(self) -> Result<E, ViewConstructionError<T, Nt, Infallible>> {
        match self {
            ViewConstructionError::Error(e) => Ok(e),
            ViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            } => Err(ViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            }),
            ViewConstructionError::UnexpectedExtraNode { node } => {
                Err(ViewConstructionError::UnexpectedExtraNode { node })
            }
            ViewConstructionError::UnexpectedEndOfChildren { parent } => {
                Err(ViewConstructionError::UnexpectedEndOfChildren { parent })
            }
            ViewConstructionError::UnexpectedEmptyChildren { node } => {
                Err(ViewConstructionError::UnexpectedEmptyChildren { node })
            }
            ViewConstructionError::NodeIdNotFound { node } => {
                Err(ViewConstructionError::NodeIdNotFound { node })
            }
        }
    }
}

impl<T, Nt> ViewConstructionError<T, Nt, Infallible> {
    pub fn into_any_error<E>(self) -> ViewConstructionError<T, Nt, E> {
        match self {
            ViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            } => ViewConstructionError::UnexpectedNode {
                node,
                data,
                expected_kind,
            },
            ViewConstructionError::UnexpectedExtraNode { node } => {
                ViewConstructionError::UnexpectedExtraNode { node }
            }
            ViewConstructionError::UnexpectedEndOfChildren { parent } => {
                ViewConstructionError::UnexpectedEndOfChildren { parent }
            }
            ViewConstructionError::UnexpectedEmptyChildren { node } => {
                ViewConstructionError::UnexpectedEmptyChildren { node }
            }
            ViewConstructionError::NodeIdNotFound { node } => {
                ViewConstructionError::NodeIdNotFound { node }
            }
        }
    }
}

impl<T, Nt> ViewConstructionError<T, Nt, Infallible>
where
    T: Copy,
    Nt: Copy,
{
    pub fn unexpected_node(&self) -> Option<UnexpectedNode<T, Nt>> {
        match self {
            ViewConstructionError::UnexpectedNode {
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
