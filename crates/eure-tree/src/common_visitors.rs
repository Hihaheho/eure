use thiserror::Error;

use crate::{prelude::*, tree::LineNumbers};

/// Extract position information from error context
fn get_error_position(
    line_numbers: &LineNumbers,
    node_data: &Option<CstNode>,
    error: &CstConstructError,
) -> Option<(u32, u32)> {
    // Try to get position from the error itself
    if let CstConstructError::UnexpectedNode { data, .. } = error {
        match data {
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

    // Fall back to node_data if available
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

pub struct FormatVisitor<'f, 't> {
    input: &'t str,
    line_numbers: LineNumbers<'t>,
    f: &'f mut dyn std::fmt::Write,
}

impl<'f, 't> FormatVisitor<'f, 't> {
    pub fn new(input: &'t str, f: &'f mut dyn std::fmt::Write) -> Self {
        Self {
            input,
            line_numbers: LineNumbers::new(input),
            f,
        }
    }
}

#[derive(Error, Debug)]
pub enum FormatVisitorError {
    #[error(transparent)]
    FmtError(#[from] std::fmt::Error),
    #[error(transparent)]
    ViewConstructionError(#[from] CstConstructError),
    #[error("Dynamic token not found: {id:?}")]
    DynamicTokenNotFound { id: DynamicTokenId },
}

impl<F: CstFacade> CstVisitor<F> for FormatVisitor<'_, '_> {
    type Error = FormatVisitorError;

    fn then_construct_error(
        &mut self,
        node_data: Option<CstNode>,
        parent: CstNodeId,
        kind: NodeKind,
        error: CstConstructError,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some((line, column)) = get_error_position(&self.line_numbers, &node_data, &error) {
            eprintln!(
                "Syntax error at line {}, column {}: {} expected {:?}",
                line, column, error, kind
            );
        } else {
            eprintln!("Syntax error: {} expected {:?}", error, kind);
        }
        self.recover_error(node_data, parent, kind, tree)
    }

    fn visit_terminal(
        &mut self,
        _id: CstNodeId,
        _kind: TerminalKind,
        terminal: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        match terminal {
            TerminalData::Input(input_span) => {
                write!(
                    self.f,
                    "{}",
                    &self.input[input_span.start as usize..input_span.end as usize]
                )?;
            }
            TerminalData::Dynamic(id) => {
                let str = tree
                    .dynamic_token(id)
                    .ok_or(FormatVisitorError::DynamicTokenNotFound { id })?;
                write!(self.f, "{}", str)?;
            }
        }
        Ok(())
    }
}

pub struct InspectVisitor<'f, 't> {
    input: &'t str,
    line_numbers: LineNumbers<'t>,
    indent: usize,
    f: &'f mut dyn std::fmt::Write,
}

impl<'f, 't> InspectVisitor<'f, 't> {
    pub fn new(input: &'t str, f: &'f mut dyn std::fmt::Write) -> Self {
        Self {
            input,
            line_numbers: LineNumbers::new(input),
            f,
            indent: 0,
        }
    }
}

impl<F: CstFacade> CstVisitor<F> for InspectVisitor<'_, '_> {
    type Error = FormatVisitorError;
    fn then_construct_error(
        &mut self,
        node_data: Option<CstNode>,
        parent: CstNodeId,
        kind: NodeKind,
        error: CstConstructError,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some((line, column)) = get_error_position(&self.line_numbers, &node_data, &error) {
            eprintln!(
                "Syntax error at line {}, column {}: {}",
                line, column, error
            );
        } else {
            eprintln!("Syntax error: {}", error);
        }
        self.recover_error(node_data, parent, kind, tree)
    }
    fn visit_terminal(
        &mut self,
        _id: CstNodeId,
        kind: TerminalKind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        match data {
            TerminalData::Input(input_span) => writeln!(
                self.f,
                "{}{} ({:?})",
                " ".repeat(self.indent),
                &self.input[input_span.start as usize..input_span.end as usize]
                    .replace("\n", "\\n")
                    .replace("\t", "\\t")
                    .replace(" ", "_"),
                kind,
            )?,
            TerminalData::Dynamic(token_id) => writeln!(
                self.f,
                "{}{:?} ({:?})",
                " ".repeat(self.indent),
                tree.dynamic_token(token_id)
                    .ok_or(FormatVisitorError::DynamicTokenNotFound { id: token_id })?,
                kind
            )?,
        }
        Ok(())
    }
    fn visit_non_terminal(
        &mut self,
        _id: CstNodeId,
        kind: NonTerminalKind,
        _data: NonTerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        writeln!(self.f, "{}{:?}", " ".repeat(self.indent), kind)?;
        self.indent += 2;
        Ok(())
    }
    fn visit_non_terminal_close(
        &mut self,
        _id: CstNodeId,
        _kind: NonTerminalKind,
        _data: NonTerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.indent -= 2;
        Ok(())
    }
}
