use crate::prelude_internal::*;

#[derive(Debug)]
pub struct Node {
    pub content: NodeValue,
    pub extensions: Map<Identifier, NodeId>,
}
impl Node {
    pub fn as_map(&self) -> Option<&NodeMap> {
        match &self.content {
            NodeValue::Map(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&NodeArray> {
        match &self.content {
            NodeValue::Array(array) => Some(array),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> Option<&NodeTuple> {
        match &self.content {
            NodeValue::Tuple(tuple) => Some(tuple),
            _ => None,
        }
    }

    pub(crate) fn require_map(&mut self) -> Result<&mut NodeMap, InsertErrorKind> {
        match &mut self.content {
            NodeValue::Map(map) => Ok(map),
            _ => Err(InsertErrorKind::ExpectedMap),
        }
    }

    pub(crate) fn require_tuple(&mut self) -> Result<&mut NodeTuple, InsertErrorKind> {
        match &mut self.content {
            NodeValue::Tuple(tuple) => Ok(tuple),
            _ => Err(InsertErrorKind::ExpectedTuple),
        }
    }

    pub(crate) fn require_array(&mut self) -> Result<&mut NodeArray, InsertErrorKind> {
        match &mut self.content {
            NodeValue::Array(array) => Ok(array),
            _ => Err(InsertErrorKind::ExpectedArray),
        }
    }
}

#[derive(Debug)]
pub enum NodeValue {
    /// A node that has not any value.
    Uninitialized,
    Primitive(PrimitiveValue),
    Array(NodeArray),
    Map(NodeMap),
    Tuple(NodeTuple),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Plural)]
pub struct NodeArray(pub Vec<NodeId>);

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Plural)]
#[plural(len, is_empty, iter, into_iter, into_iter_ref, new)]
pub struct NodeMap(pub Map<DocumentKey, NodeId>);

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Plural)]
pub struct NodeTuple(pub Vec<NodeId>);

impl NodeMap {
    pub fn get(&self, key: &DocumentKey) -> Option<&NodeId> {
        self.0.get(key)
    }

    pub fn add(&mut self, key: DocumentKey, node_id: NodeId) -> Result<(), InsertErrorKind> {
        if self.0.contains_key(&key) {
            return Err(InsertErrorKind::AlreadyAssigned { key });
        }
        self.0.insert(key, node_id);
        Ok(())
    }

    pub fn replace(&mut self, key: DocumentKey, node_id: NodeId) {
        self.0.insert(key, node_id);
    }

    pub fn remove(&mut self, key: &DocumentKey) -> Option<NodeId> {
        self.0.remove(key)
    }
}

impl NodeTuple {
    pub fn push(&mut self, node_id: NodeId) -> Result<(), InsertErrorKind> {
        self.0.push(node_id);
        Ok(())
    }

    pub fn add_at(&mut self, index: u8, node_id: NodeId) -> Result<(), InsertErrorKind> {
        if index as usize != self.0.len() {
            return Err(InsertErrorKind::TupleIndexInvalid {
                index,
                expected_index: self.0.len(),
            });
        }
        self.0.insert(index as usize, node_id);
        Ok(())
    }
}

impl NodeArray {
    pub fn push(&mut self, node_id: NodeId) -> Result<(), InsertErrorKind> {
        self.0.push(node_id);
        Ok(())
    }

    pub fn add_at(&mut self, index: usize, node_id: NodeId) -> Result<(), InsertErrorKind> {
        if index != self.0.len() {
            return Err(InsertErrorKind::ArrayIndexInvalid {
                index,
                expected_index: self.0.len(),
            });
        }
        self.0.insert(index, node_id);
        Ok(())
    }
}
