use std::collections::HashSet;

use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::parse::ParseError;
use eure_document::parse::union::extract_explicit_variant_path;
use eure_document::parse::variant_path::VariantPath;
use eure_document::path::{EurePath, PathSegment};
use eure_document::value::ObjectKey;
use indexmap::IndexMap;

use crate::type_path_trace::{
    NodeTypeTraceMap, ResolvedTypeTrace, SchemaNodePathMap, TypePathTrace,
    TypeTraceUnresolvedReason,
};
use crate::{SchemaDocument, SchemaNodeContent, SchemaNodeId, UnknownFieldsPolicy};

use super::validate_node;

pub fn resolve_node_type_traces(
    document: &EureDocument,
    schema: &SchemaDocument,
    schema_node_paths: &SchemaNodePathMap,
) -> NodeTypeTraceMap {
    let mut traces = IndexMap::new();
    for index in 0..document.node_count() {
        traces.insert(
            NodeId(index),
            ResolvedTypeTrace::Unresolved(TypeTraceUnresolvedReason::NotVisited),
        );
    }

    let root_trace = TypePathTrace::single(schema_path(schema_node_paths, schema.root));
    let mut resolver = TraceResolver {
        document,
        schema,
        schema_node_paths,
        traces,
        visiting: HashSet::new(),
    };
    resolver.resolve_node(document.get_root_id(), schema.root, root_trace, None);
    resolver.traces
}

struct TraceResolver<'a> {
    document: &'a EureDocument,
    schema: &'a SchemaDocument,
    schema_node_paths: &'a SchemaNodePathMap,
    traces: NodeTypeTraceMap,
    visiting: HashSet<(NodeId, SchemaNodeId)>,
}

