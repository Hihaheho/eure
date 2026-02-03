pub mod constructor;
pub mod interpreter_sink;
pub mod node;
pub mod source_constructor;

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
    #[error("Scope error: {0}")]
    ScopeError(#[from] constructor::ScopeError),
    #[error("Constructor error: {0}")]
    ConstructorError(#[from] ConstructorError),
}

/// Protocol errors for SourceConstructor operations.
#[derive(Debug, PartialEq, thiserror::Error, Clone)]
pub enum ConstructorError {
    #[error("set_block_value called without a preceding bind operation")]
    MissingBindBeforeSetBlockValue,
    #[error("end_binding_value called without a preceding bind operation")]
    MissingBindBeforeEndBindingValue,
    #[error("end_binding_block called without a preceding end_eure_block")]
    MissingEndEureBlockBeforeEndBindingBlock,
    #[error("end_section_block called without a preceding end_eure_block")]
    MissingEndEureBlockBeforeEndSectionBlock,
    #[error("end_eure_block called but builder stack is not in EureBlock state")]
    InvalidBuilderStackForEndEureBlock,
    #[error("end_section_items called but builder stack is not in SectionItems state")]
    InvalidBuilderStackForEndSectionItems,
    #[error("ArrayIndex must follow a key segment; standalone [] is not valid")]
    StandaloneArrayIndex,
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

        // Skip ALL extensions during literal comparison.
        // Extensions are schema metadata (like $variant, $deny-untagged, $optional, etc.)
        // and should not be part of the literal value comparison.
        // Literal types compare only the data structure, not metadata.

        // Copy content based on type. For containers, we must NOT clone the content
        // directly because it contains NodeIds from the source document. Instead,
        // create empty containers and populate with recursively copied children.
        match &src_node.content {
            NodeValue::Hole(label) => {
                dst.node_mut(dst_id).content = NodeValue::Hole(label.clone());
            }
            NodeValue::Primitive(p) => {
                dst.node_mut(dst_id).content = NodeValue::Primitive(p.clone());
            }
            NodeValue::Array(arr) => {
                dst.node_mut(dst_id).content = NodeValue::empty_array();
                for &child_src_id in arr.iter() {
                    if let Ok(result) = dst.add_array_element(None, dst_id) {
                        let child_dst_id = result.node_id;
                        self.copy_subtree(child_src_id, dst, child_dst_id);
                    }
                }
            }
            NodeValue::Tuple(tuple) => {
                dst.node_mut(dst_id).content = NodeValue::empty_tuple();
                for (idx, &child_src_id) in tuple.iter().enumerate() {
                    if let Ok(result) = dst.add_tuple_element(idx as u8, dst_id) {
                        let child_dst_id = result.node_id;
                        self.copy_subtree(child_src_id, dst, child_dst_id);
                    }
                }
            }
            NodeValue::Map(map) => {
                dst.node_mut(dst_id).content = NodeValue::empty_map();
                for (key, &child_src_id) in map.iter() {
                    if let Ok(result) = dst.add_map_child(key.clone(), dst_id) {
                        let child_dst_id = result.node_id;
                        self.copy_subtree(child_src_id, dst, child_dst_id);
                    }
                }
            }
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

    #[test]
    fn test_require_map_converts_hole() {
        let mut doc = EureDocument::new();
        let node_id = doc.create_node(NodeValue::hole());

        assert!(doc.node(node_id).content.is_hole());

        {
            let node = doc.node_mut(node_id);
            let _map = node.require_map().expect("Should convert to map");
        }

        assert!(doc.node(node_id).as_map().is_some());
    }

    #[test]
    fn test_require_array_converts_hole() {
        let mut doc = EureDocument::new();
        let node_id = doc.create_node(NodeValue::hole());

        assert!(doc.node(node_id).content.is_hole());

        {
            let node = doc.node_mut(node_id);
            let _array = node.require_array().expect("Should convert to array");
        }

        assert!(doc.node(node_id).as_array().is_some());
    }

    #[test]
    fn test_require_tuple_converts_hole() {
        let mut doc = EureDocument::new();
        let node_id = doc.create_node(NodeValue::hole());

        assert!(doc.node(node_id).content.is_hole());

        {
            let node = doc.node_mut(node_id);
            let _tuple = node.require_tuple().expect("Should convert to tuple");
        }

        assert!(doc.node(node_id).as_tuple().is_some());
    }

    #[test]
    fn test_require_methods_fail_on_wrong_type() {
        let mut doc = EureDocument::new();
        let primitive_id = doc.create_node(NodeValue::Primitive(PrimitiveValue::Null));

        let node = doc.node_mut(primitive_id);
        assert_eq!(node.require_map().err(), Some(InsertErrorKind::ExpectedMap));

        let node = doc.node_mut(primitive_id);
        assert_eq!(
            node.require_array().err(),
            Some(InsertErrorKind::ExpectedArray)
        );

        let node = doc.node_mut(primitive_id);
        assert_eq!(
            node.require_tuple().err(),
            Some(InsertErrorKind::ExpectedTuple)
        );
    }
}

#[cfg(test)]
mod proptests {
    extern crate std;

    use super::*;
    use proptest::prelude::*;
    use std::vec::Vec;

    // =========================================================================
    // Strategy generators
    // =========================================================================

    /// Characters valid as the first character of an identifier (XID_Start or underscore).
    fn xid_start_char() -> impl Strategy<Value = char> {
        prop_oneof![
            prop::char::range('a', 'z'),
            prop::char::range('A', 'Z'),
            Just('_'),
            Just('α'),
            Just('日'),
        ]
    }

    /// Characters valid in the continuation of an identifier (XID_Continue or hyphen).
    fn xid_continue_char() -> impl Strategy<Value = char> {
        prop_oneof![
            prop::char::range('a', 'z'),
            prop::char::range('A', 'Z'),
            prop::char::range('0', '9'),
            Just('_'),
            Just('-'),
            Just('α'),
            Just('日'),
        ]
    }

    /// Strategy for generating valid identifiers with broader character coverage.
    fn arb_identifier() -> impl Strategy<Value = Identifier> {
        (
            xid_start_char(),
            proptest::collection::vec(xid_continue_char(), 0..15),
        )
            .prop_map(|(first, rest)| {
                let mut s = alloc::string::String::with_capacity(1 + rest.len());
                s.push(first);
                s.extend(rest);
                s
            })
            .prop_filter_map("valid identifier", |s| s.parse::<Identifier>().ok())
    }

    /// Strategy for generating object keys with broader coverage.
    fn arb_object_key() -> impl Strategy<Value = ObjectKey> {
        prop_oneof![
            // Identifier-style string keys (broader range)
            arb_identifier().prop_map(|id| ObjectKey::String(id.to_string())),
            // Numeric keys (including negative)
            (-1000i64..1000).prop_map(|n| ObjectKey::Number(n.into())),
        ]
    }

    /// Strategy for generating primitive values.
    fn arb_primitive_value() -> impl Strategy<Value = PrimitiveValue> {
        prop_oneof![
            Just(PrimitiveValue::Null),
            proptest::bool::ANY.prop_map(PrimitiveValue::Bool),
            (-1000i64..1000).prop_map(|n| PrimitiveValue::Integer(n.into())),
            proptest::num::f64::NORMAL.prop_map(PrimitiveValue::F64),
            "[a-zA-Z0-9 ]{0,50}".prop_map(|s| PrimitiveValue::Text(Text::plaintext(s))),
        ]
    }

    // =========================================================================
    // resolve_child_by_segment idempotency tests
    // =========================================================================

    proptest! {
        /// Invariant: resolve_child_by_segment is idempotent for Ident segments.
        /// Calling twice with the same identifier returns the same NodeId.
        #[test]
        fn resolve_ident_is_idempotent(ident in arb_identifier()) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            let node_id1 = doc
                .resolve_child_by_segment(PathSegment::Ident(ident.clone()), root_id)
                .expect("First resolve failed")
                .node_id;

            let node_id2 = doc
                .resolve_child_by_segment(PathSegment::Ident(ident), root_id)
                .expect("Second resolve failed")
                .node_id;

            prop_assert_eq!(node_id1, node_id2, "Ident resolution should be idempotent");
        }

        /// Invariant: resolve_child_by_segment is idempotent for Value segments.
        #[test]
        fn resolve_value_is_idempotent(key in arb_object_key()) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            let node_id1 = doc
                .resolve_child_by_segment(PathSegment::Value(key.clone()), root_id)
                .expect("First resolve failed")
                .node_id;

            let node_id2 = doc
                .resolve_child_by_segment(PathSegment::Value(key), root_id)
                .expect("Second resolve failed")
                .node_id;

            prop_assert_eq!(node_id1, node_id2, "Value resolution should be idempotent");
        }

        /// Invariant: resolve_child_by_segment is idempotent for Extension segments.
        #[test]
        fn resolve_extension_is_idempotent(ident in arb_identifier()) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            let node_id1 = doc
                .resolve_child_by_segment(PathSegment::Extension(ident.clone()), root_id)
                .expect("First resolve failed")
                .node_id;

            let node_id2 = doc
                .resolve_child_by_segment(PathSegment::Extension(ident), root_id)
                .expect("Second resolve failed")
                .node_id;

            prop_assert_eq!(node_id1, node_id2, "Extension resolution should be idempotent");
        }

        /// Invariant: resolve_child_by_segment is idempotent for TupleIndex segments.
        #[test]
        fn resolve_tuple_index_is_idempotent(index in 0u8..10) {
            let mut doc = EureDocument::new();
            let parent_id = doc.create_node_uninitialized();

            // First add indices sequentially up to `index`
            for i in 0..index {
                doc.add_tuple_element(i, parent_id).expect("Sequential add failed");
            }

            // Now resolve the next index
            let node_id1 = doc
                .resolve_child_by_segment(PathSegment::TupleIndex(index), parent_id)
                .expect("First resolve failed")
                .node_id;

            let node_id2 = doc
                .resolve_child_by_segment(PathSegment::TupleIndex(index), parent_id)
                .expect("Second resolve failed")
                .node_id;

            prop_assert_eq!(node_id1, node_id2, "TupleIndex resolution should be idempotent");
        }

        /// Invariant: resolve_child_by_segment is idempotent for ArrayIndex(Some(n)) segments.
        #[test]
        fn resolve_array_index_some_is_idempotent(index in 0usize..10) {
            let mut doc = EureDocument::new();
            let parent_id = doc.create_node_uninitialized();

            // First add indices sequentially up to `index`
            for i in 0..index {
                doc.add_array_element(Some(i), parent_id).expect("Sequential add failed");
            }

            // Now resolve the next index
            let node_id1 = doc
                .resolve_child_by_segment(PathSegment::ArrayIndex(Some(index)), parent_id)
                .expect("First resolve failed")
                .node_id;

            let node_id2 = doc
                .resolve_child_by_segment(PathSegment::ArrayIndex(Some(index)), parent_id)
                .expect("Second resolve failed")
                .node_id;

            prop_assert_eq!(node_id1, node_id2, "ArrayIndex(Some) resolution should be idempotent");
        }

        /// Invariant: ArrayIndex(None) always creates new elements (NOT idempotent - push behavior).
        #[test]
        fn resolve_array_index_none_always_creates_new(count in 1usize..10) {
            let mut doc = EureDocument::new();
            let parent_id = doc.create_node_uninitialized();

            let mut node_ids = Vec::new();
            for _ in 0..count {
                let node_id = doc
                    .resolve_child_by_segment(PathSegment::ArrayIndex(None), parent_id)
                    .expect("Resolve failed")
                    .node_id;
                node_ids.push(node_id);
            }

            // All node IDs should be unique
            for i in 0..node_ids.len() {
                for j in (i+1)..node_ids.len() {
                    prop_assert_ne!(node_ids[i], node_ids[j],
                        "ArrayIndex(None) should create unique nodes");
                }
            }

            // Array length should match push count
            let array = doc.node(parent_id).as_array().expect("Expected array");
            prop_assert_eq!(array.len(), count, "Array length should match push count");
        }

        /// Error: ArrayIndex segment on non-array parent fails.
        #[test]
        fn resolve_array_index_on_map_fails(index in 0usize..10) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id(); // Root starts as a hole/map

            // Make root explicitly a map
            doc.node_mut(root_id).content = NodeValue::empty_map();

            let result = doc.resolve_child_by_segment(PathSegment::ArrayIndex(Some(index)), root_id);
            prop_assert!(result.is_err(), "ArrayIndex on map should fail");
            prop_assert_eq!(result.err(), Some(InsertErrorKind::ExpectedArray));
        }

        /// Error: TupleIndex segment on non-tuple parent fails.
        #[test]
        fn resolve_tuple_index_on_array_fails(index in 0u8..10) {
            let mut doc = EureDocument::new();
            let parent_id = doc.create_node(NodeValue::empty_array());

            let result = doc.resolve_child_by_segment(PathSegment::TupleIndex(index), parent_id);
            prop_assert!(result.is_err(), "TupleIndex on array should fail");
            prop_assert_eq!(result.err(), Some(InsertErrorKind::ExpectedTuple));
        }

        /// Error: ArrayIndex segment on primitive fails.
        #[test]
        fn resolve_array_index_on_primitive_fails(value in arb_primitive_value()) {
            let mut doc = EureDocument::new();
            let node_id = doc.create_node(NodeValue::Primitive(value));

            let result = doc.resolve_child_by_segment(PathSegment::ArrayIndex(Some(0)), node_id);
            prop_assert!(result.is_err(), "ArrayIndex on primitive should fail");
            prop_assert_eq!(result.err(), Some(InsertErrorKind::ExpectedArray));
        }

        /// Error: Non-sequential ArrayIndex fails.
        #[test]
        fn resolve_array_index_non_sequential_fails(skip in 1usize..10) {
            let mut doc = EureDocument::new();
            let parent_id = doc.create_node(NodeValue::empty_array());

            // Try to add at index `skip` without filling 0..skip first
            let result = doc.resolve_child_by_segment(PathSegment::ArrayIndex(Some(skip)), parent_id);
            prop_assert!(result.is_err(), "Non-sequential ArrayIndex should fail");

            match result.err() {
                Some(InsertErrorKind::ArrayIndexInvalid { index, expected_index }) => {
                    prop_assert_eq!(index, skip);
                    prop_assert_eq!(expected_index, 0);
                }
                other => prop_assert!(false, "Expected ArrayIndexInvalid, got {:?}", other),
            }
        }

        /// Error: Non-sequential TupleIndex fails.
        #[test]
        fn resolve_tuple_index_non_sequential_fails(skip in 1u8..10) {
            let mut doc = EureDocument::new();
            let parent_id = doc.create_node(NodeValue::empty_tuple());

            let result = doc.resolve_child_by_segment(PathSegment::TupleIndex(skip), parent_id);
            prop_assert!(result.is_err(), "Non-sequential TupleIndex should fail");

            match result.err() {
                Some(InsertErrorKind::TupleIndexInvalid { index, expected_index }) => {
                    prop_assert_eq!(index, skip);
                    prop_assert_eq!(expected_index, 0);
                }
                other => prop_assert!(false, "Expected TupleIndexInvalid, got {:?}", other),
            }
        }
    }

    // =========================================================================
    // Extension uniqueness tests
    // =========================================================================

    proptest! {
        /// Invariant: Multiple different extensions can be added to a single node.
        #[test]
        fn multiple_different_extensions_allowed(
            ext1 in arb_identifier(),
            ext2 in arb_identifier(),
        ) {
            prop_assume!(ext1 != ext2);

            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            let node_id1 = doc.add_extension(ext1.clone(), root_id)
                .expect("First extension failed")
                .node_id;
            let node_id2 = doc.add_extension(ext2.clone(), root_id)
                .expect("Second extension failed")
                .node_id;

            let node = doc.node(root_id);
            prop_assert_eq!(node.extensions.get(&ext1), Some(&node_id1));
            prop_assert_eq!(node.extensions.get(&ext2), Some(&node_id2));
        }

        /// Invariant: Duplicate extension identifier fails with AlreadyAssignedExtension.
        #[test]
        fn duplicate_extension_fails(ext in arb_identifier()) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            let _first = doc.add_extension(ext.clone(), root_id)
                .expect("First extension should succeed");

            let result = doc.add_extension(ext.clone(), root_id);
            prop_assert_eq!(
                result.err(),
                Some(InsertErrorKind::AlreadyAssignedExtension { identifier: ext }),
                "Duplicate extension should fail"
            );
        }
    }

    // =========================================================================
    // Document equality tests
    // =========================================================================

    proptest! {
        /// Invariant: Two documents with same structure are equal even with different NodeIds.
        #[test]
        fn document_equality_ignores_node_ids(
            keys in proptest::collection::vec(arb_object_key(), 1..5)
                .prop_filter("unique keys", |keys| {
                    let unique: std::collections::HashSet<_> = keys.iter().collect();
                    unique.len() == keys.len()
                }),
        ) {
            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            // Add an unrelated node to doc1 to offset NodeIds
            let _ = doc1.create_node(NodeValue::Primitive(PrimitiveValue::Null));

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            // Add same children to both documents
            for key in &keys {
                doc1.add_map_child(key.clone(), root1).expect("Add failed");
                doc2.add_map_child(key.clone(), root2).expect("Add failed");
            }

            prop_assert_eq!(doc1, doc2, "Documents with same structure should be equal");
        }

        /// Invariant: Reflexive equality - a document equals itself.
        #[test]
        fn document_equality_reflexive(
            keys in proptest::collection::vec(arb_object_key(), 0..5)
                .prop_filter("unique keys", |keys| {
                    let unique: std::collections::HashSet<_> = keys.iter().collect();
                    unique.len() == keys.len()
                }),
        ) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            for key in &keys {
                doc.add_map_child(key.clone(), root_id).expect("Add failed");
            }

            prop_assert_eq!(&doc, &doc, "Document should equal itself");
        }

        /// Invariant: Documents with different content are not equal.
        #[test]
        fn document_equality_different_content(
            key1 in arb_object_key(),
            key2 in arb_object_key(),
        ) {
            prop_assume!(key1 != key2);

            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            doc1.add_map_child(key1, root1).expect("Add failed");
            doc2.add_map_child(key2, root2).expect("Add failed");

            prop_assert_ne!(doc1, doc2, "Documents with different keys should not be equal");
        }

        /// Invariant: Document equality works for arrays.
        #[test]
        fn document_equality_for_arrays(count in 1usize..10) {
            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            // Offset NodeIds in doc1
            let _ = doc1.create_node(NodeValue::Primitive(PrimitiveValue::Null));

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            // Convert roots to arrays
            doc1.node_mut(root1).content = NodeValue::empty_array();
            doc2.node_mut(root2).content = NodeValue::empty_array();

            for _ in 0..count {
                doc1.add_array_element(None, root1).expect("Add failed");
                doc2.add_array_element(None, root2).expect("Add failed");
            }

            prop_assert_eq!(doc1, doc2, "Documents with same array structure should be equal");
        }

        /// Invariant: Document equality works for tuples.
        #[test]
        fn document_equality_for_tuples(count in 1u8..10) {
            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            let _ = doc1.create_node(NodeValue::Primitive(PrimitiveValue::Null));

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            doc1.node_mut(root1).content = NodeValue::empty_tuple();
            doc2.node_mut(root2).content = NodeValue::empty_tuple();

            for i in 0..count {
                doc1.add_tuple_element(i, root1).expect("Add failed");
                doc2.add_tuple_element(i, root2).expect("Add failed");
            }

            prop_assert_eq!(doc1, doc2, "Documents with same tuple structure should be equal");
        }

        /// Invariant: Document equality works for primitive values.
        #[test]
        fn document_equality_for_primitives(value in arb_primitive_value()) {
            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            let _ = doc1.create_node(NodeValue::Primitive(PrimitiveValue::Null));

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            doc1.node_mut(root1).content = NodeValue::Primitive(value.clone());
            doc2.node_mut(root2).content = NodeValue::Primitive(value);

            prop_assert_eq!(doc1, doc2, "Documents with same primitive value should be equal");
        }

        /// Invariant: Document equality considers extensions.
        #[test]
        fn document_equality_considers_extensions(
            ext_name in arb_identifier(),
            value in arb_primitive_value(),
        ) {
            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            let _ = doc1.create_node(NodeValue::Primitive(PrimitiveValue::Null));

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            // Add same extension to both
            let ext1_id = doc1.add_extension(ext_name.clone(), root1).expect("Add ext failed").node_id;
            let ext2_id = doc2.add_extension(ext_name.clone(), root2).expect("Add ext failed").node_id;

            // Set same value in extensions
            doc1.node_mut(ext1_id).content = NodeValue::Primitive(value.clone());
            doc2.node_mut(ext2_id).content = NodeValue::Primitive(value);

            prop_assert_eq!(doc1, doc2, "Documents with same extensions should be equal");
        }

        /// Invariant: Documents with different extensions are not equal.
        #[test]
        fn document_equality_different_extensions(
            ext1 in arb_identifier(),
            ext2 in arb_identifier(),
        ) {
            prop_assume!(ext1 != ext2);

            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            doc1.add_extension(ext1, root1).expect("Add ext failed");
            doc2.add_extension(ext2, root2).expect("Add ext failed");

            prop_assert_ne!(doc1, doc2, "Documents with different extensions should not be equal");
        }
    }

    // =========================================================================
    // Copy subtree tests
    // =========================================================================

    proptest! {
        /// Invariant: copy_subtree preserves primitive values.
        #[test]
        fn copy_subtree_preserves_primitive(value in arb_primitive_value()) {
            let mut src = EureDocument::new();
            let src_id = src.create_node(NodeValue::Primitive(value.clone()));

            let dst = src.node_subtree_to_document(src_id);

            let dst_node = dst.root();
            match &dst_node.content {
                NodeValue::Primitive(copied_value) => {
                    prop_assert_eq!(copied_value, &value, "Primitive value should be preserved");
                }
                other => {
                    prop_assert!(false, "Expected Primitive, got {:?}", other);
                }
            }
        }

        /// Invariant: copy_subtree for arrays must create valid NodeIds in destination.
        /// All NodeIds stored in the copied array must exist in the destination document.
        #[test]
        fn copy_subtree_array_has_valid_node_ids(count in 1usize..10) {
            let mut src = EureDocument::new();
            let array_id = src.create_node(NodeValue::empty_array());

            for _ in 0..count {
                src.add_array_element(None, array_id).expect("Add failed");
            }

            let dst = src.node_subtree_to_document(array_id);

            // The destination array should have exactly `count` elements
            let dst_array = dst.root().as_array().expect("Expected array");
            prop_assert_eq!(dst_array.len(), count, "Copied array should have correct length");

            // All NodeIds in the destination array must be valid
            for i in 0..dst_array.len() {
                let child_id = dst_array.get(i).expect("Should have element");
                prop_assert!(
                    dst.get_node(child_id).is_some(),
                    "NodeId {:?} at index {} must exist in destination", child_id, i
                );
            }
        }

        /// Invariant: copy_subtree for maps must create valid NodeIds in destination.
        #[test]
        fn copy_subtree_map_has_valid_node_ids(
            keys in proptest::collection::vec(arb_object_key(), 1..5)
                .prop_filter("unique keys", |keys| {
                    let unique: std::collections::HashSet<_> = keys.iter().collect();
                    unique.len() == keys.len()
                }),
        ) {
            let mut src = EureDocument::new();
            let map_id = src.create_node(NodeValue::empty_map());

            for key in &keys {
                src.add_map_child(key.clone(), map_id).expect("Add failed");
            }

            let dst = src.node_subtree_to_document(map_id);

            let dst_map = dst.root().as_map().expect("Expected map");
            prop_assert_eq!(dst_map.len(), keys.len(), "Copied map should have correct size");

            // All NodeIds in the destination map must be valid
            for (key, &child_id) in dst_map.iter() {
                prop_assert!(
                    dst.get_node(child_id).is_some(),
                    "NodeId {:?} for key {:?} must exist in destination", child_id, key
                );
            }
        }

        /// Invariant: copy_subtree for tuples must create valid NodeIds in destination.
        #[test]
        fn copy_subtree_tuple_has_valid_node_ids(count in 1u8..10) {
            let mut src = EureDocument::new();
            let tuple_id = src.create_node(NodeValue::empty_tuple());

            for i in 0..count {
                src.add_tuple_element(i, tuple_id).expect("Add failed");
            }

            let dst = src.node_subtree_to_document(tuple_id);

            let dst_tuple = dst.root().as_tuple().expect("Expected tuple");
            prop_assert_eq!(dst_tuple.len(), count as usize, "Copied tuple should have correct length");

            // All NodeIds in the destination tuple must be valid
            for i in 0..dst_tuple.len() {
                let child_id = dst_tuple.get(i).expect("Should have element");
                prop_assert!(
                    dst.get_node(child_id).is_some(),
                    "NodeId {:?} at index {} must exist in destination", child_id, i
                );
            }
        }

        /// Invariant: copy_subtree preserves nested mixed content (map → array → primitive).
        #[test]
        fn copy_subtree_preserves_nested_mixed_content(
            key in arb_object_key(),
            value in arb_primitive_value(),
        ) {
            let mut src = EureDocument::new();
            let map_id = src.create_node(NodeValue::empty_map());

            // Create map → array → primitive structure
            let child_id = src.add_map_child(key.clone(), map_id).expect("Add failed").node_id;
            src.node_mut(child_id).content = NodeValue::empty_array();
            let elem_id = src.add_array_element(None, child_id).expect("Add failed").node_id;
            src.node_mut(elem_id).content = NodeValue::Primitive(value.clone());

            let dst = src.node_subtree_to_document(map_id);

            // Verify structure
            let dst_map = dst.root().as_map().expect("Expected map");
            let dst_child_id = dst_map.get(&key).expect("Should have key");
            let dst_child = dst.get_node(*dst_child_id).expect("Child should exist");
            let dst_array = dst_child.as_array().expect("Expected array");
            prop_assert_eq!(dst_array.len(), 1);

            let dst_elem_id = dst_array.get(0).expect("Should have element");
            let dst_elem = dst.get_node(dst_elem_id).expect("Element should exist");
            prop_assert_eq!(
                &dst_elem.content,
                &NodeValue::Primitive(value),
                "Primitive value should be preserved in nested structure"
            );
        }
    }

    // =========================================================================
    // Map key uniqueness tests
    // =========================================================================

    proptest! {
        /// Invariant: Map keys must be unique; duplicate key fails with AlreadyAssigned.
        #[test]
        fn map_key_uniqueness(key in arb_object_key()) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            let _first = doc.add_map_child(key.clone(), root_id)
                .expect("First add should succeed");

            let result = doc.add_map_child(key.clone(), root_id);
            prop_assert_eq!(
                result.err(),
                Some(InsertErrorKind::AlreadyAssigned { key }),
                "Duplicate map key should fail"
            );
        }

        /// Invariant: Multiple different map keys can coexist.
        #[test]
        fn multiple_map_keys_allowed(
            keys in proptest::collection::vec(arb_object_key(), 2..10)
                .prop_filter("unique keys", |keys| {
                    let unique: std::collections::HashSet<_> = keys.iter().collect();
                    unique.len() == keys.len()
                })
        ) {
            let mut doc = EureDocument::new();
            let root_id = doc.get_root_id();

            for key in &keys {
                doc.add_map_child(key.clone(), root_id).expect("Add should succeed");
            }

            let map = doc.node(root_id).as_map().expect("Expected map");
            prop_assert_eq!(map.len(), keys.len(), "All unique keys should be present");
        }
    }

    // =========================================================================
    // Node validity tests
    // =========================================================================

    proptest! {
        /// Invariant: All created nodes can be accessed via get_node.
        #[test]
        fn created_nodes_are_accessible(count in 1usize..20) {
            let mut doc = EureDocument::new();
            let mut node_ids = Vec::new();

            for _ in 0..count {
                let id = doc.create_node_uninitialized();
                node_ids.push(id);
            }

            for id in node_ids {
                prop_assert!(doc.get_node(id).is_some(),
                    "Created node {:?} should be accessible", id);
            }
        }

        /// Invariant: Invalid NodeIds return None from get_node.
        #[test]
        fn invalid_node_ids_return_none(count in 1usize..10) {
            let mut doc = EureDocument::new();

            for _ in 0..count {
                doc.create_node_uninitialized();
            }

            // Access an invalid NodeId (beyond the nodes vector)
            let invalid_id = NodeId(count + 100);
            prop_assert!(doc.get_node(invalid_id).is_none(),
                "Invalid NodeId should return None");
        }

        /// Invariant: Root node is always accessible.
        #[test]
        fn root_is_always_accessible(count in 0usize..10) {
            let mut doc = EureDocument::new();

            // Create some additional nodes
            for _ in 0..count {
                doc.create_node_uninitialized();
            }

            let root_id = doc.get_root_id();
            prop_assert!(doc.get_node(root_id).is_some(), "Root should always be accessible");
            prop_assert_eq!(root_id, NodeId(0), "Root should be NodeId(0)");
        }
    }

    // =========================================================================
    // Nested structure tests
    // =========================================================================

    proptest! {
        /// Invariant: Nested structures can be built and are equal across documents.
        #[test]
        fn nested_structures_are_equal(
            keys in proptest::collection::vec(arb_object_key(), 1..3),
            depth in 1usize..4,
        ) {
            let mut doc1 = EureDocument::new();
            let mut doc2 = EureDocument::new();

            fn build_nested(
                doc: &mut EureDocument,
                parent_id: NodeId,
                keys: &[ObjectKey],
                depth: usize,
            ) {
                if depth == 0 {
                    return;
                }
                // First collect child node IDs to avoid borrow conflict
                let child_ids: Vec<NodeId> = keys
                    .iter()
                    .filter_map(|key| {
                        doc.add_map_child(key.clone(), parent_id).ok().map(|c| c.node_id)
                    })
                    .collect();

                // Then recursively build nested structures
                for child_id in child_ids {
                    build_nested(doc, child_id, keys, depth - 1);
                }
            }

            let root1 = doc1.get_root_id();
            let root2 = doc2.get_root_id();

            build_nested(&mut doc1, root1, &keys, depth);
            build_nested(&mut doc2, root2, &keys, depth);

            prop_assert_eq!(doc1, doc2, "Nested structures should be equal");
        }
    }
}
