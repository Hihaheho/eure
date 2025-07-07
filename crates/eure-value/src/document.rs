use ahash::AHashMap;
use alloc::{vec, vec::Vec};

use crate::{
    identifier::Identifier,
    value::{KeyCmpValue, Path, PathSegment},
};

/// This does not include MetaExt since, PathSegment::Extension is encoded into Node::extensions, and PathSegment::MetaExt is encoded as InternalKey::Extension, and PathSegment::Array is encoded as NodeContent::Array.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocumentKey {
    /// Regular identifiers like id, description
    Ident(Identifier),
    /// Extension namespace fields starting with $ like $eure, $variant
    Extension(Identifier),
    /// Arbitrary value used as key
    Value(KeyCmpValue),
    /// Tuple element index (0-255)
    TupleIndex(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

pub struct EureDocument<T> {
    root: NodeId,
    nodes: Vec<Node<T>>,
}

#[derive(Debug)]
pub struct Node<T> {
    pub content: NodeContent<T>,
    pub extensions: AHashMap<Identifier, NodeId>,
}

#[derive(Debug)]
pub enum NodeContent<T> {
    Value(T),
    Map(Vec<(DocumentKey, NodeId)>),
    Array(Vec<NodeId>),
}

#[derive(Debug, thiserror::Error)]
pub enum InsertError<T> {
    #[error("This path is not map")]
    ExpectedMap { path: Path, got: T },
    #[error("Already assigned")]
    AlreadyAssigned { path: Path, got: T },
    #[error("Missing array index")]
    MissingArrayIndex {
        path: Path,
        insert_index: usize,
        but_actual_length: usize,
    },
    #[error("Path conflict: expected map but found {found}")]
    PathConflict { path: Path, found: &'static str },
}

impl<T> Default for EureDocument<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> EureDocument<T> {
    pub fn new() -> Self {
        Self {
            root: NodeId(0),
            nodes: vec![Node {
                content: NodeContent::Map(vec![]),
                extensions: AHashMap::new(),
            }],
        }
    }

    pub fn get_root(&self) -> &Node<T> {
        &self.nodes[self.root.0]
    }

    pub fn get_node(&self, id: NodeId) -> &Node<T> {
        &self.nodes[id.0]
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> &mut Node<T> {
        &mut self.nodes[id.0]
    }

    /// Get a mutable node or insert a new one recursively if it doesn't exist.
    pub fn get_node_mut_or_insert(
        &mut self,
        path: impl Iterator<Item = PathSegment>,
    ) -> Result<&mut Node<T>, InsertError<T>> {
        // Collect the path segments so we can iterate multiple times and also
        // build Paths for error values when needed.
        let segments: alloc::vec::Vec<PathSegment> = path.collect();
        let node_id = self.traverse_or_insert_path(&segments)?;
        Ok(self.get_node_mut(node_id))
    }

    /// Insert a value at the given path with recursively insert missing map nodes.
    pub fn insert_node(
        &mut self,
        path: impl Iterator<Item = PathSegment>,
        value: T,
    ) -> Result<NodeId, InsertError<T>> {
        let segments: alloc::vec::Vec<PathSegment> = path.collect();
        let node_id = self.traverse_or_insert_path(&segments)?;

        // If target has any existing content (Value, Map, or Array), treat as already assigned.
        if !matches!(self.nodes[node_id.0].content, NodeContent::Map(ref m) if m.is_empty()) {
            return Err(InsertError::AlreadyAssigned {
                path: Path(segments),
                got: value,
            });
        }

        // Otherwise assign the value (only if the current map is empty with no extensions).
        self.nodes[node_id.0].content = NodeContent::Value(value);
        Ok(node_id)
    }

    /// Internal helper – traverse the document following the given path, inserting
    /// intermediate nodes as necessary. Returns the `NodeId` of the final segment.
    fn traverse_or_insert_path(
        &mut self,
        segments: &[PathSegment],
    ) -> Result<NodeId, InsertError<T>> {
        use PathSegment::*;
        let mut current_id = self.root;

        for (i, segment) in segments.iter().enumerate() {
            let current_path = &segments[..=i];
            match segment {
                Ident(id) => {
                    current_id = self.get_or_insert_child_map(
                        current_id,
                        DocumentKey::Ident(id.clone()),
                        current_path,
                    )?;
                }
                MetaExt(id) => {
                    current_id = self.get_or_insert_child_map(
                        current_id,
                        DocumentKey::Extension(id.clone()),
                        current_path,
                    )?;
                }
                Value(key_val) => {
                    current_id = self.get_or_insert_child_map(
                        current_id,
                        DocumentKey::Value(key_val.clone()),
                        current_path,
                    )?;
                }
                TupleIndex(idx) => {
                    current_id = self.get_or_insert_child_map(
                        current_id,
                        DocumentKey::TupleIndex(*idx),
                        current_path,
                    )?;
                }
                Extension(id) => {
                    current_id =
                        self.get_or_insert_extension_child(current_id, id.clone(), current_path)?;
                }
                ArrayIndex(idx) => {
                    current_id =
                        self.get_or_insert_array_child(current_id, *idx as usize, current_path)?;
                }
            }
        }

        Ok(current_id)
    }

    /// Ensure the current node is a map and return the `NodeId` of the child under the given key.
    /// If either the map or the child does not exist yet, they will be created.
    fn get_or_insert_child_map(
        &mut self,
        parent_id: NodeId,
        key: DocumentKey,
        current_path: &[PathSegment],
    ) -> Result<NodeId, InsertError<T>> {
        // First, check if the parent is already a non-empty map/array/value - if so, error
        // The conflict is at the parent path (excluding the current segment)
        let conflict_path = if current_path.is_empty() {
            current_path
        } else {
            &current_path[..current_path.len() - 1]
        };

        match &self.nodes[parent_id.0].content {
            NodeContent::Value(_) => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "value",
                });
            }
            NodeContent::Array(_) => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "array",
                });
            }
            NodeContent::Map(_) => {
                // This is fine, continue
            }
        }

        // Find existing child.
        let existing_child = if let NodeContent::Map(ref entries) = self.nodes[parent_id.0].content
        {
            entries.iter().find(|(k, _)| k == &key).map(|(_, id)| *id)
        } else {
            None
        };

        if let Some(child_id) = existing_child {
            return Ok(child_id);
        }

        // Need to insert a new node.
        let new_node_id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            content: NodeContent::Map(vec![]),
            extensions: AHashMap::new(),
        });

        // Now we can insert the mapping entry.
        if let NodeContent::Map(ref mut entries) = self.nodes[parent_id.0].content {
            entries.push((key, new_node_id));
        }

        Ok(new_node_id)
    }

    /// Similar to `get_or_insert_child_map` but for extension namespace – utilises the
    /// `extensions` hashmap on each node instead of the map content.
    fn get_or_insert_extension_child(
        &mut self,
        parent_id: NodeId,
        name: Identifier,
        _path: &[PathSegment],
    ) -> Result<NodeId, InsertError<T>> {
        // Check if existing child exists
        {
            let parent_node = &self.nodes[parent_id.0];
            if let Some(child_id) = parent_node.extensions.get(&name).copied() {
                return Ok(child_id);
            }
        }

        // Create new child node.
        let new_node_id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            content: NodeContent::Map(vec![]),
            extensions: AHashMap::new(),
        });

        // Insert into the extensions map.
        self.nodes[parent_id.0].extensions.insert(name, new_node_id);

        Ok(new_node_id)
    }

    /// Ensure the current node is an array and return the `NodeId` of the element at the given
    /// index, creating intermediate elements as necessary. If `index` is `None`, a new element is
    /// appended and its `NodeId` is returned.
    fn get_or_insert_array_child(
        &mut self,
        parent_id: NodeId,
        index: usize,
        current_path: &[PathSegment],
    ) -> Result<NodeId, InsertError<T>> {
        // Check if the parent is already a non-array value - if so, error
        // The conflict is at the parent path (excluding the current array segment)
        let conflict_path = if current_path.is_empty() {
            current_path
        } else {
            &current_path[..current_path.len() - 1]
        };

        match &self.nodes[parent_id.0].content {
            NodeContent::Value(_) => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "value",
                });
            }
            NodeContent::Map(m) if !m.is_empty() => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "map",
                });
            }
            NodeContent::Array(_) | NodeContent::Map(_) => {
                // This is fine, continue
            }
        }

        // Ensure the parent content is an array.
        if !matches!(self.nodes[parent_id.0].content, NodeContent::Array(_)) {
            self.nodes[parent_id.0].content = NodeContent::Array(vec![]);
        }

        // Helper to read current array length without holding the borrow across self.nodes push.
        let get_len = |nodes: &Vec<Node<T>>, pid: NodeId| -> usize {
            match &nodes[pid.0].content {
                NodeContent::Array(arr) => arr.len(),
                _ => 0,
            }
        };

        // If target index is specified, ensure the array is long enough and return the child id.
        let resolved_id = if index < get_len(&self.nodes, parent_id) {
            // Safe to borrow immutably now because we are not mutating nodes.
            if let NodeContent::Array(ref arr) = self.nodes[parent_id.0].content {
                arr[index]
            } else {
                unreachable!()
            }
        } else {
            // Extend array until it reaches the target index
            while get_len(&self.nodes, parent_id) <= index {
                let new_node_id = NodeId(self.nodes.len());
                self.nodes.push(Node {
                    content: NodeContent::Map(vec![]),
                    extensions: AHashMap::new(),
                });
                // Push into array in a separate scope to avoid overlapping borrows.
                {
                    if let NodeContent::Array(ref mut arr) = self.nodes[parent_id.0].content {
                        arr.push(new_node_id);
                    }
                }
            }

            // Now the element must exist.
            if let NodeContent::Array(ref arr) = self.nodes[parent_id.0].content {
                arr[index]
            } else {
                unreachable!()
            }
        };

        Ok(resolved_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifier::Identifier;
    use crate::value::{KeyCmpValue, PathSegment};
    use alloc::string::{String, ToString};
    use alloc::vec;
    use core::str::FromStr;

    fn make_ident(s: &str) -> Identifier {
        Identifier::from_str(s).unwrap()
    }

    #[test]
    fn test_new_document() {
        let doc: EureDocument<String> = EureDocument::new();
        assert_eq!(doc.nodes.len(), 1);
        assert!(matches!(doc.get_root().content, NodeContent::Map(ref m) if m.is_empty()));
        assert!(doc.get_root().extensions.is_empty());
    }

    #[test]
    fn test_insert_simple_value() {
        let mut doc = EureDocument::new();
        let path = vec![PathSegment::Ident(make_ident("name"))];

        let result = doc.insert_node(path.into_iter(), "Alice".to_string());
        assert!(result.is_ok());

        let node_id = result.unwrap();
        let node = doc.get_node(node_id);
        assert!(matches!(node.content, NodeContent::Value(ref v) if v == "Alice"));
    }

    #[test]
    fn test_insert_nested_path() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("user")),
            PathSegment::Ident(make_ident("name")),
        ];

        let result = doc.insert_node(path.into_iter(), "Bob".to_string());
        assert!(result.is_ok());

        // Check that intermediate nodes were created
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            assert_eq!(entries.len(), 1);
            let (key, user_node_id) = &entries[0];
            assert_eq!(key, &DocumentKey::Ident(make_ident("user")));

            let user_node = doc.get_node(*user_node_id);
            if let NodeContent::Map(ref user_entries) = user_node.content {
                assert_eq!(user_entries.len(), 1);
                let (name_key, name_node_id) = &user_entries[0];
                assert_eq!(name_key, &DocumentKey::Ident(make_ident("name")));

                let name_node = doc.get_node(*name_node_id);
                assert!(matches!(name_node.content, NodeContent::Value(ref v) if v == "Bob"));
            } else {
                panic!("Expected user node to be a map");
            }
        } else {
            panic!("Expected root to be a map");
        }
    }

    #[test]
    fn test_insert_already_assigned_error() {
        let mut doc = EureDocument::new();
        let path = vec![PathSegment::Ident(make_ident("name"))];

        // First insertion should succeed
        let result1 = doc.insert_node(path.clone().into_iter(), "Alice".to_string());
        assert!(result1.is_ok());

        // Second insertion should fail with AlreadyAssigned
        let result2 = doc.insert_node(path.into_iter(), "Bob".to_string());
        assert!(result2.is_err());
        assert!(matches!(
            result2.unwrap_err(),
            InsertError::AlreadyAssigned { .. }
        ));
    }

    #[test]
    fn test_path_conflict_value_to_map() {
        let mut doc = EureDocument::new();

        // Insert a value
        let path1 = vec![PathSegment::Ident(make_ident("config"))];
        let result1 = doc.insert_node(path1.into_iter(), "simple".to_string());
        assert!(result1.is_ok());

        // Try to insert into config.database (should fail)
        let path2 = vec![
            PathSegment::Ident(make_ident("config")),
            PathSegment::Ident(make_ident("database")),
        ];
        let result2 = doc.insert_node(path2.into_iter(), "postgres".to_string());
        assert!(result2.is_err());
        assert!(matches!(
            result2.unwrap_err(),
            InsertError::PathConflict { found: "value", .. }
        ));
    }

    #[test]
    fn test_path_conflict_array_to_map() {
        let mut doc = EureDocument::new();

        // Insert into an array
        let path1 = vec![
            PathSegment::Ident(make_ident("items")),
            PathSegment::ArrayIndex(0),
        ];
        let result1 = doc.insert_node(path1.into_iter(), "first".to_string());
        assert!(result1.is_ok());

        // Try to insert into items.name (should fail because items is an array)
        let path2 = vec![
            PathSegment::Ident(make_ident("items")),
            PathSegment::Ident(make_ident("name")),
        ];
        let result2 = doc.insert_node(path2.into_iter(), "invalid".to_string());
        assert!(result2.is_err());
        assert!(matches!(
            result2.unwrap_err(),
            InsertError::PathConflict { found: "array", .. }
        ));
    }

    #[test]
    fn test_extension_paths() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("user")),
            PathSegment::Extension(make_ident("variant")),
        ];

        let result = doc.insert_node(path.into_iter(), "admin".to_string());
        assert!(result.is_ok());

        // Check that the extension was created
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            let (_, user_node_id) = &entries[0];
            let user_node = doc.get_node(*user_node_id);
            assert!(user_node.extensions.contains_key(&make_ident("variant")));
        }
    }

    #[test]
    fn test_meta_extension_paths() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("field")),
            PathSegment::MetaExt(make_ident("type")),
        ];

        let result = doc.insert_node(path.into_iter(), "string".to_string());
        assert!(result.is_ok());

        // Check that the meta extension was stored as regular extension in DocumentKey
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            let (_, field_node_id) = &entries[0];
            let field_node = doc.get_node(*field_node_id);
            if let NodeContent::Map(ref field_entries) = field_node.content {
                assert_eq!(field_entries.len(), 1);
                let (key, _) = &field_entries[0];
                assert!(matches!(key, DocumentKey::Extension(_)));
            }
        }
    }

    #[test]
    fn test_value_key_paths() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("map")),
            PathSegment::Value(KeyCmpValue::String("dynamic_key".to_string())),
        ];

        let result = doc.insert_node(path.into_iter(), "value".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_tuple_index_paths() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("tuple")),
            PathSegment::TupleIndex(0),
        ];

        let result = doc.insert_node(path.into_iter(), "first".to_string());
        assert!(result.is_ok());

        let path2 = vec![
            PathSegment::Ident(make_ident("tuple")),
            PathSegment::TupleIndex(1),
        ];

        let result2 = doc.insert_node(path2.into_iter(), "second".to_string());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_array_with_index() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("items")),
            PathSegment::ArrayIndex(0),
        ];

        let result = doc.insert_node(path.into_iter(), "first_item".to_string());
        assert!(result.is_ok());

        // Insert at index 2 (should create intermediate empty nodes)
        let path2 = vec![
            PathSegment::Ident(make_ident("items")),
            PathSegment::ArrayIndex(2),
        ];

        let result2 = doc.insert_node(path2.into_iter(), "third_item".to_string());
        assert!(result2.is_ok());

        // Check array structure
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            let (_, items_node_id) = &entries[0];
            let items_node = doc.get_node(*items_node_id);
            if let NodeContent::Array(ref arr) = items_node.content {
                assert_eq!(arr.len(), 3); // Should have 3 elements (0, 1, 2)

                // Check first element
                let first_node = doc.get_node(arr[0]);
                assert!(
                    matches!(first_node.content, NodeContent::Value(ref v) if v == "first_item")
                );

                // Check second element (should be empty map)
                let second_node = doc.get_node(arr[1]);
                assert!(matches!(second_node.content, NodeContent::Map(ref m) if m.is_empty()));

                // Check third element
                let third_node = doc.get_node(arr[2]);
                assert!(
                    matches!(third_node.content, NodeContent::Value(ref v) if v == "third_item")
                );
            } else {
                panic!("Expected items to be an array");
            }
        }
    }

    #[test]
    fn test_array_append() {
        let mut doc = EureDocument::new();

        // Insert without index (should append)
        let path1 = vec![
            PathSegment::Ident(make_ident("list")),
            PathSegment::ArrayIndex(0),
        ];
        let result1 = doc.insert_node(path1.into_iter(), "item1".to_string());
        assert!(result1.is_ok());

        let path2 = vec![
            PathSegment::Ident(make_ident("list")),
            PathSegment::ArrayIndex(1),
        ];
        let result2 = doc.insert_node(path2.into_iter(), "item2".to_string());
        assert!(result2.is_ok());

        // Check array has 2 elements
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            let (_, list_node_id) = &entries[0];
            let list_node = doc.get_node(*list_node_id);
            if let NodeContent::Array(ref arr) = list_node.content {
                assert_eq!(arr.len(), 2);
            }
        }
    }

    #[test]
    fn test_get_node_mut_or_insert() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("user")),
            PathSegment::Ident(make_ident("profile")),
        ];

        let result = doc.get_node_mut_or_insert(path.into_iter());
        assert!(result.is_ok());

        let node = result.unwrap();
        // Should be an empty map initially
        assert!(matches!(node.content, NodeContent::Map(ref m) if m.is_empty()));

        // Manually set content
        node.content = NodeContent::Value("test".to_string());

        // Verify it was set
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            let (_, user_node_id) = &entries[0];
            let user_node = doc.get_node(*user_node_id);
            if let NodeContent::Map(ref user_entries) = user_node.content {
                let (_, profile_node_id) = &user_entries[0];
                let profile_node = doc.get_node(*profile_node_id);
                assert!(matches!(profile_node.content, NodeContent::Value(ref v) if v == "test"));
            }
        }
    }

    #[test]
    fn test_get_node_mut_or_insert_path_conflict() {
        let mut doc = EureDocument::new();

        // Insert a value first
        let path1 = vec![PathSegment::Ident(make_ident("config"))];
        let result1 = doc.insert_node(path1.into_iter(), "value".to_string());
        assert!(result1.is_ok());

        // Try to get mutable reference to config.database (should fail)
        let path2 = vec![
            PathSegment::Ident(make_ident("config")),
            PathSegment::Ident(make_ident("database")),
        ];
        let result2 = doc.get_node_mut_or_insert(path2.into_iter());
        assert!(result2.is_err());
        assert!(matches!(
            result2.unwrap_err(),
            InsertError::PathConflict { found: "value", .. }
        ));
    }

    #[test]
    fn test_complex_nested_structure() {
        let mut doc = EureDocument::new();

        // Build a complex structure: app.database.host
        let path1 = vec![
            PathSegment::Ident(make_ident("app")),
            PathSegment::Ident(make_ident("database")),
            PathSegment::Ident(make_ident("host")),
        ];
        let result1 = doc.insert_node(path1.into_iter(), "localhost".to_string());
        assert!(result1.is_ok());

        // Add app.database.port
        let path2 = vec![
            PathSegment::Ident(make_ident("app")),
            PathSegment::Ident(make_ident("database")),
            PathSegment::Ident(make_ident("port")),
        ];
        let result2 = doc.insert_node(path2.into_iter(), "5432".to_string());
        assert!(result2.is_ok());

        // Add app.name
        let path3 = vec![
            PathSegment::Ident(make_ident("app")),
            PathSegment::Ident(make_ident("name")),
        ];
        let result3 = doc.insert_node(path3.into_iter(), "MyApp".to_string());
        assert!(result3.is_ok());

        // Verify the structure
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            assert_eq!(entries.len(), 1);
            let (_, app_node_id) = &entries[0];
            let app_node = doc.get_node(*app_node_id);

            if let NodeContent::Map(ref app_entries) = app_node.content {
                assert_eq!(app_entries.len(), 2); // database and name

                // Find database node
                let db_entry = app_entries.iter().find(
                    |(k, _)| matches!(k, DocumentKey::Ident(id) if id.as_ref() == "database"),
                );
                assert!(db_entry.is_some());

                let (_, db_node_id) = db_entry.unwrap();
                let db_node = doc.get_node(*db_node_id);

                if let NodeContent::Map(ref db_entries) = db_node.content {
                    assert_eq!(db_entries.len(), 2); // host and port
                }
            }
        }
    }

    #[test]
    fn test_mixed_path_types() {
        let mut doc = EureDocument::new();

        // Complex path with different segment types
        let path = vec![
            PathSegment::Ident(make_ident("root")),
            PathSegment::Extension(make_ident("meta")),
            PathSegment::Value(KeyCmpValue::String("dynamic".to_string())),
            PathSegment::TupleIndex(0),
        ];

        let result = doc.insert_node(path.into_iter(), "complex_value".to_string());
        assert!(result.is_ok());

        // Verify the structure was created correctly
        let root = doc.get_root();
        if let NodeContent::Map(ref entries) = root.content {
            let (_, root_node_id) = &entries[0];
            let root_node = doc.get_node(*root_node_id);

            // Should have extension
            assert!(root_node.extensions.contains_key(&make_ident("meta")));

            let meta_node_id = root_node.extensions[&make_ident("meta")];
            let meta_node = doc.get_node(meta_node_id);

            // Meta node should have the dynamic key
            if let NodeContent::Map(ref meta_entries) = meta_node.content {
                assert_eq!(meta_entries.len(), 1);
                let (key, _) = &meta_entries[0];
                assert!(
                    matches!(key, DocumentKey::Value(KeyCmpValue::String(s)) if s == "dynamic")
                );
            }
        }
    }

    #[test]
    fn test_path_conflict_reports_correct_path() {
        let mut doc = EureDocument::new();

        // Insert a = 1
        let path1 = vec![PathSegment::Ident(make_ident("a"))];
        let result1 = doc.insert_node(path1.into_iter(), "1".to_string());
        assert!(result1.is_ok());

        // Try to insert a.b = 2 (should fail with conflict at path "a", not "a.b")
        let path2 = vec![
            PathSegment::Ident(make_ident("a")),
            PathSegment::Ident(make_ident("b")),
        ];
        let result2 = doc.insert_node(path2.into_iter(), "2".to_string());
        assert!(result2.is_err());

        if let Err(InsertError::PathConflict { path, found }) = result2 {
            // The conflict should be reported at path "a", not "a.b"
            assert_eq!(path.0.len(), 1);
            assert!(matches!(path.0[0], PathSegment::Ident(ref id) if id.as_ref() == "a"));
            assert_eq!(found, "value");
        } else {
            panic!("Expected PathConflict error");
        }
    }

    #[test]
    fn test_path_conflict_nested_reports_correct_path() {
        let mut doc = EureDocument::new();

        // Insert user.profile = "admin"
        let path1 = vec![
            PathSegment::Ident(make_ident("user")),
            PathSegment::Ident(make_ident("profile")),
        ];
        let result1 = doc.insert_node(path1.into_iter(), "admin".to_string());
        assert!(result1.is_ok());

        // Try to insert user.profile.settings = "dark" (should fail with conflict at path "user.profile")
        let path2 = vec![
            PathSegment::Ident(make_ident("user")),
            PathSegment::Ident(make_ident("profile")),
            PathSegment::Ident(make_ident("settings")),
        ];
        let result2 = doc.insert_node(path2.into_iter(), "dark".to_string());
        assert!(result2.is_err());

        if let Err(InsertError::PathConflict { path, found }) = result2 {
            // The conflict should be reported at path "user.profile", not "user.profile.settings"
            assert_eq!(path.0.len(), 2);
            assert!(matches!(path.0[0], PathSegment::Ident(ref id) if id.as_ref() == "user"));
            assert!(matches!(path.0[1], PathSegment::Ident(ref id) if id.as_ref() == "profile"));
            assert_eq!(found, "value");
        } else {
            panic!("Expected PathConflict error");
        }
    }
}
