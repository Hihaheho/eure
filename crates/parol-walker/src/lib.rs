//! Type-safe visitor pattern generator for Parol parsers - runtime library
//!
//! This crate provides the runtime traits and types needed for generated visitor code.
//! Use `parol-walker-gen` to generate visitor implementations from your Parol grammar.

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

/// Data associated with a terminal node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerminalData {
    /// Span in the input source
    pub span: Option<InputSpan>,
}

/// Data associated with a non-terminal node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NonTerminalData {
    /// Span in the input source
    pub span: Option<InputSpan>,
}

/// Represents a span in the input source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputSpan {
    pub start: u32,
    pub end: u32,
}

impl InputSpan {
    pub const EMPTY: Self = Self { start: 0, end: 0 };

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
}

/// Errors that can occur during view construction
#[derive(Debug, Error)]
pub enum ViewConstructionError {
    #[error("Node ID not found: {node}")]
    NodeIdNotFound { node: CstNodeId },

    #[error("Unexpected end of children for parent: {parent}")]
    UnexpectedEndOfChildren { parent: CstNodeId },

    #[error("Unexpected extra node: {node}")]
    UnexpectedExtraNode { node: CstNodeId },

    #[error("Unexpected node: {node}")]
    UnexpectedNode { node: CstNodeId },
}

/// Errors that can occur during CST construction
#[derive(Debug, Error)]
pub enum CstConstructError<E = ViewConstructionError> {
    #[error("View construction error: {0}")]
    ViewConstruction(#[from] ViewConstructionError),

    #[error("Visitor error: {0}")]
    Visitor(E),
}

impl<E> CstConstructError<E> {
    pub fn extract_error(self) -> Result<ViewConstructionError, E> {
        match self {
            CstConstructError::ViewConstruction(e) => Ok(e),
            CstConstructError::Visitor(e) => Err(e),
        }
    }
}

/// Trait for accessing CST structure (implemented by generated code)
///
/// The generic parameters `T` and `Nt` represent terminal and non-terminal kinds respectively.
pub trait CstFacade<T, Nt>: Sized {
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
    fn get_terminal(&self, node: CstNodeId, kind: T) -> Result<TerminalData, CstConstructError>;

    /// Get data for a non-terminal node of specific kind
    fn get_non_terminal(
        &self,
        node: CstNodeId,
        kind: Nt,
    ) -> Result<NonTerminalData, CstConstructError>;

    /// Get parent of a node
    fn parent(&self, node: CstNodeId) -> Option<CstNodeId>;
}

/// Node data in the CST
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CstNodeData<T, Nt> {
    Terminal { kind: T, data: TerminalData },
    NonTerminal { kind: Nt, data: NonTerminalData },
}

impl<T, Nt> CstNodeData<T, Nt> {
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

/// Trait for terminal node handles (implemented by generated code)
pub trait TerminalHandle<T> {
    /// Get the node ID
    fn node_id(&self) -> CstNodeId;

    /// Get the terminal kind
    fn kind(&self) -> T;

    /// Get the terminal data from the tree
    fn get_data<F: CstFacade<T, Nt>, Nt>(
        &self,
        tree: &F,
    ) -> Result<TerminalData, CstConstructError> {
        tree.get_terminal(self.node_id(), self.kind())
    }
}

/// Trait for non-terminal node handles (implemented by generated code)
pub trait NonTerminalHandle<T, Nt>: Sized {
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
    ) -> Result<Self, CstConstructError<E>>;

    /// Get the view and call visitor on it
    fn get_view_with_visit<'v, F: CstFacade<T, Nt>, V: BuiltinTerminalVisitor<T, Nt, E, F>, O, E>(
        &self,
        tree: &F,
        visit: impl FnMut(Self::View, &'v mut V) -> (O, &'v mut V),
        visit_ignored: &'v mut V,
    ) -> Result<O, CstConstructError<E>>;
}

/// Trait for visiting builtin terminals (whitespace, comments, etc.)
///
/// This is implemented by the main CstVisitor trait to delegate to specific visit methods.
pub trait BuiltinTerminalVisitor<T, Nt, E, F: CstFacade<T, Nt>> {
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
pub trait RecursiveView<T, Nt, F: CstFacade<T, Nt>>: Copy {
    type Item;

    /// Get all items from this recursive view
    fn get_all_with_visit<E>(
        &self,
        tree: &F,
        visit_ignored: &mut impl BuiltinTerminalVisitor<T, Nt, E, F>,
    ) -> Result<Vec<Self::Item>, CstConstructError<E>>;
}
