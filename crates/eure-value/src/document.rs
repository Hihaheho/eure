pub mod command;

use alloc::{string::String, vec, vec::Vec};

use crate::{
    document::command::DocumentCommand,
    identifier::Identifier,
    value::{Code, EurePath, KeyCmpValue},
};
#[cfg(feature = "std")]
use ahash::AHashMap as Map;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;

/// This does not include MetaExt since, PathSegment::Extension is encoded into Node::extensions, and PathSegment::MetaExt is encoded as InternalKey::MetaExtension, and PathSegment::Array is encoded as NodeContent::Array.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocumentKey {
    /// Regular identifiers like id, description
    Ident(Identifier),
    /// Meta-extension fields starting with $$ like $$optional, $$type
    MetaExtension(Identifier),
    /// Arbitrary value used as key
    Value(KeyCmpValue),
    /// Tuple element index (0-255)
    TupleIndex(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

pub struct EureDocument {
    pub(crate) root: NodeId,
    nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct Node {
    pub content: NodeValue,
    pub extensions: Map<Identifier, NodeId>,
}

#[derive(Debug)]
pub enum NodeValue {
    // Primitive values with typed handles
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Code(Code),
    CodeBlock(Code),
    NamedCode { name: String, code: Code },
    Path(EurePath),
    Hole,
    Array(Vec<NodeId>),
    Map(Vec<(DocumentKey, NodeId)>),
    Tuple(Vec<NodeId>),
}

#[derive(Debug, thiserror::Error)]
pub enum InsertError {
    #[error("Already assigned")]
    AlreadyAssigned { path: EurePath, key: DocumentKey },
    #[error("Path conflict: expected map but found {found}")]
    PathConflict { path: EurePath, found: &'static str },
    #[error("Expected array")]
    ExpectedArray { path: EurePath },
    #[error("Array index invalid: expected {expected_index} but got {index}")]
    ArrayIndexInvalid {
        path: EurePath,
        index: usize,
        expected_index: usize,
    },
    #[error("Expected map")]
    ExpectedMap { path: EurePath },
    #[error("Expected tuple")]
    ExpectedTuple { path: EurePath },
    #[error("Tuple index invalid: expected {expected_index} but got {index}")]
    TupleIndexInvalid {
        path: EurePath,
        index: usize,
        expected_index: usize,
    },
}

impl Default for EureDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl EureDocument {
    pub fn new() -> Self {
        Self {
            root: NodeId(0),
            nodes: vec![Node {
                content: NodeValue::Map(vec![]),
                extensions: Map::new(),
            }],
        }
    }

    pub fn get_root(&self) -> &Node {
        &self.nodes[self.root.0]
    }

    pub fn get_root_id(&self) -> NodeId {
        self.root
    }

    pub fn get_node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.0]
    }

    pub fn apply_command(&mut self, command: DocumentCommand) -> Result<NodeId, InsertError> {
        todo!()
    }

    pub fn apply_commands<const N: usize>(
        &mut self,
        commands: [DocumentCommand; N],
    ) -> [Result<NodeId, InsertError>; N] {
        commands.map(|command| self.apply_command(command))
    }
}
