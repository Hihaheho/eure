use crate::{prelude_internal::*, value::ValueKind};

#[derive(Debug, Clone)]
/// A node in the Eure document.
///
/// This does not implement PartialEq since content may refer to other nodes, and so equality is not well-defined.
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
        match self.document.get_node(self.node_id) {
            Some(node) => f
                .debug_tuple("NodeMut")
                .field(&self.node_id)
                .field(node)
                .finish(),
            None => f
                .debug_tuple("NodeMut")
                .field(&self.node_id)
                .field(&"<invalid>")
                .finish(),
        }
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

    pub fn get_extension(self, ident: &Identifier) -> Option<NodeMut<'d>> {
        let node_id = self.document.node(self.node_id).extensions.get(ident)?;
        Some(NodeMut::new(self.document, *node_id))
    }

    // Content access methods

    pub fn as_map(self) -> Option<&'d NodeMap> {
        self.document.node(self.node_id).as_map()
    }

    pub fn as_array(self) -> Option<&'d NodeArray> {
        self.document.node(self.node_id).as_array()
    }

    pub fn as_tuple(self) -> Option<&'d NodeTuple> {
        self.document.node(self.node_id).as_tuple()
    }

    pub fn require_map(self) -> Result<&'d mut NodeMap, InsertErrorKind> {
        self.document.node_mut(self.node_id).require_map()
    }

    pub fn require_tuple(self) -> Result<&'d mut NodeTuple, InsertErrorKind> {
        self.document.node_mut(self.node_id).require_tuple()
    }

    pub fn require_array(self) -> Result<&'d mut NodeArray, InsertErrorKind> {
        self.document.node_mut(self.node_id).require_array()
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

    pub fn as_primitive(&self) -> Option<&PrimitiveValue> {
        match &self.content {
            NodeValue::Primitive(primitive) => Some(primitive),
            _ => None,
        }
    }

    pub fn get_extension(&self, ident: &Identifier) -> Option<NodeId> {
        self.extensions.get(ident).copied()
    }

    pub(crate) fn require_map(&mut self) -> Result<&mut NodeMap, InsertErrorKind> {
        if self.content.is_hole() {
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
        if self.content.is_hole() {
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
        if self.content.is_hole() {
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
    /// A hole represents an uninitialized or placeholder value.
    /// Optionally includes a label for identification (e.g., `!todo`, `!wip`).
    Hole(Option<Identifier>),
    Primitive(PrimitiveValue),
    Array(NodeArray),
    Map(NodeMap),
    Tuple(NodeTuple),
}

impl NodeValue {
    /// Creates an anonymous hole (no label).
    pub fn hole() -> Self {
        Self::Hole(None)
    }

    /// Creates a labeled hole.
    pub fn labeled_hole(label: Identifier) -> Self {
        Self::Hole(Some(label))
    }

    /// Returns true if this is a hole (labeled or anonymous).
    pub fn is_hole(&self) -> bool {
        matches!(self, Self::Hole(_))
    }

    pub fn empty_map() -> Self {
        Self::Map(NodeMap::new())
    }

    pub fn empty_array() -> Self {
        Self::Array(NodeArray::new())
    }

    pub fn empty_tuple() -> Self {
        Self::Tuple(NodeTuple::new())
    }

    pub fn value_kind(&self) -> Option<ValueKind> {
        match self {
            Self::Hole(_) => None,
            Self::Primitive(primitive) => Some(primitive.kind()),
            Self::Array(_) => Some(ValueKind::Array),
            Self::Map(_) => Some(ValueKind::Map),
            Self::Tuple(_) => Some(ValueKind::Tuple),
        }
    }
}

// ============================================================================
// From implementations for NodeValue
// ============================================================================

impl From<PrimitiveValue> for NodeValue {
    fn from(p: PrimitiveValue) -> Self {
        NodeValue::Primitive(p)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Plural)]
#[plural(len, is_empty, iter, into_iter, new)]
pub struct NodeArray(Vec<NodeId>);

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Plural)]
#[plural(len, is_empty, iter, into_iter, new)]
pub struct NodeTuple(Vec<NodeId>);

pub type NodeMap = Map<ObjectKey, NodeId>;

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

    pub fn to_vec(&self) -> Vec<NodeId> {
        self.0.clone()
    }

    pub fn from_vec(vec: Vec<NodeId>) -> Self {
        Self(vec)
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

    pub fn to_vec(&self) -> Vec<NodeId> {
        self.0.clone()
    }

    pub fn from_vec(vec: Vec<NodeId>) -> Self {
        Self(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(s: &str) -> Identifier {
        s.parse().unwrap()
    }

    #[test]
    fn test_require_map_on_uninitialized() {
        let mut node = Node {
            content: NodeValue::hole(),
            extensions: Map::new(),
        };

        let map = node.require_map().expect("Should convert to map");
        assert_eq!(map.len(), 0);

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
        assert_eq!(map.len(), 0);
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
            content: NodeValue::hole(),
            extensions: Map::new(),
        };

        let tuple = node.require_tuple().expect("Should convert to tuple");
        assert_eq!(tuple.len(), 0);

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
        assert_eq!(tuple.len(), 0);
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
            content: NodeValue::hole(),
            extensions: Map::new(),
        };

        let array = node.require_array().expect("Should convert to array");
        assert_eq!(array.len(), 0);

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
        assert_eq!(array.len(), 0);
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

    #[test]
    fn test_node_get_extension_exists() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let ext_identifier = identifier("test_ext");

        // Add an extension
        let ext_node_id = doc
            .add_extension(ext_identifier.clone(), root_id)
            .expect("Failed to add extension")
            .node_id;

        // Test get_extension on the node
        let root_node = doc.node(root_id);
        let result = root_node.get_extension(&ext_identifier);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), ext_node_id);
    }

    #[test]
    fn test_node_get_extension_missing() {
        let doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let ext_identifier = identifier("nonexistent");

        // Test get_extension for a missing extension
        let root_node = doc.node(root_id);
        let result = root_node.get_extension(&ext_identifier);

        assert!(result.is_none());
    }

    #[test]
    fn test_node_mut_get_extension_exists() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let ext_identifier = identifier("test_ext");

        // Add an extension
        let ext_node_id = doc
            .add_extension(ext_identifier.clone(), root_id)
            .expect("Failed to add extension")
            .node_id;

        // Test NodeMut::get_extension
        let node_mut = NodeMut::new(&mut doc, root_id);
        let result = node_mut.get_extension(&ext_identifier);

        assert!(result.is_some());
        let ext_node_mut = result.unwrap();
        assert_eq!(ext_node_mut.node_id, ext_node_id);
    }

    #[test]
    fn test_node_mut_get_extension_missing() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let ext_identifier = identifier("nonexistent");

        // Test NodeMut::get_extension for a missing extension
        let node_mut = NodeMut::new(&mut doc, root_id);
        let result = node_mut.get_extension(&ext_identifier);

        assert!(result.is_none());
    }

    #[test]
    fn test_node_mut_debug_valid_node() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Create a NodeMut with valid node_id
        let node_mut = NodeMut::new(&mut doc, root_id);
        let debug_output = alloc::format!("{:?}", node_mut);

        // Should contain NodeMut, node_id, and delegate to Node's Debug
        assert!(debug_output.contains("NodeMut"));
        assert!(debug_output.contains("NodeId"));
        assert!(debug_output.contains("Node"));
        assert!(debug_output.contains("Hole"));
    }

    #[test]
    fn test_node_mut_debug_invalid_node() {
        let mut doc = EureDocument::new();
        let invalid_id = NodeId(999999); // Invalid NodeId

        // Create a NodeMut with invalid node_id
        let node_mut = NodeMut::new(&mut doc, invalid_id);
        let debug_output = alloc::format!("{:?}", node_mut);

        // Should contain NodeMut and "<invalid>"
        assert!(debug_output.contains("NodeMut"));
        assert!(debug_output.contains("<invalid>"));
    }
}

