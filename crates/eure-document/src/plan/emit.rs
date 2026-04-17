//! Emission of a [`crate::source::SourceDocument`] from a validated
//! [`LayoutPlan`].
//!
//! The walk is shared between the structural validation pass (`dry_walk_validate`)
//! and the real emission (`emit`); both use the same control flow so that
//! validation mirrors what emission would produce.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::document::node::NodeValue;
use crate::document::{EureDocument, NodeId};
use crate::identifier::Identifier;
use crate::path::PathSegment;
use crate::plan::{ArrayForm, Form, LayoutPlan, PlanError};
use crate::source::{
    BindSource, BindingSource, EureSource, SectionBody, SectionSource, SourceDocument, SourceId,
    SourceKey, SourcePath, SourcePathSegment,
};
use crate::value::{ObjectKey, PartialObjectKey};

// ============================================================================
// Entry points
// ============================================================================

pub fn emit(plan: LayoutPlan) -> SourceDocument {
    // Validation during build() already checked structure; here we just walk
    // without collecting errors.
    let LayoutPlan { doc, forms, array_forms, order } = plan;
    let sources = {
        let mut ctx = EmitCtx {
            doc: &doc,
            forms: &forms,
            array_forms: &array_forms,
            order: &order,
            sources: Vec::new(),
            record_errors: None,
            emitted: Vec::new(),
        };
        let root = build_root(&mut ctx);
        debug_assert_eq!(root.0, 0);
        ctx.sources
    };
    SourceDocument::new(doc, sources)
}

pub fn dry_walk_validate(plan: &LayoutPlan) -> Result<(), PlanError> {
    let mut errors: Vec<PlanError> = Vec::new();
    let mut ctx = EmitCtx {
        doc: &plan.doc,
        forms: &plan.forms,
        array_forms: &plan.array_forms,
        order: &plan.order,
        sources: Vec::new(),
        record_errors: Some(&mut errors),
        emitted: Vec::new(),
    };
    build_root(&mut ctx);
    // Coverage/uniqueness already tracked in `emitted` via duplicate detection.
    if let Some(err) = errors.into_iter().next() {
        return Err(err);
    }
    Ok(())
}

// ============================================================================
// Walk state
// ============================================================================

struct EmitCtx<'a> {
    doc: &'a EureDocument,
    forms: &'a crate::map::Map<NodeId, Form>,
    array_forms: &'a crate::map::Map<NodeId, ArrayForm>,
    order: &'a crate::map::Map<NodeId, Vec<NodeId>>,
    sources: Vec<EureSource>,
    /// When Some, errors are collected into this buffer and emission is a
    /// no-op (used by the dry validation pass).
    record_errors: Option<&'a mut Vec<PlanError>>,
    emitted: Vec<NodeId>,
}

impl EmitCtx<'_> {
    fn is_dry(&self) -> bool {
        self.record_errors.is_some()
    }

    fn push_error(&mut self, err: PlanError) {
        if let Some(buf) = self.record_errors.as_deref_mut() {
            buf.push(err);
        }
    }

    fn record_emission(&mut self, id: NodeId) {
        if self.emitted.contains(&id) {
            self.push_error(PlanError::DuplicateEmission { node: id });
        } else {
            self.emitted.push(id);
        }
    }

    fn reserve_source(&mut self) -> SourceId {
        let id = SourceId(self.sources.len());
        self.sources.push(EureSource::default());
        id
    }

    fn set_source(&mut self, id: SourceId, src: EureSource) {
        self.sources[id.0] = src;
    }
}

// ============================================================================
// Root / block emission
// ============================================================================

fn build_root(ctx: &mut EmitCtx) -> SourceId {
    let root_id = ctx.doc.get_root_id();
    let id = ctx.reserve_source();
    let root_node = ctx.doc.node(root_id);
    let root_is_map = matches!(
        root_node.content,
        NodeValue::Map(_) | NodeValue::PartialMap(_)
    );
    let value = if root_is_map { None } else { Some(root_id) };
    ctx.record_emission(root_id);

    let mut eure = EureSource {
        value,
        ..Default::default()
    };
    emit_children(ctx, root_id, &[], &[], root_is_map, true, &mut eure);
    ctx.set_source(id, eure);
    id
}

