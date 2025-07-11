use ahash::AHashMap;
use std::{string::String, vec, vec::Vec};

use crate::nodes::*;
use eure_value::{
    identifier::Identifier,
    value::{
        Array, Code, KeyCmpValue, Map as ValueMap, Path, PathSegment, Tuple as ValueTuple, Value,
    },
};

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
    pub extensions: AHashMap<Identifier, NodeId>,
}

#[derive(Debug)]
pub enum NodeValue {
    // Primitive values with typed handles
    Null {
        handle: NullHandle,
    },
    Bool {
        handle: BooleanHandle,
        value: bool,
    },
    I64 {
        handle: IntegerHandle,
        value: i64,
    },
    U64 {
        handle: IntegerHandle,
        value: u64,
    },
    F32 {
        handle: IntegerHandle,
        value: f32,
    },
    F64 {
        handle: IntegerHandle,
        value: f64,
    },
    String {
        handle: StringConstructionHandle,
        value: String,
    },
    Code {
        handle: CodeHandle,
        value: Code,
    },
    CodeBlock {
        handle: CodeBlockHandle,
        value: Code,
    },
    NamedCode {
        handle: NamedCodeHandle,
        value: Code,
    },
    Path {
        handle: PathHandle,
        value: Path,
    },
    Hole {
        handle: HoleHandle,
    },

    // Complex types with typed handles
    Array {
        handle: ArrayConstructionHandle,
        children: Vec<NodeId>,
    },
    Map {
        handle: MapConstructionHandle,
        entries: Vec<(DocumentKey, NodeId)>,
    },
    Tuple {
        handle: TupleHandle,
        children: Vec<NodeId>,
    },
}


