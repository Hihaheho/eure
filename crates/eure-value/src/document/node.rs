use crate::prelude_internal::*;

#[derive(Debug)]
pub struct Node {
    pub content: NodeValue,
    pub extensions: Map<Identifier, NodeId>,
}

pub struct NodeMut<'d> {
    document: &'d mut EureDocument,
    pub node_id: NodeId,
}

impl<'d> core::fmt::Debug for NodeMut<'d> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NodeMut")
            .field("node_id", &self.node_id)
            .finish_non_exhaustive()
    }
}

impl<'d> NodeMut<'d> {
    pub fn new(document: &'d mut EureDocument, node_id: NodeId) -> Self {
        Self { document, node_id }
    }

    pub fn add_map_child(self, object_key: ObjectKey) -> Result<NodeMut<'d>, InsertErrorKind> {
        self.document.add_map_child(object_key, self.node_id)
    }

    pub fn add_extension(self, identifier: Identifier) -> Result<NodeMut<'d>, InsertErrorKind> {
        self.document.add_extension(identifier, self.node_id)
    }

    pub fn add_meta_extension(
        self,
        identifier: Identifier,
    ) -> Result<NodeMut<'d>, InsertErrorKind> {
        self.document.add_meta_extension(identifier, self.node_id)
    }

    pub fn add_tuple_element(self, index: u8) -> Result<NodeMut<'d>, InsertErrorKind> {
        self.document.add_tuple_element(index, self.node_id)
    }

    pub fn add_array_element(self, index: Option<usize>) -> Result<NodeMut<'d>, InsertErrorKind> {
        self.document.add_array_element(index, self.node_id)
    }

    pub fn add_child_by_segment(
        self,
        segment: PathSegment,
    ) -> Result<NodeMut<'d>, InsertErrorKind> {
        self.document.add_child_by_segment(segment, self.node_id)
    }

    // Content access methods

    pub fn as_map(&self) -> Option<&NodeMap> {
        self.document.get_node(self.node_id).as_map()
    }

    pub fn as_array(&self) -> Option<&NodeArray> {
        self.document.get_node(self.node_id).as_array()
    }

    pub fn as_tuple(&self) -> Option<&NodeTuple> {
        self.document.get_node(self.node_id).as_tuple()
    }

    pub fn require_map(&mut self) -> Result<&mut NodeMap, InsertErrorKind> {
        self.document.get_node_mut(self.node_id).require_map()
    }

    pub fn require_tuple(&mut self) -> Result<&mut NodeTuple, InsertErrorKind> {
        self.document.get_node_mut(self.node_id).require_tuple()
    }

    pub fn require_array(&mut self) -> Result<&mut NodeArray, InsertErrorKind> {
        self.document.get_node_mut(self.node_id).require_array()
    }
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
        if self.content == NodeValue::Uninitialized {
            self.content = NodeValue::Map(Default::default());
            let NodeValue::Map(map) = &mut self.content else {
                unreachable!();
            };
            Ok(map)
        } else if let NodeValue::Map(map) = &mut self.content {
            Ok(map)
        } else {
            Err(InsertErrorKind::ExpectedMap)
        }
    }

    pub(crate) fn require_tuple(&mut self) -> Result<&mut NodeTuple, InsertErrorKind> {
        if self.content == NodeValue::Uninitialized {
            self.content = NodeValue::Tuple(Default::default());
            let NodeValue::Tuple(tuple) = &mut self.content else {
                unreachable!();
            };
            Ok(tuple)
        } else if let NodeValue::Tuple(tuple) = &mut self.content {
            Ok(tuple)
        } else {
            Err(InsertErrorKind::ExpectedTuple)
        }
    }

    pub(crate) fn require_array(&mut self) -> Result<&mut NodeArray, InsertErrorKind> {
        if self.content == NodeValue::Uninitialized {
            self.content = NodeValue::Array(Default::default());
            let NodeValue::Array(array) = &mut self.content else {
                unreachable!();
            };
            Ok(array)
        } else if let NodeValue::Array(array) = &mut self.content {
            Ok(array)
        } else {
            Err(InsertErrorKind::ExpectedArray)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeValue {
    /// A node that has not any value.
    Uninitialized,
    Primitive(PrimitiveValue),
    Array(NodeArray),
    Map(NodeMap),
    Tuple(NodeTuple),
}

impl NodeValue {
    pub fn empty_map() -> Self {
        Self::Map(NodeMap::new())
    }

    pub fn empty_array() -> Self {
        Self::Array(NodeArray::new())
    }

    pub fn empty_tuple() -> Self {
        Self::Tuple(NodeTuple::new())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Plural)]
pub struct NodeArray(pub Vec<NodeId>);

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Plural)]
#[plural(len, is_empty, iter, into_iter, into_iter_ref, new)]
pub struct NodeMap(pub Map<DocumentKey, NodeId>);

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Plural)]
pub struct NodeTuple(pub Vec<NodeId>);

