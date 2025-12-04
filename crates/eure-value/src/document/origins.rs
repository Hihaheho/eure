use alloc::vec::Vec;

use crate::prelude_internal::*;

use super::NodeId;

/// Tracks origin information for nodes and object keys.
///
/// - `node_origins`: Maps `NodeId` to a list of origins (a node can have multiple origins)
/// - `key_origins`: Maps `(NodeId, ObjectKey)` to the origin of that specific map key
#[derive(Debug, Clone)]
pub struct NodeOrigins<O> {
    node_origins: Map<NodeId, Vec<O>>,
    key_origins: Map<(NodeId, ObjectKey), O>,
}

impl<O> Default for NodeOrigins<O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O> NodeOrigins<O> {
    pub fn new() -> Self {
        Self {
            node_origins: Map::new(),
            key_origins: Map::new(),
        }
    }

    /// Record an origin for a node.
    pub fn record_node_origin(&mut self, node_id: NodeId, origin: O) {
        self.node_origins.entry(node_id).or_default().push(origin);
    }

    /// Record an origin for a map key.
    pub fn record_key_origin(&mut self, parent_id: NodeId, key: ObjectKey, origin: O) {
        self.key_origins.insert((parent_id, key), origin);
    }

    /// Get all origins for a node.
    pub fn get_node_origins(&self, node_id: NodeId) -> Option<&[O]> {
        self.node_origins.get(&node_id).map(|v| v.as_slice())
    }

    /// Get the origin for a specific map key.
    pub fn get_key_origin(&self, parent_id: NodeId, key: &ObjectKey) -> Option<&O> {
        self.key_origins.get(&(parent_id, key.clone()))
    }

    /// Get the underlying node origins map.
    pub fn node_origins(&self) -> &Map<NodeId, Vec<O>> {
        &self.node_origins
    }

    /// Get the underlying key origins map.
    pub fn key_origins(&self) -> &Map<(NodeId, ObjectKey), O> {
        &self.key_origins
    }

    /// Consume and return the underlying maps.
    pub fn into_parts(self) -> (Map<NodeId, Vec<O>>, Map<(NodeId, ObjectKey), O>) {
        (self.node_origins, self.key_origins)
    }
}
