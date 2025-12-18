use node_kind::{NonTerminalKind, TerminalKind};
use tree::ConcreteSyntaxTree;

pub mod action;
pub mod builder;
pub mod constructors;
#[allow(clippy::uninlined_format_args)]
pub mod node_kind;
pub mod nodes;
pub mod tree;
pub mod visitor;

pub type Cst = ConcreteSyntaxTree<TerminalKind, NonTerminalKind>;
pub type CstNode = tree::CstNodeData<TerminalKind, NonTerminalKind>;
// Re-export CstConstructError from parol-walker
pub use tree::CstConstructError;
pub type NodeKind = tree::NodeKind<TerminalKind, NonTerminalKind>;

pub mod prelude {
    pub use crate::action::CstCommands;
    pub use crate::node_kind::{NonTerminalKind, TerminalKind};
    pub use crate::nodes::*;
    pub use crate::tree::{
        CharInfo, CstFacade, CstFacadeExt as _, CstNodeId, DynamicTokenId, LineNumbers,
        NonTerminalData, NonTerminalHandle as _, TerminalData, TerminalHandle as _,
    };
    pub use crate::visitor::{CstVisitor, CstVisitorSuper as _};
    pub use crate::{Cst, CstConstructError, CstNode, NodeKind};
}
