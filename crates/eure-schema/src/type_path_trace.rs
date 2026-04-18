use std::collections::HashSet;

use eure_document::document::{EureDocument, NodeId};
use eure_document::path::{EurePath, PathSegment};
use eure_document::plan::traverse as plan_traverse;
use eure_document::plan::{ArrayForm, Form, LayoutPlan, PlanError};
use eure_document::value::ValueKind;
use indexmap::IndexMap;
use thiserror::Error;

use crate::SchemaNodeId;

/// Single-node layout strategy: a [`Form`] taken from the seven-variant
/// taxonomy in [`eure_document::plan`].
///
/// For arrays the same [`Form`] is interpreted as the element form of a
/// [`ArrayForm::PerElement`] (except `Inline`, which maps to
/// [`ArrayForm::Inline`], and `Flatten`, which is rejected).
pub type LayoutStrategy = Form;

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

/// Build a fully-validated [`LayoutPlan`] by applying the given schema-derived
/// [`LayoutStrategies`] to `doc`.
///
/// Unlike the old `materialize_doc_layout` (which silently fell back to
/// `LayoutStyle::Auto` on conflicts), every mismatch surfaces as a typed
/// [`PlanError`] so callers cannot accidentally emit partial data.
pub fn materialize_layout_plan(
    doc: EureDocument,
    node_traces: &NodeTypeTraceMap,
    strategies: &LayoutStrategies,
) -> Result<LayoutPlan, PlanError> {
    let node_paths = collect_document_node_paths(&doc);
    let mut builder = LayoutPlan::builder(doc);
    let root = builder.document().get_root_id();

    for (node_id, node_path) in node_paths {
        if node_id == root {
            if let Some(order) = node_traces
                .get(&node_id)
                .and_then(|trace| resolve_order_for_trace(strategies, trace))
            {
                apply_order(&mut builder, node_id, &node_path, order)?;
            }
            continue;
        }

        let Some(trace) = node_traces.get(&node_id) else {
            continue;
        };

        if let Some(style) = resolve_style_for_trace(strategies, trace, node_id)? {
            let kind = builder.document().node(node_id).content.value_kind();
            if matches!(kind, ValueKind::Array) {
                let array_form = form_to_array_form(node_id, style)?;
                builder.set_array_form(node_id, array_form)?;
            } else {
                builder.set_form(node_id, style)?;
            }
        }

        if let Some(order) = resolve_order_for_trace(strategies, trace) {
            apply_order(&mut builder, node_id, &node_path, order)?;
        }
    }

    builder.build()
}

fn apply_order(
    builder: &mut eure_document::plan::PlanBuilder,
    node_id: NodeId,
    node_path: &[PathSegment],
    order: Vec<PathSegment>,
) -> Result<(), PlanError> {
    if !is_orderable(builder.document(), node_id) {
        return Ok(());
    }
    let present: Vec<PathSegment> = {
        let direct = plan_traverse::children_of(builder.document(), node_id);
        order
            .into_iter()
            .filter(|seg| direct.iter().any(|(s, _)| s == seg))
            .collect()
    };
    if present.is_empty() {
        return Ok(());
    }
    builder.order_at(node_path, present)?;
    Ok(())
}

fn is_orderable(doc: &EureDocument, node: NodeId) -> bool {
    matches!(
        doc.node(node).content.value_kind(),
        ValueKind::Map | ValueKind::PartialMap
    )
}

fn form_to_array_form(node: NodeId, form: Form) -> Result<ArrayForm, PlanError> {
    match form {
        Form::Inline => Ok(ArrayForm::Inline),
        Form::Flatten => Err(PlanError::IncompatibleArrayForm {
            node,
            form: ArrayForm::PerElement(Form::Flatten),
            reason: eure_document::plan::ArrayFormReason::FlattenElementDisallowed,
        }),
        element => Ok(ArrayForm::PerElement(element)),
    }
}

fn resolve_style_for_trace(
    strategies: &LayoutStrategies,
    trace: &ResolvedTypeTrace,
    node: NodeId,
) -> Result<Option<LayoutStrategy>, PlanError> {
    match trace {
        ResolvedTypeTrace::Resolved(trace) => Ok(strategies.resolve(trace).map(|r| r.strategy)),
        ResolvedTypeTrace::Ambiguous(candidates) => {
            let mut resolved: Option<LayoutStrategy> = None;
            for candidate in candidates {
                let candidate_style = match strategies.resolve(candidate) {
                    Some(r) => r.strategy,
                    None => return Ok(None),
                };
                match resolved {
                    Some(existing) if existing != candidate_style => {
                        return Err(PlanError::ConflictingOverride { node });
                    }
                    None => resolved = Some(candidate_style),
                    _ => {}
                }
            }
            Ok(resolved)
        }
        ResolvedTypeTrace::Unresolved(_) => Ok(None),
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
    use eure_document::value::ObjectKey;

    #[test]
    fn resolve_first_hop_wins() {
        let first = EurePath::root();
        let second = EurePath(vec![PathSegment::Value(ObjectKey::String("a".to_string()))]);
        let trace = TypePathTrace::from_hops(vec![first.clone(), second.clone()]).unwrap();

        let mut layout = LayoutStrategies::default();
        layout.by_path.insert(second, Form::Section);
        layout.by_path.insert(first.clone(), Form::Inline);

        let resolved = layout.resolve(&trace).expect("should resolve");
        assert_eq!(resolved.strategy, Form::Inline);
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
        layout.by_path.insert(parent, Form::Section);

        assert!(layout.resolve(&trace).is_none());
    }

    #[test]
    fn ambiguous_trace_resolves_when_all_candidates_have_same_strategy() {
        let hop_a = EurePath(vec![PathSegment::Value(ObjectKey::String("a".to_string()))]);
        let hop_b = EurePath(vec![PathSegment::Value(ObjectKey::String("b".to_string()))]);
        let mut strategies = LayoutStrategies::default();
        strategies.by_path.insert(hop_a.clone(), Form::Inline);
        strategies.by_path.insert(hop_b.clone(), Form::Inline);

        let trace = ResolvedTypeTrace::Ambiguous(vec![
            TypePathTrace::single(hop_a),
            TypePathTrace::single(hop_b),
        ]);
        assert_eq!(
            resolve_style_for_trace(&strategies, &trace, NodeId(0)).unwrap(),
            Some(Form::Inline)
        );
    }

    #[test]
    fn ambiguous_trace_rejects_when_candidates_conflict() {
        let hop_a = EurePath(vec![PathSegment::Value(ObjectKey::String("a".to_string()))]);
        let hop_b = EurePath(vec![PathSegment::Value(ObjectKey::String("b".to_string()))]);
        let mut strategies = LayoutStrategies::default();
        strategies.by_path.insert(hop_a.clone(), Form::Inline);
        strategies.by_path.insert(hop_b.clone(), Form::BindingBlock);

        let trace = ResolvedTypeTrace::Ambiguous(vec![
            TypePathTrace::single(hop_a),
            TypePathTrace::single(hop_b),
        ]);
        assert!(matches!(
            resolve_style_for_trace(&strategies, &trace, NodeId(0)),
            Err(PlanError::ConflictingOverride { .. })
        ));
    }
}
