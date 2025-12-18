//! Type-safe visitor pattern generator for Parol parsers - runtime library
//!
//! This crate provides the runtime traits and types needed for generated visitor code.
//! Use `parol-walker-gen` to generate visitor implementations from your Parol grammar.

use ahash::HashMap;
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

/// Unique identifier for a node in the CST
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct CstNodeId(pub usize);

impl fmt::Display for CstNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifier for a dynamically created token (not from input source)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DynamicTokenId(pub u32);

/// Data associated with a terminal node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalData {
    /// Terminal from input source
    Input(InputSpan),
    /// Dynamically created terminal (e.g., for tree transformations)
    Dynamic(DynamicTokenId),
}

/// Data associated with a non-terminal node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonTerminalData {
    /// Non-terminal from input source
    Input(InputSpan),
    /// Dynamically created non-terminal
    Dynamic,
}

/// Represents a span in the input source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputSpan {
    pub start: u32,
    pub end: u32,
}

impl InputSpan {
    /// Empty span, useful as initial value for merging
    pub const EMPTY: Self = Self {
        start: u32::MAX,
        end: 0,
    };

    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn as_str<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start as usize..self.end as usize]
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn merge_many(self, others: impl IntoIterator<Item = Self>) -> Self {
        others.into_iter().fold(self, |acc, other| acc.merge(other))
    }
}

/// Errors that can occur during view construction
///
/// Generic over `T` (terminal kind) and `Nt` (non-terminal kind) to carry
/// rich error context including the actual node data and expected kind.
#[derive(Debug, Clone, Error)]
pub enum ViewConstructionError<T = (), Nt = ()> {
    #[error("Node ID not found: {node}")]
    NodeIdNotFound { node: CstNodeId },

    #[error("Unexpected end of children for parent: {parent}")]
    UnexpectedEndOfChildren { parent: CstNodeId },

    #[error("Unexpected extra node: {node}")]
    UnexpectedExtraNode { node: CstNodeId },

    #[error("Unexpected node for expected kind: {expected_kind:?} but got {data:?}")]
    UnexpectedNode {
        /// The index of the node.
        node: CstNodeId,
        /// The data of the node.
        data: CstNodeData<T, Nt>,
        /// The expected kind.
        expected_kind: NodeKind<T, Nt>,
    },

    /// Simpler variant when we don't have the expected kind information
    /// (e.g., when checking alternatives where any of several kinds would be valid)
    #[error("Unexpected node: {node}, got {data:?}")]
    UnexpectedNodeData {
        /// The index of the node.
        node: CstNodeId,
        /// The data of the node.
        data: CstNodeData<T, Nt>,
    },

    #[error("Unexpected empty children for a non-terminal: {node}")]
    UnexpectedEmptyChildren { node: CstNodeId },
}

impl<T, Nt> ViewConstructionError<T, Nt> {
    /// Convert to a simpler error type that only carries node IDs
    pub fn into_simple(self) -> ViewConstructionError<(), ()> {
        match self {
            ViewConstructionError::NodeIdNotFound { node } => {
                ViewConstructionError::NodeIdNotFound { node }
            }
            ViewConstructionError::UnexpectedEndOfChildren { parent } => {
                ViewConstructionError::UnexpectedEndOfChildren { parent }
            }
            ViewConstructionError::UnexpectedExtraNode { node } => {
                ViewConstructionError::UnexpectedExtraNode { node }
            }
            ViewConstructionError::UnexpectedNode { node, .. } => {
                ViewConstructionError::UnexpectedNode {
                    node,
                    data: CstNodeData::Terminal {
                        kind: (),
                        data: TerminalData::Dynamic(DynamicTokenId(0)),
                    },
                    expected_kind: NodeKind::Terminal(()),
                }
            }
            ViewConstructionError::UnexpectedNodeData { node, .. } => {
                ViewConstructionError::UnexpectedNodeData {
                    node,
                    data: CstNodeData::Terminal {
                        kind: (),
                        data: TerminalData::Dynamic(DynamicTokenId(0)),
                    },
                }
            }
            ViewConstructionError::UnexpectedEmptyChildren { node } => {
                ViewConstructionError::UnexpectedEmptyChildren { node }
            }
        }
    }