#[derive(Debug, thiserror::Error)]
pub enum InsertError {
    #[error("Already assigned")]
    AlreadyAssigned { path: Path, key: DocumentKey },
    #[error("Path conflict: expected map but found {found}")]
    PathConflict { path: Path, found: &'static str },
    #[error("Expected array")]
    ExpectedArray { path: Path },
    #[error("Array index invalid: expected {expected_index} but got {index}")]
    ArrayIndexInvalid {
        path: Path,
        index: usize,
        expected_index: usize,
    },
    #[error("Expected map")]
    ExpectedMap { path: Path },
    #[error("Expected tuple")]
    ExpectedTuple { path: Path },
    #[error("Tuple index invalid: expected {expected_index} but got {index}")]
    TupleIndexInvalid {
        path: Path,
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
                content: NodeValue::Map {
                    handle: MapConstructionHandle::Root,
                    entries: vec![],
                },
                extensions: AHashMap::new(),
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

    /// Get a mutable node or insert a new one recursively if it doesn't exist.
    pub fn get_node_mut_or_insert(
        &mut self,
        path: impl Iterator<Item = PathSegment>,
    ) -> Result<&mut Node, InsertError> {
        // Collect the path segments so we can iterate multiple times and also
        // build Paths for error values when needed.
        let segments: Vec<PathSegment> = path.collect();
        let node_id = self.traverse_or_insert_path(&segments)?;
        Ok(self.get_node_mut(node_id))
    }

    /// Insert a node content at the given path with recursively insert missing map nodes.
    /// Returns the NodeId of the inserted node and the resolved path (with actual array indices)
    pub fn insert_node_with_resolved_path(
        &mut self,
        path: impl Iterator<Item = PathSegment>,
        content: NodeValue,
    ) -> Result<(NodeId, Vec<PathSegment>), InsertError> {
        let segments: Vec<PathSegment> = path.collect();
        
        // Track if we're creating a new array element
        let _creating_array_element = segments.windows(2).any(|w| {
            matches!(&w[1], PathSegment::ArrayIndex(None))
        }) || matches!(segments.last(), Some(PathSegment::ArrayIndex(None)));
        
        let node_id = self.traverse_or_insert_path(&segments)?;
        
        // TODO: Build the resolved path with actual array indices
        let resolved_path = segments.clone(); // For now, just use the original path
        
        self.insert_node_at_id(node_id, content, &segments)?;
        
        Ok((node_id, resolved_path))
    }
    
    /// Insert a node content at the given path with recursively insert missing map nodes.
    pub fn insert_node(
        &mut self,
        path: impl Iterator<Item = PathSegment>,
        content: NodeValue,
    ) -> Result<NodeId, InsertError> {
        let (node_id, _) = self.insert_node_with_resolved_path(path, content)?;
        Ok(node_id)
    }
    
    fn insert_node_at_id(
        &mut self,
        node_id: NodeId,
        content: NodeValue,
        segments: &[PathSegment],
    ) -> Result<(), InsertError> {

        // Check if this is an array element assignment (path ends with array index)
        let is_array_element = segments.len() >= 2 && 
            matches!(segments[segments.len() - 1], PathSegment::ArrayIndex(_));

        // If target has any existing content (not an empty map), treat as already assigned.
        // Exception: for array elements, we can replace synthetic map nodes or null placeholders
        if !matches!(&self.nodes[node_id.0].content, NodeValue::Map { handle: _, entries } if entries.is_empty()) {
            if is_array_element {
                // For array elements, check if it's a placeholder that can be replaced
                match &self.nodes[node_id.0].content {
                    NodeValue::Map { handle: MapConstructionHandle::Synthetic, entries } if entries.is_empty() && self.nodes[node_id.0].extensions.is_empty() => {
                        // It's a synthetic empty map created for array element, safe to replace
                        self.nodes[node_id.0].content = content;
                        return Ok(());
                    }
                    NodeValue::Null { .. } => {
                        // It's a null placeholder, safe to replace
                        self.nodes[node_id.0].content = content;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            
            return Err(InsertError::AlreadyAssigned {
                path: Path(segments.to_vec()),
                key: DocumentKey::Value(KeyCmpValue::Null), // TODO: proper key
            });
        }

        // Otherwise assign the content (only if the current map is empty with no extensions).
        self.nodes[node_id.0].content = content;
        Ok(())
    }

    /// Internal helper – traverse the document following the given path, inserting
    /// intermediate nodes as necessary. Returns the `NodeId` of the final segment.
    fn traverse_or_insert_path(&mut self, segments: &[PathSegment]) -> Result<NodeId, InsertError> {
        self.traverse_or_insert_path_from(self.root, segments, &[])
    }
    
    /// Recursive helper to traverse paths with proper extension handling
    fn traverse_or_insert_path_from(
        &mut self, 
        start_id: NodeId,
        segments: &[PathSegment],
        path_so_far: &[PathSegment],
    ) -> Result<NodeId, InsertError> {
        use PathSegment::*;
        
        
        if segments.is_empty() {
            return Ok(start_id);
        }
        
        let (first, rest) = segments.split_first().unwrap();
        let mut current_path = path_so_far.to_vec();
        current_path.push(first.clone());
        
        // Check if the next segment is an array index to determine if we should create an array
        let should_create_array = rest.first().is_some_and(|next| matches!(next, ArrayIndex(_)));
        
        let next_id = match first {
            Ident(id) => {
                if should_create_array {
                    // Create or get an array node
                    self.get_or_insert_array_node(
                        start_id,
                        DocumentKey::Ident(id.clone()),
                        &current_path,
                    )?
                } else {
                    self.get_or_insert_child_map(
                        start_id,
                        DocumentKey::Ident(id.clone()),
                        &current_path,
                    )?
                }
            }
            MetaExt(id) => {
                self.get_or_insert_child_map(
                    start_id,
                    DocumentKey::MetaExtension(id.clone()),
                    &current_path,
                )?
            }
            Value(key_val) => {
                if should_create_array {
                    // Create or get an array node
                    self.get_or_insert_array_node(
                        start_id,
                        DocumentKey::Value(key_val.clone()),
                        &current_path,
                    )?
                } else {
                    self.get_or_insert_child_map(
                        start_id,
                        DocumentKey::Value(key_val.clone()),
                        &current_path,
                    )?
                }
            }
            TupleIndex(idx) => {
                self.get_or_insert_child_map(
                    start_id,
                    DocumentKey::TupleIndex(*idx),
                    &current_path,
                )?
            }
            Extension(id) => {
                // Get or create extension node
                let parent_node = &self.nodes[start_id.0];
                if let Some(&existing_id) = parent_node.extensions.get(id) {
                    existing_id
                } else {
                    // Create new extension node
                    let new_node_id = NodeId(self.nodes.len());
                    self.nodes.push(Node {
                        content: NodeValue::Map {
                            handle: MapConstructionHandle::Synthetic,
                            entries: vec![],
                        },
                        extensions: AHashMap::new(),
                    });
                    self.nodes[start_id.0].extensions.insert(id.clone(), new_node_id);
                    new_node_id
                }
            }
            ArrayIndex(idx) => {
                
                if let Some(index) = idx {
                    self.get_or_insert_array_child(start_id, *index as usize, &current_path)?
                } else {
                    self.get_or_insert_array_append(start_id, &current_path)?
                }
            }
        };
        
        // Recursively process the rest of the path
        self.traverse_or_insert_path_from(next_id, rest, &current_path)
    }

    /// Ensure the current node is a map and return the `NodeId` of the child under the given key.
    /// If either the map or the child does not exist yet, they will be created.
    fn get_or_insert_child_map(
        &mut self,
        parent_id: NodeId,
        key: DocumentKey,
        current_path: &[PathSegment],
    ) -> Result<NodeId, InsertError> {
        // First, check if the parent is already a non-empty map/array/value - if so, error
        // The conflict is at the parent path (excluding the current segment)
        let conflict_path = if current_path.is_empty() {
            current_path
        } else {
            &current_path[..current_path.len() - 1]
        };

        match &self.nodes[parent_id.0].content {
            NodeValue::Null { .. }
            | NodeValue::Bool { .. }
            | NodeValue::I64 { .. }
            | NodeValue::U64 { .. }
            | NodeValue::F32 { .. }
            | NodeValue::F64 { .. }
            | NodeValue::String { .. }
            | NodeValue::Code { .. }
            | NodeValue::CodeBlock { .. }
            | NodeValue::NamedCode { .. }
            | NodeValue::Path { .. }
            | NodeValue::Hole { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "value",
                });
            }
            NodeValue::Array { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "array",
                });
            }
            NodeValue::Tuple { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "tuple",
                });
            }
            NodeValue::Map { .. } => {
                // This is fine, continue
            }
        }

