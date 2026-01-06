pub mod constructor;
pub mod node;

use crate::document::node::{NodeArray, NodeTuple};
use crate::prelude_internal::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone)]
pub struct EureDocument {
    pub(crate) root: NodeId,
    nodes: Vec<Node>,
}

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
#[error("Insert error: {kind} at {path}")]
pub struct InsertError {
    pub kind: InsertErrorKind,
    pub path: EurePath,
}

#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum InsertErrorKind {
    #[error("Already assigned")]
    AlreadyAssigned { key: ObjectKey },
    #[error("Extension already assigned: {identifier}")]
    AlreadyAssignedExtension { identifier: Identifier },
    #[error("Expected array")]
    ExpectedArray,
    #[error("Array index invalid: expected {expected_index} but got {index}")]
    ArrayIndexInvalid { index: usize, expected_index: usize },
    #[error("Expected map")]
    ExpectedMap,
    #[error("Expected tuple")]
    ExpectedTuple,
    #[error("Tuple index invalid: expected {expected_index} but got {index}")]
    TupleIndexInvalid { index: u8, expected_index: usize },
    #[error("Binding target already has a value")]
    BindingTargetHasValue,
}

impl Default for EureDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for EureDocument {
    fn eq(&self, other: &Self) -> bool {
        self.nodes_equal(self.root, other, other.root)
    }
}

