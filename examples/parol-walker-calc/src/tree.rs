// Local tree types for the calculator example
// This mirrors eure-tree structure but works with locally generated TerminalKind/NonTerminalKind

use ahash::HashMap;
use std::convert::Infallible;
use thiserror::Error;

use crate::node_kind::{NonTerminalKind, TerminalKind};

// NodeKind enum - parol doesn't generate this so we define it here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    Terminal(TerminalKind),
    NonTerminal(NonTerminalKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputSpan {
    pub start: u32,
    pub end: u32,
}

impl InputSpan {
    pub const EMPTY: Self = Self { start: 0, end: 0 };

    pub fn as_str<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start as usize..self.end as usize]
    }

    pub fn merge(self, other: Self) -> Self {
        if self == Self::EMPTY {
            other
        } else if other == Self::EMPTY {
            self
        } else {
            Self {
                start: self.start.min(other.start),
                end: self.end.max(other.end),
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalData {
    Input(InputSpan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonTerminalData {
    Input(InputSpan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CstNodeData {
    Terminal { kind: TerminalKind, data: TerminalData },
    NonTerminal { kind: NonTerminalKind, data: NonTerminalData },
}

impl CstNodeData {
    pub fn new_terminal(kind: TerminalKind, data: TerminalData) -> Self {
        Self::Terminal { kind, data }
    }

    pub fn new_non_terminal(kind: NonTerminalKind, data: NonTerminalData) -> Self {
        Self::NonTerminal { kind, data }
    }

    pub fn node_kind(&self) -> NodeKind {
        match self {
            CstNodeData::Terminal { kind, .. } => NodeKind::Terminal(*kind),
            CstNodeData::NonTerminal { kind, .. } => NodeKind::NonTerminal(*kind),
        }
    }
}

// Type aliases for generated code
pub type CstNode = CstNodeData;
pub type Cst = ConcreteSyntaxTree;

#[derive(Debug, Clone, Error)]
pub enum ViewConstructionError<E = Infallible> {
    #[error("Unexpected node: expected {expected_kind:?} but got {data:?}")]
    UnexpectedNode {
        node: CstNodeId,
        data: CstNodeData,
        expected_kind: NodeKind,
    },
    #[error("Unexpected extra node")]
    UnexpectedExtraNode { node: CstNodeId },
    #[error("Unexpected end of children")]
    UnexpectedEndOfChildren { parent: CstNodeId },
    #[error("Node ID not found: {node}")]
    NodeIdNotFound { node: CstNodeId },
    #[error(transparent)]
    Error(#[from] E),
}

impl<E> ViewConstructionError<E> {
    pub fn extract_error(self) -> Result<E, ViewConstructionError<Infallible>> {
        match self {
            ViewConstructionError::Error(e) => Ok(e),
            ViewConstructionError::UnexpectedNode { node, data, expected_kind } => {
                Err(ViewConstructionError::UnexpectedNode { node, data, expected_kind })
            }
            ViewConstructionError::UnexpectedExtraNode { node } => {
                Err(ViewConstructionError::UnexpectedExtraNode { node })
            }
            ViewConstructionError::UnexpectedEndOfChildren { parent } => {
                Err(ViewConstructionError::UnexpectedEndOfChildren { parent })
            }
            ViewConstructionError::NodeIdNotFound { node } => {
                Err(ViewConstructionError::NodeIdNotFound { node })
            }
        }
    }
}

impl ViewConstructionError<Infallible> {
    pub fn into_any_error<E>(self) -> ViewConstructionError<E> {
        match self {
            ViewConstructionError::UnexpectedNode { node, data, expected_kind } => {
                ViewConstructionError::UnexpectedNode { node, data, expected_kind }
            }
            ViewConstructionError::UnexpectedExtraNode { node } => {
                ViewConstructionError::UnexpectedExtraNode { node }
            }
            ViewConstructionError::UnexpectedEndOfChildren { parent } => {
                ViewConstructionError::UnexpectedEndOfChildren { parent }
            }
            ViewConstructionError::NodeIdNotFound { node } => {
                ViewConstructionError::NodeIdNotFound { node }
            }
        }
    }
}

pub type CstConstructError<E = Infallible> = ViewConstructionError<E>;

// Re-export BuiltinTerminalVisitor from visitor module (generated there)
pub use crate::visitor::BuiltinTerminalVisitor;

#[derive(Debug, Clone)]
pub struct ConcreteSyntaxTree {
    nodes: Vec<CstNodeData>,
    children: HashMap<CstNodeId, Vec<CstNodeId>>,
    root: CstNodeId,
}

impl ConcreteSyntaxTree {
    pub fn new(root_data: CstNodeData) -> Self {
        let nodes = vec![root_data];
        let root = CstNodeId(0);
        Self {
            nodes,
            children: HashMap::default(),
            root,
        }
    }

    pub fn root(&self) -> CstNodeId {
        self.root
    }

    pub fn set_root(&mut self, new_root: CstNodeId) {
        self.root = new_root;
    }

    pub fn add_node(&mut self, data: CstNodeData) -> CstNodeId {
        let id = CstNodeId(self.nodes.len());
        self.nodes.push(data);
        id
    }

    pub fn add_child(&mut self, parent: CstNodeId, child: CstNodeId) {
        self.children.entry(parent).or_default().push(child);
    }

    pub fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId> + '_ {
        self.children
            .get(&node)
            .into_iter()
            .flat_map(|children| children.iter().copied())
    }

    pub fn has_no_children(&self, node: CstNodeId) -> bool {
        self.children
            .get(&node)
            .is_none_or(|children| children.is_empty())
    }

    pub fn node_data(&self, node: CstNodeId) -> Option<CstNodeData> {
        self.nodes.get(node.0).copied()
    }

    pub fn update_node(&mut self, id: CstNodeId, data: CstNodeData) -> Option<CstNodeData> {
        if id.0 < self.nodes.len() {
            Some(std::mem::replace(&mut self.nodes[id.0], data))
        } else {
            None
        }
    }

    pub fn update_children(&mut self, id: CstNodeId, new_children: impl IntoIterator<Item = CstNodeId>) {
        let new_children: Vec<_> = new_children.into_iter().collect();
        if new_children.is_empty() {
            self.children.remove(&id);
        } else {
            self.children.insert(id, new_children);
        }
    }

    pub fn get_terminal(&self, id: CstNodeId, kind: TerminalKind) -> Result<TerminalData, CstConstructError> {
        let node_data = self.node_data(id).ok_or(ViewConstructionError::NodeIdNotFound { node: id })?;
        match node_data {
            CstNodeData::Terminal { kind: k, data } if k == kind => Ok(data),
            _ => Err(ViewConstructionError::UnexpectedNode {
                node: id,
                data: node_data,
                expected_kind: NodeKind::Terminal(kind),
            }),
        }
    }

    pub fn get_non_terminal(&self, id: CstNodeId, kind: NonTerminalKind) -> Result<NonTerminalData, CstConstructError> {
        let node_data = self.node_data(id).ok_or(ViewConstructionError::NodeIdNotFound { node: id })?;
        match node_data {
            CstNodeData::NonTerminal { kind: k, data } if k == kind => Ok(data),
            _ => Err(ViewConstructionError::UnexpectedNode {
                node: id,
                data: node_data,
                expected_kind: NodeKind::NonTerminal(kind),
            }),
        }
    }

    pub fn root_handle(&self) -> crate::nodes::CalcHandle {
        crate::nodes::CalcHandle(self.root())
    }

    pub fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        nodes: [NodeKind; N],
        mut visitor: impl FnMut([CstNodeId; N], &'v mut V) -> Result<(O, &'v mut V), CstConstructError<E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        let children_vec = self.children(parent).collect::<Vec<_>>();
        let mut children = children_vec.into_iter();
        let mut result = Vec::with_capacity(N);
        let mut ignored = Vec::with_capacity(N);

        'outer: for expected_kind in nodes {
            'inner: for child in children.by_ref() {
                let child_data = self
                    .node_data(child)
                    .ok_or(ViewConstructionError::NodeIdNotFound { node: child })?;
                match child_data {
                    CstNodeData::Terminal { kind, data } => {
                        if NodeKind::Terminal(kind) == expected_kind {
                            result.push(child);
                            continue 'outer;
                        } else if kind.is_builtin_whitespace() || kind.is_builtin_new_line() {
                            ignored.push((child, kind, data));
                            continue 'inner;
                        } else if kind.is_builtin_line_comment() || kind.is_builtin_block_comment() {
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

        // Visit ignored trivia nodes
        for (child, kind, data) in ignored {
            match kind {
                TerminalKind::Whitespace => {
                    visit_ignored.visit_builtin_whitespace_terminal(crate::nodes::Whitespace(child), data, self)?;
                }
                TerminalKind::NewLine => {
                    visit_ignored.visit_builtin_new_line_terminal(crate::nodes::NewLine(child), data, self)?;
                }
                TerminalKind::LineComment => {
                    visit_ignored.visit_builtin_line_comment_terminal(crate::nodes::LineComment(child), data, self)?;
                }
                TerminalKind::BlockComment => {
                    visit_ignored.visit_builtin_block_comment_terminal(crate::nodes::BlockComment(child), data, self)?;
                }
                _ => unreachable!(),
            }
        }

        let (result, visit_ignored) = visitor(
            result.try_into().expect("Result should have the same length as nodes"),
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
                        match kind {
                            TerminalKind::Whitespace => {
                                visit_ignored.visit_builtin_whitespace_terminal(crate::nodes::Whitespace(child), data, self)?;
                            }
                            TerminalKind::NewLine => {
                                visit_ignored.visit_builtin_new_line_terminal(crate::nodes::NewLine(child), data, self)?;
                            }
                            TerminalKind::LineComment => {
                                visit_ignored.visit_builtin_line_comment_terminal(crate::nodes::LineComment(child), data, self)?;
                            }
                            TerminalKind::BlockComment => {
                                visit_ignored.visit_builtin_block_comment_terminal(crate::nodes::BlockComment(child), data, self)?;
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        return Err(ViewConstructionError::UnexpectedExtraNode { node: child });
                    }
                }
                CstNodeData::NonTerminal { .. } => {
                    return Err(ViewConstructionError::UnexpectedExtraNode { node: child });
                }
            }
        }

        Ok(result)
    }
}

pub trait CstFacade: Sized {
    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData>;
    fn has_no_children(&self, node: CstNodeId) -> bool;
    fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId>;
    fn get_terminal(&self, node: CstNodeId, kind: TerminalKind) -> Result<TerminalData, CstConstructError>;
    fn get_non_terminal(&self, node: CstNodeId, kind: NonTerminalKind) -> Result<NonTerminalData, CstConstructError>;
    fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        nodes: [NodeKind; N],
        visitor: impl FnMut([CstNodeId; N], &'v mut V) -> Result<(O, &'v mut V), CstConstructError<E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>>;
}

impl CstFacade for ConcreteSyntaxTree {
    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData> {
        ConcreteSyntaxTree::node_data(self, node)
    }

    fn has_no_children(&self, node: CstNodeId) -> bool {
        ConcreteSyntaxTree::has_no_children(self, node)
    }

    fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId> {
        ConcreteSyntaxTree::children(self, node)
    }

    fn get_terminal(&self, node: CstNodeId, kind: TerminalKind) -> Result<TerminalData, CstConstructError> {
        ConcreteSyntaxTree::get_terminal(self, node, kind)
    }

    fn get_non_terminal(&self, node: CstNodeId, kind: NonTerminalKind) -> Result<NonTerminalData, CstConstructError> {
        ConcreteSyntaxTree::get_non_terminal(self, node, kind)
    }

    fn collect_nodes<'v, const N: usize, V: BuiltinTerminalVisitor<E, Self>, O, E>(
        &self,
        parent: CstNodeId,
        nodes: [NodeKind; N],
        visitor: impl FnMut([CstNodeId; N], &'v mut V) -> Result<(O, &'v mut V), CstConstructError<E>>,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>> {
        ConcreteSyntaxTree::collect_nodes(self, parent, nodes, visitor, visit_ignored)
    }
}

struct DummyTerminalVisitor;

// DummyTerminalVisitor implements CstVisitor which provides BuiltinTerminalVisitor via blanket impl
impl<F: CstFacade> crate::visitor::CstVisitor<F> for DummyTerminalVisitor {
    type Error = Infallible;
}

pub trait TerminalHandle {
    fn node_id(&self) -> CstNodeId;
    fn kind(&self) -> TerminalKind;
    fn get_data<F: CstFacade>(&self, tree: &F) -> Result<TerminalData, CstConstructError> {
        tree.get_terminal(self.node_id(), self.kind())
    }
}

pub trait NonTerminalHandle: Sized {
    type View;

    fn node_id(&self) -> CstNodeId;

    fn new<F: CstFacade>(index: CstNodeId, tree: &F) -> Result<Self, CstConstructError> {
        Self::new_with_visit(index, tree, &mut DummyTerminalVisitor)
    }

    fn new_with_visit<F: CstFacade, E>(
        index: CstNodeId,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<E, F>,
    ) -> Result<Self, CstConstructError<E>>;

    fn get_view<C: CstFacade>(&self, tree: &C) -> Result<Self::View, CstConstructError> {
        self.get_view_with_visit(tree, |view, visit_ignored| (view, visit_ignored), &mut DummyTerminalVisitor)
    }

    fn get_view_with_visit<'v, F: CstFacade, V: BuiltinTerminalVisitor<E, F>, O, E>(
        &self,
        tree: &F,
        visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>>;

    fn kind(&self) -> NonTerminalKind;
}

pub trait RecursiveView<F: CstFacade> {
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

// CST Builder implementing TreeConstruct
#[derive(Debug, Clone)]
pub struct CstBuilder {
    tree: ConcreteSyntaxTree,
    node_stack: Vec<NodeStackItem>,
    root_node: Option<CstNodeId>,
}

#[derive(Debug, Clone)]
struct NodeStackItem {
    node: CstNodeId,
    span: InputSpan,
    children: Vec<CstNodeId>,
}

impl Default for CstBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CstBuilder {
    pub fn new() -> Self {
        let temp_root_data = CstNodeData::new_non_terminal(NonTerminalKind::Root, NonTerminalData::Input(InputSpan::EMPTY));
        Self {
            tree: ConcreteSyntaxTree::new(temp_root_data),
            node_stack: Vec::new(),
            root_node: None,
        }
    }

    fn add_terminal_node(&mut self, kind: TerminalKind, span: InputSpan) -> CstNodeId {
        let node = self.tree.add_node(CstNodeData::Terminal {
            kind,
            data: TerminalData::Input(span),
        });

        let parent = self.node_stack.last_mut().expect("node stack is empty");
        parent.children.push(node);
        parent.span = parent.span.merge(span);

        node
    }

    fn open_non_terminal_node(&mut self, kind: NonTerminalKind) -> CstNodeId {
        let node = self.tree.add_node(CstNodeData::NonTerminal {
            kind,
            data: NonTerminalData::Input(InputSpan::EMPTY),
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
                let updated_data = CstNodeData::NonTerminal {
                    kind,
                    data: NonTerminalData::Input(item.span),
                };
                self.tree.update_node(parent, updated_data);
            }

            if let Some(parent_item) = self.node_stack.last_mut() {
                parent_item.span = parent_item.span.merge(item.span);
            }
        }
        popped.map(|item| item.node)
    }

    pub fn build_tree(mut self) -> ConcreteSyntaxTree {
        while !self.node_stack.is_empty() {
            self.close_non_terminal_node();
        }

        if let Some(root_node) = self.root_node {
            self.tree.set_root(root_node);
        }

        self.tree
    }
}

impl<'t> parol_runtime::parser::parse_tree_type::TreeConstruct<'t> for CstBuilder {
    type Error = parol_runtime::ParolError;
    type Tree = ConcreteSyntaxTree;

    fn open_non_terminal(
        &mut self,
        name: &'static str,
        _size_hint: Option<usize>,
    ) -> Result<(), Self::Error> {
        let kind = NonTerminalKind::from_non_terminal_name(name);
        self.open_non_terminal_node(kind);
        Ok(())
    }

    fn close_non_terminal(&mut self) -> Result<(), Self::Error> {
        self.close_non_terminal_node();
        Ok(())
    }

    fn add_token(&mut self, token: &parol_runtime::Token<'t>) -> Result<(), Self::Error> {
        let kind = TerminalKind::from_terminal_index(token.token_type);
        let span = InputSpan {
            start: token.location.start,
            end: token.location.end,
        };
        self.add_terminal_node(kind, span);
        Ok(())
    }

    fn build(self) -> Result<Self::Tree, Self::Error> {
        Ok(self.build_tree())
    }
}
