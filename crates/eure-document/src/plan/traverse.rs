//! Reachable-node traversal helpers for a [`LayoutPlan`].

use alloc::vec::Vec;

use alloc::string::ToString;

use crate::document::node::NodeValue;
use crate::document::{EureDocument, NodeId};
use crate::path::PathSegment;
use crate::value::{ObjectKey, PartialObjectKey};

/// Return every [`NodeId`] reachable from the document root, in a stable
/// (document-order) traversal: extensions first, then content children.
pub fn all_reachable_ids(doc: &EureDocument) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = alloc::vec![doc.get_root_id()];
    let mut seen = Vec::new();
    while let Some(id) = stack.pop() {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        out.push(id);
        let node = doc.node(id);
        // Push children in reverse so that pop order equals document order.
        let mut children: Vec<NodeId> = Vec::new();
        for (_, &ext) in node.extensions.iter() {
            children.push(ext);
        }
        match &node.content {
            NodeValue::Map(map) => {
                for (_, &cid) in map.iter() {
                    children.push(cid);
                }
            }
            NodeValue::PartialMap(pm) => {
                for (_, &cid) in pm.iter() {
                    children.push(cid);
                }
            }
            NodeValue::Array(arr) => {
                for &cid in arr.iter() {
                    children.push(cid);
                }
            }
            NodeValue::Tuple(tup) => {
                for &cid in tup.iter() {
                    children.push(cid);
                }
            }
            _ => {}
        }
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    // Stable order: we want root first, then DFS document order. Since we
    // pushed in reverse and popped, `out` already reflects that. Sort by
    // NodeId to be deterministic.
    out.sort_by_key(|id| id.0);
    out
}

/// Return the direct children of `parent` along with the [`PathSegment`]
/// used to address each one. Extensions come first, then content children
/// in their document order.
pub fn children_of(doc: &EureDocument, parent: NodeId) -> Vec<(PathSegment, NodeId)> {
    let mut out = Vec::new();
    let node = doc.node(parent);
    for (ident, &child) in node.extensions.iter() {
        out.push((PathSegment::Extension(ident.clone()), child));
    }
    match &node.content {
        NodeValue::Map(map) => {
            for (key, &child) in map.iter() {
                out.push((PathSegment::Value(key.clone()), child));
            }
        }
        NodeValue::PartialMap(pm) => {
            for (key, &child) in pm.iter() {
                out.push((PathSegment::from_partial_object_key(key.clone()), child));
            }
        }
        NodeValue::Array(arr) => {
            for (i, &child) in arr.iter().enumerate() {
                out.push((PathSegment::ArrayIndex(Some(i)), child));
            }
        }
        NodeValue::Tuple(tup) => {
            for (i, &child) in tup.iter().enumerate() {
                out.push((PathSegment::TupleIndex(i as u8), child));
            }
        }
        _ => {}
    }
    out
}

/// Return the child node at a single [`PathSegment`] under `parent`.
///
/// This is ported verbatim from the previous `layout.rs` so that lookups
/// against both `Map` and `PartialMap` forms succeed regardless of which
/// segment flavor the caller uses.
pub fn child_node_id(
    doc: &EureDocument,
    parent_id: NodeId,
    segment: &PathSegment,
) -> Option<NodeId> {
    let parent = doc.node(parent_id);
    match segment {
        PathSegment::Extension(ext) => parent.extensions.get(ext).copied(),
        PathSegment::Ident(ident) => match &parent.content {
            NodeValue::Map(map) => map.get(&ObjectKey::String(ident.to_string())).copied(),
            NodeValue::PartialMap(map) => map
                .find(&PartialObjectKey::String(ident.to_string()))
                .copied(),
            _ => None,
        },
        PathSegment::Value(key) => match &parent.content {
            NodeValue::Map(map) => map.get(key).copied(),
            NodeValue::PartialMap(map) => map.find(&PartialObjectKey::from(key.clone())).copied(),
            _ => None,
        },
        PathSegment::PartialValue(key) => match &parent.content {
            NodeValue::Map(map) => ObjectKey::try_from(key.clone())
                .ok()
                .and_then(|object_key| map.get(&object_key))
                .copied(),
            NodeValue::PartialMap(map) => map.find(key).copied(),
            _ => None,
        },
        PathSegment::ArrayIndex(index) => match &parent.content {
            NodeValue::Array(array) => index.and_then(|i| array.get(i)),
            _ => None,
        },
        PathSegment::TupleIndex(index) => match &parent.content {
            NodeValue::Tuple(tuple) => tuple.get(*index as usize),
            _ => None,
        },
        PathSegment::HoleKey(label) => match &parent.content {
            NodeValue::PartialMap(map) => map.find(&PartialObjectKey::Hole(label.clone())).copied(),
            _ => None,
        },
    }
}