impl<'a> TraceResolver<'a> {
    fn resolve_node(
        &mut self,
        node_id: NodeId,
        schema_id: SchemaNodeId,
        trace: TypePathTrace,
        forced_variant: Option<VariantPath>,
    ) {
        if !self.visiting.insert((node_id, schema_id)) {
            self.set_trace(
                node_id,
                ResolvedTypeTrace::Unresolved(TypeTraceUnresolvedReason::ReferenceCycle),
            );
            return;
        }

        let schema_node = self.schema.node(schema_id);
        match &schema_node.content {
            SchemaNodeContent::Reference(type_ref) => {
                if let Some(namespace) = &type_ref.namespace {
                    self.set_trace(
                        node_id,
                        ResolvedTypeTrace::Unresolved(
                            TypeTraceUnresolvedReason::CrossSchemaReference {
                                namespace: namespace.clone(),
                                name: type_ref.name.to_string(),
                            },
                        ),
                    );
                } else if let Some(target_id) = self.schema.get_type(&type_ref.name) {
                    let next_trace = trace.with_hop(schema_path(self.schema_node_paths, target_id));
                    self.resolve_node(node_id, target_id, next_trace, forced_variant);
                } else {
                    self.set_trace(
                        node_id,
                        ResolvedTypeTrace::Unresolved(
                            TypeTraceUnresolvedReason::UndefinedTypeReference {
                                name: type_ref.name.to_string(),
                            },
                        ),
                    );
                }
            }
            SchemaNodeContent::Union(union_schema) => {
                let explicit_variant = if let Some(forced_variant) = forced_variant {
                    Some(forced_variant)
                } else {
                    match explicit_variant_from_document(self.document, node_id) {
                        Ok(variant) => variant,
                        Err(reason) => {
                            self.set_trace(node_id, ResolvedTypeTrace::Unresolved(reason.clone()));
                            self.mark_subtree_unresolved(node_id, reason);
                            self.visiting.remove(&(node_id, schema_id));
                            return;
                        }
                    }
                };

                if let Some(variant_path) = explicit_variant {
                    let variant_name = variant_path
                        .first()
                        .map(|v| v.as_ref().to_string())
                        .unwrap_or_default();
                    if let Some(variant_schema_id) = union_schema.variants.get(&variant_name) {
                        let rest = variant_path.rest();
                        let next_trace =
                            trace.with_hop(schema_path(self.schema_node_paths, *variant_schema_id));
                        self.resolve_node(node_id, *variant_schema_id, next_trace, rest);
                    } else {
                        self.set_trace(
                            node_id,
                            ResolvedTypeTrace::Unresolved(
                                TypeTraceUnresolvedReason::InvalidVariantTag { tag: variant_name },
                            ),
                        );
                        self.mark_subtree_unresolved(
                            node_id,
                            TypeTraceUnresolvedReason::InvalidVariantTag {
                                tag: "$variant".to_string(),
                            },
                        );
                    }
                    self.visiting.remove(&(node_id, schema_id));
                    return;
                }

                let mut chosen: Option<SchemaNodeId> = None;
                let mut ambiguous_candidates: Vec<TypePathTrace> = Vec::new();
                for (variant_name, &variant_schema_id) in &union_schema.variants {
                    let trial =
                        validate_node(self.document, self.schema, node_id, variant_schema_id);
                    if !trial.errors.is_empty() {
                        continue;
                    }

                    if union_schema.unambiguous.contains(variant_name) {
                        ambiguous_candidates.push(
                            trace.with_hop(schema_path(self.schema_node_paths, variant_schema_id)),
                        );
                        continue;
                    }

                    if union_schema.deny_untagged.contains(variant_name) {
                        self.set_trace(
                            node_id,
                            ResolvedTypeTrace::Unresolved(
                                TypeTraceUnresolvedReason::RequiresExplicitVariant {
                                    variant: variant_name.clone(),
                                },
                            ),
                        );
                        self.mark_subtree_unresolved(
                            node_id,
                            TypeTraceUnresolvedReason::RequiresExplicitVariant {
                                variant: variant_name.clone(),
                            },
                        );
                        self.visiting.remove(&(node_id, schema_id));
                        return;
                    }

                    chosen = Some(variant_schema_id);
                    break;
                }

                if let Some(chosen_id) = chosen {
                    let next_trace = trace.with_hop(schema_path(self.schema_node_paths, chosen_id));
                    self.resolve_node(node_id, chosen_id, next_trace, None);
                    self.visiting.remove(&(node_id, schema_id));
                    return;
                }

                match ambiguous_candidates.len() {
                    0 => {
                        let all_candidates = union_schema
                            .variants
                            .values()
                            .copied()
                            .map(|sid| trace.with_hop(schema_path(self.schema_node_paths, sid)))
                            .collect();
                        self.set_trace(
                            node_id,
                            ResolvedTypeTrace::Unresolved(
                                TypeTraceUnresolvedReason::NoMatchingUnionVariant {
                                    candidates: all_candidates,
                                },
                            ),
                        );
                        self.mark_subtree_unresolved(
                            node_id,
                            TypeTraceUnresolvedReason::NoMatchingUnionVariant {
                                candidates: Vec::new(),
                            },
                        );
                    }
                    1 => {
                        let chosen_id = union_schema
                            .variants
                            .values()
                            .find(|sid| {
                                let path =
                                    trace.with_hop(schema_path(self.schema_node_paths, **sid));
                                ambiguous_candidates[0] == path
                            })
                            .copied();
                        if let Some(chosen_id) = chosen_id {
                            self.resolve_node(
                                node_id,
                                chosen_id,
                                ambiguous_candidates[0].clone(),
                                None,
                            );
                        } else {
                            self.set_trace(
                                node_id,
                                ResolvedTypeTrace::Unresolved(
                                    TypeTraceUnresolvedReason::InternalInvariant,
                                ),
                            );
                        }
                    }
                    _ => {
                        self.set_trace(
                            node_id,
                            ResolvedTypeTrace::Ambiguous(ambiguous_candidates.clone()),
                        );
                        self.mark_subtree_unresolved(
                            node_id,
                            TypeTraceUnresolvedReason::AmbiguousUnion {
                                candidates: ambiguous_candidates,
                            },
                        );
                    }
                }
            }
            _ => {
                self.set_trace(node_id, ResolvedTypeTrace::Resolved(trace.clone()));
                self.resolve_extensions(node_id, schema_id);
                self.resolve_children(node_id, schema_id);
            }
        }

        self.visiting.remove(&(node_id, schema_id));
    }

    fn resolve_extensions(&mut self, node_id: NodeId, schema_id: SchemaNodeId) {
        let node = self.document.node(node_id);
        let schema_node = self.schema.node(schema_id);
        for (ext_name, &ext_node_id) in node.extensions.iter() {
            if let Some(ext_schema) = schema_node.ext_types.get(ext_name) {
                let ext_trace =
                    TypePathTrace::single(schema_path(self.schema_node_paths, ext_schema.schema));
                self.resolve_node(ext_node_id, ext_schema.schema, ext_trace, None);
            } else {
                self.set_trace(
                    ext_node_id,
                    ResolvedTypeTrace::Unresolved(TypeTraceUnresolvedReason::UnknownExtension {
                        extension: ext_name.to_string(),
                    }),
                );
                self.mark_subtree_unresolved(
                    ext_node_id,
                    TypeTraceUnresolvedReason::UnknownExtension {
                        extension: ext_name.to_string(),
                    },
                );
            }
        }
    }

