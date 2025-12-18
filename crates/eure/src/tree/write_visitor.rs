use eure_tree::node_kind::{NonTerminalKind, TerminalKind};
use eure_tree::tree::{CstFacade, CstFacadeExt};
use eure_tree::{prelude::*, tree::LineNumbers};

use crate::tree::{WriteError, get_error_position_from_error};

pub struct WriteVisitor<'f, 't> {
    input: &'t str,
    line_numbers: LineNumbers<'t>,
    f: &'f mut dyn std::fmt::Write,
}

impl<'f, 't> WriteVisitor<'f, 't> {
    pub fn new(input: &'t str, f: &'f mut dyn std::fmt::Write) -> Self {
        Self {
            input,
            line_numbers: LineNumbers::new(input),
            f,
        }
    }
}

impl<F: CstFacade<TerminalKind, NonTerminalKind> + CstFacadeExt> CstVisitor<F>
    for WriteVisitor<'_, '_>
{
    type Error = WriteError;

    fn then_construct_error(
        &mut self,
        node_data: Option<CstNode>,
        parent: CstNodeId,
        kind: NodeKind,
        error: CstConstructError<Self::Error>,
        tree: &F,
    ) -> Result<(), WriteError> {
        if let Some((line, column)) =
            get_error_position_from_error(&self.line_numbers, &node_data, &error)
        {
            eprintln!("Syntax error at line {line}, column {column}: {error} expected {kind:?}");
        } else {
            eprintln!("Syntax error: {error} expected {kind:?}");
        }
        self.recover_error(node_data, parent, kind, tree)
    }

    fn visit_terminal(
        &mut self,
        _id: CstNodeId,
        _kind: TerminalKind,
        terminal: TerminalData,
        tree: &F,
    ) -> Result<(), WriteError> {
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
                    .ok_or(WriteError::DynamicTokenNotFound { id })?;
                write!(self.f, "{str}")?;
            }
        }
        Ok(())
    }
}