    /// Get the node ID associated with this error
    pub fn node_id(&self) -> CstNodeId {
        match self {
            ViewConstructionError::NodeIdNotFound { node } => *node,
            ViewConstructionError::UnexpectedEndOfChildren { parent } => *parent,
            ViewConstructionError::UnexpectedExtraNode { node } => *node,
            ViewConstructionError::UnexpectedNode { node, .. } => *node,
            ViewConstructionError::UnexpectedNodeData { node, .. } => *node,
            ViewConstructionError::UnexpectedEmptyChildren { node } => *node,
        }
    }
}

/// Errors that can occur during CST construction
///
/// Generic over:
/// - `T`: terminal kind
/// - `Nt`: non-terminal kind
/// - `E`: visitor error type
#[derive(Debug, Error)]
pub enum CstConstructError<T = (), Nt = (), E = std::convert::Infallible> {
    #[error("View construction error: {0}")]
    ViewConstruction(ViewConstructionError<T, Nt>),

    #[error("Visitor error: {0}")]
    Visitor(E),
}

impl<T, Nt, E> From<ViewConstructionError<T, Nt>> for CstConstructError<T, Nt, E> {
    fn from(e: ViewConstructionError<T, Nt>) -> Self {
        CstConstructError::ViewConstruction(e)
    }
}

impl<T, Nt, E> CstConstructError<T, Nt, E> {
    pub fn extract_error(self) -> Result<ViewConstructionError<T, Nt>, E> {
        match self {
            CstConstructError::ViewConstruction(e) => Ok(e),
            CstConstructError::Visitor(e) => Err(e),
        }
    }
}

/// Trait for accessing CST structure (implemented by generated code)
///
/// The generic parameters `T` and `Nt` represent terminal and non-terminal kinds respectively.
pub trait CstFacade<T, Nt>: Sized
where
    T: Copy,
    Nt: Copy,
{
    /// Get the string slice for a terminal node
    fn get_str<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        terminal: TerminalData,
        input: &'b str,
    ) -> Option<&'c str>;

    /// Get node data for a given node ID
    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData<T, Nt>>;

    /// Check if a node has no children
    fn has_no_children(&self, node: CstNodeId) -> bool;

    /// Get an iterator over a node's children
    fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId>;

    /// Get data for a terminal node of specific kind
    fn get_terminal(
        &self,
        node: CstNodeId,
        kind: T,
    ) -> Result<TerminalData, ViewConstructionError<T, Nt>>;

    /// Get data for a non-terminal node of specific kind
    fn get_non_terminal(
        &self,
        node: CstNodeId,
        kind: Nt,
    ) -> Result<NonTerminalData, ViewConstructionError<T, Nt>>;

    /// Get parent of a node
    fn parent(&self, node: CstNodeId) -> Option<CstNodeId>;

    /// Collect child nodes matching expected kinds, skipping builtin terminals
    fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<T, Nt, E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        expected_kinds: [NodeKind<T, Nt>; N],
        visitor: impl FnMut(
            [CstNodeId; N],
            &'v mut V,
        ) -> Result<(O, &'v mut V), CstConstructError<T, Nt, E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<T, Nt, E>>
    where
        T: BuiltinTerminalKind;
}

/// Node data in the CST
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CstNodeData<T, Nt> {
    Terminal { kind: T, data: TerminalData },
    NonTerminal { kind: Nt, data: NonTerminalData },
}

impl<T, Nt> CstNodeData<T, Nt> {
    /// Create a new terminal node
    pub fn new_terminal(kind: T, data: TerminalData) -> Self {
        Self::Terminal { kind, data }
    }

    /// Create a new non-terminal node
    pub fn new_non_terminal(kind: Nt, data: NonTerminalData) -> Self {
        Self::NonTerminal { kind, data }
    }

