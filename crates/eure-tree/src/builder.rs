use crate::{
    node_kind::{NonTerminalKind, TerminalKind},
    tree::{ConcreteSyntaxTree, CstNodeData, CstNodeId, NonTerminalData, TerminalData},
};

/// A specialized builder for constructing CST nodes.
/// This is a simpler alternative to CstCommands, designed specifically for tree construction.
#[derive(Debug, Clone, Default)]
pub struct CstBuilder {
    commands: Vec<BuildCommand>,
}

#[derive(Debug, Clone)]
pub enum BuildCommand {
    Terminal {
        kind: TerminalKind,
        data: String,
    },
    NonTerminal {
        kind: NonTerminalKind,
        children: Vec<BuilderNodeId>,
    },
    Nested {
        builder: CstBuilder,
    },
}

/// A node ID within a CstBuilder context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuilderNodeId(usize);

impl CstBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a terminal and return its ID
    pub fn terminal(&mut self, kind: TerminalKind, data: impl Into<String>) -> BuilderNodeId {
        let id = BuilderNodeId(self.commands.len());
        self.commands.push(BuildCommand::Terminal {
            kind,
            data: data.into(),
        });
        id
    }

    /// Insert a non-terminal with children and return its ID
    pub fn non_terminal(
        &mut self,
        kind: NonTerminalKind,
        children: Vec<impl Into<BuilderNodeId>>,
    ) -> BuilderNodeId {
        let id = BuilderNodeId(self.commands.len());
        self.commands.push(BuildCommand::NonTerminal {
            kind,
            children: children.into_iter().map(|c| c.into()).collect(),
        });
        id
    }

    /// Embed another builder and return its ID
    pub fn embed(&mut self, builder: CstBuilder) -> BuilderNodeId {
        let id = BuilderNodeId(self.commands.len());
        self.commands.push(BuildCommand::Nested { builder });
        id
    }

    /// Apply to tree and return the root node ID
    pub fn apply<T, NT>(self, tree: &mut ConcreteSyntaxTree<T, NT>) -> CstNodeId
    where
        T: From<TerminalKind> + Clone,
        NT: From<NonTerminalKind> + Clone,
    {
        self.apply_to_tree(tree)
    }

    fn apply_to_tree<T, NT>(self, tree: &mut ConcreteSyntaxTree<T, NT>) -> CstNodeId
    where
        T: From<TerminalKind> + Clone,
        NT: From<NonTerminalKind> + Clone,
    {
        // Track the root node for each command
        let mut command_roots = Vec::with_capacity(self.commands.len());

        for command in self.commands {
            let root_id = match command {
                BuildCommand::Terminal { kind, data } => {
                    let token_id = tree.insert_dynamic_terminal(data);
                    tree.add_node(CstNodeData::Terminal {
                        kind: kind.into(),
                        data: TerminalData::Dynamic(token_id),
                    })
                }
                BuildCommand::NonTerminal { kind, children } => {
                    let node_id = tree.add_node(CstNodeData::NonTerminal {
                        kind: kind.into(),
                        data: NonTerminalData::Dynamic,
                    });

                    // Look up children from command_roots
                    let child_ids: Vec<_> = children
                        .iter()
                        .map(|&BuilderNodeId(idx)| command_roots[idx])
                        .collect();

                    tree.update_children(node_id, child_ids);
                    node_id
                }
                BuildCommand::Nested { builder } => {
                    // The nested builder returns its root node
                    builder.apply_to_tree(tree)
                }
            };
            command_roots.push(root_id);
        }

        // Return the root (last node created)
        *command_roots
            .last()
            .expect("Builder must have at least one command")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{CstNodeData, DynamicTokenId, TerminalData};
    use parol_walker::CstFacade;

    #[test]
    fn test_builder_basic() {
        let mut builder = CstBuilder::new();
        let term_id = builder.terminal(TerminalKind::Integer, "42");
        let node_id = builder.non_terminal(NonTerminalKind::Value, vec![term_id]);

        assert_eq!(term_id, BuilderNodeId(0));
        assert_eq!(node_id, BuilderNodeId(1));
    }

    #[test]
    fn test_builder_nested() {
        // Build inner structure: Boolean(True)
        let mut inner = CstBuilder::new();
        let inner_term = inner.terminal(TerminalKind::True, "true");
        let inner_bool = inner.non_terminal(NonTerminalKind::Boolean, vec![inner_term]);

        // Build outer structure: Value(embedded inner)
        let mut outer = CstBuilder::new();
        let nested_id = outer.embed(inner);
        let wrapper = outer.non_terminal(NonTerminalKind::Value, vec![nested_id]);

        // Apply to tree
        let mut tree: ConcreteSyntaxTree<TerminalKind, NonTerminalKind> =
            ConcreteSyntaxTree::new(CstNodeData::Terminal {
                kind: TerminalKind::Whitespace,
                data: TerminalData::Dynamic(DynamicTokenId(0)),
            });
        tree.insert_dynamic_terminal("");

        let root_id = outer.apply(&mut tree);
        tree.set_root(root_id);

        // Verify tree structure
        // Root should be Value node
        match tree.node_data(root_id) {
            Some(CstNodeData::NonTerminal { kind, .. }) => {
                assert_eq!(kind, NonTerminalKind::Value, "Root should be Value node");
            }
            _ => panic!("Expected Value non-terminal at root"),
        }

        // Value should have one child (the embedded Boolean)
        let value_children: Vec<_> = tree.children(root_id).collect();
        assert_eq!(value_children.len(), 1, "Value should have one child");

        // The child should be Boolean node
        let boolean_id = value_children[0];
        match tree.node_data(boolean_id) {
            Some(CstNodeData::NonTerminal { kind, .. }) => {
                assert_eq!(
                    kind,
                    NonTerminalKind::Boolean,
                    "Child should be Boolean node"
                );
            }
            _ => panic!("Expected Boolean non-terminal"),
        }

        // Boolean should have one child (True terminal)
        let boolean_children: Vec<_> = tree.children(boolean_id).collect();
        assert_eq!(boolean_children.len(), 1, "Boolean should have one child");

        // The child should be True terminal
        let true_id = boolean_children[0];
        match tree.node_data(true_id) {
            Some(CstNodeData::Terminal { kind, data }) => {
                assert_eq!(
                    kind,
                    TerminalKind::True,
                    "Grandchild should be True terminal"
                );
                // Verify the data contains "true"
                if let TerminalData::Dynamic(token_id) = data {
                    // The tree structure is correct, token content was set during terminal creation
                    assert_eq!(tree.dynamic_token(token_id).unwrap(), "true");
                } else {
                    panic!("Expected dynamic terminal data");
                }
            }
            _ => panic!("Expected True terminal"),
        }

        // The inner_bool ID in the inner builder should be 1 (after the terminal)
        assert_eq!(inner_bool, BuilderNodeId(1));
        // The nested_id in outer builder should be 0 (first command)
        assert_eq!(nested_id, BuilderNodeId(0));
        // The wrapper in outer builder should be 1 (second command)
        assert_eq!(wrapper, BuilderNodeId(1));
    }
}