#[cfg(test)]
mod proptests {
    extern crate std;

    use super::*;
    use proptest::prelude::*;
    use std::vec::Vec;

    // =========================================================================
    // NodeArray sequential index invariants
    // =========================================================================

    proptest! {
        /// Invariant: NodeArray requires sequential indices from 0.
        /// add_at(0) on empty array succeeds.
        #[test]
        fn array_add_at_zero_on_empty_succeeds(_dummy in Just(())) {
            let mut array = NodeArray::new();
            let result = array.add_at(0, NodeId(1));
            prop_assert!(result.is_ok(), "add_at(0) on empty array should succeed");
            prop_assert_eq!(array.len(), 1);
        }

        /// Invariant: NodeArray add_at(n) fails when length != n.
        #[test]
        fn array_add_at_wrong_index_fails(index in 1usize..100) {
            let mut array = NodeArray::new();

            let result = array.add_at(index, NodeId(1));
            prop_assert!(result.is_err(), "add_at({}) on empty array should fail", index);

            match result {
                Err(InsertErrorKind::ArrayIndexInvalid { index: i, expected_index }) => {
                    prop_assert_eq!(i, index);
                    prop_assert_eq!(expected_index, 0);
                }
                other => prop_assert!(false, "Expected ArrayIndexInvalid, got {:?}", other),
            }
        }

        /// Invariant: NodeArray add_at succeeds for sequential indices.
        #[test]
        fn array_sequential_add_succeeds(count in 1usize..50) {
            let mut array = NodeArray::new();

            for i in 0..count {
                let result = array.add_at(i, NodeId(i));
                prop_assert!(result.is_ok(), "add_at({}) should succeed", i);
            }

            prop_assert_eq!(array.len(), count);
        }

        /// Invariant: NodeArray add_at fails when skipping an index.
        #[test]
        fn array_skip_index_fails(
            fill_count in 0usize..10,
            skip_amount in 1usize..10,
        ) {
            let mut array = NodeArray::new();

            // Fill sequentially
            for i in 0..fill_count {
                array.add_at(i, NodeId(i)).expect("Sequential add failed");
            }

            // Try to skip indices
            let bad_index = fill_count + skip_amount;
            let result = array.add_at(bad_index, NodeId(bad_index));

            match result {
                Err(InsertErrorKind::ArrayIndexInvalid { index, expected_index }) => {
                    prop_assert_eq!(index, bad_index);
                    prop_assert_eq!(expected_index, fill_count);
                }
                other => prop_assert!(false, "Expected ArrayIndexInvalid, got {:?}", other),
            }
        }

        /// Invariant: NodeArray push always succeeds and appends.
        #[test]
        fn array_push_always_succeeds(count in 1usize..50) {
            let mut array = NodeArray::new();

            for i in 0..count {
                let result = array.push(NodeId(i));
                prop_assert!(result.is_ok(), "push should always succeed");
                prop_assert_eq!(array.len(), i + 1);
            }
        }

        /// Invariant: NodeArray get returns correct values for valid indices.
        #[test]
        fn array_get_returns_correct_values(count in 1usize..20) {
            let mut array = NodeArray::new();
            let mut expected = Vec::new();

            for i in 0..count {
                let node_id = NodeId(i * 10); // Use distinct values
                array.push(node_id).unwrap();
                expected.push(node_id);
            }

            for (i, &expected_id) in expected.iter().enumerate() {
                prop_assert_eq!(array.get(i), Some(expected_id),
                    "get({}) should return {:?}", i, expected_id);
            }

            // Out of bounds
            prop_assert_eq!(array.get(count), None);
            prop_assert_eq!(array.get(count + 100), None);
        }

        /// Invariant: NodeArray to_vec preserves order and values.
        #[test]
        fn array_to_vec_preserves_order(count in 0usize..20) {
            let mut array = NodeArray::new();
            let mut expected = Vec::new();

            for i in 0..count {
                let node_id = NodeId(i);
                array.push(node_id).unwrap();
                expected.push(node_id);
            }

            prop_assert_eq!(array.to_vec(), expected);
        }

        /// Invariant: NodeArray from_vec creates correct array.
        #[test]
        fn array_from_vec_roundtrip(ids in proptest::collection::vec(0usize..1000, 0..20)) {
            let node_ids: Vec<NodeId> = ids.iter().map(|&i| NodeId(i)).collect();
            let array = NodeArray::from_vec(node_ids.clone());

            prop_assert_eq!(array.len(), node_ids.len());
            prop_assert_eq!(array.to_vec(), node_ids);
        }
    }

