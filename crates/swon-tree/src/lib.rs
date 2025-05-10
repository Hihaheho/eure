use std::convert::Infallible;

use node_kind::{NonTerminalKind, TerminalKind};
use tree::{ConcreteSyntaxTree, ViewConstructionError};

pub mod action;
pub mod common_visitors;
pub mod node_kind;
pub mod nodes;
pub mod tree;
pub mod value_visitor;
pub mod visitor;

pub type Cst = ConcreteSyntaxTree<TerminalKind, NonTerminalKind>;
pub type CstNode = tree::CstNodeData<TerminalKind, NonTerminalKind>;
pub type CstConstructError<E = Infallible> =
    ViewConstructionError<TerminalKind, NonTerminalKind, E>;
pub type NodeKind = node_kind::NodeKind<TerminalKind, NonTerminalKind>;

pub mod prelude {
    pub use crate::action::CstCommands;
    pub use crate::node_kind::{NonTerminalKind, TerminalKind};
    pub use crate::nodes::*;
    pub use crate::tree::{CstNodeId, DynamicTokenId, NonTerminalData, TerminalData};
    pub use crate::visitor::{CstVisitor, CstVisitorSuper as _};
    pub use crate::{Cst, CstConstructError, CstNode, NodeKind};
}