impl NodeMap {
    pub fn get(&self, key: &DocumentKey) -> Option<NodeId> {
        self.0.get(key).copied()
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
    pub fn get(&self, index: usize) -> Option<NodeId> {
        self.0.get(index).copied()
    }

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
    pub fn get(&self, index: usize) -> Option<NodeId> {
        self.0.get(index).copied()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_map_on_uninitialized() {
        let mut node = Node {
            content: NodeValue::Uninitialized,
            extensions: Map::new(),
        };

        let map = node.require_map().expect("Should convert to map");
        assert_eq!(map.0.len(), 0);

        // Verify content was changed
        assert!(node.as_map().is_some());
    }

    #[test]
    fn test_require_map_on_existing_map() {
        let mut node = Node {
            content: NodeValue::Map(Default::default()),
            extensions: Map::new(),
        };

        let map = node.require_map().expect("Should return existing map");
        assert_eq!(map.0.len(), 0);
    }

    #[test]
    fn test_require_map_on_wrong_type() {
        let mut node = Node {
            content: NodeValue::Primitive(PrimitiveValue::Null),
            extensions: Map::new(),
        };

        let result = node.require_map();
        assert_eq!(result, Err(InsertErrorKind::ExpectedMap));
    }

    #[test]
    fn test_require_tuple_on_uninitialized() {
        let mut node = Node {
            content: NodeValue::Uninitialized,
            extensions: Map::new(),
        };

        let tuple = node.require_tuple().expect("Should convert to tuple");
        assert_eq!(tuple.0.len(), 0);

        // Verify content was changed
        assert!(node.as_tuple().is_some());
    }

    #[test]
    fn test_require_tuple_on_existing_tuple() {
        let mut node = Node {
            content: NodeValue::Tuple(Default::default()),
            extensions: Map::new(),
        };

        let tuple = node.require_tuple().expect("Should return existing tuple");
        assert_eq!(tuple.0.len(), 0);
    }

    #[test]
    fn test_require_tuple_on_wrong_type() {
        let mut node = Node {
            content: NodeValue::Primitive(PrimitiveValue::Null),
            extensions: Map::new(),
        };

        let result = node.require_tuple();
        assert_eq!(result, Err(InsertErrorKind::ExpectedTuple));
    }

    #[test]
    fn test_require_array_on_uninitialized() {
        let mut node = Node {
            content: NodeValue::Uninitialized,
            extensions: Map::new(),
        };

        let array = node.require_array().expect("Should convert to array");
        assert_eq!(array.0.len(), 0);

        // Verify content was changed
        assert!(node.as_array().is_some());
    }

    #[test]
    fn test_require_array_on_existing_array() {
        let mut node = Node {
            content: NodeValue::Array(Default::default()),
            extensions: Map::new(),
        };

        let array = node.require_array().expect("Should return existing array");
        assert_eq!(array.0.len(), 0);
    }

    #[test]
    fn test_require_array_on_wrong_type() {
        let mut node = Node {
            content: NodeValue::Primitive(PrimitiveValue::Null),
            extensions: Map::new(),
        };

        let result = node.require_array();
        assert_eq!(result, Err(InsertErrorKind::ExpectedArray));
    }
}