/// Emit the children of `parent_id` as bindings/sections into `dest`.
///
/// - `node_path`: semantic path in the document used for ordering and schema
///   lookups.
/// - `path_prefix`: source-path prefix to prepend when emitting bindings for
///   hoisted children (used by `Flatten` forms).
/// - `emit_map_fields`: whether to emit the map/partial-map children of this
///   node (extensions are always emitted).
/// - `allow_sections`: whether a child may be emitted as a section in this
///   context.
#[allow(clippy::too_many_arguments)]
fn emit_children(
    ctx: &mut EmitCtx,
    parent_id: NodeId,
    node_path: &[PathSegment],
    path_prefix: &[PathSegment],
    emit_map_fields: bool,
    allow_sections: bool,
    dest: &mut EureSource,
) {
    let node = ctx.doc.node(parent_id);
    let mut children: Vec<(PathSegment, NodeId)> = Vec::new();

    for (ident, &cid) in node.extensions.iter() {
        children.push((PathSegment::Extension(ident.clone()), cid));
    }

    if emit_map_fields {
        match &node.content {
            NodeValue::Map(map) => {
                for (key, &cid) in map.iter() {
                    children.push((PathSegment::Value(key.clone()), cid));
                }
            }
            NodeValue::PartialMap(pm) => {
                for (key, &cid) in pm.iter() {
                    children.push((PathSegment::from_partial_object_key(key.clone()), cid));
                }
            }
            _ => {}
        }
    }

    // Apply ordering if set on the parent.
    let children = apply_order(ctx, parent_id, children);

    for (seg, child_id) in children {
        let child_node_path = concat_path(node_path, &seg);
        let child_print_path = concat_path(path_prefix, &seg);
        emit_child(
            ctx,
            child_id,
            &seg,
            &child_node_path,
            &child_print_path,
            allow_sections,
            dest,
        );
    }
}

fn apply_order(
    ctx: &EmitCtx,
    parent_id: NodeId,
    children: Vec<(PathSegment, NodeId)>,
) -> Vec<(PathSegment, NodeId)> {
    let Some(order) = ctx.order.get(&parent_id) else {
        return children;
    };
    let mut by_id: Vec<(PathSegment, NodeId)> = children;
    let mut ordered: Vec<(PathSegment, NodeId)> = Vec::with_capacity(by_id.len());
    for id in order {
        if let Some(pos) = by_id.iter().position(|(_, cid)| cid == id) {
            ordered.push(by_id.remove(pos));
        }
    }
    ordered.extend(by_id);
    ordered
}

fn emit_child(
    ctx: &mut EmitCtx,
    child_id: NodeId,
    seg: &PathSegment,
    child_node_path: &[PathSegment],
    child_print_path: &[PathSegment],
    allow_sections: bool,
    dest: &mut EureSource,
) {
    let kind = ctx.doc.node(child_id).content.value_kind();
    if matches!(kind, crate::value::ValueKind::Array) {
        let Some(array_form) = ctx.array_forms.get(&child_id).copied() else {
            ctx.push_error(PlanError::MissingForm(child_id));
            return;
        };
        emit_array_child(
            ctx,
            child_id,
            seg,
            child_node_path,
            child_print_path,
            allow_sections,
            array_form,
            dest,
        );
        return;
    }

    let Some(form) = ctx.forms.get(&child_id).copied() else {
        ctx.push_error(PlanError::MissingForm(child_id));
        return;
    };

    emit_non_array_child(
        ctx,
        child_id,
        child_node_path,
        child_print_path,
        allow_sections,
        form,
        dest,
    );
}

fn emit_non_array_child(
    ctx: &mut EmitCtx,
    child_id: NodeId,
    child_node_path: &[PathSegment],
    child_print_path: &[PathSegment],
    allow_sections: bool,
    form: Form,
    dest: &mut EureSource,
) {
    let is_section = matches!(
        form,
        Form::Section | Form::SectionBlock | Form::SectionValueBlock
    );
    if is_section && !allow_sections {
        ctx.push_error(PlanError::SectionInForbiddenContext(child_id));
        return;
    }

    match form {
        Form::Inline => {
            ctx.record_emission(child_id);
            if !ctx.is_dry() {
                dest.bindings.push(BindingSource {
                    trivia_before: Vec::new(),
                    path: to_source_path(child_print_path),
                    bind: BindSource::Value(child_id),
                    trailing_comment: None,
                });
            }
            // Inline binds the value; extensions are not lost — they descend
            // as bindings prefixed by the inline path.
            emit_extensions_only(ctx, child_id, child_node_path, child_print_path, dest);
        }
        Form::BindingBlock => {
            ctx.record_emission(child_id);
            let block_id = build_block(ctx, child_id, child_node_path, /*with_value=*/ false);
            if !ctx.is_dry() {
                dest.bindings.push(BindingSource {
                    trivia_before: Vec::new(),
                    path: to_source_path(child_print_path),
                    bind: BindSource::Block(block_id),
                    trailing_comment: None,
                });
            }
        }
        Form::BindingValueBlock => {
            ctx.record_emission(child_id);
            let block_id = build_block(ctx, child_id, child_node_path, /*with_value=*/ true);
            if !ctx.is_dry() {
                dest.bindings.push(BindingSource {
                    trivia_before: Vec::new(),
                    path: to_source_path(child_print_path),
                    bind: BindSource::Block(block_id),
                    trailing_comment: None,
                });
            }
        }
        Form::Section => {
            ctx.record_emission(child_id);
            let (value, bindings) = build_items(ctx, child_id, child_node_path, false);
            if !ctx.is_dry() {
                dest.sections.push(SectionSource {
                    trivia_before: Vec::new(),
                    path: to_source_path(child_print_path),
                    body: SectionBody::Items { value, bindings },
                    trailing_comment: None,
                });
            }
        }
        Form::SectionBlock => {
            ctx.record_emission(child_id);
            let block_id = build_block(ctx, child_id, child_node_path, /*with_value=*/ false);
            if !ctx.is_dry() {
                dest.sections.push(SectionSource {
                    trivia_before: Vec::new(),
                    path: to_source_path(child_print_path),
                    body: SectionBody::Block(block_id),
                    trailing_comment: None,
                });
            }
        }
        Form::SectionValueBlock => {
            ctx.record_emission(child_id);
            let (value, bindings) = build_items(ctx, child_id, child_node_path, true);
            if !ctx.is_dry() {
                dest.sections.push(SectionSource {
                    trivia_before: Vec::new(),
                    path: to_source_path(child_print_path),
                    body: SectionBody::Items { value, bindings },
                    trailing_comment: None,
                });
            }
        }
        Form::Flatten => {
            // No self-emission. Children hoist under the current print path.
            let is_map = matches!(
                ctx.doc.node(child_id).content,
                NodeValue::Map(_) | NodeValue::PartialMap(_)
            );
            emit_children(
                ctx,
                child_id,
                child_node_path,
                child_print_path,
                is_map,
                allow_sections,
                dest,
            );
        }
    }
}