    // =========================================================================
    // NodeTuple sequential index invariants
    // =========================================================================

    proptest! {
        /// Invariant: NodeTuple requires sequential indices from 0.
        /// add_at(0) on empty tuple succeeds.
        #[test]
        fn tuple_add_at_zero_on_empty_succeeds(_dummy in Just(())) {
            let mut tuple = NodeTuple::new();
            let result = tuple.add_at(0, NodeId(1));
            prop_assert!(result.is_ok(), "add_at(0) on empty tuple should succeed");
            prop_assert_eq!(tuple.len(), 1);
        }

        /// Invariant: NodeTuple add_at(n) fails when length != n.
        #[test]
        fn tuple_add_at_wrong_index_fails(index in 1u8..100) {
            let mut tuple = NodeTuple::new();

            let result = tuple.add_at(index, NodeId(1));
            prop_assert!(result.is_err(), "add_at({}) on empty tuple should fail", index);

            match result {
                Err(InsertErrorKind::TupleIndexInvalid { index: i, expected_index }) => {
                    prop_assert_eq!(i, index);
                    prop_assert_eq!(expected_index, 0);
                }
                other => prop_assert!(false, "Expected TupleIndexInvalid, got {:?}", other),
            }
        }

        /// Invariant: NodeTuple add_at succeeds for sequential indices.
        #[test]
        fn tuple_sequential_add_succeeds(count in 1u8..50) {
            let mut tuple = NodeTuple::new();

            for i in 0..count {
                let result = tuple.add_at(i, NodeId(i as usize));
                prop_assert!(result.is_ok(), "add_at({}) should succeed", i);
            }

            prop_assert_eq!(tuple.len(), count as usize);
        }

        /// Invariant: NodeTuple add_at fails when skipping an index.
        #[test]
        fn tuple_skip_index_fails(
            fill_count in 0u8..10,
            skip_amount in 1u8..10,
        ) {
            let mut tuple = NodeTuple::new();

            // Fill sequentially
            for i in 0..fill_count {
                tuple.add_at(i, NodeId(i as usize)).expect("Sequential add failed");
            }

            // Try to skip indices
            let bad_index = fill_count + skip_amount;
            let result = tuple.add_at(bad_index, NodeId(bad_index as usize));

            match result {
                Err(InsertErrorKind::TupleIndexInvalid { index, expected_index }) => {
                    prop_assert_eq!(index, bad_index);
                    prop_assert_eq!(expected_index, fill_count as usize);
                }
                other => prop_assert!(false, "Expected TupleIndexInvalid, got {:?}", other),
            }
        }

        /// Invariant: NodeTuple push always succeeds and appends.
        #[test]
        fn tuple_push_always_succeeds(count in 1usize..50) {
            let mut tuple = NodeTuple::new();

            for i in 0..count {
                let result = tuple.push(NodeId(i));
                prop_assert!(result.is_ok(), "push should always succeed");
                prop_assert_eq!(tuple.len(), i + 1);
            }
        }

        /// Invariant: NodeTuple get returns correct values for valid indices.
        #[test]
        fn tuple_get_returns_correct_values(count in 1usize..20) {
            let mut tuple = NodeTuple::new();
            let mut expected = Vec::new();

            for i in 0..count {
                let node_id = NodeId(i * 10);
                tuple.push(node_id).unwrap();
                expected.push(node_id);
            }

            for (i, &expected_id) in expected.iter().enumerate() {
                prop_assert_eq!(tuple.get(i), Some(expected_id),
                    "get({}) should return {:?}", i, expected_id);
            }

            // Out of bounds
            prop_assert_eq!(tuple.get(count), None);
            prop_assert_eq!(tuple.get(count + 100), None);
        }

        /// Invariant: NodeTuple to_vec preserves order and values.
        #[test]
        fn tuple_to_vec_preserves_order(count in 0usize..20) {
            let mut tuple = NodeTuple::new();
            let mut expected = Vec::new();

            for i in 0..count {
                let node_id = NodeId(i);
                tuple.push(node_id).unwrap();
                expected.push(node_id);
            }

            prop_assert_eq!(tuple.to_vec(), expected);
        }

        /// Invariant: NodeTuple from_vec creates correct tuple.
        #[test]
        fn tuple_from_vec_roundtrip(ids in proptest::collection::vec(0usize..1000, 0..20)) {
            let node_ids: Vec<NodeId> = ids.iter().map(|&i| NodeId(i)).collect();
            let tuple = NodeTuple::from_vec(node_ids.clone());

            prop_assert_eq!(tuple.len(), node_ids.len());
            prop_assert_eq!(tuple.to_vec(), node_ids);
        }
    }

