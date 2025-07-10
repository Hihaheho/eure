//! Schema document representation
//!
//! This module provides a document-like structure for representing schemas,
//! similar to EureDocument but optimized for schema-specific needs.

use crate::schema::*;
use ahash::AHashMap;
use eure_tree::document::{DocumentKey, NodeId};
use eure_value::identifier::Identifier;

/// A document structure for representing EURE schemas
pub struct EureSchema {
    pub(crate) root: NodeId,
    nodes: Vec<SchemaNode>,
}

/// A node in the schema document
#[derive(Debug)]
pub struct SchemaNode {
    pub content: SchemaValue,
    pub extensions: AHashMap<Identifier, NodeId>,
}

/// Content types for schema nodes
#[derive(Debug)]
pub enum SchemaValue {
    // Schema definitions
    Type(Type),
    Field(FieldSchema),
    Object(ObjectSchema),
    Variant(VariantSchema),
    
    // Container types for building schema structure
    Map {
        entries: Vec<(DocumentKey, NodeId)>,
    },
    Array {
        children: Vec<NodeId>,
    },
}

impl Default for EureSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl EureSchema {
    pub fn new() -> Self {
        Self {
            root: NodeId(0),
            nodes: vec![SchemaNode {
                content: SchemaValue::Map {
                    entries: vec![],
                },
                extensions: AHashMap::new(),
            }],
        }
    }

    pub fn get_root(&self) -> &SchemaNode {
        &self.nodes[self.root.0]
    }

    pub fn get_root_id(&self) -> NodeId {
        self.root
    }

    pub fn get_node(&self, id: NodeId) -> &SchemaNode {
        &self.nodes[id.0]
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> &mut SchemaNode {
        &mut self.nodes[id.0]
    }

    /// Insert a new node and return its ID
    pub fn insert_node(&mut self, content: SchemaValue) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        self.nodes.push(SchemaNode {
            content,
            extensions: AHashMap::new(),
        });
        node_id
    }

    /// Add a child to a map node
    pub fn add_map_entry(&mut self, parent_id: NodeId, key: DocumentKey, child_id: NodeId) -> Result<(), &'static str> {
        match &mut self.nodes[parent_id.0].content {
            SchemaValue::Map { entries } => {
                entries.push((key, child_id));
                Ok(())
            }
            _ => Err("Parent is not a map"),
        }
    }

    /// Add a child to an array node
    pub fn add_array_child(&mut self, parent_id: NodeId, child_id: NodeId) -> Result<(), &'static str> {
        match &mut self.nodes[parent_id.0].content {
            SchemaValue::Array { children } => {
                children.push(child_id);
                Ok(())
            }
            _ => Err("Parent is not an array"),
        }
    }

    /// Add an extension to a node
    pub fn add_extension(&mut self, node_id: NodeId, name: Identifier, extension_id: NodeId) {
        self.nodes[node_id.0].extensions.insert(name, extension_id);
    }
}