use thiserror::Error;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTarget {
    CstNodeId(CstNodeId),
    CommandNodeId(CommandNodeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandNodeId(usize);

impl From<CommandNodeId> for NodeTarget {
    fn from(value: CommandNodeId) -> Self {
        NodeTarget::CommandNodeId(value)
    }
}

impl From<CstNodeId> for NodeTarget {
    fn from(value: CstNodeId) -> Self {
        NodeTarget::CstNodeId(value)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct CstCommands {
    insert_num: usize,
    commands: Vec<Command>,
}

impl CstCommands {
    pub fn delete_node(&mut self, id: impl Into<NodeTarget>) {
        self.commands.push(Command::DeleteNode(id.into()));
    }

    pub fn delete_recursive(&mut self, id: impl Into<NodeTarget>) {
        self.commands.push(Command::DeleteRecursive(id.into()));
    }

    pub fn insert_dynamic_terminal(
        &mut self,
        kind: TerminalKind,
        data: impl Into<String>,
    ) -> CommandNodeId {
        self.commands.push(Command::InsertDynamicTerminal {
            kind,
            data: data.into(),
        });
        let id = CommandNodeId(self.insert_num);
        self.insert_num += 1;
        id
    }

    pub fn insert_node(&mut self, parent: impl Into<NodeTarget>, data: CstNode) -> CommandNodeId {
        self.commands.push(Command::Insert {
            parent: parent.into(),
            data,
        });
        let id = CommandNodeId(self.insert_num);
        self.insert_num += 1;
        id
    }

    pub fn update_node(&mut self, id: impl Into<NodeTarget>, data: CstNode) {
        self.commands.push(Command::Update {
            id: id.into(),
            data,
        });
    }

    /// Add nodes before the target child node
    pub fn add_nodes_before(
        &mut self,
        id: impl Into<NodeTarget>,
        before: impl Into<NodeTarget>,
        data: impl IntoIterator<Item = impl Into<NodeTarget>>,
    ) {
        self.commands.push(Command::AddNodesBefore {
            id: id.into(),
            before: before.into(),
            data: data.into_iter().map(|d| d.into()).collect(),
        });
    }
}

#[derive(Debug, Error)]
pub enum CommandApplyError {
    #[error("before node not found")]
    BeforeNodeNotFound { id: CstNodeId, before: CstNodeId },
}

impl CstCommands {
    pub fn apply_to(self, tree: &mut Cst) -> Result<(), CommandApplyError> {
        let mut inserted = vec![];
        let to_id = |inserted: &[CstNodeId], target: NodeTarget| -> CstNodeId {
            match target {
                NodeTarget::CstNodeId(id) => id,
                NodeTarget::CommandNodeId(id) => inserted[id.0],
            }
        };
        for command in self.commands.into_iter() {
            match command {
                Command::Insert { parent, data } => {
                    let id = tree.add_node_with_parent(data, to_id(&inserted, parent));
                    inserted.push(id);
                }
                Command::DeleteNode(node_target) => {
                    tree.delete_node(to_id(&inserted, node_target));
                }
                Command::DeleteRecursive(node_target) => {
                    tree.delete_node(to_id(&inserted, node_target));
                }
                Command::ChangeParent { id, parent } => {
                    tree.change_parent(to_id(&inserted, id), to_id(&inserted, parent));
                }
                Command::Update { id, data } => {
                    tree.update_node(to_id(&inserted, id), data);
                }
                Command::AddNodesBefore { id, before, data } => {
                    let mut children = tree.children(to_id(&inserted, id)).collect::<Vec<_>>();
                    let Some(before_index) =
                        children.iter().position(|c| to_id(&inserted, before) == *c)
                    else {
                        return Err(CommandApplyError::BeforeNodeNotFound {
                            id: to_id(&inserted, id),
                            before: to_id(&inserted, before),
                        });
                    };
                    children.splice(
                        before_index..before_index,
                        data.into_iter().map(|d| to_id(&inserted, d)),
                    );
                    tree.update_children(to_id(&inserted, id), children);
                }
                Command::InsertDynamicTerminal { kind, data } => {
                    let token_id = tree.insert_dynamic_terminal(data);
                    let node_id = tree.add_node(CstNode::Terminal {
                        kind,
                        data: TerminalData::Dynamic(token_id),
                    });
                    inserted.push(node_id);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    DeleteNode(NodeTarget),
    DeleteRecursive(NodeTarget),
    ChangeParent {
        id: NodeTarget,
        parent: NodeTarget,
    },
    Insert {
        parent: NodeTarget,
        data: CstNode,
    },
    Update {
        id: NodeTarget,
        data: CstNode,
    },
    AddNodesBefore {
        id: NodeTarget,
        before: NodeTarget,
        data: Vec<NodeTarget>,
    },
    InsertDynamicTerminal {
        kind: TerminalKind,
        data: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{ConcreteSyntaxTree, CstNodeId, CstNodeData, NonTerminalData, TerminalData, DynamicTokenId};
    use crate::node_kind::{NonTerminalKind, TerminalKind};
    use petgraph::graph::{DiGraph, NodeIndex};
    
    fn create_test_tree() -> ConcreteSyntaxTree<TerminalKind, NonTerminalKind> {
        ConcreteSyntaxTree::new(
            CstNodeId(NodeIndex::new(0)),
            DiGraph::new(),
        )
    }
    
    #[test]
    fn test_delete_node() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        // Add a child node
        let node_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let child = tree.add_node_with_parent(node_data, root);
        
        let mut commands = CstCommands::default();
        commands.delete_node(child);
        
        commands.apply_to(&mut tree).unwrap();
        
        assert!(tree.has_no_children(root));
    }
    
    #[test]
    fn test_delete_recursive() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        // Add a child node
        let node_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let child = tree.add_node_with_parent(node_data, root);
        
        // Add a grandchild node
        let grandchild_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Swon,
            NonTerminalData::Dynamic,
        );
        let _grandchild = tree.add_node_with_parent(grandchild_data, child);
        
        let mut commands = CstCommands::default();
        commands.delete_node(child);
        
        commands.apply_to(&mut tree).unwrap();
        
        let root_children: Vec<_> = tree.children(root).collect();
        assert!(!root_children.contains(&child));
        
        let mut tree = create_test_tree();
        let root = tree.root();
        
        // Add a child node again
        let node_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let child = tree.add_node_with_parent(node_data, root);
        
        let mut commands = CstCommands::default();
        commands.delete_recursive(child);
        
        commands.apply_to(&mut tree).unwrap();
        
        let root_children: Vec<_> = tree.children(root).collect();
        assert!(!root_children.contains(&child));
    }
    
    #[test]
    fn test_change_parent() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        let node_data1 = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let child1 = tree.add_node_with_parent(node_data1, root);
        
        let node_data2 = CstNodeData::new_non_terminal(
            NonTerminalKind::Swon,
            NonTerminalData::Dynamic,
        );
        let child2 = tree.add_node_with_parent(node_data2, root);
        
        let grandchild_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let grandchild = tree.add_node_with_parent(grandchild_data, child1);
        
        let mut commands = CstCommands::default();
        commands.commands.push(Command::ChangeParent {
            id: grandchild.into(),
            parent: child2.into(),
        });
        
        commands.apply_to(&mut tree).unwrap();
        
        let child2_children: Vec<_> = tree.children(child2).collect();
        assert!(child2_children.contains(&grandchild));
        
    }
    
    #[test]
    fn test_insert() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        let node_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        
        let mut commands = CstCommands::default();
        let child_id = commands.insert_node(root, node_data);
        
        commands.apply_to(&mut tree).unwrap();
        
        let children: Vec<_> = tree.children(root).collect();
        assert_eq!(children.len(), 1);
        
        let child_data = tree.node_data(children[0]).unwrap();
        assert!(matches!(child_data, CstNodeData::NonTerminal { kind: NonTerminalKind::Root, .. }));
    }
    
    #[test]
    fn test_update() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        // Add a child node
        let node_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let child = tree.add_node_with_parent(node_data, root);
        
        let new_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Swon,
            NonTerminalData::Dynamic,
        );
        
        let mut commands = CstCommands::default();
        commands.update_node(child, new_data);
        
        commands.apply_to(&mut tree).unwrap();
        
        let updated_data = tree.node_data(child).unwrap();
        assert!(matches!(updated_data, CstNodeData::NonTerminal { kind: NonTerminalKind::Swon, .. }));
    }
    
    #[test]
    fn test_add_nodes_before() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        let node_data1 = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let child1 = tree.add_node_with_parent(node_data1, root);
        
        let node_data2 = CstNodeData::new_non_terminal(
            NonTerminalKind::Swon,
            NonTerminalData::Dynamic,
        );
        let child2 = tree.add_node_with_parent(node_data2, root);
        
        let node_data3 = CstNodeData::new_non_terminal(
            NonTerminalKind::Section,
            NonTerminalData::Dynamic,
        );
        let child3 = tree.add_node(node_data3);
        
        let mut commands = CstCommands::default();
        commands.add_nodes_before(root, child2, vec![child3]);
        
        commands.apply_to(&mut tree).unwrap();
        
        let children: Vec<_> = tree.children(root).collect();
        
        assert_eq!(children.len(), 3);
        
        assert!(children.contains(&child1));
        assert!(children.contains(&child2));
        assert!(children.contains(&child3));
        
        assert!(tree.children(root).any(|id| id == child3));
    }
    
    #[test]
    fn test_insert_dynamic_terminal() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        let mut commands = CstCommands::default();
        let token_id = commands.insert_dynamic_terminal(TerminalKind::Text, "test_text");
        
        commands.apply_to(&mut tree).unwrap();
        
        assert_eq!(tree.dynamic_token(DynamicTokenId(0)), Some("test_text"));
    }
    
    #[test]
    fn test_commands_with_errors() {
        let mut tree = create_test_tree();
        let root = tree.root();
        
        let invalid_node = CstNodeId(NodeIndex::new(999));
        
        let mut commands = CstCommands::default();
        commands.delete_node(invalid_node);
        
        assert!(commands.apply_to(&mut tree).is_ok());
        
        let mut commands = CstCommands::default();
        let node_data = CstNodeData::new_non_terminal(
            NonTerminalKind::Root,
            NonTerminalData::Dynamic,
        );
        let child = tree.add_node(node_data);
        commands.add_nodes_before(root, invalid_node, vec![child]);
        
        let result = commands.apply_to(&mut tree);
        assert!(result.is_err());
    }
}
