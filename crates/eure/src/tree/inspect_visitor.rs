use eure_tree::node_kind::{NonTerminalKind, TerminalKind};
use eure_tree::tree::{CstFacade, CstFacadeExt};
use eure_tree::{prelude::*, tree::LineNumbers};

use crate::tree::{WriteError, get_error_position_from_error};

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

impl<F: CstFacade<TerminalKind, NonTerminalKind> + CstFacadeExt> CstVisitor<F>
    for InspectVisitor<'_, '_>
{
    type Error = WriteError;
    fn then_construct_error(
        &mut self,
        node_data: Option<CstNode>,
        parent: CstNodeId,
        kind: NodeKind,
        error: CstConstructError<Self::Error>,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if let Some((line, column)) =
            get_error_position_from_error(&self.line_numbers, &node_data, &error)
        {
            eprintln!("Syntax error at line {line}, column {column}: {error}");
        } else {
            eprintln!("Syntax error: {error}");
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
                    .ok_or(WriteError::DynamicTokenNotFound { id: token_id })?,
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