    /// Check if this is a terminal node
    pub fn is_terminal(&self) -> bool {
        matches!(self, CstNodeData::Terminal { .. })
    }

    /// Check if this is a non-terminal node
    pub fn is_non_terminal(&self) -> bool {
        matches!(self, CstNodeData::NonTerminal { .. })
    }

    /// Get the node kind (terminal or non-terminal)
    pub fn node_kind(&self) -> NodeKind<T, Nt>
    where
        T: Copy,
        Nt: Copy,
    {
        match self {
            CstNodeData::Terminal { kind, .. } => NodeKind::Terminal(*kind),
            CstNodeData::NonTerminal { kind, .. } => NodeKind::NonTerminal(*kind),
        }
    }
}

/// Generic node kind (either Terminal or NonTerminal)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind<T, Nt> {
    Terminal(T),
    NonTerminal(Nt),
}

/// Trait for terminal kinds that can identify builtin terminals (whitespace, comments, etc.)
///
/// This is implemented by generated TerminalKind enums.
pub trait BuiltinTerminalKind: Copy + PartialEq {
    fn is_builtin_whitespace(&self) -> bool;
    fn is_builtin_new_line(&self) -> bool;
    fn is_builtin_line_comment(&self) -> bool;
    fn is_builtin_block_comment(&self) -> bool;

    fn is_builtin_terminal(&self) -> bool {
        self.is_builtin_whitespace()
            || self.is_builtin_new_line()
            || self.is_builtin_line_comment()
            || self.is_builtin_block_comment()
    }
}

/// Trait for terminal node handles (implemented by generated code)
pub trait TerminalHandle<T: Copy> {
    /// Get the node ID
    fn node_id(&self) -> CstNodeId;

    /// Get the terminal kind
    fn kind(&self) -> T;

    /// Get the terminal data from the tree
    fn get_data<F: CstFacade<T, Nt>, Nt: Copy>(
        &self,
        tree: &F,
    ) -> Result<TerminalData, ViewConstructionError<T, Nt>> {
        tree.get_terminal(self.node_id(), self.kind())
    }
}

/// Trait for non-terminal node handles (implemented by generated code)
pub trait NonTerminalHandle<T: Copy, Nt: Copy>: Sized {
    /// The view type for this non-terminal
    type View;

    /// Get the node ID
    fn node_id(&self) -> CstNodeId;

    /// Get the non-terminal kind
    fn kind(&self) -> Nt;

    /// Construct a handle from a node ID with visitor for ignored nodes
    fn new_with_visit<F: CstFacade<T, Nt>, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<T, Nt, E, F>,
    ) -> Result<Self, CstConstructError<T, Nt, E>>;

    /// Get the view and call visitor on it
    fn get_view_with_visit<'v, F: CstFacade<T, Nt>, V: BuiltinTerminalVisitor<T, Nt, E, F>, O, E>(
        &self,
        tree: &F,
        visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<T, Nt, E>>;

    /// Get the view without visiting ignored nodes
    fn get_view<F: CstFacade<T, Nt>>(
        &self,
        tree: &F,
    ) -> Result<Self::View, ViewConstructionError<T, Nt>>
    where
        T: BuiltinTerminalKind,
    {
        struct NoOpVisitor;
        impl<T: Copy, Nt: Copy, F: CstFacade<T, Nt>>
            BuiltinTerminalVisitor<T, Nt, std::convert::Infallible, F> for NoOpVisitor
        {
            fn visit_builtin_new_line_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
            fn visit_builtin_whitespace_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
            fn visit_builtin_line_comment_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
            fn visit_builtin_block_comment_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
        }

        let mut visitor = NoOpVisitor;
        match self.get_view_with_visit(tree, |view, v| (view, v), &mut visitor) {
            Ok(view) => Ok(view),
            Err(CstConstructError::ViewConstruction(e)) => Err(e),
            Err(CstConstructError::Visitor(e)) => match e {},
        }
    }
}

