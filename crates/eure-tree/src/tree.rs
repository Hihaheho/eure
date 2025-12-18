mod span;

use ahash::HashMap;
use std::collections::BTreeMap;

// Re-export types from parol-walker
pub use parol_walker::{
    BuiltinTerminalKind, CstNodeData, CstNodeId, DynamicTokenId, InputSpan, NodeKind,
    NonTerminalData, TerminalData, ViewConstructionError,
};

// Re-export traits from parol-walker for use by generated code
pub use parol_walker::{CstFacade, NonTerminalHandle, RecursiveView, TerminalHandle};

// Re-export CstConstructError from parol-walker
pub use parol_walker::CstConstructError;

// Re-export BuiltinTerminalVisitor from visitor module (local trait to avoid orphan rules)
pub use crate::visitor::BuiltinTerminalVisitor;

// Import parol_walker's BuiltinTerminalVisitor for use in CstFacade impl
use parol_walker::BuiltinTerminalVisitor as PwBuiltinTerminalVisitor;

pub use span::{CharInfo, LineNumbers};

use crate::node_kind::{NonTerminalKind, TerminalKind};

/// Type alias for CstNodeData with grammar-specific types
pub type CstNode = CstNodeData<TerminalKind, NonTerminalKind>;

/// Type alias for the CST - used by generated code
pub type Cst = ConcreteSyntaxTree<TerminalKind, NonTerminalKind>;

