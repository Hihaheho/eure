use eure_tree::visitor::{NodeVisitor, NodeVisitorSuper};
use eure_tree::{
    Cst,
    node_kind::{NonTerminalKind, TerminalKind},
    tree::{CstNodeData, CstNodeId, TerminalData},
};
use std::convert::Infallible;

pub struct CstPathExtractor {
    target_byte_offset: u32,
    current_path: Vec<String>,
    found_path: Option<Vec<String>>,
    section_path: Vec<String>, // The current section path
    in_binding: bool,          // Whether we're inside a binding
    input: String,
}

impl CstPathExtractor {
    pub fn new(input: String, target_byte_offset: u32) -> Self {
        Self {
            target_byte_offset,
            current_path: vec![],
            found_path: None,
            section_path: vec![],
            in_binding: false,
            input,
        }
    }

    pub fn extract_path(&mut self, cst: &Cst) -> Vec<String> {
        let _ = self.visit_node_id(cst.root(), cst);
        self.found_path.clone().unwrap_or_default()
    }
}

impl NodeVisitor for CstPathExtractor {
    type Error = Infallible;

    fn visit_node(
        &mut self,
        id: CstNodeId,
        node: CstNodeData<TerminalKind, NonTerminalKind>,
        tree: &Cst,
    ) -> Result<(), Self::Error> {
        match &node {
            CstNodeData::NonTerminal { kind, .. } => {
                match kind {
                    NonTerminalKind::Section => {
                        // Reset section_path when entering a new section
                        self.section_path.clear();
                    }
                    NonTerminalKind::Binding => {
                        self.in_binding = true;
                        // When entering a binding, set current path to section path
                        self.current_path = self.section_path.clone();
                    }
                    NonTerminalKind::Keys => {
                        // Clear current path only if we're starting a new path
                        if !self.in_binding {
                            self.current_path.clear();
                        }
                    }
                    _ => {}
                }
            }
            CstNodeData::Terminal { kind, data } => {
                if let (TerminalKind::Ident, TerminalData::Input(span)) = (kind, data) {
                    // Extract the identifier text
                    let ident_text = &self.input[span.start as usize..span.end as usize];

                    // Check if cursor is within or after this identifier
                    let cursor_in_or_after = span.start <= self.target_byte_offset;

                    if cursor_in_or_after {
                        // If we're in a binding, return the section path (not including the binding key)
                        if self.in_binding {
                            self.found_path = Some(self.section_path.clone());
                        } else {
                            // In section keys, build path up to and including this identifier
                            let mut path = self.current_path.clone();
                            path.push(ident_text.to_string());
                            self.found_path = Some(path.clone());
                        }
                    }

                    // Only add to current_path if we're not in a binding
                    if !self.in_binding {
                        self.current_path.push(ident_text.to_string());
                    }
                }
            }
        }

        self.visit_node_super(id, node, tree)?;

        // After visiting children
        if let CstNodeData::NonTerminal { kind, .. } = &node {
            match kind {
                NonTerminalKind::Keys => {
                    // If we just finished section keys, save as section path
                    if !self.in_binding {
                        self.section_path = self.current_path.clone();
                    }
                }
                NonTerminalKind::Binding => {
                    self.in_binding = false;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