    // =========================================================================
    // NodeValue type tests
    // =========================================================================

    proptest! {
        /// Invariant: NodeValue::hole() returns a hole.
        #[test]
        fn node_value_hole_is_hole(_dummy in Just(())) {
            let value = NodeValue::hole();
            prop_assert!(value.is_hole());
            prop_assert_eq!(value, NodeValue::Hole(None));
        }

        /// Invariant: NodeValue::labeled_hole preserves label.
        #[test]
        fn node_value_labeled_hole_preserves_label(label in "[a-z][a-z0-9_-]{0,10}") {
            let identifier: Identifier = label.parse().unwrap();
            let value = NodeValue::labeled_hole(identifier.clone());

            prop_assert!(value.is_hole());
            prop_assert_eq!(value, NodeValue::Hole(Some(identifier)));
        }

        /// Invariant: Empty containers are empty.
        #[test]
        fn empty_containers_are_empty(_dummy in Just(())) {
            let map = NodeValue::empty_map();
            let array = NodeValue::empty_array();
            let tuple = NodeValue::empty_tuple();

            if let NodeValue::Map(m) = map {
                prop_assert!(m.is_empty());
            } else {
                prop_assert!(false, "empty_map should create Map");
            }

            if let NodeValue::Array(a) = array {
                prop_assert!(a.is_empty());
            } else {
                prop_assert!(false, "empty_array should create Array");
            }

            if let NodeValue::Tuple(t) = tuple {
                prop_assert!(t.is_empty());
            } else {
                prop_assert!(false, "empty_tuple should create Tuple");
            }
        }

        /// Invariant: value_kind returns correct kind for each variant.
        #[test]
        fn value_kind_correct(_dummy in Just(())) {
            use crate::value::ValueKind;

            let hole = NodeValue::hole();
            prop_assert_eq!(hole.value_kind(), None);

            let primitive = NodeValue::Primitive(PrimitiveValue::Null);
            prop_assert_eq!(primitive.value_kind(), Some(ValueKind::Null));

            let bool_val = NodeValue::Primitive(PrimitiveValue::Bool(true));
            prop_assert_eq!(bool_val.value_kind(), Some(ValueKind::Bool));

            let array = NodeValue::empty_array();
            prop_assert_eq!(array.value_kind(), Some(ValueKind::Array));

            let map = NodeValue::empty_map();
            prop_assert_eq!(map.value_kind(), Some(ValueKind::Map));

            let tuple = NodeValue::empty_tuple();
            prop_assert_eq!(tuple.value_kind(), Some(ValueKind::Tuple));
        }
    }

