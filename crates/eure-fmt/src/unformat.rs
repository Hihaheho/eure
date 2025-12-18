use std::convert::Infallible;

use eure_tree::node_kind::{NonTerminalKind, TerminalKind};
use eure_tree::prelude::*;
use eure_tree::tree::CstFacade;
use rand::prelude::*;

pub fn unformat(tree: &mut Cst) {
    let mut unformatter = Unformatter::new();
    let Ok(_) = tree.visit_from_root(&mut unformatter);
    unformatter.commands.apply_to(tree).unwrap();
}

pub fn unformat_with_seed(tree: &mut Cst, seed: u64) {
    let mut unformatter = Unformatter {
        rng: rand::rngs::SmallRng::seed_from_u64(seed),
        ..Default::default()
    };
    let Ok(_) = tree.visit_from_root(&mut unformatter);
    unformatter.commands.apply_to(tree).unwrap();
}

pub struct Unformatter {
    commands: CstCommands,
    rng: rand::rngs::SmallRng,
    /// When true, the section cannot be unformatted like InStr, CodeBlock, etc.
    no_white_space_here: bool,
    /// Probability of inserting weird spaces
    weird_space_probability: f32,
    /// Probability of inserting empty lines
    empty_line_probability: f32,
    /// Probability of removing a line
    line_removal_probability: f32,
    /// Probability of removing whitespace
    whitespace_removal_probability: f32,
    /// Current parent non-terminal
    parent: Option<CstNodeId>,
}

impl Unformatter {
    pub fn new() -> Self {
        Self {
            commands: CstCommands::default(),
            rng: rand::rngs::SmallRng::from_os_rng(),
            no_white_space_here: false,
            weird_space_probability: 0.2,
            empty_line_probability: 0.2,
            line_removal_probability: 0.2,
            whitespace_removal_probability: 0.2,
            parent: None,
        }
    }

    fn random_whitespace(&mut self) -> String {
        let ws_kind = [' ', '\t', '\u{3000}'];
        let ws_kind = ws_kind[self.rng.random_range(0..ws_kind.len())];
        ws_kind.to_string().repeat(self.rng.random_range(1..=5))
    }
}

impl Default for Unformatter {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: CstFacade<TerminalKind, NonTerminalKind>> CstVisitor<F> for Unformatter {
    type Error = Infallible;

    fn visit_str(&mut self, handle: StrHandle, view: StrView, tree: &F) -> Result<(), Self::Error> {
        self.no_white_space_here = true;
        self.visit_str_super(handle, view, tree)?;
        self.no_white_space_here = false;
        Ok(())
    }

    fn visit_code_block(
        &mut self,
        handle: CodeBlockHandle,
        view: CodeBlockView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.no_white_space_here = true;
        self.visit_code_block_super(handle, view, tree)?;
        self.no_white_space_here = false;
        Ok(())
    }

    fn visit_non_terminal(
        &mut self,
        id: CstNodeId,
        _kind: NonTerminalKind,
        _data: NonTerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        self.parent = Some(id);
        Ok(())
    }

    fn visit_terminal(
        &mut self,
        id: CstNodeId,
        kind: TerminalKind,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        if kind.is_builtin_terminal() {
            return Ok(());
        }
        let Some(parent) = self.parent else {
            return Ok(());
        };
        if !self.no_white_space_here && self.rng.random::<f32>() < self.weird_space_probability {
            let spaces = (0..self.rng.random_range(1..3))
                .map(|_| {
                    let ws = self.random_whitespace();
                    self.commands
                        .insert_dynamic_terminal(TerminalKind::Whitespace, ws)
                })
                .collect::<Vec<_>>();
            self.commands.add_nodes_before(parent, id, spaces);
        }
        if self.rng.random::<f32>() < self.empty_line_probability {
            let inserted = self
                .commands
                .insert_dynamic_terminal(TerminalKind::NewLine, "\n");
            self.commands.add_nodes_before(parent, id, vec![inserted]);
        }
        Ok(())
    }

    fn visit_whitespace_terminal(
        &mut self,
        terminal: Whitespace,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        if self.rng.random::<f32>() < self.whitespace_removal_probability {
            self.commands.delete_node(terminal.node_id());
        }
        Ok(())
    }

    fn visit_new_line_terminal(
        &mut self,
        terminal: NewLine,
        _data: TerminalData,
        _tree: &F,
    ) -> Result<(), Self::Error> {
        if self.rng.random::<f32>() < self.line_removal_probability {
            self.commands.delete_node(terminal.node_id());
        }
        Ok(())
    }
}