        // Find existing child.
        let existing_child = if let NodeValue::Map {
            handle: _,
            ref entries,
        } = self.nodes[parent_id.0].content
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
            content: NodeValue::Map {
                handle: MapConstructionHandle::Synthetic,
                entries: vec![],
            },
            extensions: AHashMap::new(),
        });

        // Now we can insert the mapping entry.
        if let NodeValue::Map { handle: _, entries } = &mut self.nodes[parent_id.0].content {
            entries.push((key, new_node_id));
        }

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
    ) -> Result<NodeId, InsertError> {
        // Check if the parent is already a non-array value - if so, error
        // The conflict is at the parent path (excluding the current array segment)
        let conflict_path = if current_path.is_empty() {
            current_path
        } else {
            &current_path[..current_path.len() - 1]
        };

        match &self.nodes[parent_id.0].content {
            NodeValue::Null { .. }
            | NodeValue::Bool { .. }
            | NodeValue::I64 { .. }
            | NodeValue::U64 { .. }
            | NodeValue::F32 { .. }
            | NodeValue::F64 { .. }
            | NodeValue::String { .. }
            | NodeValue::Code { .. }
            | NodeValue::CodeBlock { .. }
            | NodeValue::NamedCode { .. }
            | NodeValue::Path { .. }
            | NodeValue::Hole { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "value",
                });
            }
            NodeValue::Map { handle: _, entries } if !entries.is_empty() => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "map",
                });
            }
            NodeValue::Tuple { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "tuple",
                });
            }
            NodeValue::Array { .. } | NodeValue::Map { .. } => {
                // This is fine, continue
            }
        }

        // Ensure the parent content is an array.
        if !matches!(&self.nodes[parent_id.0].content, NodeValue::Array { .. }) {
            self.nodes[parent_id.0].content = NodeValue::Array {
                handle: ArrayConstructionHandle::ArrayMarker(ArrayMarkerHandle(
                    crate::tree::CstNodeId(0),
                )),
                children: vec![],
            };
        }

        // Helper to read current array length without holding the borrow across self.nodes push.
        let get_len = |nodes: &Vec<Node>, pid: NodeId| -> usize {
            match &nodes[pid.0].content {
                NodeValue::Array {
                    handle: _,
                    children,
                } => children.len(),
                _ => 0,
            }
        };

        // If target index is specified, ensure the array is long enough and return the child id.
        let resolved_id = if index < get_len(&self.nodes, parent_id) {
            // Safe to borrow immutably now because we are not mutating nodes.
            if let NodeValue::Array {
                handle: _,
                children,
            } = &self.nodes[parent_id.0].content
            {
                children[index]
            } else {
                unreachable!()
            }
        } else {
            // Extend array until it reaches the target index
            while get_len(&self.nodes, parent_id) <= index {
                let new_node_id = NodeId(self.nodes.len());
                self.nodes.push(Node {
                    content: NodeValue::Null {
                        handle: NullHandle(crate::tree::CstNodeId(0)),
                    },
                    extensions: AHashMap::new(),
                });
                // Push into array in a separate scope to avoid overlapping borrows.
                {
                    if let NodeValue::Array {
                        handle: _,
                        children,
                    } = &mut self.nodes[parent_id.0].content
                    {
                        children.push(new_node_id);
                    }
                }
            }

            // Now the element must exist.
            if let NodeValue::Array {
                handle: _,
                children,
            } = &self.nodes[parent_id.0].content
            {
                children[index]
            } else {
                unreachable!()
            }
        };

        Ok(resolved_id)
    }
    
    /// Get or insert an array node at the given key
    fn get_or_insert_array_node(
        &mut self,
        parent_id: NodeId,
        key: DocumentKey,
        current_path: &[PathSegment],
    ) -> Result<NodeId, InsertError> {
        
        // First, check if the parent is already a non-map value - if so, error
        let conflict_path = if current_path.is_empty() {
            current_path
        } else {
            &current_path[..current_path.len() - 1]
        };

        match &self.nodes[parent_id.0].content {
            NodeValue::Null { .. }
            | NodeValue::Bool { .. }
            | NodeValue::I64 { .. }
            | NodeValue::U64 { .. }
            | NodeValue::F32 { .. }
            | NodeValue::F64 { .. }
            | NodeValue::String { .. }
            | NodeValue::Code { .. }
            | NodeValue::CodeBlock { .. }
            | NodeValue::NamedCode { .. }
            | NodeValue::Path { .. }
            | NodeValue::Hole { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "value",
                });
            }
            NodeValue::Array { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "array",
                });
            }
            NodeValue::Tuple { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "tuple",
                });
            }
            NodeValue::Map { .. } => {
                // This is fine, continue
            }
        }

        // Find existing child
        let existing_child = if let NodeValue::Map {
            handle: _,
            ref entries,
        } = self.nodes[parent_id.0].content
        {
            entries.iter().find(|(k, _)| k == &key).map(|(_, id)| *id)
        } else {
            None
        };

        if let Some(child_id) = existing_child {
            // Child exists - ensure it's an array or can be converted
            match &self.nodes[child_id.0].content {
                NodeValue::Array { .. } => Ok(child_id),
                NodeValue::Map { handle: _, entries } if entries.is_empty() => {
                    // Convert empty map to array
                    self.nodes[child_id.0].content = NodeValue::Array {
                        handle: ArrayConstructionHandle::ArrayMarker(ArrayMarkerHandle(
                            crate::tree::CstNodeId(0),
                        )),
                        children: vec![],
                    };
                    Ok(child_id)
                }
                _ => Err(InsertError::PathConflict {
                    path: Path(current_path.to_vec()),
                    found: "non-array value",
                }),
            }
        } else {
            // Create new array node
            let new_node_id = NodeId(self.nodes.len());
            self.nodes.push(Node {
                content: NodeValue::Array {
                    handle: ArrayConstructionHandle::ArrayMarker(ArrayMarkerHandle(
                        crate::tree::CstNodeId(0),
                    )),
                    children: vec![],
                },
                extensions: AHashMap::new(),
            });

            // Insert into parent map
            if let NodeValue::Map {
                handle: _,
                ref mut entries,
            } = self.nodes[parent_id.0].content
            {
                entries.push((key, new_node_id));
            } else {
                // Parent should be a map at this point
                self.nodes[parent_id.0].content = NodeValue::Map {
                    handle: MapConstructionHandle::Synthetic,
                    entries: vec![(key, new_node_id)],
                };
            }

            Ok(new_node_id)
        }
    }
    
    /// Append a new element to an array node
    fn get_or_insert_array_append(
        &mut self,
        parent_id: NodeId,
        current_path: &[PathSegment],
    ) -> Result<NodeId, InsertError> {
        
        // The conflict is at the parent path (excluding the current array segment)
        let conflict_path = if current_path.is_empty() {
            current_path
        } else {
            &current_path[..current_path.len() - 1]
        };

        // Check if the parent is already a non-array value - if so, error
        match &self.nodes[parent_id.0].content {
            NodeValue::Null { .. }
            | NodeValue::Bool { .. }
            | NodeValue::I64 { .. }
            | NodeValue::U64 { .. }
            | NodeValue::F32 { .. }
            | NodeValue::F64 { .. }
            | NodeValue::String { .. }
            | NodeValue::Code { .. }
            | NodeValue::CodeBlock { .. }
            | NodeValue::NamedCode { .. }
            | NodeValue::Path { .. }
            | NodeValue::Hole { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "value",
                });
            }
            NodeValue::Map { handle: _, entries } if !entries.is_empty() => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "map",
                });
            }
            NodeValue::Tuple { .. } => {
                return Err(InsertError::PathConflict {
                    path: Path(conflict_path.to_vec()),
                    found: "tuple",
                });
            }
            NodeValue::Array { .. } | NodeValue::Map { .. } => {
                // This is fine, continue
            }
        }

        // Ensure the parent content is an array
        if !matches!(&self.nodes[parent_id.0].content, NodeValue::Array { .. }) {
            // Use a dummy handle for synthetic arrays
            self.nodes[parent_id.0].content = NodeValue::Array {
                handle: ArrayConstructionHandle::ArrayMarker(ArrayMarkerHandle(
                    crate::tree::CstNodeId(0),
                )),
                children: vec![],
            };
        }

        // Create new child node
        let new_node_id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            content: NodeValue::Map {
                handle: MapConstructionHandle::Synthetic,
                entries: vec![],
            },
            extensions: AHashMap::new(),
        });

        // Append to the array
        if let NodeValue::Array { handle: _, children } = &mut self.nodes[parent_id.0].content {
            children.push(new_node_id);
        }

        Ok(new_node_id)
    }
}