/// Extension trait for CstNodeData with Eure-specific validation methods
pub trait CstNodeDataExt<T, Nt> {
    /// Check if this node is a terminal of the expected kind, returning an error if not
    fn expected_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: T,
    ) -> Result<(T, TerminalData), ViewConstructionError<T, Nt>>;

    /// Check if this node is a non-terminal of the expected kind, returning an error if not
    fn expected_non_terminal_or_error(
        &self,
        node: CstNodeId,
        expected: Nt,
    ) -> Result<(Nt, NonTerminalData), ViewConstructionError<T, Nt>>;
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

    fn expected_non_terminal_or_error(
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

    pub fn has_no_children_impl(&self, node: CstNodeId) -> bool {
        self.children
            .get(&node)
            .is_none_or(|children| children.is_empty())
    }

    pub fn children_impl(
        &self,
        node: CstNodeId,
    ) -> impl DoubleEndedIterator<Item = CstNodeId> + '_ {
        self.children
            .get(&node)
            .into_iter()
            .flat_map(|children| children.iter().copied())
    }

    pub fn parent_impl(&self, node: CstNodeId) -> Option<CstNodeId> {
        self.parent.get(&node).copied()
    }

    pub fn get_str_impl<'a: 'c, 'b: 'c, 'c>(
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
    pub fn node_data_impl(&self, node: CstNodeId) -> Option<CstNodeData<T, Nt>> {
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

// Implement parol_walker's CstFacade trait
impl CstFacade<TerminalKind, NonTerminalKind>
    for ConcreteSyntaxTree<TerminalKind, NonTerminalKind>
{
    fn get_str<'a: 'c, 'b: 'c, 'c>(
        &'a self,
        terminal: TerminalData,
        input: &'b str,
    ) -> Option<&'c str> {
        self.get_str_impl(terminal, input)
    }

    fn node_data(&self, node: CstNodeId) -> Option<CstNodeData<TerminalKind, NonTerminalKind>> {
        self.node_data_impl(node)
    }

    fn has_no_children(&self, node: CstNodeId) -> bool {
        self.has_no_children_impl(node)
    }

    fn children(&self, node: CstNodeId) -> impl DoubleEndedIterator<Item = CstNodeId> {
        self.children_impl(node)
    }

    fn get_terminal(
        &self,
        node: CstNodeId,
        kind: TerminalKind,
    ) -> Result<TerminalData, ViewConstructionError<TerminalKind, NonTerminalKind>> {
        let node_data = self
            .node_data_impl(node)
            .ok_or(ViewConstructionError::NodeIdNotFound { node })?;
        let (_, data) = node_data.expected_terminal_or_error(node, kind)?;
        Ok(data)
    }

    fn get_non_terminal(
        &self,
        node: CstNodeId,
        kind: NonTerminalKind,
    ) -> Result<NonTerminalData, ViewConstructionError<TerminalKind, NonTerminalKind>> {
        let node_data = self
            .node_data_impl(node)
            .ok_or(ViewConstructionError::NodeIdNotFound { node })?;
        let (_, data) = node_data.expected_non_terminal_or_error(node, kind)?;
        Ok(data)
    }

    fn parent(&self, node: CstNodeId) -> Option<CstNodeId> {
        self.parent_impl(node)
    }

    fn collect_nodes<
        'v,
        const N: usize,
        V: PwBuiltinTerminalVisitor<TerminalKind, NonTerminalKind, E, Self>,
        O,
        E,
    >(
        &self,
        parent: CstNodeId,
        nodes: [NodeKind<TerminalKind, NonTerminalKind>; N],
        mut visitor: impl FnMut(
            [CstNodeId; N],
            &'v mut V,
        ) -> Result<
            (O, &'v mut V),
            CstConstructError<TerminalKind, NonTerminalKind, E>,
        >,
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<TerminalKind, NonTerminalKind, E>> {
        let children = self.children_impl(parent).collect::<Vec<_>>();
        let mut children = children.into_iter();
        let mut result = Vec::with_capacity(N);
        let mut ignored = Vec::with_capacity(N);
        'outer: for expected_kind in nodes {
            'inner: for (idx, child) in children.by_ref().enumerate() {
                let child_data = self
                    .node_data_impl(child)
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
                                }
                                .into());
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
        for (child, kind, data) in ignored {
            match kind {
                TerminalKind::Whitespace => visit_ignored
                    .visit_builtin_whitespace_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?,
                TerminalKind::NewLine => visit_ignored
                    .visit_builtin_new_line_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?,
                TerminalKind::LineComment => visit_ignored
                    .visit_builtin_line_comment_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?,
                TerminalKind::BlockComment => visit_ignored
                    .visit_builtin_block_comment_terminal(child, data, self)
                    .map_err(CstConstructError::Visitor)?,
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
                .node_data_impl(child)
                .ok_or(ViewConstructionError::NodeIdNotFound { node: child })?;
            match child_data {
                CstNodeData::Terminal { kind, data } => {
                    if kind.is_builtin_terminal() {
                        match kind {
                            TerminalKind::Whitespace => visit_ignored
                                .visit_builtin_whitespace_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?,
                            TerminalKind::NewLine => visit_ignored
                                .visit_builtin_new_line_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?,
                            TerminalKind::LineComment => visit_ignored
                                .visit_builtin_line_comment_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?,
                            TerminalKind::BlockComment => visit_ignored
                                .visit_builtin_block_comment_terminal(child, data, self)
                                .map_err(CstConstructError::Visitor)?,
                            _ => unreachable!(),
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

/// Extension trait for Eure-specific CST methods
pub trait CstFacadeExt: CstFacade<TerminalKind, NonTerminalKind> {
    /// Get the content of a dynamic token
    fn dynamic_token(&self, id: DynamicTokenId) -> Option<&str>;

    /// Get the root handle
    fn root_handle(&self) -> crate::nodes::RootHandle;

    /// Returns the string representation of a terminal. Returns None if the terminal is a dynamic token and not found.
    fn get_terminal_str<'a: 'c, 'b: 'c, 'c, T: TerminalHandle<TerminalKind>>(
        &'a self,
        input: &'b str,
        handle: T,
    ) -> Result<Result<&'c str, DynamicTokenId>, ViewConstructionError<TerminalKind, NonTerminalKind>>
    {
        let data = self.get_terminal(handle.node_id(), handle.kind())?;
        match data {
            TerminalData::Input(input_span) => Ok(Ok(
                &input[input_span.start as usize..input_span.end as usize]
            )),
            TerminalData::Dynamic(id) => Ok(self.dynamic_token(id).ok_or(id)),
        }
    }

    /// Returns a span that excludes leading and trailing trivia (whitespace, newlines, comments).
    fn span(&self, node_id: CstNodeId) -> Option<InputSpan> {
        match self.node_data(node_id)? {
            CstNodeData::Terminal { data, .. } => match data {
                TerminalData::Input(span) => Some(span),
                TerminalData::Dynamic(_) => None,
            },
            CstNodeData::NonTerminal { data, .. } => {
                let first_span = self
                    .children(node_id)
                    .find_map(|child| self.find_first_non_trivia_span(child));
                let last_span = self
                    .children(node_id)
                    .rev()
                    .find_map(|child| self.find_last_non_trivia_span(child));

                match (first_span, last_span) {
                    (Some(first), Some(last)) => Some(first.merge(last)),
                    (Some(span), None) | (None, Some(span)) => Some(span),
                    (None, None) => match data {
                        NonTerminalData::Input(span) if span != InputSpan::EMPTY => Some(span),
                        _ => None,
                    },
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

impl CstFacadeExt for ConcreteSyntaxTree<TerminalKind, NonTerminalKind> {
    fn dynamic_token(&self, id: DynamicTokenId) -> Option<&str> {
        ConcreteSyntaxTree::dynamic_token(self, id)
    }

    fn root_handle(&self) -> crate::nodes::RootHandle {
        crate::nodes::RootHandle(self.root())
    }
}

impl ConcreteSyntaxTree<TerminalKind, NonTerminalKind> {
    pub fn visit_from_root<V: crate::visitor::CstVisitor<Self>>(
        &self,
        visitor: &mut V,
    ) -> Result<(), V::Error> {
        visitor.visit_root_handle(self.root_handle(), self)
    }
}
