use std::collections::HashSet;

use eure_document::document::{EureDocument, NodeId};
use eure_document::layout::{DocLayout, LayoutStyle};
use eure_document::path::{EurePath, PathSegment};
use indexmap::IndexMap;
use thiserror::Error;

use crate::SchemaNodeId;

pub type LayoutStrategy = LayoutStyle;
pub type NodeTypeTraceMap = IndexMap<NodeId, ResolvedTypeTrace>;
pub type SchemaNodePathMap = IndexMap<SchemaNodeId, EurePath>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypePathTrace(Vec<EurePath>);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TypePathTraceError {
    #[error("type path trace must contain at least one hop")]
    EmptyTrace,
}

impl TypePathTrace {
    pub fn single(path: EurePath) -> Self {
        Self(vec![path])
    }

    pub fn from_hops(hops: Vec<EurePath>) -> Result<Self, TypePathTraceError> {
        if hops.is_empty() {
            return Err(TypePathTraceError::EmptyTrace);
        }
        Ok(Self(hops))
    }

    pub fn with_hop(&self, path: EurePath) -> Self {
        let mut hops = self.0.clone();
        hops.push(path);
        Self(hops)
    }

    pub fn hops(&self) -> &[EurePath] {
        &self.0
    }

    pub fn current(&self) -> &EurePath {
        debug_assert!(!self.0.is_empty(), "TypePathTrace must be non-empty");
        &self.0[self.0.len() - 1]
    }