impl EureDocument {
    /// Compare two nodes structurally, ignoring NodeId values
    fn nodes_equal(&self, id1: NodeId, other: &EureDocument, id2: NodeId) -> bool {
        let node1 = &self.nodes[id1.0];
        let node2 = &other.nodes[id2.0];

        // Compare extensions
        if node1.extensions.len() != node2.extensions.len() {
            return false;
        }

        for (key1, &child_id1) in &node1.extensions {
            match node2.extensions.get(key1) {
                Some(&child_id2) => {
                    if !self.nodes_equal(child_id1, other, child_id2) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Compare content
        self.node_values_equal(&node1.content, other, &node2.content)
    }

    /// Compare two NodeValues structurally
    fn node_values_equal(
        &self,
        value1: &NodeValue,
        other: &EureDocument,
        value2: &NodeValue,
    ) -> bool {
        match (value1, value2) {
            (NodeValue::Hole(l1), NodeValue::Hole(l2)) => l1 == l2,
            (NodeValue::Primitive(p1), NodeValue::Primitive(p2)) => p1 == p2,
            (NodeValue::Array(arr1), NodeValue::Array(arr2)) => {
                self.node_arrays_equal(arr1, other, arr2)
            }
            (NodeValue::Tuple(tup1), NodeValue::Tuple(tup2)) => {
                self.node_tuples_equal(tup1, other, tup2)
            }
            (NodeValue::Map(map1), NodeValue::Map(map2)) => self.node_maps_equal(map1, other, map2),
            _ => false,
        }
    }

    fn node_arrays_equal(&self, arr1: &NodeArray, other: &EureDocument, arr2: &NodeArray) -> bool {
        if arr1.len() != arr2.len() {
            return false;
        }

        for (child_id1, child_id2) in arr1.iter().zip(arr2.iter()) {
            if !self.nodes_equal(*child_id1, other, *child_id2) {
                return false;
            }
        }

        true
    }

    fn node_tuples_equal(&self, tup1: &NodeTuple, other: &EureDocument, tup2: &NodeTuple) -> bool {
        if tup1.len() != tup2.len() {
            return false;
        }

        for (child_id1, child_id2) in tup1.iter().zip(tup2.iter()) {
            if !self.nodes_equal(*child_id1, other, *child_id2) {
                return false;
            }
        }

        true
    }

    fn node_maps_equal(&self, map1: &NodeMap, other: &EureDocument, map2: &NodeMap) -> bool {
        if map1.len() != map2.len() {
            return false;
        }

        for (key1, &child_id1) in map1.iter() {
            match map2.get(key1) {
                Some(&child_id2) => {
                    if !self.nodes_equal(child_id1, other, child_id2) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }

    pub fn new() -> Self {
        Self {
            root: NodeId(0),
            nodes: vec![Node {
                content: NodeValue::hole(),
                extensions: Map::new(),
            }],
        }
    }

    pub fn new_empty() -> Self {
        Self {
            root: NodeId(0),
            nodes: vec![Node {
                content: NodeValue::Map(Default::default()),
                extensions: Map::new(),
            }],
        }
    }

    pub fn new_primitive(value: PrimitiveValue) -> Self {
        Self {
            root: NodeId(0),
            nodes: vec![Node {
                content: NodeValue::Primitive(value),
                extensions: Map::new(),
            }],
        }
    }

    pub fn root(&self) -> &Node {
        &self.nodes[self.root.0]
    }

    pub fn get_root_id(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.0]
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.0)
    }

    pub fn create_node(&mut self, new: NodeValue) -> NodeId {
        self.nodes.push(Node {
            content: new,
            extensions: Map::new(),
        });
        NodeId(self.nodes.len() - 1)
    }

    pub fn create_node_uninitialized(&mut self) -> NodeId {
        self.create_node(NodeValue::hole())
    }

    /// Set the content of a node directly
    pub fn set_content(&mut self, node_id: NodeId, content: NodeValue) {
        self.nodes[node_id.0].content = content;
    }

    pub fn add_child_by_segment(
        &mut self,
        segment: PathSegment,
        parent_node_id: NodeId,
    ) -> Result<NodeMut<'_>, InsertErrorKind> {
        match segment {
            PathSegment::Ident(identifier) => {
                self.add_map_child(ObjectKey::String(identifier.into_string()), parent_node_id)
            }
            PathSegment::Value(object_key) => self.add_map_child(object_key, parent_node_id),
            PathSegment::Extension(identifier) => self.add_extension(identifier, parent_node_id),
            PathSegment::TupleIndex(index) => self.add_tuple_element(index, parent_node_id),
            PathSegment::ArrayIndex(index) => self.add_array_element(index, parent_node_id),
        }
    }

    pub fn add_map_child(
        &mut self,
        object_key: ObjectKey,
        parent_node_id: NodeId,
    ) -> Result<NodeMut<'_>, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.node_mut(parent_node_id);
        let map = node.require_map()?;
        map.add(object_key, node_id)?;
        Ok(NodeMut::new(self, node_id))
    }

    pub fn add_extension(
        &mut self,
        identifier: Identifier,
        parent_node_id: NodeId,
    ) -> Result<NodeMut<'_>, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.node_mut(parent_node_id);
        if node.extensions.contains_key(&identifier) {
            return Err(InsertErrorKind::AlreadyAssignedExtension { identifier });
        }
        node.extensions.insert(identifier, node_id);
        Ok(NodeMut::new(self, node_id))
    }

    pub fn add_tuple_element(
        &mut self,
        index: u8,
        parent_node_id: NodeId,
    ) -> Result<NodeMut<'_>, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.node_mut(parent_node_id);
        let tuple = node.require_tuple()?;
        tuple.add_at(index, node_id)?;
        Ok(NodeMut::new(self, node_id))
    }