    // =========================================================================
    // Node require_* method tests
    // =========================================================================

    proptest! {
        /// Invariant: require_map is idempotent on map.
        #[test]
        fn require_map_idempotent(_dummy in Just(())) {
            let mut node = Node {
                content: NodeValue::empty_map(),
                extensions: Map::new(),
            };

            // Call multiple times
            for _ in 0..5 {
                let result = node.require_map();
                prop_assert!(result.is_ok());
            }

            // Still a map
            prop_assert!(node.as_map().is_some());
        }

        /// Invariant: require_array is idempotent on array.
        #[test]
        fn require_array_idempotent(_dummy in Just(())) {
            let mut node = Node {
                content: NodeValue::empty_array(),
                extensions: Map::new(),
            };

            for _ in 0..5 {
                let result = node.require_array();
                prop_assert!(result.is_ok());
            }

            prop_assert!(node.as_array().is_some());
        }

        /// Invariant: require_tuple is idempotent on tuple.
        #[test]
        fn require_tuple_idempotent(_dummy in Just(())) {
            let mut node = Node {
                content: NodeValue::empty_tuple(),
                extensions: Map::new(),
            };

            for _ in 0..5 {
                let result = node.require_tuple();
                prop_assert!(result.is_ok());
            }

            prop_assert!(node.as_tuple().is_some());
        }

        /// Invariant: require_* methods fail on incompatible types.
        #[test]
        fn require_methods_type_mismatch(_dummy in Just(())) {
            // Array node
            let mut array_node = Node {
                content: NodeValue::empty_array(),
                extensions: Map::new(),
            };
            prop_assert_eq!(array_node.require_map().err(), Some(InsertErrorKind::ExpectedMap));
            prop_assert_eq!(array_node.require_tuple().err(), Some(InsertErrorKind::ExpectedTuple));

            // Map node
            let mut map_node = Node {
                content: NodeValue::empty_map(),
                extensions: Map::new(),
            };
            prop_assert_eq!(map_node.require_array().err(), Some(InsertErrorKind::ExpectedArray));
            prop_assert_eq!(map_node.require_tuple().err(), Some(InsertErrorKind::ExpectedTuple));

            // Tuple node
            let mut tuple_node = Node {
                content: NodeValue::empty_tuple(),
                extensions: Map::new(),
            };
            prop_assert_eq!(tuple_node.require_map().err(), Some(InsertErrorKind::ExpectedMap));
            prop_assert_eq!(tuple_node.require_array().err(), Some(InsertErrorKind::ExpectedArray));
        }
    }
}