impl EureDocument {
    /// Convert the EureDocument to a Value, discarding span information
    pub fn to_value(&self) -> Value {
        self.node_to_value(self.root)
    }

    /// Convert a specific node to a Value
    pub fn node_to_value(&self, node_id: NodeId) -> Value {
        let node = &self.nodes[node_id.0];

        match &node.content {
            NodeValue::Null { .. } => Value::Null,
            NodeValue::Bool { value, .. } => Value::Bool(*value),
            NodeValue::I64 { value, .. } => Value::I64(*value),
            NodeValue::U64 { value, .. } => Value::U64(*value),
            NodeValue::F32 { value, .. } => Value::F32(*value),
            NodeValue::F64 { value, .. } => Value::F64(*value),
            NodeValue::String { value, .. } => Value::String(value.clone()),
            NodeValue::Code { value, .. } => Value::Code(value.clone()),
            NodeValue::CodeBlock { value, .. } => Value::CodeBlock(value.clone()),
            NodeValue::NamedCode { value, .. } => Value::Code(value.clone()),
            NodeValue::Path { value, .. } => Value::Path(value.clone()),
            NodeValue::Hole { .. } => Value::Hole,
            NodeValue::Array { children, .. } => {
                let values: Vec<Value> = children
                    .iter()
                    .map(|&child_id| self.node_to_value(child_id))
                    .collect();
                Value::Array(Array(values))
            }
            NodeValue::Tuple { children, .. } => {
                let values: Vec<Value> = children
                    .iter()
                    .map(|&child_id| self.node_to_value(child_id))
                    .collect();
                Value::Tuple(ValueTuple(values))
            }
            NodeValue::Map { entries, .. } => {
                let mut map = ValueMap::default();

                // First, add regular entries
                for (key, value_id) in entries {
                    let key_value = match key {
                        DocumentKey::Ident(ident) => KeyCmpValue::String(ident.to_string()),
                        DocumentKey::MetaExtension(ident) => KeyCmpValue::MetaExtension(ident.clone()),
                        DocumentKey::Value(v) => v.clone(),
                        DocumentKey::TupleIndex(idx) => {
                            // Convert tuple index to string for map key
                            KeyCmpValue::String(idx.to_string())
                        }
                    };
                    let value = self.node_to_value(*value_id);
                    map.0.insert(key_value, value);
                }

                // Extensions are metadata, not data - skip them when converting to Value

                Value::Map(map)
            }
        }
    }
}