/// Trait for visiting builtin terminals (whitespace, comments, etc.)
///
/// This is implemented by the main CstVisitor trait to delegate to specific visit methods.
pub trait BuiltinTerminalVisitor<T: Copy, Nt: Copy, E, F: CstFacade<T, Nt>> {
    fn visit_builtin_new_line_terminal(
        &mut self,
        terminal: CstNodeId,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;

    fn visit_builtin_whitespace_terminal(
        &mut self,
        terminal: CstNodeId,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;

    fn visit_builtin_line_comment_terminal(
        &mut self,
        terminal: CstNodeId,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;

    fn visit_builtin_block_comment_terminal(
        &mut self,
        terminal: CstNodeId,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), E>;
}

/// Trait for views of recursive non-terminals (lists)
pub trait RecursiveView<T: Copy, Nt: Copy, F: CstFacade<T, Nt>>: Copy {
    type Item;

    /// Get all items from this recursive view
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<T, Nt, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<T, Nt, E>>;

    /// Get all items from this recursive view, ignoring trivia
    fn get_all(&self, tree: &F) -> Result<Vec<Self::Item>, ViewConstructionError<T, Nt>>
    where
        T: BuiltinTerminalKind,
    {
        struct NoOpVisitor;
        impl<T: Copy, Nt: Copy, F: CstFacade<T, Nt>>
            BuiltinTerminalVisitor<T, Nt, std::convert::Infallible, F> for NoOpVisitor
        {
            fn visit_builtin_new_line_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
            fn visit_builtin_whitespace_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
            fn visit_builtin_line_comment_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
            fn visit_builtin_block_comment_terminal(
                &mut self,
                _terminal: CstNodeId,
                _data: TerminalData,
                _tree: &F,
            ) -> Result<(), std::convert::Infallible> {
                Ok(())
            }
        }

        let mut visitor = NoOpVisitor;
        match self.get_all_with_visit(tree, &mut visitor) {
            Ok(items) => Ok(items),
            Err(CstConstructError::ViewConstruction(e)) => Err(e),
            Err(CstConstructError::Visitor(e)) => match e {},
        }
    }
}

/// Generic concrete syntax tree implementation
#[derive(Debug, Clone)]
pub struct ConcreteSyntaxTree<T, Nt> {
    nodes: Vec<CstNodeData<T, Nt>>,
    children: HashMap<CstNodeId, Vec<CstNodeId>>,
    parents: HashMap<CstNodeId, CstNodeId>,
    dynamic_tokens: BTreeMap<DynamicTokenId, String>,
    next_dynamic_token_id: u32,
    root: CstNodeId,
}

impl<T: Copy + PartialEq, Nt: Copy + PartialEq> ConcreteSyntaxTree<T, Nt> {
    /// Create a new CST with a root node
    pub fn new(root_data: CstNodeData<T, Nt>) -> Self {
        let nodes = vec![root_data];
        let root = CstNodeId(0);
        Self {
            nodes,
            children: HashMap::default(),
            parents: HashMap::default(),
            dynamic_tokens: BTreeMap::new(),
            next_dynamic_token_id: 0,
            root,
        }
    }

    /// Insert a dynamic token and return its ID
    pub fn insert_dynamic_terminal(&mut self, data: impl Into<String>) -> DynamicTokenId {
        let id = DynamicTokenId(self.next_dynamic_token_id);
        self.dynamic_tokens.insert(id, data.into());
        self.next_dynamic_token_id += 1;
        id
    }

    /// Get the content of a dynamic token
    pub fn dynamic_token(&self, id: DynamicTokenId) -> Option<&str> {
        self.dynamic_tokens.get(&id).map(|s| s.as_str())
    }

    /// Get the root node ID
    pub fn root(&self) -> CstNodeId {
        self.root
    }

    /// Set the root node ID
    pub fn set_root(&mut self, new_root: CstNodeId) {
        self.root = new_root;
    }

    /// Add a new node and return its ID
    pub fn add_node(&mut self, data: CstNodeData<T, Nt>) -> CstNodeId {
        let id = CstNodeId(self.nodes.len());
        self.nodes.push(data);
        id
    }

    /// Add a child to a parent node
    pub fn add_child(&mut self, parent: CstNodeId, child: CstNodeId) {
        self.children.entry(parent).or_default().push(child);
        self.parents.insert(child, parent);
    }

    /// Update a node's data
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

    /// Update a node's children
    pub fn update_children(
        &mut self,
        id: CstNodeId,
        new_children: impl IntoIterator<Item = CstNodeId>,
    ) {
        // Remove old parent references
        if let Some(old_children) = self.children.get(&id) {
            for child in old_children {
                self.parents.remove(child);
            }
        }

        let new_children: Vec<_> = new_children.into_iter().collect();

        // Add new parent references
        for &child in &new_children {
            self.parents.insert(child, id);
        }

        if new_children.is_empty() {
            self.children.remove(&id);
        } else {
            self.children.insert(id, new_children);
        }
    }
}

impl<T: Copy + PartialEq, Nt: Copy + PartialEq> CstFacade<T, Nt> for ConcreteSyntaxTree<T, Nt> {
    fn get_str<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        terminal: TerminalData,
        input: &'b str,
    ) -> Option<&'c str> {
        match terminal {
            TerminalData::Input(span) => Some(span.as_str(input)),
            TerminalData::Dynamic(id) => self.dynamic_token(id),
        }
    }

    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData<T, Nt>> {
        self.nodes.get(node.0).copied()
    }

    fn has_no_children(&self, node: CstNodeId) -> bool {
        self.children
            .get(&node)
            .is_none_or(|children| children.is_empty())
    }

    fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId> {
        self.children
            .get(&node)
            .into_iter()
            .flat_map(|children| children.iter().copied())
    }

    fn get_terminal(
        &self,
        node: CstNodeId,
        kind: T,
    ) -> Result<TerminalData, ViewConstructionError<T, Nt>> {
        let node_data = self
            .node_data(node)
            .ok_or(ViewConstructionError::NodeIdNotFound { node })?;
        match node_data {
            CstNodeData::Terminal { kind: k, data } if k == kind => Ok(data),
            _ => Err(ViewConstructionError::UnexpectedNode {
                node,
                data: node_data,
                expected_kind: NodeKind::Terminal(kind),
            }),
        }
    }

    fn get_non_terminal(
        &self,
        node: CstNodeId,
        kind: Nt,
    ) -> Result<NonTerminalData, ViewConstructionError<T, Nt>> {
        let node_data = self
            .node_data(node)
            .ok_or(ViewConstructionError::NodeIdNotFound { node })?;
        match node_data {
            CstNodeData::NonTerminal { kind: k, data } if k == kind => Ok(data),
            _ => Err(ViewConstructionError::UnexpectedNode {
                node,
                data: node_data,
                expected_kind: NodeKind::NonTerminal(kind),
            }),
        }
    }

    fn parent(&self, node: CstNodeId) -> Option<CstNodeId> {
        self.parents.get(&node).copied()
    }

    fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<T, Nt, E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        expected_kinds: [NodeKind<T, Nt>; N],
        visitor: impl FnMut(
            [CstNodeId; N],
            &'v mut V,
        ) -> Result<(O, &'v mut V), CstConstructError<T, Nt, E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<T, Nt, E>>
    where
        T: BuiltinTerminalKind,
    {
        // Delegate to the inherent impl
        ConcreteSyntaxTree::collect_nodes_impl(self, parent, expected_kinds, visitor, visit_ignored)
    }
}

impl<T: BuiltinTerminalKind, Nt: Copy + PartialEq> ConcreteSyntaxTree<T, Nt> {
    /// Internal implementation of collect_nodes
    /// Collect child nodes matching expected kinds, skipping builtin terminals
    ///
    /// This method iterates through children of `parent`, collecting nodes that match
    /// the `expected_kinds` array while skipping builtin terminals (whitespace, comments).
    /// Builtin terminals are passed to the visitor via `visit_ignored`.
    fn collect_nodes_impl<'v, const N: usize, V: BuiltinTerminalVisitor<T, Nt, E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        expected_kinds: [NodeKind<T, Nt>; N],
        mut visitor: impl FnMut(
            [CstNodeId; N],
            &'v mut V,
        ) -> Result<(O, &'v mut V), CstConstructError<T, Nt, E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<T, Nt, E>> {
        let children_vec: Vec<_> = self.children(parent).collect();
        let mut children = children_vec.into_iter();
        let mut result = Vec::with_capacity(N);
        let mut ignored = Vec::with_capacity(N);

        'outer: for expected_kind in expected_kinds {
            'inner: for child in children.by_ref() {
                let child_data = self
                    .node_data(child)
                    .ok_or(ViewConstructionError::NodeIdNotFound { node: child })?;
                match child_data {
                    CstNodeData::Terminal { kind, data } => {
                        if NodeKind::Terminal(kind) == expected_kind {
                            result.push(child);
                            continue 'outer;
                        } else if kind.is_builtin_terminal() {
                            ignored.push((child, kind, data));
                            continue 'inner;
                        } else {
                            return Err(ViewConstructionError::UnexpectedNode {
                                node: child,
                                data: child_data,
                                expected_kind,
                            }
                            .into());
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
                            }
                            .into());
                        }
                    }
                }
            }
            return Err(ViewConstructionError::UnexpectedEndOfChildren { parent }.into());
        }

        // Visit ignored trivia nodes
        for (child, kind, data) in ignored {
            if kind.is_builtin_whitespace() {
                visit_ignored
                    .visit_builtin_whitespace_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?;
            } else if kind.is_builtin_new_line() {
                visit_ignored
                    .visit_builtin_new_line_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?;
            } else if kind.is_builtin_line_comment() {
                visit_ignored
                    .visit_builtin_line_comment_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?;
            } else if kind.is_builtin_block_comment() {
                visit_ignored
                    .visit_builtin_block_comment_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?;
            }
        }

        let (result, visit_ignored) = visitor(
            result
                .try_into()
                .expect("Result should have the same length as expected_kinds"),
            visit_ignored,
        )?;

        // Check for any remaining non-trivia children
        for child in children {
            let child_data = self
                .node_data(child)
                .ok_or(ViewConstructionError::NodeIdNotFound { node: child })?;
            match child_data {
                CstNodeData::Terminal { kind, data } => {
                    if kind.is_builtin_terminal() {
                        if kind.is_builtin_whitespace() {
                            visit_ignored
                                .visit_builtin_whitespace_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?;
                        } else if kind.is_builtin_new_line() {
                            visit_ignored
                                .visit_builtin_new_line_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?;
                        } else if kind.is_builtin_line_comment() {
                            visit_ignored
                                .visit_builtin_line_comment_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?;
                        } else if kind.is_builtin_block_comment() {
                            visit_ignored
                                .visit_builtin_block_comment_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?;
                        }
                    } else {
                        return Err(
                            ViewConstructionError::UnexpectedExtraNode { node: child }.into()
                        );
                    }
                }
                CstNodeData::NonTerminal { .. } => {
                    return Err(ViewConstructionError::UnexpectedExtraNode { node: child }.into());
                }
            }
        }

        Ok(result)
    }
}