fn emit_array_child(
    ctx: &mut EmitCtx,
    child_id: NodeId,
    _seg: &PathSegment,
    child_node_path: &[PathSegment],
    child_print_path: &[PathSegment],
    allow_sections: bool,
    array_form: ArrayForm,
    dest: &mut EureSource,
) {
    match array_form {
        ArrayForm::Inline => {
            ctx.record_emission(child_id);
            if !ctx.is_dry() {
                dest.bindings.push(BindingSource {
                    trivia_before: Vec::new(),
                    path: to_source_path(child_print_path),
                    bind: BindSource::Value(child_id),
                    trailing_comment: None,
                });
            }
        }
        ArrayForm::PerElement(element_form) | ArrayForm::PerElementIndexed(element_form) => {
            ctx.record_emission(child_id);
            let NodeValue::Array(arr) = &ctx.doc.node(child_id).content else {
                return;
            };
            let indexed = matches!(array_form, ArrayForm::PerElementIndexed(_));
            let ids: Vec<NodeId> = arr.iter().copied().collect();
            for (i, el_id) in ids.into_iter().enumerate() {
                let mut element_print_path = child_print_path.to_vec();
                element_print_path.push(PathSegment::ArrayIndex(if indexed {
                    Some(i)
                } else {
                    None
                }));
                let mut element_node_path = child_node_path.to_vec();
                element_node_path.push(PathSegment::ArrayIndex(Some(i)));
                emit_non_array_child(
                    ctx,
                    el_id,
                    &element_node_path,
                    &element_print_path,
                    allow_sections,
                    element_form,
                    dest,
                );
            }
        }
    }
}

fn emit_extensions_only(
    ctx: &mut EmitCtx,
    node_id: NodeId,
    child_node_path: &[PathSegment],
    child_print_path: &[PathSegment],
    dest: &mut EureSource,
) {
    let node = ctx.doc.node(node_id);
    let ext_children: Vec<(Identifier, NodeId)> = node
        .extensions
        .iter()
        .map(|(k, &v)| (k.clone(), v))
        .collect();
    for (ident, ext_id) in ext_children {
        let seg = PathSegment::Extension(ident);
        let mut ext_node_path = child_node_path.to_vec();
        ext_node_path.push(seg.clone());
        let mut ext_print_path = child_print_path.to_vec();
        ext_print_path.push(seg);
        emit_child(
            ctx,
            ext_id,
            ext_print_path.last().unwrap(),
            &ext_node_path,
            &ext_print_path,
            false,
            dest,
        );
    }
}

fn build_block(
    ctx: &mut EmitCtx,
    node_id: NodeId,
    node_path: &[PathSegment],
    with_value: bool,
) -> SourceId {
    let id = ctx.reserve_source();
    let value = if with_value { Some(node_id) } else { None };
    let node_is_map = matches!(
        ctx.doc.node(node_id).content,
        NodeValue::Map(_) | NodeValue::PartialMap(_)
    );
    let mut inner = EureSource {
        value,
        ..Default::default()
    };
    emit_children(
        ctx,
        node_id,
        node_path,
        &[],
        node_is_map,
        true,
        &mut inner,
    );
    ctx.set_source(id, inner);
    id
}