    fn resolve_children(&mut self, node_id: NodeId, schema_id: SchemaNodeId) {
        let node = self.document.node(node_id);
        let schema_node = self.schema.node(schema_id);
        match (&node.content, &schema_node.content) {
            (NodeValue::Map(map), SchemaNodeContent::Record(record_schema)) => {
                for (field_name, field_schema) in &record_schema.properties {
                    let key = ObjectKey::String(field_name.clone());
                    if let Some(&child_id) = map.get(&key) {
                        let child_trace = TypePathTrace::single(schema_path(
                            self.schema_node_paths,
                            field_schema.schema,
                        ));
                        self.resolve_node(child_id, field_schema.schema, child_trace, None);
                    }
                }
                for (key, &child_id) in map.iter() {
                    if let ObjectKey::String(field_name) = key {
                        if record_schema.properties.contains_key(field_name) {
                            continue;
                        }
                        match &record_schema.unknown_fields {
                            UnknownFieldsPolicy::Deny | UnknownFieldsPolicy::Allow => {
                                self.set_trace(
                                    child_id,
                                    ResolvedTypeTrace::Unresolved(
                                        TypeTraceUnresolvedReason::UnknownField {
                                            field: field_name.clone(),
                                        },
                                    ),
                                );
                                self.mark_subtree_unresolved(
                                    child_id,
                                    TypeTraceUnresolvedReason::UnknownField {
                                        field: field_name.clone(),
                                    },
                                );
                            }
                            UnknownFieldsPolicy::Schema(unknown_schema_id) => {
                                let child_trace = TypePathTrace::single(schema_path(
                                    self.schema_node_paths,
                                    *unknown_schema_id,
                                ));
                                self.resolve_node(child_id, *unknown_schema_id, child_trace, None);
                            }
                        }
                    } else {
                        self.set_trace(
                            child_id,
                            ResolvedTypeTrace::Unresolved(
                                TypeTraceUnresolvedReason::UnknownField {
                                    field: key.to_string(),
                                },
                            ),
                        );
                        self.mark_subtree_unresolved(
                            child_id,
                            TypeTraceUnresolvedReason::UnknownField {
                                field: key.to_string(),
                            },
                        );
                    }
                }
            }
            (NodeValue::Array(array), SchemaNodeContent::Array(array_schema)) => {
                for &child_id in array.iter() {
                    let child_trace = TypePathTrace::single(schema_path(
                        self.schema_node_paths,
                        array_schema.item,
                    ));
                    self.resolve_node(child_id, array_schema.item, child_trace, None);
                }
            }
            (NodeValue::Tuple(tuple), SchemaNodeContent::Tuple(tuple_schema)) => {
                for (index, &child_id) in tuple.iter().enumerate() {
                    if let Some(&child_schema_id) = tuple_schema.elements.get(index) {
                        let child_trace = TypePathTrace::single(schema_path(
                            self.schema_node_paths,
                            child_schema_id,
                        ));
                        self.resolve_node(child_id, child_schema_id, child_trace, None);
                    }
                }
            }
            (NodeValue::Map(map), SchemaNodeContent::Map(map_schema)) => {
                for (_, &child_id) in map.iter() {
                    let child_trace = TypePathTrace::single(schema_path(
                        self.schema_node_paths,
                        map_schema.value,
                    ));
                    self.resolve_node(child_id, map_schema.value, child_trace, None);
                }
            }
            _ => {}
        }
    }

    fn set_trace(&mut self, node_id: NodeId, trace: ResolvedTypeTrace) {
        self.traces.insert(node_id, trace);
    }

    fn mark_subtree_unresolved(&mut self, node_id: NodeId, reason: TypeTraceUnresolvedReason) {
        let mut stack = vec![node_id];
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            let replace = matches!(
                self.traces.get(&current),
                Some(ResolvedTypeTrace::Unresolved(
                    TypeTraceUnresolvedReason::NotVisited
                ))
            );
            if replace {
                self.traces
                    .insert(current, ResolvedTypeTrace::Unresolved(reason.clone()));
            }

            let node = self.document.node(current);
            for (_, &child) in node.extensions.iter() {
                stack.push(child);
            }
            match &node.content {
                NodeValue::Array(array) => {
                    for &child in array.iter() {
                        stack.push(child);
                    }
                }
                NodeValue::Tuple(tuple) => {
                    for &child in tuple.iter() {
                        stack.push(child);
                    }
                }
                NodeValue::Map(map) => {
                    for (_, &child) in map.iter() {
                        stack.push(child);
                    }
                }
                NodeValue::PartialMap(map) => {
                    for (_, &child) in map.iter() {
                        stack.push(child);
                    }
                }
                NodeValue::Primitive(_) | NodeValue::Hole(_) => {}
            }
        }
    }
}

fn schema_path(paths: &SchemaNodePathMap, schema_id: SchemaNodeId) -> EurePath {
    paths.get(&schema_id).cloned().unwrap_or_else(|| {
        EurePath(vec![PathSegment::Value(ObjectKey::String(format!(
            "schema-node-{}",
            schema_id.0
        )))])
    })
}

fn explicit_variant_from_document(
    doc: &EureDocument,
    node_id: NodeId,
) -> Result<Option<VariantPath>, TypeTraceUnresolvedReason> {
    extract_explicit_variant_path(doc, node_id)
        .map_err(type_trace_parse_error)
        .map(|variant| variant.and_then(|path| (!path.is_empty()).then_some(path)))
}

fn type_trace_parse_error(error: ParseError) -> TypeTraceUnresolvedReason {
    TypeTraceUnresolvedReason::InvalidVariantTag {
        tag: error.to_string(),
    }
}
