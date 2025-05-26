use eure_tree::{
    Cst,
    node_kind::{NonTerminalKind, TerminalKind},
    tree::{ConcreteSyntaxTree, CstNodeData, CstNodeId, InputSpan, NonTerminalData, TerminalData},
};
use parol_runtime::{ParolError, Token, parser::parse_tree_type::TreeConstruct};
use petgraph::graph::{DiGraph, NodeIndex};

/// EURE-specific tree builder that handles the peculiarities of the EURE grammar,
/// particularly the reversed ordering of list elements
#[derive(Debug, Clone)]
pub struct CstBuilder {
    tree: DiGraph<Option<CstNodeData<TerminalKind, NonTerminalKind>>, ()>,
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
        Self {
            tree: DiGraph::new(),
            node_stack: Vec::new(),
            root_node: None,
        }
    }

    // Adds a terminal node to the current non-terminal in the stack
    fn add_terminal_node(&mut self, kind: TerminalKind, span: InputSpan) -> NodeIndex {
        let node = self.tree.add_node(Some(CstNodeData::Terminal {
            kind,
            data: TerminalData::Input(span),
        }));

        let parent = self.node_stack.last_mut().expect("node stack is empty");
        parent.children.push(CstNodeId(node));
        parent.span = parent.span.merge(span);

        node
    }

    // Opens a new non-terminal and adds it to the stack
    fn open_non_terminal_node(&mut self, kind: NonTerminalKind) -> NodeIndex {
        // span will be filled later
        let node = self.tree.add_node(Some(CstNodeData::NonTerminal {
            kind,
            data: NonTerminalData::Input(InputSpan::EMPTY),
        }));

        if let Some(parent) = self.node_stack.last_mut() {
            parent.children.push(CstNodeId(node));
        } else {
            // This is a root level node
            self.root_node = Some(CstNodeId(node));
        }

        self.node_stack.push(NodeStackItem {
            node: CstNodeId(node),
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
            item.children.iter().rev().for_each(|node| {
                self.tree.add_edge(parent.0, node.0, ());
            });
            let node_data = self
                .tree
                .node_weight_mut(parent.0)
                .expect("this node must be created on open_non_terminal_node");
            let Some(CstNodeData::NonTerminal { data, .. }) = node_data else {
                panic!("this node must be created as NonTerminal");
            };
            let NonTerminalData::Input(span) = data else {
                panic!("this node must be created as input node");
            };
            *span = item.span;
            if let Some(parent) = self.node_stack.last_mut() {
                parent.span = parent.span.merge(item.span);
            }
        }
        popped.map(|item| item.node)
    }

    /// Builds the tree
    pub fn build_tree(mut self) -> Cst {
        while !self.node_stack.is_empty() {
            self.close_non_terminal_node();
        }
        ConcreteSyntaxTree::new(
            self.root_node.expect("Root node always provided"),
            self.tree,
        )
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