/// Builder for constructing a CST during parsing
///
/// Implements `TreeConstruct` from parol_runtime for integration with parol parsers.
#[derive(Debug, Clone)]
pub struct CstBuilder<T, Nt, F>
where
    F: Fn(&'static str) -> Nt,
{
    tree: ConcreteSyntaxTree<T, Nt>,
    node_stack: Vec<NodeStackItem>,
    root_node: Option<CstNodeId>,
    terminal_from_index: fn(u16) -> T,
    non_terminal_from_name: F,
}

#[derive(Debug, Clone)]
struct NodeStackItem {
    node: CstNodeId,
    span: InputSpan,
    children: Vec<CstNodeId>,
}

impl<T, Nt, F> CstBuilder<T, Nt, F>
where
    T: Copy + PartialEq,
    Nt: Copy + PartialEq,
    F: Fn(&'static str) -> Nt,
{
    /// Create a new CST builder
    ///
    /// # Arguments
    /// * `terminal_from_index` - Function to convert parol terminal index to terminal kind
    /// * `non_terminal_from_name` - Function to convert parol non-terminal name to non-terminal kind
    /// * `root_non_terminal` - The non-terminal kind to use for the root node
    pub fn new(
        terminal_from_index: fn(u16) -> T,
        non_terminal_from_name: F,
        root_non_terminal: Nt,
    ) -> Self {
        let temp_root_data = CstNodeData::NonTerminal {
            kind: root_non_terminal,
            data: NonTerminalData::Dynamic,
        };
        Self {
            tree: ConcreteSyntaxTree::new(temp_root_data),
            node_stack: Vec::new(),
            root_node: None,
            terminal_from_index,
            non_terminal_from_name,
        }
    }

    fn add_terminal_node(&mut self, kind: T, span: InputSpan) -> CstNodeId {
        let node = self.tree.add_node(CstNodeData::Terminal {
            kind,
            data: TerminalData::Input(span),
        });

        let parent = self.node_stack.last_mut().expect("node stack is empty");
        parent.children.push(node);
        parent.span = parent.span.merge(span);

        node
    }

    fn open_non_terminal_node(&mut self, kind: Nt) -> CstNodeId {
        let node = self.tree.add_node(CstNodeData::NonTerminal {
            kind,
            data: NonTerminalData::Dynamic,
        });

        if let Some(parent) = self.node_stack.last_mut() {
            parent.children.push(node);
        } else {
            self.root_node = Some(node);
        }

        self.node_stack.push(NodeStackItem {
            node,
            span: InputSpan::EMPTY,
            children: Vec::new(),
        });
        node
    }

    fn close_non_terminal_node(&mut self) -> Option<CstNodeId> {
        let popped = self.node_stack.pop();
        if let Some(item) = &popped {
            let parent = item.node;
            self.tree.update_children(parent, item.children.clone());

            if let Some(CstNodeData::NonTerminal { kind, .. }) = self.tree.node_data(parent) {
                let data = if item.span == InputSpan::EMPTY {
                    NonTerminalData::Dynamic
                } else {
                    NonTerminalData::Input(item.span)
                };
                let updated_data = CstNodeData::NonTerminal { kind, data };
                self.tree.update_node(parent, updated_data);
            }

            if let Some(parent_item) = self.node_stack.last_mut() {
                parent_item.span = parent_item.span.merge(item.span);
            }
        }
        popped.map(|item| item.node)
    }

    /// Build the final CST
    pub fn build_tree(mut self) -> ConcreteSyntaxTree<T, Nt> {
        while !self.node_stack.is_empty() {
            self.close_non_terminal_node();
        }

        if let Some(root_node) = self.root_node {
            self.tree.set_root(root_node);
        }

        self.tree
    }
}

impl<'t, T, Nt, F> parol_runtime::parser::parse_tree_type::TreeConstruct<'t>
    for CstBuilder<T, Nt, F>
where
    T: Copy + PartialEq,
    Nt: Copy + PartialEq,
    F: Fn(&'static str) -> Nt,
{
    type Error = parol_runtime::ParolError;
    type Tree = ConcreteSyntaxTree<T, Nt>;

    fn open_non_terminal(
        &mut self,
        name: &'static str,
        _size_hint: Option<usize>,
    ) -> Result<(), Self::Error> {
        let kind = (self.non_terminal_from_name)(name);
        self.open_non_terminal_node(kind);
        Ok(())
    }

    fn close_non_terminal(&mut self) -> Result<(), Self::Error> {
        self.close_non_terminal_node();
        Ok(())
    }

    fn add_token(&mut self, token: &parol_runtime::Token<'t>) -> Result<(), Self::Error> {
        let kind = (self.terminal_from_index)(token.token_type);
        let span = InputSpan::new(token.location.start, token.location.end);
        self.add_terminal_node(kind, span);
        Ok(())
    }

    fn build(self) -> Result<Self::Tree, Self::Error> {
        Ok(self.build_tree())
    }
}
