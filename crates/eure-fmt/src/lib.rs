#[cfg(any(feature = "unformat", test))]
pub mod unformat;

use std::convert::Infallible;

use eure_tree::prelude::*;

#[cfg(any(feature = "unformat", test))]
pub use unformat::unformat_with_seed;

pub fn fmt(input: &str, cst: &mut Cst) -> Result<(), String> {
    let mut formatter = Formatter::new(input);
    formatter.desired_indent = 4;
    
    formatter.visit_root_handle(cst.root_handle(), cst)
        .map_err(|_| "Failed to visit CST".to_string())?;
    
    for (_, commands) in formatter.errors {
        commands.apply_to(cst).map_err(|e| e.to_string())?;
    }
    
    let mut second_formatter = Formatter::new(input);
    second_formatter.desired_indent = 4;
    second_formatter.visit_root_handle(cst.root_handle(), cst)
        .map_err(|_| "Failed to visit CST in second pass".to_string())?;
    
    for (_, commands) in second_formatter.errors {
        commands.apply_to(cst).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

pub enum FmtError {
    InvalidWhitespace { id: CstNodeId },
}

pub struct Formatter<'a> {
    input: &'a str,
    desired_indent: usize,
    new_line_count: usize,
    current_indent: usize,
    errors: Vec<(FmtError, CstCommands)>,
}

impl<'a> Formatter<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            desired_indent: 0,
            new_line_count: 0,
            current_indent: 0,
            errors: vec![],
        }
    }
    
    fn add_indent(&mut self) {
        self.desired_indent += 4;
    }

    fn remove_indent(&mut self) {
        self.desired_indent = self.desired_indent.saturating_sub(4);
    }
}



impl<'a, F: CstFacade> CstVisitor<F> for Formatter<'a> {
    type Error = Infallible;
    
    fn visit_terminal(
        &mut self,
        id: CstNodeId,
        kind: TerminalKind,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        match kind {
            TerminalKind::LBrace => {
                self.add_indent();
                
                let mut commands = CstCommands::default();
                commands.insert_dynamic_terminal(TerminalKind::Whitespace, " ".to_string());
                self.errors.push((
                    FmtError::InvalidWhitespace { id },
                    commands
                ));
            },
            TerminalKind::RBrace => {
                self.remove_indent();
            },
            TerminalKind::Bind => {
                let mut commands = CstCommands::default();
                commands.insert_dynamic_terminal(TerminalKind::Whitespace, " ".to_string());
                self.errors.push((
                    FmtError::InvalidWhitespace { id },
                    commands
                ));
            },
            TerminalKind::At => {
                let mut commands = CstCommands::default();
                commands.insert_dynamic_terminal(TerminalKind::Whitespace, " ".to_string());
                self.errors.push((
                    FmtError::InvalidWhitespace { id },
                    commands
                ));
            },
            TerminalKind::Dot => {
            },
            TerminalKind::Dollar => {
                let mut commands = CstCommands::default();
                commands.insert_dynamic_terminal(TerminalKind::Whitespace, " ".to_string());
                self.errors.push((
                    FmtError::InvalidWhitespace { id },
                    commands
                ));
            },
            TerminalKind::Comma => {
                let mut commands = CstCommands::default();
                commands.insert_dynamic_terminal(TerminalKind::Whitespace, " ".to_string());
                self.errors.push((
                    FmtError::InvalidWhitespace { id },
                    commands
                ));
            },
            _ => {}
        }
        
        self.visit_terminal_super(id, kind, data, tree)
    }

    fn visit_whitespace_terminal(
        &mut self,
        terminal: Whitespace,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        if self.current_indent == 0 && self.new_line_count > 0 {
            let mut commands = CstCommands::default();
            commands.delete_node(terminal.node_id());
            
            let ws = " ".repeat(self.desired_indent);
            commands.insert_dynamic_terminal(TerminalKind::Whitespace, ws);
            
            self.errors.push((
                FmtError::InvalidWhitespace { id: terminal.node_id() },
                commands
            ));
            
            self.current_indent = self.desired_indent;
        }
        
        self.visit_whitespace_terminal_super(terminal, data, tree)
    }
    
    fn visit_new_line_terminal(
        &mut self,
        terminal: NewLine,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        
        self.new_line_count += 1;
        self.current_indent = 0;
        self.visit_new_line_terminal_super(terminal, data, tree)
    }
    
    fn visit_line_comment_terminal(
        &mut self,
        terminal: LineComment,
        data: TerminalData,
        tree: &F,
    ) -> Result<(), Self::Error> {
        
        let mut commands = CstCommands::default();
        
        if self.new_line_count > 0 && self.current_indent == 0 {
            commands.insert_dynamic_terminal(TerminalKind::Whitespace, " ".repeat(self.desired_indent));
        } else {
            commands.insert_dynamic_terminal(TerminalKind::Whitespace, " ".to_string());
        }
        
        self.errors.push((FmtError::InvalidWhitespace { id: terminal.node_id() }, commands));
        
        self.visit_line_comment_terminal_super(terminal, data, tree)
    }
    

}
