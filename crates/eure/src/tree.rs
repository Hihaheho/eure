mod inspect_visitor;
mod write_visitor;

pub use eure_tree::prelude::*;
pub use eure_tree::tree::ViewConstructionError;
use thiserror::Error;

use crate::tree::{inspect_visitor::InspectVisitor, write_visitor::WriteVisitor};

#[derive(Error, Debug)]
pub enum WriteError {
    #[error(transparent)]
    FmtError(#[from] std::fmt::Error),
    #[error(transparent)]
    ViewConstructionError(#[from] ViewConstructionError),
    #[error("Dynamic token not found: {id:?}")]
    DynamicTokenNotFound { id: DynamicTokenId },
}

pub fn write_cst(input: &str, cst: &Cst, w: &mut impl std::fmt::Write) -> Result<(), WriteError> {
    let mut visitor = WriteVisitor::new(input, w);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    Ok(())
}

pub fn inspect_cst(input: &str, cst: &Cst, w: &mut impl std::fmt::Write) -> Result<(), WriteError> {
    let mut visitor = InspectVisitor::new(input, w);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    Ok(())
}

/// Extract position information from error context
pub fn get_error_position_from_error<E>(
    line_numbers: &LineNumbers,
    node_data: &Option<CstNode>,
    _error: &CstConstructError<E>,
) -> Option<(u32, u32)> {
    // Try to get position from node_data
    if let Some(node) = node_data {
        match node {
            CstNode::Terminal {
                data: TerminalData::Input(span),
                ..
            } => {
                let char_info = line_numbers.get_char_info(span.start);
                return Some((char_info.line_number + 1, char_info.column_number + 1)); // Convert to 1-indexed
            }
            CstNode::NonTerminal {
                data: NonTerminalData::Input(span),
                ..
            } => {
                let char_info = line_numbers.get_char_info(span.start);
                return Some((char_info.line_number + 1, char_info.column_number + 1)); // Convert to 1-indexed
            }
            _ => {}
        }
    }

    None
}