    pub fn is_single_hop(&self) -> bool {
        self.0.len() == 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeTraceUnresolvedReason {
    NotVisited,
    UnknownField { field: String },
    UnknownExtension { extension: String },
    UndefinedTypeReference { name: String },
    CrossSchemaReference { namespace: String, name: String },
    AmbiguousUnion { candidates: Vec<TypePathTrace> },
    NoMatchingUnionVariant { candidates: Vec<TypePathTrace> },
    InvalidVariantTag { tag: String },
    RequiresExplicitVariant { variant: String },
    ReferenceCycle,
    InternalInvariant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTypeTrace {
    Resolved(TypePathTrace),
    Ambiguous(Vec<TypePathTrace>),
    Unresolved(TypeTraceUnresolvedReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutStrategies {
    pub by_path: IndexMap<EurePath, LayoutStrategy>,
    pub order_by_path: IndexMap<EurePath, Vec<PathSegment>>,
    pub schema_node_paths: SchemaNodePathMap,
}

impl Default for LayoutStrategies {
    fn default() -> Self {
        Self {
            by_path: IndexMap::new(),
            order_by_path: IndexMap::new(),
            schema_node_paths: IndexMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLayout {
    pub strategy: LayoutStrategy,
    pub matched_path: EurePath,
    pub hop_index: usize,
}

impl LayoutStrategies {
    pub fn resolve(&self, trace: &TypePathTrace) -> Option<ResolvedLayout> {
        for (hop_index, hop) in trace.hops().iter().enumerate() {
            if let Some(strategy) = self.by_path.get(hop) {
                return Some(ResolvedLayout {
                    strategy: *strategy,
                    matched_path: hop.clone(),
                    hop_index,
                });
            }
        }
        None
    }
}

pub fn materialize_doc_layout(
    doc: &EureDocument,
    node_traces: &NodeTypeTraceMap,
    strategies: &LayoutStrategies,
    fallback_style: LayoutStrategy,
) -> DocLayout {
    let mut layout = DocLayout::new();
    layout.fallback_style = fallback_style;

    let node_paths = collect_document_node_paths(doc);
    for (node_id, node_path) in node_paths {
        let Some(trace) = node_traces.get(&node_id) else {
            continue;
        };

        if !node_path.is_empty()
            && let Some(style) = resolve_style_for_trace(strategies, trace)
        {
            layout.add_style_rule(node_path.clone(), style);
        }

        if let Some(order) = resolve_order_for_trace(strategies, trace)
            && !order.is_empty()
        {
            layout.add_order_rule(node_path, order, false);
        }
    }

    layout
}

fn resolve_style_for_trace(
    strategies: &LayoutStrategies,
    trace: &ResolvedTypeTrace,
) -> Option<LayoutStrategy> {
    match trace {
        ResolvedTypeTrace::Resolved(trace) => {
            strategies.resolve(trace).map(|resolved| resolved.strategy)
        }
        ResolvedTypeTrace::Ambiguous(candidates) => {
            let mut resolved: Option<LayoutStrategy> = None;
            for candidate in candidates {
                let candidate_style = strategies.resolve(candidate).map(|r| r.strategy)?;
                if let Some(existing) = resolved {
                    if existing != candidate_style {
                        return None;
                    }
                } else {
                    resolved = Some(candidate_style);
                }
            }
            resolved
        }
        ResolvedTypeTrace::Unresolved(_) => None,
    }
}

fn resolve_order_for_trace(
    strategies: &LayoutStrategies,
    trace: &ResolvedTypeTrace,
) -> Option<Vec<PathSegment>> {
    match trace {
        ResolvedTypeTrace::Resolved(trace) => resolve_order_for_hops(strategies, trace),
        ResolvedTypeTrace::Ambiguous(candidates) => {
            let mut resolved: Option<Vec<PathSegment>> = None;
            for candidate in candidates {
                let candidate_order = resolve_order_for_hops(strategies, candidate)?;
                if let Some(existing) = resolved.as_ref() {
                    if *existing != candidate_order {
                        return None;
                    }
                } else {
                    resolved = Some(candidate_order);
                }
            }
            resolved
        }
        ResolvedTypeTrace::Unresolved(_) => None,
    }
}

fn resolve_order_for_hops(
    strategies: &LayoutStrategies,
    trace: &TypePathTrace,
) -> Option<Vec<PathSegment>> {
    for hop in trace.hops() {
        if let Some(order) = strategies.order_by_path.get(hop) {
            return Some(order.clone());
        }
    }
    None
}

fn collect_document_node_paths(doc: &EureDocument) -> IndexMap<NodeId, Vec<PathSegment>> {
    let mut out = IndexMap::new();
    let mut visited = HashSet::new();
    collect_document_node_paths_rec(
        doc,
        doc.get_root_id(),
        &mut Vec::new(),
        &mut out,
        &mut visited,
    );
    out
}

fn collect_document_node_paths_rec(
    doc: &EureDocument,
    node_id: NodeId,
    path: &mut Vec<PathSegment>,
    out: &mut IndexMap<NodeId, Vec<PathSegment>>,
    visited: &mut HashSet<NodeId>,
) {
    if !visited.insert(node_id) {
        return;
    }
    out.insert(node_id, path.clone());
    let node = doc.node(node_id);

    for (ext, &child_id) in node.extensions.iter() {
        path.push(PathSegment::Extension(ext.clone()));
        collect_document_node_paths_rec(doc, child_id, path, out, visited);
        path.pop();
    }

    match &node.content {
        eure_document::document::node::NodeValue::Array(array) => {
            for (index, &child_id) in array.iter().enumerate() {
                path.push(PathSegment::ArrayIndex(Some(index)));
                collect_document_node_paths_rec(doc, child_id, path, out, visited);
                path.pop();
            }
        }
        eure_document::document::node::NodeValue::Tuple(tuple) => {
            for (index, &child_id) in tuple.iter().enumerate() {
                path.push(PathSegment::TupleIndex(index as u8));
                collect_document_node_paths_rec(doc, child_id, path, out, visited);
                path.pop();
            }
        }
        eure_document::document::node::NodeValue::Map(map) => {
            for (key, &child_id) in map.iter() {
                path.push(PathSegment::Value(key.clone()));
                collect_document_node_paths_rec(doc, child_id, path, out, visited);
                path.pop();
            }
        }
        eure_document::document::node::NodeValue::PartialMap(map) => {
            for (key, &child_id) in map.iter() {
                path.push(PathSegment::from_partial_object_key(key.clone()));
                collect_document_node_paths_rec(doc, child_id, path, out, visited);
                path.pop();
            }
        }
        eure_document::document::node::NodeValue::Primitive(_)
        | eure_document::document::node::NodeValue::Hole(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::layout::LayoutStyle;
    use eure_document::value::ObjectKey;

    #[test]
    fn resolve_first_hop_wins() {
        let first = EurePath::root();
        let second = EurePath(vec![PathSegment::Value(ObjectKey::String("a".to_string()))]);
        let trace = TypePathTrace::from_hops(vec![first.clone(), second.clone()]).unwrap();

        let mut layout = LayoutStrategies::default();
        layout.by_path.insert(second, LayoutStyle::Section);
        layout.by_path.insert(first.clone(), LayoutStyle::Binding);

        let resolved = layout.resolve(&trace).expect("should resolve");
        assert_eq!(resolved.strategy, LayoutStyle::Binding);
        assert_eq!(resolved.matched_path, first);
        assert_eq!(resolved.hop_index, 0);
    }

    #[test]
    fn resolve_no_match() {
        let trace = TypePathTrace::single(EurePath::root());
        let layout = LayoutStrategies::default();
        assert!(layout.resolve(&trace).is_none());
    }

    #[test]
    fn type_path_trace_rejects_empty_hops() {
        let err = TypePathTrace::from_hops(Vec::new()).expect_err("empty trace must be rejected");
        assert_eq!(err, TypePathTraceError::EmptyTrace);
    }

    #[test]
    fn resolve_exact_match_only() {
        let parent = EurePath(vec![PathSegment::Value(ObjectKey::String(
            "item".to_string(),
        ))]);
        let child = EurePath(vec![
            PathSegment::Value(ObjectKey::String("item".to_string())),
            PathSegment::Value(ObjectKey::String("value".to_string())),
        ]);
        let trace = TypePathTrace::single(child);

        let mut layout = LayoutStrategies::default();
        layout.by_path.insert(parent, LayoutStyle::Section);

        assert!(layout.resolve(&trace).is_none());
    }

    #[test]
    fn ambiguous_trace_resolves_when_all_candidates_have_same_strategy() {
        let hop_a = EurePath(vec![PathSegment::Value(ObjectKey::String("a".to_string()))]);
        let hop_b = EurePath(vec![PathSegment::Value(ObjectKey::String("b".to_string()))]);
        let mut strategies = LayoutStrategies::default();
        strategies
            .by_path
            .insert(hop_a.clone(), LayoutStyle::Binding);
        strategies
            .by_path
            .insert(hop_b.clone(), LayoutStyle::Binding);

        let trace = ResolvedTypeTrace::Ambiguous(vec![
            TypePathTrace::single(hop_a),
            TypePathTrace::single(hop_b),
        ]);
        assert_eq!(
            resolve_style_for_trace(&strategies, &trace),
            Some(LayoutStyle::Binding)
        );
    }

    #[test]
    fn ambiguous_trace_falls_back_when_candidates_conflict() {
        let hop_a = EurePath(vec![PathSegment::Value(ObjectKey::String("a".to_string()))]);
        let hop_b = EurePath(vec![PathSegment::Value(ObjectKey::String("b".to_string()))]);
        let mut strategies = LayoutStrategies::default();
        strategies
            .by_path
            .insert(hop_a.clone(), LayoutStyle::Binding);
        strategies
            .by_path
            .insert(hop_b.clone(), LayoutStyle::SectionBinding);

        let trace = ResolvedTypeTrace::Ambiguous(vec![
            TypePathTrace::single(hop_a),
            TypePathTrace::single(hop_b),
        ]);
        assert!(resolve_style_for_trace(&strategies, &trace).is_none());
    }
}