/// Handle for array construction methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrayConstructionHandle {
    /// Array literal: `= [1, 2, 3]`
    ArrayLiteral(ArrayHandle),
    /// Array indexing in path: `key[0]`
    ArrayMarker(ArrayMarkerHandle),
}

/// Handle for map/object construction methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapConstructionHandle {
    /// Object literal: `= {key: value}`
    ObjectLiteral(ObjectHandle),
    /// Binding: `key = value`
    Binding(BindingHandle),
    /// Section: `@key { ... }`
    Section(SectionHandle),
    /// Section binding: `key { ... }`
    SectionBinding(SectionBindingHandle),
    /// Root node (not from CST)
    Root,
    /// Synthetic intermediate node created during path traversal (not from CST)
    Synthetic,
}

/// Handle for string construction methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringConstructionHandle {
    /// Single string: `"hello"`
    String(StrHandle),
    /// Concatenated strings: `"hello" \ "world"`
    Strings(StringsHandle),
    /// Text binding
    TextBinding(TextBindingHandle),
}


#[cfg(test)]
mod tests {
    use super::*;
    use eure_value::identifier::Identifier;
    use eure_value::value::{KeyCmpValue, PathSegment};
    use std::str::FromStr;
    use std::string::{String, ToString};
    use std::vec;

    fn make_ident(s: &str) -> Identifier {
        Identifier::from_str(s).unwrap()
    }

    #[test]
    fn test_new_document() {
        let doc = EureDocument::new();
        assert_eq!(doc.nodes.len(), 1);
        assert!(
            matches!(&doc.get_root().content, NodeValue::Map { handle: _, entries } if entries.is_empty())
        );
        assert!(doc.get_root().extensions.is_empty());
    }