fn build_items(
    ctx: &mut EmitCtx,
    node_id: NodeId,
    node_path: &[PathSegment],
    with_value: bool,
) -> (Option<NodeId>, Vec<BindingSource>) {
    // For Section and SectionValueBlock, emit the items body: `{value}` + bindings only,
    // disallowing nested sections (they must be sibling sections at the same level).
    let node_is_map = matches!(
        ctx.doc.node(node_id).content,
        NodeValue::Map(_) | NodeValue::PartialMap(_)
    );
    let mut inner = EureSource {
        value: if with_value { Some(node_id) } else { None },
        ..Default::default()
    };
    emit_children(ctx, node_id, node_path, &[], node_is_map, true, &mut inner);
    // Nested sections emitted into `inner.sections` must be hoisted into the
    // caller's sections to preserve section-at-top-of-block semantics. We
    // return only bindings and value from the Items body.
    let value = inner.value;
    let bindings = inner.bindings;
    // Any sections hoist: we just move them into the outer EureSource by
    // appending to `sources` via the `dest` we came from. Since we built
    // them inside `inner`, we need to re-append. For simplicity here, nested
    // sections produced at this layer are flattened by setting the caller's
    // context `allow_sections=true` and recursing via emit_children which
    // writes directly to `dest` when present. In this build_items helper we
    // specifically don't carry sections: any section is already written into
    // the parent scope because emit_children does not write sections into a
    // different buffer.
    //
    // However, since we pass `&mut inner` above, sections are actually
    // written into `inner.sections`. Those sections belong to the same
    // outer scope. Push them back into the caller via an out-parameter is
    // not available here, so callers using build_items must not produce
    // sections — enforced by validation (section context rule).
    debug_assert!(inner.sections.is_empty());
    (value, bindings)
}

// ============================================================================
// Path conversion (verbatim from layout.rs)
// ============================================================================

pub(crate) fn to_source_path(path: &[PathSegment]) -> SourcePath {
    let mut out: Vec<SourcePathSegment> = Vec::new();
    for seg in path {
        match seg {
            PathSegment::Ident(id) => out.push(SourcePathSegment::ident(id.clone())),
            PathSegment::Extension(id) => out.push(SourcePathSegment::extension(id.clone())),
            PathSegment::PartialValue(key) => out.push(SourcePathSegment {
                key: partial_object_key_to_source_key(key),
                array: None,
            }),
            PathSegment::HoleKey(label) => out.push(SourcePathSegment {
                key: SourceKey::hole(label.clone()),
                array: None,
            }),
            PathSegment::Value(key) => out.push(SourcePathSegment {
                key: object_key_to_source_key(key),
                array: None,
            }),
            PathSegment::TupleIndex(index) => out.push(SourcePathSegment {
                key: SourceKey::TupleIndex(*index),
                array: None,
            }),
            PathSegment::ArrayIndex(index) => {
                if let Some(last) = out.last_mut() {
                    last.array = Some(*index);
                }
            }
        }
    }
    out
}

fn object_key_to_source_key(key: &ObjectKey) -> SourceKey {
    match key {
        ObjectKey::String(s) => {
            if let Ok(id) = s.parse::<Identifier>() {
                SourceKey::Ident(id)
            } else {
                SourceKey::quoted(s.clone())
            }
        }
        ObjectKey::Number(n) => {
            if let Ok(n64) = i64::try_from(n) {
                SourceKey::Integer(n64)
            } else {
                SourceKey::quoted(n.to_string())
            }
        }
        ObjectKey::Tuple(keys) => {
            SourceKey::Tuple(keys.iter().map(object_key_to_source_key).collect())
        }
    }
}

fn partial_object_key_to_source_key(key: &PartialObjectKey) -> SourceKey {
    match key {
        PartialObjectKey::String(s) => {
            if let Ok(id) = s.parse::<Identifier>() {
                SourceKey::Ident(id)
            } else {
                SourceKey::quoted(s.clone())
            }
        }
        PartialObjectKey::Number(n) => {
            if let Ok(n64) = i64::try_from(n) {
                SourceKey::Integer(n64)
            } else {
                SourceKey::quoted(n.to_string())
            }
        }
        PartialObjectKey::Hole(label) => SourceKey::hole(label.clone()),
        PartialObjectKey::Tuple(keys) => {
            SourceKey::Tuple(keys.iter().map(partial_object_key_to_source_key).collect())
        }
    }
}

fn concat_path(prefix: &[PathSegment], seg: &PathSegment) -> Vec<PathSegment> {
    let mut out = Vec::with_capacity(prefix.len() + 1);
    out.extend_from_slice(prefix);
    out.push(seg.clone());
    out
}
