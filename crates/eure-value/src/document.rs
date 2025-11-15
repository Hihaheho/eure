pub mod constructor;
pub mod node;

use crate::prelude_internal::*;

/// This does not include MetaExt since, PathSegment::Extension is encoded into Node::extensions, and PathSegment::MetaExt is encoded as InternalKey::MetaExtension, and PathSegment::Array is encoded as NodeContent::Array.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DocumentKey {
    /// Regular identifiers like id, description
    Ident(Identifier),
    /// Meta-extension fields starting with $$ like $$optional, $$type
    MetaExtension(Identifier),
    /// Arbitrary value used as key
    Value(ObjectKey),
    /// Tuple element index (0-255)
    TupleIndex(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

pub struct EureDocument {
    pub(crate) root: NodeId,
    nodes: Vec<Node>,
}

#[derive(Debug, thiserror::Error)]
#[error("Insert error: {kind} at {path}")]
pub struct InsertError {
    pub kind: InsertErrorKind,
    pub path: EurePath,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum InsertErrorKind {
    #[error("Already assigned")]
    AlreadyAssigned { key: DocumentKey },
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

    pub fn create_node(&mut self, new: NodeValue) -> NodeId {
        self.nodes.push(Node {
            content: new,
            extensions: Map::new(),
        });
        NodeId(self.nodes.len() - 1)
    }

    pub fn create_node_uninitialized(&mut self) -> NodeId {
        self.create_node(NodeValue::Uninitialized)
    }

    pub fn add_child_by_segment(
        &mut self,
        segment: PathSegment,
        parent_node_id: NodeId,
    ) -> Result<NodeId, InsertErrorKind> {
        match segment {
            PathSegment::Ident(identifier) => {
                self.add_map_child(ObjectKey::String(identifier.into_string()), parent_node_id)
            }
            PathSegment::Value(object_key) => self.add_map_child(object_key, parent_node_id),
            PathSegment::Extension(identifier) => self.add_extension(identifier, parent_node_id),
            PathSegment::MetaExt(identifier) => self.add_meta_extension(identifier, parent_node_id),
            PathSegment::TupleIndex(index) => self.add_tuple_element(index, parent_node_id),
            PathSegment::ArrayIndex(index) => self.add_array_element(index, parent_node_id),
        }
    }

    pub fn add_map_child(
        &mut self,
        object_key: ObjectKey,
        parent_node_id: NodeId,
    ) -> Result<NodeId, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.get_node_mut(parent_node_id);
        let map = node.require_map()?;
        map.add(DocumentKey::Value(object_key), node_id)?;
        Ok(node_id)
    }

    pub fn add_extension(
        &mut self,
        identifier: Identifier,
        parent_node_id: NodeId,
    ) -> Result<NodeId, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.get_node_mut(parent_node_id);
        if node.extensions.contains_key(&identifier) {
            return Err(InsertErrorKind::AlreadyAssignedExtension { identifier });
        }
        node.extensions.insert(identifier, node_id);
        Ok(node_id)
    }

    pub fn add_meta_extension(
        &mut self,
        identifier: Identifier,
        parent_node_id: NodeId,
    ) -> Result<NodeId, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.get_node_mut(parent_node_id);
        let map = node.require_map()?;
        map.add(DocumentKey::MetaExtension(identifier), node_id)?;
        Ok(node_id)
    }

    pub fn add_tuple_element(
        &mut self,
        index: u8,
        parent_node_id: NodeId,
    ) -> Result<NodeId, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.get_node_mut(parent_node_id);
        let tuple = node.require_tuple()?;
        tuple.add_at(index, node_id)?;
        Ok(node_id)
    }

    pub fn add_array_element(
        &mut self,
        index: Option<usize>,
        parent_node_id: NodeId,
    ) -> Result<NodeId, InsertErrorKind> {
        let node_id = self.create_node_uninitialized();
        let node = self.get_node_mut(parent_node_id);
        let array = node.require_array()?;
        if let Some(index) = index {
            array.add_at(index, node_id)?;
        } else {
            array.push(node_id)?;
        }
        Ok(node_id)
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

    pub fn prepare_node(&mut self, path: &[PathSegment]) -> Result<NodeId, InsertError> {
        let mut node_id = self.root;
        for (index, segment) in path.iter().enumerate() {
            match self.add_child_by_segment(segment.clone(), node_id) {
                Ok(new_node_id) => node_id = new_node_id,
                Err(error) => {
                    return Err(InsertError {
                        kind: error,
                        path: EurePath::from_segments(path.iter().take(index).cloned()),
                    });
                }
            }
        }
        Ok(node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifier::IdentifierParser;

    fn create_identifier(s: &str) -> Identifier {
        let parser = IdentifierParser::init();
        parser.parse(s).unwrap()
    }

    fn create_map_node(doc: &mut EureDocument) -> NodeId {
        doc.create_node(NodeValue::Map(Default::default()))
    }

    fn create_tuple_node(doc: &mut EureDocument) -> NodeId {
        doc.create_node(NodeValue::Tuple(Default::default()))
    }

    fn create_array_node(doc: &mut EureDocument) -> NodeId {
        doc.create_node(NodeValue::Array(Default::default()))
    }

    fn create_primitive_node(doc: &mut EureDocument) -> NodeId {
        doc.create_node(NodeValue::Primitive(PrimitiveValue::Null))
    }

    #[test]
    fn test_add_map_child_success() {
        let mut doc = EureDocument::new();
        let map_id = create_map_node(&mut doc);
        let key = ObjectKey::String("test_key".to_string());

        let node_id = doc
            .add_map_child(key.clone(), map_id)
            .expect("Failed to add map child");

        let node = doc.get_node(map_id);
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => panic!("Expected map"),
        };
        assert_eq!(map.get(&DocumentKey::Value(key)), Some(&node_id));
    }

    #[test]
    fn test_add_map_child_error_expected_map() {
        let mut doc = EureDocument::new();
        let primitive_id = create_primitive_node(&mut doc);
        let key = ObjectKey::String("test".to_string());

        let result = doc.add_map_child(key, primitive_id);
        assert_eq!(result, Err(InsertErrorKind::ExpectedMap));
    }

    #[test]
    fn test_add_map_child_error_already_assigned() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let key = ObjectKey::String("test".to_string());

        let result1 = doc.add_map_child(key.clone(), root_id);
        assert!(result1.is_ok());

        let result2 = doc.add_map_child(key.clone(), root_id);
        assert_eq!(
            result2,
            Err(InsertErrorKind::AlreadyAssigned {
                key: DocumentKey::Value(key)
            })
        );
    }

    #[test]
    fn test_add_extension_success_multiple() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let id1 = create_identifier("ext1");
        let id2 = create_identifier("ext2");

        let node_id1 = doc
            .add_extension(id1.clone(), root_id)
            .expect("Failed to add extension");

        let node_id2 = doc
            .add_extension(id2.clone(), root_id)
            .expect("Failed to add extension");

        let node = doc.get_node(root_id);
        assert_eq!(node.extensions.get(&id1), Some(&node_id1));
        assert_eq!(node.extensions.get(&id2), Some(&node_id2));
    }

    #[test]
    fn test_add_extension_success() {
        let mut doc = EureDocument::new();
        let primitive_id = create_primitive_node(&mut doc);
        let identifier = create_identifier("ext");

        let node_id = doc
            .add_extension(identifier.clone(), primitive_id)
            .expect("Failed to add extension");

        let node = doc.get_node(primitive_id);
        assert_eq!(node.extensions.get(&identifier), Some(&node_id));
    }

    #[test]
    fn test_add_extension_error_already_assigned() {
        let mut doc = EureDocument::new();
        let map_id = create_map_node(&mut doc);
        let identifier = create_identifier("ext");

        let result1 = doc.add_extension(identifier.clone(), map_id);
        assert!(result1.is_ok());

        let result2 = doc.add_extension(identifier.clone(), map_id);
        assert_eq!(
            result2,
            Err(InsertErrorKind::AlreadyAssignedExtension { identifier })
        );
    }

    #[test]
    fn test_add_meta_extension_success() {
        let mut doc = EureDocument::new();
        let map_id = create_map_node(&mut doc);
        let identifier = create_identifier("meta_ext");

        let node_id = doc
            .add_meta_extension(identifier.clone(), map_id)
            .expect("Failed to add meta extension");

        let node = doc.get_node(map_id);
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => panic!("Expected map"),
        };
        assert_eq!(
            map.get(&DocumentKey::MetaExtension(identifier)),
            Some(&node_id)
        );
    }

    #[test]
    fn test_add_meta_extension_error_expected_map() {
        let mut doc = EureDocument::new();
        let primitive_id = create_primitive_node(&mut doc);
        let identifier = create_identifier("meta_ext");

        let result = doc.add_meta_extension(identifier, primitive_id);
        assert_eq!(result, Err(InsertErrorKind::ExpectedMap));
    }

    #[test]
    fn test_add_meta_extension_error_already_assigned() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = create_identifier("meta_ext");

        let result1 = doc.add_meta_extension(identifier.clone(), root_id);
        assert!(result1.is_ok());

        let result2 = doc.add_meta_extension(identifier.clone(), root_id);
        assert_eq!(
            result2,
            Err(InsertErrorKind::AlreadyAssigned {
                key: DocumentKey::MetaExtension(identifier)
            })
        );
    }

    #[test]
    fn test_add_tuple_element_success_index_0() {
        let mut doc = EureDocument::new();
        let tuple_id = create_tuple_node(&mut doc);

        let node_id = doc
            .add_tuple_element(0, tuple_id)
            .expect("Failed to add tuple element");

        let node = doc.get_node(tuple_id);
        let tuple = match &node.content {
            NodeValue::Tuple(t) => t,
            _ => panic!("Expected tuple"),
        };
        assert_eq!(tuple.0, vec![node_id]);
    }

    #[test]
    fn test_add_tuple_element_success_sequential() {
        let mut doc = EureDocument::new();
        let tuple_id = create_tuple_node(&mut doc);

        let result1 = doc
            .add_tuple_element(0, tuple_id)
            .expect("Failed to add tuple element");

        let result2 = doc
            .add_tuple_element(1, tuple_id)
            .expect("Failed to add tuple element");

        let node = doc.get_node(tuple_id);
        let tuple = match &node.content {
            NodeValue::Tuple(t) => t,
            _ => panic!("Expected tuple"),
        };
        assert_eq!(tuple.0, vec![result1, result2]);
    }

    #[test]
    fn test_add_tuple_element_error_expected_tuple() {
        let mut doc = EureDocument::new();
        let map_id = create_map_node(&mut doc);

        let result = doc.add_tuple_element(0, map_id);
        assert_eq!(result, Err(InsertErrorKind::ExpectedTuple));
    }

    #[test]
    fn test_add_tuple_element_error_invalid_index() {
        let mut doc = EureDocument::new();
        let tuple_id = create_tuple_node(&mut doc);

        let result = doc.add_tuple_element(1, tuple_id);
        assert_eq!(
            result,
            Err(InsertErrorKind::TupleIndexInvalid {
                index: 1,
                expected_index: 0
            })
        );
    }

    #[test]
    fn test_add_array_element_success_push() {
        let mut doc = EureDocument::new();
        let array_id = create_array_node(&mut doc);

        let node_id = doc
            .add_array_element(None, array_id)
            .expect("Failed to add array element");

        let node = doc.get_node(array_id);
        let array = match &node.content {
            NodeValue::Array(a) => a,
            _ => panic!("Expected array"),
        };
        assert_eq!(array.0, vec![node_id]);
    }

    #[test]
    fn test_add_array_element_success_at_index() {
        let mut doc = EureDocument::new();
        let array_id = create_array_node(&mut doc);

        let node_id1 = doc
            .add_array_element(Some(0), array_id)
            .expect("Failed to add array element");

        let node_id2 = doc
            .add_array_element(Some(1), array_id)
            .expect("Failed to add array element");

        let node = doc.get_node(array_id);
        let array = match &node.content {
            NodeValue::Array(a) => a,
            _ => panic!("Expected array"),
        };
        assert_eq!(array.0, vec![node_id1, node_id2]);
    }

    #[test]
    fn test_add_array_element_error_expected_array() {
        let mut doc = EureDocument::new();
        let map_id = create_map_node(&mut doc);

        let result = doc.add_array_element(None, map_id);
        assert_eq!(result, Err(InsertErrorKind::ExpectedArray));
    }

    #[test]
    fn test_add_array_element_error_invalid_index() {
        let mut doc = EureDocument::new();
        let array_id = create_array_node(&mut doc);

        let result = doc.add_array_element(Some(1), array_id);
        assert_eq!(
            result,
            Err(InsertErrorKind::ArrayIndexInvalid {
                index: 1,
                expected_index: 0
            })
        );
    }

    #[test]
    fn test_add_child_by_segment_ident() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = create_identifier("test");
        let segment = PathSegment::Ident(identifier.clone());

        let result = doc.add_child_by_segment(segment, root_id);
        assert!(result.is_ok());

        let node = doc.get_node(root_id);
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => panic!("Expected map"),
        };
        let key = DocumentKey::Value(ObjectKey::String(identifier.into_string()));
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

        let node = doc.get_node(root_id);
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => panic!("Expected map"),
        };
        assert!(map.get(&DocumentKey::Value(key)).is_some());
    }

    #[test]
    fn test_add_child_by_segment_extension() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = create_identifier("ext");
        let segment = PathSegment::Extension(identifier.clone());

        let result = doc.add_child_by_segment(segment, root_id);
        assert!(result.is_ok());

        let node = doc.get_node(root_id);
        assert!(node.extensions.contains_key(&identifier));
    }

    #[test]
    fn test_add_child_by_segment_meta_ext() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let identifier = create_identifier("meta");
        let segment = PathSegment::MetaExt(identifier.clone());

        let result = doc.add_child_by_segment(segment, root_id);
        assert!(result.is_ok());

        let node = doc.get_node(root_id);
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => panic!("Expected map"),
        };
        assert!(map.get(&DocumentKey::MetaExtension(identifier)).is_some());
    }

    #[test]
    fn test_add_child_by_segment_tuple_index() {
        let mut doc = EureDocument::new();
        let tuple_id = create_tuple_node(&mut doc);
        let segment = PathSegment::TupleIndex(0);

        let result = doc.add_child_by_segment(segment, tuple_id);
        assert!(result.is_ok());

        let node = doc.get_node(tuple_id);
        let tuple = match &node.content {
            NodeValue::Tuple(t) => t,
            _ => panic!("Expected tuple"),
        };
        assert_eq!(tuple.0.len(), 1);
    }

    #[test]
    fn test_add_child_by_segment_array_index_none() {
        let mut doc = EureDocument::new();
        let array_id = create_array_node(&mut doc);
        let segment = PathSegment::ArrayIndex(None);

        let result = doc.add_child_by_segment(segment, array_id);
        assert!(result.is_ok());

        let node = doc.get_node(array_id);
        let array = match &node.content {
            NodeValue::Array(a) => a,
            _ => panic!("Expected array"),
        };
        assert_eq!(array.0.len(), 1);
    }

    #[test]
    fn test_add_child_by_segment_array_index_some() {
        let mut doc = EureDocument::new();
        let array_id = create_array_node(&mut doc);
        let segment = PathSegment::ArrayIndex(Some(0));

        let result = doc.add_child_by_segment(segment, array_id);
        assert!(result.is_ok());

        let node = doc.get_node(array_id);
        let array = match &node.content {
            NodeValue::Array(a) => a,
            _ => panic!("Expected array"),
        };
        assert_eq!(array.0.len(), 1);
    }

    #[test]
    fn test_prepare_node_empty_path() {
        let mut doc = EureDocument::new();
        let path = &[];

        let result = doc.prepare_node(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), doc.get_root_id());
    }

    #[test]
    fn test_prepare_node_single_segment() {
        let mut doc = EureDocument::new();
        let identifier = create_identifier("test");
        let path = &[PathSegment::Ident(identifier.clone())];

        let result = doc.prepare_node(path);
        assert!(result.is_ok());

        let root_node = doc.get_node(doc.get_root_id());
        let map = match &root_node.content {
            NodeValue::Map(m) => m,
            _ => panic!("Expected map"),
        };
        let key = DocumentKey::Value(ObjectKey::String(identifier.into_string()));
        assert!(map.get(&key).is_some());
    }

    #[test]
    fn test_prepare_node_multiple_segments() {
        let mut doc = EureDocument::new();
        let id1 = create_identifier("ext1");
        let id2 = create_identifier("ext2");
        // Use Extension segments which work on any node type
        let path = &[
            PathSegment::Extension(id1.clone()),
            PathSegment::Extension(id2.clone()),
        ];

        let result = doc.prepare_node(path);
        assert!(result.is_ok());

        // Verify first extension was added to root
        let root_node = doc.get_node(doc.get_root_id());
        assert!(root_node.extensions.contains_key(&id1));

        // Verify second extension was added to the node created by first segment
        let first_child_id = root_node.extensions.get(&id1).unwrap();
        let first_child_node = doc.get_node(*first_child_id);
        assert!(first_child_node.extensions.contains_key(&id2));
    }

    #[test]
    fn test_prepare_node_error_at_first_segment() {
        let mut doc = EureDocument::new();
        let primitive_id = create_primitive_node(&mut doc);
        doc.root = primitive_id;

        let identifier = create_identifier("test");
        let path = &[PathSegment::Ident(identifier)];

        let result = doc.prepare_node(path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.kind, InsertErrorKind::ExpectedMap);
        assert!(error.path.is_root());
    }

    #[test]
    fn test_prepare_node_error_at_middle_segment() {
        let mut doc = EureDocument::new();
        let id1 = create_identifier("a");
        let path = &[PathSegment::Ident(id1.clone()), PathSegment::TupleIndex(0)];

        let result = doc.prepare_node(path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.kind, InsertErrorKind::ExpectedTuple);
        assert_eq!(error.path.0.len(), 1);
        assert_eq!(error.path.0[0], PathSegment::Ident(id1));
    }
}
