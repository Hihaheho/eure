use eure_tree::{
    Cst,
    node_kind::{NonTerminalKind, TerminalKind},
    tree::{
        ConcreteSyntaxTree, CstFacade, CstNodeData, CstNodeId, InputSpan, NonTerminalData,
        TerminalData,
    },
};
use parol_runtime::{ParolError, Token, parser::parse_tree_type::TreeConstruct};

/// Eure-specific tree builder that handles the peculiarities of the Eure grammar,
/// particularly the reversed ordering of list elements
#[derive(Debug, Clone)]
pub struct CstBuilder {
    tree: ConcreteSyntaxTree<TerminalKind, NonTerminalKind>,
    node_stack: Vec<NodeStackItem>,
    root_node: Option<CstNodeId>,
}

#[derive(Debug, Clone)]
struct NodeStackItem {
    node: CstNodeId,
    span: InputSpan,
    children: Vec<CstNodeId>,
}

impl Default for CstBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CstBuilder {
    pub fn new() -> Self {
        // Create a temporary root node that we'll replace later
        let temp_root_data =
            CstNodeData::new_non_terminal(NonTerminalKind::Root, NonTerminalData::Dynamic);
        Self {
            tree: ConcreteSyntaxTree::new(temp_root_data),
            node_stack: Vec::new(),
            root_node: None,
        }
    }

    // Adds a terminal node to the current non-terminal in the stack
    fn add_terminal_node(&mut self, kind: TerminalKind, span: InputSpan) -> CstNodeId {
        let node = self.tree.add_node(CstNodeData::Terminal {
            kind,
            data: TerminalData::Input(span),
        });

        let parent = self.node_stack.last_mut().expect("node stack is empty");
        parent.children.push(node);
        parent.span = parent.span.merge(span);

        node
    }

    // Opens a new non-terminal and adds it to the stack
    fn open_non_terminal_node(&mut self, kind: NonTerminalKind) -> CstNodeId {
        // span will be filled later
        let node = self.tree.add_node(CstNodeData::NonTerminal {
            kind,
            data: NonTerminalData::Input(InputSpan::EMPTY),
        });

        if let Some(parent) = self.node_stack.last_mut() {
            parent.children.push(node);
        } else {
            // This is a root level node
            self.root_node = Some(node);
        }

        self.node_stack.push(NodeStackItem {
            node,
            span: InputSpan::EMPTY,
            children: Vec::new(),
        });
        node
    }

    // Closes the current non-terminal
    fn close_non_terminal_node(&mut self) -> Option<CstNodeId> {
        let popped = self.node_stack.pop();
        if let Some(item) = &popped {
            let parent = item.node;

            // Set children in original order (our HashMap-based implementation preserves insertion order)
            self.tree.update_children(parent, item.children.clone());

            // Update the node's span
            if let Some(CstNodeData::NonTerminal { kind, .. }) = self.tree.node_data(parent) {
                let updated_data = CstNodeData::NonTerminal {
                    kind,
                    data: NonTerminalData::Input(item.span),
                };
                self.tree.update_node(parent, updated_data);
            }

            if let Some(parent_item) = self.node_stack.last_mut() {
                parent_item.span = parent_item.span.merge(item.span);
            }
        }
        popped.map(|item| item.node)
    }

    /// Builds the tree
    pub fn build_tree(mut self) -> Cst {
        while !self.node_stack.is_empty() {
            self.close_non_terminal_node();
        }

        // Set the actual root if we have one
        if let Some(root_node) = self.root_node {
            self.tree.set_root(root_node);
        }

        self.tree
    }
}

impl<'t> TreeConstruct<'t> for CstBuilder {
    type Error = ParolError;
    type Tree = Cst;

    fn open_non_terminal(
        &mut self,
        name: &'static str,
        _size_hint: Option<usize>,
    ) -> Result<(), Self::Error> {
        let kind = NonTerminalKind::from_non_terminal_name(name);
        self.open_non_terminal_node(kind);
        Ok(())
    }

    fn close_non_terminal(&mut self) -> Result<(), Self::Error> {
        self.close_non_terminal_node();
        Ok(())
    }

    fn add_token(&mut self, token: &Token<'t>) -> Result<(), Self::Error> {
        let kind = TerminalKind::from_terminal_index(token.token_type);
        let span = InputSpan {
            start: token.location.start,
            end: token.location.end,
        };
        self.add_terminal_node(kind, span);
        Ok(())
    }

    fn build(self) -> Result<Self::Tree, Self::Error> {
        Ok(self.build_tree())
    }
}