    #[test]
    fn test_insert_simple_value() {
        let mut doc = EureDocument::new();
        let path = vec![PathSegment::Ident(make_ident("name"))];

        let content = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "Alice".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content);
        assert!(result.is_ok());

        let node_id = result.unwrap();
        let node = doc.get_node(node_id);
        assert!(matches!(&node.content, NodeValue::String { handle: _, value: v } if v == "Alice"));
    }

    #[test]
    fn test_insert_nested_path() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("user")),
            PathSegment::Ident(make_ident("name")),
        ];

        let content = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "Bob".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content);
        assert!(result.is_ok());

        // Check that intermediate nodes were created
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
            assert_eq!(entries.len(), 1);
            let (key, user_node_id) = &entries[0];
            assert_eq!(key, &DocumentKey::Ident(make_ident("user")));

            let user_node = doc.get_node(*user_node_id);
            if let NodeValue::Map {
                handle: _,
                entries: ref user_entries,
            } = user_node.content
            {
                assert_eq!(user_entries.len(), 1);
                let (name_key, name_node_id) = &user_entries[0];
                assert_eq!(name_key, &DocumentKey::Ident(make_ident("name")));

                let name_node = doc.get_node(*name_node_id);
                assert!(
                    matches!(&name_node.content, NodeValue::String { handle: _, value: v } if v == "Bob")
                );
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
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "Alice".to_string(),
        };
        let result1 = doc.insert_node(path.clone().into_iter(), content1);
        assert!(result1.is_ok());

        // Second insertion should fail with AlreadyAssigned
        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(1))),
            value: "Bob".to_string(),
        };
        let result2 = doc.insert_node(path.into_iter(), content2);
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
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "simple".to_string(),
        };
        let result1 = doc.insert_node(path1.into_iter(), content1);
        assert!(result1.is_ok());

        // Try to insert into config.database (should fail)
        let path2 = vec![
            PathSegment::Ident(make_ident("config")),
            PathSegment::Ident(make_ident("database")),
        ];
        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(1))),
            value: "postgres".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
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
            PathSegment::ArrayIndex(Some(0)),
        ];
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "first".to_string(),
        };
        let result1 = doc.insert_node(path1.into_iter(), content1);
        assert!(result1.is_ok());

        // Try to insert into items.name (should fail because items is an array)
        let path2 = vec![
            PathSegment::Ident(make_ident("items")),
            PathSegment::Ident(make_ident("name")),
        ];
        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(1))),
            value: "invalid".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
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

        let content = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "admin".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content);
        assert!(result.is_ok());

        // Check that the extension was created
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
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

        let content = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "string".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content);
        assert!(result.is_ok());

        // Check that the meta extension was stored as regular extension in DocumentKey
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
            let (_, field_node_id) = &entries[0];
            let field_node = doc.get_node(*field_node_id);
            if let NodeValue::Map {
                handle: _,
                entries: ref field_entries,
            } = field_node.content
            {
                assert_eq!(field_entries.len(), 1);
                let (key, _) = &field_entries[0];
                assert!(matches!(key, DocumentKey::MetaExtension(_)));
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

        let content = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "value".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tuple_index_paths() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("tuple")),
            PathSegment::TupleIndex(0),
        ];

        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "first".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content1);
        assert!(result.is_ok());

        let path2 = vec![
            PathSegment::Ident(make_ident("tuple")),
            PathSegment::TupleIndex(1),
        ];

        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "second".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_array_with_index() {
        let mut doc = EureDocument::new();
        let path = vec![
            PathSegment::Ident(make_ident("items")),
            PathSegment::ArrayIndex(Some(0)),
        ];

        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "first_item".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content1);
        assert!(result.is_ok());

        // Insert at index 2 (should create intermediate empty nodes)
        let path2 = vec![
            PathSegment::Ident(make_ident("items")),
            PathSegment::ArrayIndex(Some(2)),
        ];

        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "third_item".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
        assert!(result2.is_ok());

        // Check array structure
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
            let (_, items_node_id) = &entries[0];
            let items_node = doc.get_node(*items_node_id);
            if let NodeValue::Array {
                handle: _,
                children: ref arr,
            } = items_node.content
            {
                assert_eq!(arr.len(), 3); // Should have 3 elements (0, 1, 2)

                // Check first element
                let first_node = doc.get_node(arr[0]);
                assert!(
                    matches!(&first_node.content, NodeValue::String { handle: _, value: v } if v == "first_item")
                );

                // Check second element (should be empty map)
                let second_node = doc.get_node(arr[1]);
                assert!(
                    matches!(&second_node.content, NodeValue::Map { handle: _, entries } if entries.is_empty())
                );

                // Check third element
                let third_node = doc.get_node(arr[2]);
                assert!(
                    matches!(&third_node.content, NodeValue::String { handle: _, value: v } if v == "third_item")
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
            PathSegment::ArrayIndex(Some(0)),
        ];
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "item1".to_string(),
        };
        let result1 = doc.insert_node(path1.into_iter(), content1);
        assert!(result1.is_ok());

        let path2 = vec![
            PathSegment::Ident(make_ident("list")),
            PathSegment::ArrayIndex(Some(1)),
        ];
        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "item2".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
        assert!(result2.is_ok());

        // Check array has 2 elements
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
            let (_, list_node_id) = &entries[0];
            let list_node = doc.get_node(*list_node_id);
            if let NodeValue::Array {
                handle: _,
                children: ref arr,
            } = list_node.content
            {
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
        assert!(
            matches!(&node.content, NodeValue::Map { handle: _, entries } if entries.is_empty())
        );

        // Manually set content
        node.content = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "test".to_string(),
        };

        // Verify it was set
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
            let (_, user_node_id) = &entries[0];
            let user_node = doc.get_node(*user_node_id);
            if let NodeValue::Map {
                handle: _,
                entries: ref user_entries,
            } = user_node.content
            {
                let (_, profile_node_id) = &user_entries[0];
                let profile_node = doc.get_node(*profile_node_id);
                assert!(
                    matches!(&profile_node.content, NodeValue::String { handle: _, value: v } if v == "test")
                );
            }
        }
    }

    #[test]
    fn test_get_node_mut_or_insert_path_conflict() {
        let mut doc = EureDocument::new();

        // Insert a value first
        let path1 = vec![PathSegment::Ident(make_ident("config"))];
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "value".to_string(),
        };
        let result1 = doc.insert_node(path1.into_iter(), content1);
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
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "localhost".to_string(),
        };
        let result1 = doc.insert_node(path1.into_iter(), content1);
        assert!(result1.is_ok());

        // Add app.database.port
        let path2 = vec![
            PathSegment::Ident(make_ident("app")),
            PathSegment::Ident(make_ident("database")),
            PathSegment::Ident(make_ident("port")),
        ];
        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "5432".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
        assert!(result2.is_ok());

        // Add app.name
        let path3 = vec![
            PathSegment::Ident(make_ident("app")),
            PathSegment::Ident(make_ident("name")),
        ];
        let content3 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "MyApp".to_string(),
        };
        let result3 = doc.insert_node(path3.into_iter(), content3);
        assert!(result3.is_ok());

        // Verify the structure
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
            assert_eq!(entries.len(), 1);
            let (_, app_node_id) = &entries[0];
            let app_node = doc.get_node(*app_node_id);

            if let NodeValue::Map {
                handle: _,
                entries: ref app_entries,
            } = app_node.content
            {
                assert_eq!(app_entries.len(), 2); // database and name

                // Find database node
                let db_entry = app_entries.iter().find(
                    |(k, _)| matches!(k, DocumentKey::Ident(id) if id.as_ref() == "database"),
                );
                assert!(db_entry.is_some());

                let (_, db_node_id) = db_entry.unwrap();
                let db_node = doc.get_node(*db_node_id);

                if let NodeValue::Map {
                    handle: _,
                    entries: ref db_entries,
                } = db_node.content
                {
                    assert_eq!(db_entries.len(), 2); // host and port
                }
            }
        }
    }

    #[test]
    fn test_to_value_conversion() {
        let mut doc = EureDocument::new();

        // Insert various types of values to test conversion

        // Null value
        let path_null = vec![PathSegment::Ident(make_ident("null_field"))];
        let content_null = NodeValue::Null {
            handle: NullHandle(crate::tree::CstNodeId(0)),
        };
        doc.insert_node(path_null.into_iter(), content_null)
            .unwrap();

        // Boolean value
        let path_bool = vec![PathSegment::Ident(make_ident("bool_field"))];
        let content_bool = NodeValue::Bool {
            handle: BooleanHandle(crate::tree::CstNodeId(1)),
            value: true,
        };
        doc.insert_node(path_bool.into_iter(), content_bool)
            .unwrap();

        // Integer values
        let path_i64 = vec![PathSegment::Ident(make_ident("i64_field"))];
        let content_i64 = NodeValue::I64 {
            handle: IntegerHandle(crate::tree::CstNodeId(2)),
            value: -42,
        };
        doc.insert_node(path_i64.into_iter(), content_i64).unwrap();

        // String value
        let path_string = vec![PathSegment::Ident(make_ident("string_field"))];
        let content_string = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(3))),
            value: "test string".to_string(),
        };
        doc.insert_node(path_string.into_iter(), content_string)
            .unwrap();

        // Array value
        let path_array = vec![PathSegment::Ident(make_ident("array_field"))];
        let content_array = NodeValue::Array {
            handle: ArrayConstructionHandle::ArrayLiteral(ArrayHandle(crate::tree::CstNodeId(4))),
            children: vec![],
        };
        doc.insert_node(path_array.clone().into_iter(), content_array)
            .unwrap();

        // Add array elements
        let mut array_elem_path = path_array.clone();
        array_elem_path.push(PathSegment::ArrayIndex(Some(0)));
        let array_elem_content = NodeValue::I64 {
            handle: IntegerHandle(crate::tree::CstNodeId(5)),
            value: 10,
        };
        doc.insert_node(array_elem_path.into_iter(), array_elem_content)
            .unwrap();

        // Nested map
        let path_nested = vec![
            PathSegment::Ident(make_ident("nested")),
            PathSegment::Ident(make_ident("inner")),
        ];
        let content_nested = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(6))),
            value: "nested value".to_string(),
        };
        doc.insert_node(path_nested.into_iter(), content_nested)
            .unwrap();

        // Convert to Value
        let value = doc.to_value();

        // Verify the conversion
        match value {
            Value::Map(map) => {
                // Check null field
                assert_eq!(
                    map.0.get(&KeyCmpValue::String("null_field".to_string())),
                    Some(&Value::Null)
                );

                // Check bool field
                assert_eq!(
                    map.0.get(&KeyCmpValue::String("bool_field".to_string())),
                    Some(&Value::Bool(true))
                );

                // Check i64 field
                assert_eq!(
                    map.0.get(&KeyCmpValue::String("i64_field".to_string())),
                    Some(&Value::I64(-42))
                );

                // Check string field
                assert_eq!(
                    map.0.get(&KeyCmpValue::String("string_field".to_string())),
                    Some(&Value::String("test string".to_string()))
                );

                // Check array field
                if let Some(Value::Array(arr)) =
                    map.0.get(&KeyCmpValue::String("array_field".to_string()))
                {
                    assert_eq!(arr.0.len(), 1);
                    assert_eq!(arr.0[0], Value::I64(10));
                } else {
                    panic!("Expected array field to be an array");
                }

                // Check nested map
                if let Some(Value::Map(nested_map)) =
                    map.0.get(&KeyCmpValue::String("nested".to_string()))
                {
                    assert_eq!(
                        nested_map.0.get(&KeyCmpValue::String("inner".to_string())),
                        Some(&Value::String("nested value".to_string()))
                    );
                } else {
                    panic!("Expected nested field to be a map");
                }
            }
            _ => panic!("Expected root to be a map"),
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

        let content = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "complex_value".to_string(),
        };
        let result = doc.insert_node(path.into_iter(), content);
        assert!(result.is_ok());

        // Verify the structure was created correctly
        let root = doc.get_root();
        if let NodeValue::Map {
            handle: _,
            entries: ref entries,
        } = root.content
        {
            let (_, root_node_id) = &entries[0];
            let root_node = doc.get_node(*root_node_id);

            // Should have extension
            assert!(root_node.extensions.contains_key(&make_ident("meta")));

            let meta_node_id = root_node.extensions[&make_ident("meta")];
            let meta_node = doc.get_node(meta_node_id);

            // Meta node should have the dynamic key
            if let NodeValue::Map {
                handle: _,
                entries: meta_entries,
            } = &meta_node.content
            {
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
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "1".to_string(),
        };
        let result1 = doc.insert_node(path1.into_iter(), content1);
        assert!(result1.is_ok());

        // Try to insert a.b = 2 (should fail with conflict at path "a", not "a.b")
        let path2 = vec![
            PathSegment::Ident(make_ident("a")),
            PathSegment::Ident(make_ident("b")),
        ];
        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "2".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
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
        let content1 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "admin".to_string(),
        };
        let result1 = doc.insert_node(path1.into_iter(), content1);
        assert!(result1.is_ok());

        // Try to insert user.profile.settings = "dark" (should fail with conflict at path "user.profile")
        let path2 = vec![
            PathSegment::Ident(make_ident("user")),
            PathSegment::Ident(make_ident("profile")),
            PathSegment::Ident(make_ident("settings")),
        ];
        let content2 = NodeValue::String {
            handle: StringConstructionHandle::String(StrHandle(crate::tree::CstNodeId(0))),
            value: "dark".to_string(),
        };
        let result2 = doc.insert_node(path2.into_iter(), content2);
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