    pub fn add_array_element(
        &mut self,
        index: Option<usize>,
        parent_node_id: NodeId,
    ) -> Result<NodeMut<'_>, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.node_mut(parent_node_id);
        let array = node.require_array()?;
        if let Some(index) = index {
            array.add_at(index, node_id)?;
        } else {
            array.push(node_id)?;
        }
        Ok(NodeMut::new(self, node_id))
    }

    /// Resolves a path segment to a node ID, creating if necessary.
    ///
    /// This operation is idempotent for most segments, reusing existing nodes.
    /// Exception: `ArrayIndex(None)` always creates a new array element (push operation).
    pub fn resolve_child_by_segment(
        &mut self,
        segment: PathSegment,
        parent_node_id: NodeId,
    ) -> Result<NodeMut<'_>, InsertErrorKind> {
        // 既存のノードを探す
        let node = self.node(parent_node_id);

        let existing = match &segment {
            PathSegment::Ident(identifier) => node
                .as_map()
                .and_then(|m| m.get(&ObjectKey::String(identifier.clone().into_string())))
                .copied(),
            PathSegment::Value(object_key) => {
                node.as_map().and_then(|m| m.get(object_key)).copied()
            }
            PathSegment::Extension(identifier) => node.get_extension(identifier),
            PathSegment::TupleIndex(index) => node.as_tuple().and_then(|t| t.get(*index as usize)),
            PathSegment::ArrayIndex(Some(index)) => node.as_array().and_then(|a| a.get(*index)),
            PathSegment::ArrayIndex(None) => None, // push always creates new
        };

        // 既存ノードがあればそれを返す
        if let Some(node_id) = existing {
            return Ok(NodeMut::new(self, node_id));
        }

        // なければ作成
        self.add_child_by_segment(segment, parent_node_id)
    }

    /// Convert a subtree of a document to a standalone document.
    pub fn node_subtree_to_document(&self, node_id: NodeId) -> EureDocument {
        let mut result = EureDocument::new();
        let root_id = result.get_root_id();
        self.copy_subtree(node_id, &mut result, root_id);
        result
    }

    pub fn copy_subtree(&self, src_id: NodeId, dst: &mut EureDocument, dst_id: NodeId) {
        let src_node = self.node(src_id);
        dst.node_mut(dst_id).content = src_node.content.clone();

        // Skip ALL extensions during literal comparison.
        // Extensions are schema metadata (like $variant, $deny-untagged, $optional, etc.)
        // and should not be part of the literal value comparison.
        // Literal types compare only the data structure, not metadata.

        // Copy children based on content type
        match &src_node.content {
            NodeValue::Array(arr) => {
                for &child_src_id in arr.iter() {
                    if let Ok(result) = dst.add_array_element(None, dst_id) {
                        let child_dst_id = result.node_id;
                        self.copy_subtree(child_src_id, dst, child_dst_id);
                    }
                }
            }
            NodeValue::Tuple(tuple) => {
                for (idx, &child_src_id) in tuple.iter().enumerate() {
                    if let Ok(result) = dst.add_tuple_element(idx as u8, dst_id) {
                        let child_dst_id = result.node_id;
                        self.copy_subtree(child_src_id, dst, child_dst_id);
                    }
                }
            }
            NodeValue::Map(map) => {
                for (key, &child_src_id) in map.iter() {
                    if let Ok(result) = dst.add_map_child(key.clone(), dst_id) {
                        let child_dst_id = result.node_id;
                        self.copy_subtree(child_src_id, dst, child_dst_id);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Commands
impl EureDocument {
    pub fn replace_with_primitive(&mut self, value: PrimitiveValue) -> Result<(), InsertErrorKind> {
        self.nodes.clear();
        self.nodes[self.root.0].content = NodeValue::Primitive(value);
        Ok(())
    }

    pub fn reset_as_map(&mut self) -> Result<(), InsertErrorKind> {
        self.nodes.clear();
        self.nodes[self.root.0].content = NodeValue::Map(Default::default());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(s: &str) -> Identifier {
        s.parse().unwrap()
    }

    #[test]
    fn test_add_map_child_success() {
        let mut doc = EureDocument::new();
        let map_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_map())
        };
        let key = ObjectKey::String("test_key".to_string());

        let child_id = doc
            .add_map_child(key.clone(), map_id)
            .expect("Failed to add map child")
            .node_id;

        let map = doc.node(map_id).as_map().expect("Expected map");
        assert_eq!(map.get(&key), Some(&child_id));
    }

    #[test]
    fn test_add_map_child_error_expected_map() {
        let mut doc = EureDocument::new();
        let primitive_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::Primitive(PrimitiveValue::Null))
        };
        let key = ObjectKey::String("test".to_string());

        let result = doc.add_map_child(key, primitive_id);
        assert_eq!(result.err(), Some(InsertErrorKind::ExpectedMap));
    }

    #[test]
    fn test_add_map_child_error_already_assigned() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let key = ObjectKey::String("test".to_string());

        let _result1 = doc
            .add_map_child(key.clone(), root_id)
            .expect("First add should succeed");

        let result2 = doc.add_map_child(key.clone(), root_id);
        assert_eq!(
            result2.err(),
            Some(InsertErrorKind::AlreadyAssigned { key })
        );
    }

    #[test]
    fn test_add_extension_success_multiple() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let id1 = identifier("ext1");
        let id2 = identifier("ext2");

        let node_id1 = doc
            .add_extension(id1.clone(), root_id)
            .expect("Failed to add extension")
            .node_id;

        let node_id2 = doc
            .add_extension(id2.clone(), root_id)
            .expect("Failed to add extension")
            .node_id;

        let node = doc.node(root_id);
        assert_eq!(node.extensions.get(&id1), Some(&node_id1));
        assert_eq!(node.extensions.get(&id2), Some(&node_id2));
    }

    #[test]
    fn test_add_extension_success() {
        let mut doc = EureDocument::new();
        let primitive_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::Primitive(PrimitiveValue::Null))
        };
        let identifier = identifier("ext");

        let node_id = doc
            .add_extension(identifier.clone(), primitive_id)
            .expect("Failed to add extension")
            .node_id;

        let node = doc.node(primitive_id);
        assert_eq!(node.extensions.get(&identifier), Some(&node_id));
    }

    #[test]
    fn test_add_extension_error_already_assigned() {
        let mut doc = EureDocument::new();
        let map_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_map())
        };
        let identifier = identifier("ext");

        let _result1 = doc
            .add_extension(identifier.clone(), map_id)
            .expect("First add should succeed");

        let result2 = doc.add_extension(identifier.clone(), map_id);
        assert_eq!(
            result2.err(),
            Some(InsertErrorKind::AlreadyAssignedExtension { identifier })
        );
    }

    #[test]
    fn test_add_tuple_element_success_index_0() {
        let mut doc = EureDocument::new();
        let tuple_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_tuple())
        };

        let node_id = doc
            .add_tuple_element(0, tuple_id)
            .expect("Failed to add tuple element")
            .node_id;

        let tuple = doc.node(tuple_id).as_tuple().expect("Expected tuple");
        assert_eq!(tuple.to_vec(), vec![node_id]);
    }

    #[test]
    fn test_add_tuple_element_success_sequential() {
        let mut doc = EureDocument::new();
        let tuple_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_tuple())
        };

        let node_id1 = doc
            .add_tuple_element(0, tuple_id)
            .expect("Failed to add tuple element")
            .node_id;

        let node_id2 = doc
            .add_tuple_element(1, tuple_id)
            .expect("Failed to add tuple element")
            .node_id;

        let tuple = doc.node(tuple_id).as_tuple().expect("Expected tuple");
        assert_eq!(tuple.to_vec(), vec![node_id1, node_id2]);
    }

    #[test]
    fn test_add_tuple_element_error_expected_tuple() {
        let mut doc = EureDocument::new();
        let map_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_map())
        };

        let result = doc.add_tuple_element(0, map_id);
        assert_eq!(result.err(), Some(InsertErrorKind::ExpectedTuple));
    }

    #[test]
    fn test_add_tuple_element_error_invalid_index() {
        let mut doc = EureDocument::new();
        let tuple_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_tuple())
        };

        let result = doc.add_tuple_element(1, tuple_id);
        assert_eq!(
            result.err(),
            Some(InsertErrorKind::TupleIndexInvalid {
                index: 1,
                expected_index: 0
            })
        );
    }

    #[test]
    fn test_add_array_element_success_push() {
        let mut doc = EureDocument::new();
        let array_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_array())
        };

        let node_id = doc
            .add_array_element(None, array_id)
            .expect("Failed to add array element")
            .node_id;

        let array = doc.node(array_id).as_array().expect("Expected array");
        assert_eq!(array.to_vec(), vec![node_id]);
    }

    #[test]
    fn test_add_array_element_success_at_index() {
        let mut doc = EureDocument::new();
        let array_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_array())
        };

        let node_id1 = doc
            .add_array_element(Some(0), array_id)
            .expect("Failed to add array element")
            .node_id;

        let node_id2 = doc
            .add_array_element(Some(1), array_id)
            .expect("Failed to add array element")
            .node_id;

        let array = doc.node(array_id).as_array().expect("Expected array");
        assert_eq!(array.to_vec(), vec![node_id1, node_id2]);
    }

    #[test]
    fn test_add_array_element_error_expected_array() {
        let mut doc = EureDocument::new();
        let map_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_map())
        };

        let result = doc.add_array_element(None, map_id);
        assert_eq!(result.err(), Some(InsertErrorKind::ExpectedArray));
    }

    #[test]
    fn test_add_array_element_error_invalid_index() {
        let mut doc = EureDocument::new();
        let array_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_array())
        };

        let result = doc.add_array_element(Some(1), array_id);
        assert_eq!(
            result.err(),
            Some(InsertErrorKind::ArrayIndexInvalid {
                index: 1,
                expected_index: 0
            })
        );
    }

    #[test]
    fn test_add_child_by_segment_ident() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = identifier("test");
        let segment = PathSegment::Ident(identifier.clone());

        let result = doc.add_child_by_segment(segment, root_id);
        assert!(result.is_ok());

        let map = doc.node(root_id).as_map().expect("Expected map");
        let key = ObjectKey::String(identifier.into_string());
        assert!(map.get(&key).is_some());
    }

    #[test]
    fn test_add_child_by_segment_value() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let key = ObjectKey::String("test".to_string());
        let segment = PathSegment::Value(key.clone());

        let result = doc.add_child_by_segment(segment, root_id);
        assert!(result.is_ok());

        let map = doc.node(root_id).as_map().expect("Expected map");
        assert!(map.get(&key).is_some());
    }

    #[test]
    fn test_add_child_by_segment_extension() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = identifier("ext");
        let segment = PathSegment::Extension(identifier.clone());

        let result = doc.add_child_by_segment(segment, root_id);
        assert!(result.is_ok());

        let node = doc.node(root_id);
        assert!(node.extensions.contains_key(&identifier));
    }

    #[test]
    fn test_add_child_by_segment_tuple_index() {
        let mut doc = EureDocument::new();
        let tuple_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_tuple())
        };
        let segment = PathSegment::TupleIndex(0);

        let result = doc.add_child_by_segment(segment, tuple_id);
        assert!(result.is_ok());

        let tuple = doc.node(tuple_id).as_tuple().expect("Expected tuple");
        assert_eq!(tuple.len(), 1);
    }

    #[test]
    fn test_add_child_by_segment_array_index_none() {
        let mut doc = EureDocument::new();
        let array_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_array())
        };
        let segment = PathSegment::ArrayIndex(None);

        let result = doc.add_child_by_segment(segment, array_id);
        assert!(result.is_ok());

        let array = doc.node(array_id).as_array().expect("Expected array");
        assert_eq!(array.len(), 1);
    }

    #[test]
    fn test_add_child_by_segment_array_index_some() {
        let mut doc = EureDocument::new();
        let array_id = {
            let doc: &mut EureDocument = &mut doc;
            doc.create_node(NodeValue::empty_array())
        };
        let segment = PathSegment::ArrayIndex(Some(0));

        let result = doc.add_child_by_segment(segment, array_id);
        assert!(result.is_ok());

        let array = doc.node(array_id).as_array().expect("Expected array");
        assert_eq!(array.len(), 1);
    }

    #[test]
    fn test_resolve_ident_idempotent() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = identifier("field");

        // First call - creates new node
        let node_id1 = doc
            .resolve_child_by_segment(PathSegment::Ident(identifier.clone()), root_id)
            .expect("First call failed")
            .node_id;

        // Second call - returns existing node
        let node_id2 = doc
            .resolve_child_by_segment(PathSegment::Ident(identifier), root_id)
            .expect("Second call failed")
            .node_id;

        assert_eq!(node_id1, node_id2);
    }

    #[test]
    fn test_resolve_value_idempotent() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let object_key = ObjectKey::String("key".to_string());

        // First call - creates new node
        let node_id1 = doc
            .resolve_child_by_segment(PathSegment::Value(object_key.clone()), root_id)
            .expect("First call failed")
            .node_id;

        // Second call - returns existing node
        let node_id2 = doc
            .resolve_child_by_segment(PathSegment::Value(object_key), root_id)
            .expect("Second call failed")
            .node_id;

        assert_eq!(node_id1, node_id2);
    }

    #[test]
    fn test_resolve_extension_idempotent() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = identifier("ext");

        // First call - creates new node
        let node_id1 = doc
            .resolve_child_by_segment(PathSegment::Extension(identifier.clone()), root_id)
            .expect("First call failed")
            .node_id;

        // Second call - returns existing node
        let node_id2 = doc
            .resolve_child_by_segment(PathSegment::Extension(identifier), root_id)
            .expect("Second call failed")
            .node_id;

        assert_eq!(node_id1, node_id2);
    }

    #[test]
    fn test_resolve_tuple_index_idempotent() {
        let mut doc = EureDocument::new();
        let parent_id = doc.create_node_uninitialized();

        // First call - creates new node
        let node_id1 = doc
            .resolve_child_by_segment(PathSegment::TupleIndex(0), parent_id)
            .expect("First call failed")
            .node_id;

        // Second call - returns existing node
        let node_id2 = doc
            .resolve_child_by_segment(PathSegment::TupleIndex(0), parent_id)
            .expect("Second call failed")
            .node_id;

        assert_eq!(node_id1, node_id2);
    }

    #[test]
    fn test_resolve_array_index_some_idempotent() {
        let mut doc = EureDocument::new();
        let parent_id = doc.create_node_uninitialized();

        // First call - creates new node
        let node_id1 = doc
            .resolve_child_by_segment(PathSegment::ArrayIndex(Some(0)), parent_id)
            .expect("First call failed")
            .node_id;

        // Second call - returns existing node
        let node_id2 = doc
            .resolve_child_by_segment(PathSegment::ArrayIndex(Some(0)), parent_id)
            .expect("Second call failed")
            .node_id;

        assert_eq!(node_id1, node_id2);
    }

    #[test]
    fn test_resolve_array_index_none_always_creates_new() {
        let mut doc = EureDocument::new();
        let parent_id = doc.create_node_uninitialized();

        // First call - creates new node
        let node_id1 = doc
            .resolve_child_by_segment(PathSegment::ArrayIndex(None), parent_id)
            .expect("First call failed")
            .node_id;

        // Second call - creates another new node (NOT idempotent)
        let node_id2 = doc
            .resolve_child_by_segment(PathSegment::ArrayIndex(None), parent_id)
            .expect("Second call failed")
            .node_id;

        // ArrayIndex(None) always creates new nodes (push operation)
        assert_ne!(node_id1, node_id2);

        // Verify both nodes exist in array
        let array = doc.node(parent_id).as_array().expect("Expected array");
        assert_eq!(array.len(), 2);
        assert_eq!(array.get(0).unwrap(), node_id1);
        assert_eq!(array.get(1).unwrap(), node_id2);
    }

    #[test]
    fn test_get_node_with_valid_id() {
        let mut doc = EureDocument::new();
        let node_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Null));

        let result = doc.get_node(node_id);
        assert!(result.is_some());

        let node = result.unwrap();
        assert_eq!(node.content, NodeValue::Primitive(PrimitiveValue::Null));
    }

    #[test]
    fn test_get_node_with_invalid_id() {
        let doc = EureDocument::new();
        // Create an invalid NodeId that's out of bounds
        let invalid_id = NodeId(9999);

        let result = doc.get_node(invalid_id);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_node_mut_with_valid_id() {
        let mut doc = EureDocument::new();
        let node_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Null));

        let result = doc.get_node_mut(node_id);
        assert!(result.is_some());

        // Verify we can mutate through the returned reference
        let node = result.unwrap();
        node.content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        // Verify the mutation persisted
        assert_eq!(
            doc.node(node_id).content,
            NodeValue::Primitive(PrimitiveValue::Bool(true))
        );
    }

    #[test]
    fn test_get_node_mut_with_invalid_id() {
        let mut doc = EureDocument::new();
        // Create an invalid NodeId that's out of bounds
        let invalid_id = NodeId(9999);

        let result = doc.get_node_mut(invalid_id);
        assert!(result.is_none());
    }

    #[test]
    fn test_partialeq_empty_documents() {
        let doc1 = EureDocument::new();
        let doc2 = EureDocument::new();
        assert_eq!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_primitive_documents() {
        let doc1 = EureDocument::new_primitive(PrimitiveValue::Bool(true));
        let doc2 = EureDocument::new_primitive(PrimitiveValue::Bool(true));
        let doc3 = EureDocument::new_primitive(PrimitiveValue::Bool(false));

        assert_eq!(doc1, doc2);
        assert_ne!(doc1, doc3);
    }

    #[test]
    fn test_partialeq_with_map_children() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        let root1 = doc1.get_root_id();
        let root2 = doc2.get_root_id();

        let key = ObjectKey::String("test".to_string());

        doc1.add_map_child(key.clone(), root1)
            .expect("Failed to add child");
        doc2.add_map_child(key.clone(), root2)
            .expect("Failed to add child");

        assert_eq!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_with_different_map_children() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        let root1 = doc1.get_root_id();
        let root2 = doc2.get_root_id();

        doc1.add_map_child(ObjectKey::String("key1".to_string()), root1)
            .expect("Failed to add child");
        doc2.add_map_child(ObjectKey::String("key2".to_string()), root2)
            .expect("Failed to add child");

        assert_ne!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_with_extensions() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        let root1 = doc1.get_root_id();
        let root2 = doc2.get_root_id();

        let ext_id = identifier("ext");

        doc1.add_extension(ext_id.clone(), root1)
            .expect("Failed to add extension");
        doc2.add_extension(ext_id.clone(), root2)
            .expect("Failed to add extension");

        assert_eq!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_with_different_extensions() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        let root1 = doc1.get_root_id();
        let root2 = doc2.get_root_id();

        doc1.add_extension(identifier("ext1"), root1)
            .expect("Failed to add extension");
        doc2.add_extension(identifier("ext2"), root2)
            .expect("Failed to add extension");

        assert_ne!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_with_arrays() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        // Create array in doc1
        let array_id1 = doc1.create_node(NodeValue::empty_array());
        doc1.add_array_element(None, array_id1)
            .expect("Failed to add array element");
        doc1.root = array_id1;

        // Create array in doc2
        let array_id2 = doc2.create_node(NodeValue::empty_array());
        doc2.add_array_element(None, array_id2)
            .expect("Failed to add array element");
        doc2.root = array_id2;

        assert_eq!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_with_tuples() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        // Create tuple in doc1
        let tuple_id1 = doc1.create_node(NodeValue::empty_tuple());
        doc1.add_tuple_element(0, tuple_id1)
            .expect("Failed to add tuple element");
        doc1.root = tuple_id1;

        // Create tuple in doc2
        let tuple_id2 = doc2.create_node(NodeValue::empty_tuple());
        doc2.add_tuple_element(0, tuple_id2)
            .expect("Failed to add tuple element");
        doc2.root = tuple_id2;

        assert_eq!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_nested_structure() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        // Create nested structure in doc1
        let root1 = doc1.get_root_id();
        let child1 = doc1
            .add_map_child(ObjectKey::String("child".to_string()), root1)
            .expect("Failed to add child")
            .node_id;
        doc1.node_mut(child1).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        // Create nested structure in doc2
        let root2 = doc2.get_root_id();
        let child2 = doc2
            .add_map_child(ObjectKey::String("child".to_string()), root2)
            .expect("Failed to add child")
            .node_id;
        doc2.node_mut(child2).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        assert_eq!(doc1, doc2);
    }

    #[test]
    fn test_partialeq_ignores_node_id_values() {
        let mut doc1 = EureDocument::new();
        let mut doc2 = EureDocument::new();

        // Create a more complex structure in doc1
        let root1 = doc1.get_root_id();
        let _intermediate = doc1.create_node(NodeValue::Primitive(PrimitiveValue::Null));
        let child1 = doc1
            .add_map_child(ObjectKey::String("key".to_string()), root1)
            .expect("Failed")
            .node_id;

        // Create the same structure in doc2 (without intermediate node)
        let root2 = doc2.get_root_id();
        let child2 = doc2
            .add_map_child(ObjectKey::String("key".to_string()), root2)
            .expect("Failed")
            .node_id;

        // Even though child1 and child2 have different NodeId values,
        // the structures should be equal
        assert_eq!(doc1, doc2);

        // Verify that NodeIds are actually different
        assert_ne!(child1.0, child2.0);
    }
}
