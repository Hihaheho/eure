//! Total, verified layout plan for projecting an [`EureDocument`] to
//! [`SourceDocument`].
//!
//! A [`LayoutPlan`] owns the document and assigns every reachable node an
//! explicit emission shape ([`Form`] for non-Array nodes, [`ArrayForm`] for
//! Array nodes). Validation runs in [`PlanBuilder::build`]; [`LayoutPlan::emit`]
//! consumes the plan and produces a [`SourceDocument`] without any silent
//! fallbacks.
//!
//! Compared to the previous `DocLayout` best-effort projection, this module:
//!
//! - Totality: every reachable [`NodeId`] is assigned a shape.
//! - Explicit errors: all failure cases are typed [`PlanError`] variants.
//! - Orthogonal arrays: array handling is a separate dimension from shape, so
//!   every grammar pattern (`items[] = v`, `items[] { ... }`, `@ items[]`,
//!   `@ items[] { ... }`, etc.) is reachable.

pub mod emit;
pub mod traverse;

use alloc::vec::Vec;

use crate::document::node::NodeValue;
use crate::document::{EureDocument, NodeId};
use crate::map::Map;
use crate::path::PathSegment;
use crate::value::ValueKind;

/// One of the seven semantic shapes a non-Array node can take.
///
/// Six correspond to the grammar patterns documented in `source.rs`;
/// `Flatten` hoists children into the parent context without emitting the
/// node itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    /// Pattern #1: `path = value`.
    Inline,
    /// Pattern #2: `path { ... }`.
    BindingBlock,
    /// Pattern #3: `path { = value ... }`.
    BindingValueBlock,
    /// Pattern #4: `@ path` with items.
    Section,
    /// Pattern #5: `@ path { ... }`.
    SectionBlock,
    /// Pattern #6: `@ path { = value ... }`.
    SectionValueBlock,
    /// No self-emission. Children are hoisted into the parent.
    Flatten,
}

/// How an [`NodeValue::Array`] node is emitted. Orthogonal to [`Form`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrayForm {
    /// Single inline binding: `path = [e1, e2, ...]`.
    Inline,
    /// Per-element emission with `[]` (push marker). Each element uses the
    /// given element [`Form`].
    PerElement(Form),
    /// Per-element emission with explicit `[i]` indices.
    PerElementIndexed(Form),
}

/// A declarative, validated layout plan for a document.
#[derive(Debug, Clone)]
pub struct LayoutPlan {
    doc: EureDocument,
    forms: Map<NodeId, Form>,
    array_forms: Map<NodeId, ArrayForm>,
    order: Map<NodeId, Vec<NodeId>>,
}

/// Mutable builder for a [`LayoutPlan`].
#[derive(Debug, Clone)]
pub struct PlanBuilder {
    doc: EureDocument,
    forms: Map<NodeId, Form>,
    array_forms: Map<NodeId, ArrayForm>,
    order: Map<NodeId, Vec<NodeId>>,
}

/// Reason an [`ArrayForm`] assignment is incompatible with the array content.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayFormReason {
    /// Some element is incompatible with the requested per-element [`Form`].
    ElementIncompatibleForm { element: NodeId, kind: ValueKind },
    /// `PerElement(Flatten)` — flattening anonymous array elements into the
    /// parent path is always rejected (it would collapse distinct elements
    /// onto the same path).
    FlattenElementDisallowed,
}

/// Errors produced during plan validation or construction.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    #[error("path {path:?} does not resolve in document")]
    PathNotFound { path: Vec<PathSegment> },

    #[error("form {form:?} is incompatible with node {node:?} of kind {kind}")]
    IncompatibleForm {
        node: NodeId,
        form: Form,
        kind: ValueKind,
    },

    #[error("array form {form:?} is incompatible with array node {node:?}: {reason:?}")]
    IncompatibleArrayForm {
        node: NodeId,
        form: ArrayForm,
        reason: ArrayFormReason,
    },

    #[error("node {0:?} has no assigned form")]
    MissingForm(NodeId),

    #[error("node {node:?} would be emitted multiple times")]
    DuplicateEmission { node: NodeId },

    #[error("section form at node {0:?} is not allowed in this context")]
    SectionInForbiddenContext(NodeId),

    #[error("schema produced conflicting forms for node {node:?}")]
    ConflictingOverride { node: NodeId },

    #[error("ordered child {child:?} is not a direct child of parent {parent:?}")]
    OrderChildNotDirect { parent: NodeId, child: NodeId },

    #[error("ordered children for parent {parent:?} contain duplicate {child:?}")]
    OrderDuplicateChild { parent: NodeId, child: NodeId },

    #[error(
        "ordered child {child:?} of parent {parent:?} is an extension; extension order is fixed"
    )]
    OrderExtensionChild { parent: NodeId, child: NodeId },

    #[error("set_form called on array node {0:?}; use set_array_form")]
    FormOnArrayNode(NodeId),

    #[error("set_array_form called on non-array node {0:?}; use set_form")]
    ArrayFormOnNonArray(NodeId),

    #[error("set_form called on root node {0:?}; root form is implicit")]
    FormOnRoot(NodeId),

    #[error("order requested on array node {0:?}; element order is data")]
    OrderOnArrayNode(NodeId),
}

// ============================================================================
// Compatibility rules
// ============================================================================

/// Returns `Ok(())` if the given [`Form`] is compatible with the given
/// [`ValueKind`], otherwise an [`PlanError::IncompatibleForm`].
pub(crate) fn check_form_compat(id: NodeId, form: Form, kind: ValueKind) -> Result<(), PlanError> {
    let compatible = match form {
        Form::Inline => !matches!(kind, ValueKind::PartialMap),
        Form::BindingBlock | Form::Section | Form::SectionBlock | Form::Flatten => {
            matches!(kind, ValueKind::Map | ValueKind::PartialMap)
        }
        Form::BindingValueBlock | Form::SectionValueBlock => {
            // A distinguished self-value is only realizable when the node has
            // primitive-like content plus extensions; grammatically this is
            // `path { = value ... }` / `@ path { = value ... }`.
            matches!(
                kind,
                ValueKind::Hole
                    | ValueKind::Null
                    | ValueKind::Bool
                    | ValueKind::Integer
                    | ValueKind::F32
                    | ValueKind::F64
                    | ValueKind::Text
                    | ValueKind::Tuple
            )
        }
    };
    if compatible {
        Ok(())
    } else {
        Err(PlanError::IncompatibleForm {
            node: id,
            form,
            kind,
        })
    }
}

/// Validate an [`ArrayForm`] assignment on an array node.
pub(crate) fn check_array_form_compat(
    doc: &EureDocument,
    id: NodeId,
    form: ArrayForm,
) -> Result<(), PlanError> {
    match form {
        ArrayForm::Inline => Ok(()),
        ArrayForm::PerElement(Form::Flatten) | ArrayForm::PerElementIndexed(Form::Flatten) => {
            Err(PlanError::IncompatibleArrayForm {
                node: id,
                form,
                reason: ArrayFormReason::FlattenElementDisallowed,
            })
        }
        ArrayForm::PerElement(element) | ArrayForm::PerElementIndexed(element) => {
            let element_ids = match &doc.node(id).content {
                NodeValue::Array(arr) => arr.iter().copied().collect::<Vec<_>>(),
                _ => return Err(PlanError::ArrayFormOnNonArray(id)),
            };
            for element_id in element_ids {
                let kind = doc.node(element_id).content.value_kind();
                if check_form_compat(element_id, element, kind).is_err() {
                    return Err(PlanError::IncompatibleArrayForm {
                        node: id,
                        form,
                        reason: ArrayFormReason::ElementIncompatibleForm {
                            element: element_id,
                            kind,
                        },
                    });
                }
            }
            Ok(())
        }
    }
}

// ============================================================================
// Builder
// ============================================================================

impl PlanBuilder {
    /// Construct a new builder for the given document.
    pub fn new(doc: EureDocument) -> Self {
        Self {
            doc,
            forms: Map::default(),
            array_forms: Map::default(),
            order: Map::default(),
        }
    }

    /// Borrow the underlying document.
    pub fn document(&self) -> &EureDocument {
        &self.doc
    }

    /// Resolve a path to a [`NodeId`] relative to the document root.
    pub fn node_at(&self, path: &[PathSegment]) -> Result<NodeId, PlanError> {
        let mut current = self.doc.get_root_id();
        for segment in path {
            match traverse::child_node_id(&self.doc, current, segment) {
                Some(id) => current = id,
                None => {
                    return Err(PlanError::PathNotFound {
                        path: path.to_vec(),
                    });
                }
            }
        }
        Ok(current)
    }

    /// Assign a [`Form`] to a non-Array node.
    pub fn set_form(&mut self, id: NodeId, form: Form) -> Result<&mut Self, PlanError> {
        if id == self.doc.get_root_id() {
            return Err(PlanError::FormOnRoot(id));
        }
        let kind = self.doc.node(id).content.value_kind();
        if matches!(kind, ValueKind::Array) {
            return Err(PlanError::FormOnArrayNode(id));
        }
        check_form_compat(id, form, kind)?;
        self.forms.insert(id, form);
        Ok(self)
    }

    /// Assign a [`Form`] to the node at the given path.
    pub fn set_form_at(
        &mut self,
        path: &[PathSegment],
        form: Form,
    ) -> Result<&mut Self, PlanError> {
        let id = self.node_at(path)?;
        self.set_form(id, form)
    }

    /// Assign an [`ArrayForm`] to an Array node.
    pub fn set_array_form(&mut self, id: NodeId, form: ArrayForm) -> Result<&mut Self, PlanError> {
        let kind = self.doc.node(id).content.value_kind();
        if !matches!(kind, ValueKind::Array) {
            return Err(PlanError::ArrayFormOnNonArray(id));
        }
        check_array_form_compat(&self.doc, id, form)?;
        self.array_forms.insert(id, form);
        Ok(self)
    }

    /// Assign an [`ArrayForm`] to the Array node at the given path.
    pub fn set_array_form_at(
        &mut self,
        path: &[PathSegment],
        form: ArrayForm,
    ) -> Result<&mut Self, PlanError> {
        let id = self.node_at(path)?;
        self.set_array_form(id, form)
    }

    /// Order the direct children of a non-Array parent. Unlisted children
    /// are appended after the listed ones in document order.
    pub fn order(&mut self, parent: NodeId, children: Vec<NodeId>) -> Result<&mut Self, PlanError> {
        if matches!(self.doc.node(parent).content.value_kind(), ValueKind::Array) {
            return Err(PlanError::OrderOnArrayNode(parent));
        }
        let direct = traverse::children_of(&self.doc, parent);

        let mut seen = Vec::with_capacity(children.len());
        for child in &children {
            let Some((segment, _)) = direct.iter().find(|(_, id)| id == child) else {
                return Err(PlanError::OrderChildNotDirect {
                    parent,
                    child: *child,
                });
            };
            if matches!(segment, PathSegment::Extension(_)) {
                return Err(PlanError::OrderExtensionChild {
                    parent,
                    child: *child,
                });
            }
            if seen.contains(child) {
                return Err(PlanError::OrderDuplicateChild {
                    parent,
                    child: *child,
                });
            }
            seen.push(*child);
        }
        self.order.insert(parent, children);
        Ok(self)
    }

    /// Order the direct children of the parent at the given path by child
    /// path segments.
    pub fn order_at(
        &mut self,
        parent_path: &[PathSegment],
        segs: Vec<PathSegment>,
    ) -> Result<&mut Self, PlanError> {
        let parent = self.node_at(parent_path)?;
        let direct = traverse::children_of(&self.doc, parent);
        let mut children = Vec::with_capacity(segs.len());
        for seg in &segs {
            let id = direct
                .iter()
                .find(|(s, _)| s == seg)
                .map(|(_, id)| *id)
                .ok_or_else(|| PlanError::PathNotFound {
                    path: {
                        let mut p = parent_path.to_vec();
                        p.push(seg.clone());
                        p
                    },
                })?;
            children.push(id);
        }
        self.order(parent, children)
    }

    /// Fill defaults, validate, and finalize.
    pub fn build(mut self) -> Result<LayoutPlan, PlanError> {
        // 1. Fill defaults via auto() policy for every unassigned reachable id.
        //    Walk from root threading `allow_sections` so that auto picks a
        //    block form (not Section) in contexts where sections are forbidden
        //    (inside a Section / SectionValueBlock items body). This prevents
        //    auto-assignment from generating structurally invalid plans.
        let root = self.doc.get_root_id();
        self.fill_defaults(root, true);

        // 2. Compatibility check (defensive — set_form already checks, but
        // auto-filled defaults must also be valid).
        let all = traverse::all_reachable_ids(&self.doc);
        for id in &all {
            let kind = self.doc.node(*id).content.value_kind();
            if matches!(kind, ValueKind::Array) {
                let form = *self
                    .array_forms
                    .get(id)
                    .ok_or(PlanError::MissingForm(*id))?;
                check_array_form_compat(&self.doc, *id, form)?;
            } else if *id != self.doc.get_root_id() {
                let form = *self.forms.get(id).ok_or(PlanError::MissingForm(*id))?;
                check_form_compat(*id, form, kind)?;
            }
        }

        // 3. Section context + 4. array feasibility + 5. coverage/uniqueness
        // are enforced during emission's dry walk.
        let plan = LayoutPlan {
            doc: self.doc,
            forms: self.forms,
            array_forms: self.array_forms,
            order: self.order,
        };
        plan.validate_structure()?;
        Ok(plan)
    }

    fn fill_defaults(&mut self, id: NodeId, allow_sections: bool) {
        let kind = self.doc.node(id).content.value_kind();
        let root = self.doc.get_root_id();

        if matches!(kind, ValueKind::Array) {
            let array_form = match self.array_forms.get(&id).copied() {
                Some(f) => f,
                None => {
                    let auto = auto::auto_array_form(&self.doc, id, allow_sections);
                    self.array_forms.insert(id, auto);
                    auto
                }
            };
            let elem_form: Option<Form> = match array_form {
                ArrayForm::Inline => None,
                ArrayForm::PerElement(f) | ArrayForm::PerElementIndexed(f) => Some(f),
            };
            for (_, elem_id) in traverse::children_of(&self.doc, id) {
                self.fill_element_defaults(elem_id, allow_sections, elem_form);
            }
            return;
        }

        let child_allow = if id == root {
            true
        } else {
            let form = match self.forms.get(&id).copied() {
                Some(f) => f,
                None => {
                    let auto = auto::auto_form(&self.doc, id, allow_sections);
                    self.forms.insert(id, auto);
                    auto
                }
            };
            child_allow_for_form(form, allow_sections)
        };

        for (_, child_id) in traverse::children_of(&self.doc, id) {
            self.fill_defaults(child_id, child_allow);
        }
    }

    /// Fill defaults for an array element. `elem_form` is the `Form` dictated
    /// by the array's `ArrayForm::PerElement*` variant; when `None`, the
    /// element is rendered as an inline value (its descendants are not
    /// emitted as standalone nodes but we still fill defaults for validation).
    fn fill_element_defaults(&mut self, id: NodeId, outer_allow: bool, elem_form: Option<Form>) {
        let kind = self.doc.node(id).content.value_kind();

        if matches!(kind, ValueKind::Array) {
            self.fill_defaults(id, outer_allow);
            return;
        }

        // Set the element's own form. This is NOT the form used to emit the
        // element (emit_array_child uses `elem_form` directly for PerElement*),
        // but we still must populate `forms` so the compat pass doesn't raise
        // `MissingForm`. Prefer `elem_form` when compatible with the element's
        // kind; otherwise fall back to the context-aware auto policy.
        if !self.forms.contains_key(&id) {
            let chosen = match elem_form {
                Some(f) if check_form_compat(id, f, kind).is_ok() => f,
                _ => auto::auto_form(&self.doc, id, outer_allow),
            };
            self.forms.insert(id, chosen);
        }

        // Descent into element's children uses the element's effective emit
        // form to determine whether sections are allowed inside it.
        let child_allow = match elem_form {
            Some(f) => child_allow_for_form(f, outer_allow),
            None => outer_allow,
        };
        for (_, child_id) in traverse::children_of(&self.doc, id) {
            self.fill_defaults(child_id, child_allow);
        }
    }
}

/// Whether children of a node with the given [`Form`] may be emitted as
/// sections. `build_items` (used by `Section` / `SectionValueBlock`) requires
/// that its body contain no sections, so those forms propagate
/// `allow_sections = false` to their descendants.
fn child_allow_for_form(form: Form, outer_allow: bool) -> bool {
    match form {
        Form::Inline => outer_allow,
        Form::BindingBlock | Form::BindingValueBlock | Form::SectionBlock => true,
        Form::Section | Form::SectionValueBlock => false,
        Form::Flatten => outer_allow,
    }
}

// ============================================================================
// auto() policy
// ============================================================================

pub(crate) mod auto {
    use super::{ArrayForm, Form};
    use crate::document::node::NodeValue;
    use crate::document::{EureDocument, NodeId};
    use crate::value::ValueKind;

    /// The default [`Form`] for a non-root, non-Array node. When
    /// `allow_sections` is false, Map/PartialMap candidates for [`Form::Section`]
    /// are downgraded to [`Form::BindingBlock`] — this is what keeps the
    /// auto policy structurally valid inside `build_items` bodies.
    pub(crate) fn auto_form(doc: &EureDocument, id: NodeId, allow_sections: bool) -> Form {
        let node = doc.node(id);
        let kind = node.content.value_kind();
        match kind {
            ValueKind::Hole
            | ValueKind::Null
            | ValueKind::Bool
            | ValueKind::Integer
            | ValueKind::F32
            | ValueKind::F64
            | ValueKind::Text
            | ValueKind::Tuple => Form::Inline,
            ValueKind::Array => unreachable!("arrays use auto_array_form"),
            ValueKind::Map | ValueKind::PartialMap => {
                if !node.extensions.is_empty() || map_has_complex_child(doc, &node.content) {
                    if allow_sections {
                        Form::Section
                    } else {
                        Form::BindingBlock
                    }
                } else if map_only_scalar_children(doc, &node.content) {
                    Form::Inline
                } else {
                    Form::BindingBlock
                }
            }
        }
    }

    /// The default [`ArrayForm`] for an Array node. When `allow_sections` is
    /// false, `PerElement(Section)` is downgraded to `PerElement(BindingBlock)`.
    pub(crate) fn auto_array_form(
        doc: &EureDocument,
        id: NodeId,
        allow_sections: bool,
    ) -> ArrayForm {
        let NodeValue::Array(arr) = &doc.node(id).content else {
            return ArrayForm::Inline;
        };
        if arr.is_empty() {
            return ArrayForm::Inline;
        }
        let all_maps = arr.iter().all(|&el| {
            matches!(
                doc.node(el).content.value_kind(),
                ValueKind::Map | ValueKind::PartialMap
            )
        });
        if all_maps {
            let elem_form = if allow_sections {
                Form::Section
            } else {
                Form::BindingBlock
            };
            ArrayForm::PerElement(elem_form)
        } else {
            ArrayForm::Inline
        }
    }

    fn map_only_scalar_children(doc: &EureDocument, content: &NodeValue) -> bool {
        let ids: alloc::vec::Vec<NodeId> = match content {
            NodeValue::Map(map) => map.iter().map(|(_, &id)| id).collect(),
            NodeValue::PartialMap(pm) => pm.iter().map(|(_, &id)| id).collect(),
            _ => return true,
        };
        ids.iter().all(|&child| {
            let child_node = doc.node(child);
            child_node.extensions.is_empty()
                && matches!(
                    child_node.content.value_kind(),
                    ValueKind::Null
                        | ValueKind::Bool
                        | ValueKind::Integer
                        | ValueKind::F32
                        | ValueKind::F64
                        | ValueKind::Text
                        | ValueKind::Hole
                        | ValueKind::Tuple
                )
        })
    }

    fn map_has_complex_child(doc: &EureDocument, content: &NodeValue) -> bool {
        let ids: alloc::vec::Vec<NodeId> = match content {
            NodeValue::Map(map) => map.iter().map(|(_, &id)| id).collect(),
            NodeValue::PartialMap(pm) => pm.iter().map(|(_, &id)| id).collect(),
            _ => return false,
        };
        ids.iter().any(|&child| {
            let child_node = doc.node(child);
            !child_node.extensions.is_empty()
                || matches!(
                    child_node.content.value_kind(),
                    ValueKind::Map | ValueKind::PartialMap | ValueKind::Array
                )
        })
    }
}

// ============================================================================
// LayoutPlan
// ============================================================================

impl LayoutPlan {
    /// Start a new [`PlanBuilder`] for the given document.
    pub fn builder(doc: EureDocument) -> PlanBuilder {
        PlanBuilder::new(doc)
    }

    /// Build a plan using the default automatic policy for every node.
    pub fn auto(doc: EureDocument) -> Result<Self, PlanError> {
        Self::builder(doc).build()
    }

    /// Build a plan that emits maps as sections wherever possible.
    pub fn sectioned(doc: EureDocument) -> Result<Self, PlanError> {
        let mut b = Self::builder(doc);
        let all = traverse::all_reachable_ids(b.document());
        let root = b.document().get_root_id();
        for id in all {
            if id == root {
                continue;
            }
            let kind = b.document().node(id).content.value_kind();
            match kind {
                ValueKind::Map | ValueKind::PartialMap => {
                    b.set_form(id, Form::Section)?;
                }
                ValueKind::Array => {
                    let form = match auto::auto_array_form(b.document(), id, true) {
                        ArrayForm::PerElement(_) | ArrayForm::PerElementIndexed(_) => {
                            ArrayForm::PerElement(Form::Section)
                        }
                        ArrayForm::Inline => ArrayForm::Inline,
                    };
                    b.set_array_form(id, form)?;
                }
                _ => {
                    b.set_form(id, Form::Inline)?;
                }
            }
        }
        b.build()
    }

    /// Build a plan with no sections — maps become `BindingBlock` and arrays
    /// of maps become `PerElement(BindingBlock)`.
    pub fn flat(doc: EureDocument) -> Result<Self, PlanError> {
        let mut b = Self::builder(doc);
        let all = traverse::all_reachable_ids(b.document());
        let root = b.document().get_root_id();
        for id in all {
            if id == root {
                continue;
            }
            let kind = b.document().node(id).content.value_kind();
            match kind {
                ValueKind::Map | ValueKind::PartialMap => {
                    b.set_form(id, Form::BindingBlock)?;
                }
                ValueKind::Array => {
                    let form = match auto::auto_array_form(b.document(), id, false) {
                        ArrayForm::PerElement(_) | ArrayForm::PerElementIndexed(_) => {
                            ArrayForm::PerElement(Form::BindingBlock)
                        }
                        ArrayForm::Inline => ArrayForm::Inline,
                    };
                    b.set_array_form(id, form)?;
                }
                _ => {
                    b.set_form(id, Form::Inline)?;
                }
            }
        }
        b.build()
    }

    /// Borrow the underlying document.
    pub fn document(&self) -> &EureDocument {
        &self.doc
    }

    /// Return the assigned [`Form`] for a non-Array node.
    pub fn form_of(&self, id: NodeId) -> Option<Form> {
        self.forms.get(&id).copied()
    }

    /// Return the assigned [`ArrayForm`] for an Array node.
    pub fn array_form_of(&self, id: NodeId) -> Option<ArrayForm> {
        self.array_forms.get(&id).copied()
    }

    /// Return the ordered children override for a parent, if any.
    pub fn order_of(&self, parent: NodeId) -> Option<&[NodeId]> {
        self.order.get(&parent).map(|v| v.as_slice())
    }

    /// Emit a [`crate::source::SourceDocument`] from this plan.
    pub fn emit(self) -> crate::source::SourceDocument {
        emit::emit(self)
    }

    fn validate_structure(&self) -> Result<(), PlanError> {
        // Structural checks: section placement, coverage, uniqueness.
        emit::dry_walk_validate(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::constructor::DocumentConstructor;
    use crate::path::ArrayIndexKind;
    use crate::value::{ObjectKey, PrimitiveValue};
    use alloc::vec;

    fn scalar_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();
        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("name".into())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::from("Alice")).unwrap();
        c.end_scope(scope).unwrap();
        c.finish()
    }

    fn array_of_maps_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();
        let outer = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("items".into())))
            .unwrap();
        c.bind_empty_array().unwrap();

        for name in ["a", "b"] {
            let elem_scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
                .unwrap();
            c.bind_empty_map().unwrap();
            let inner = c.begin_scope();
            c.navigate(PathSegment::Value(ObjectKey::String("name".into())))
                .unwrap();
            c.bind_primitive(PrimitiveValue::from(name)).unwrap();
            c.end_scope(inner).unwrap();
            c.end_scope(elem_scope).unwrap();
        }
        c.end_scope(outer).unwrap();
        c.finish()
    }

    fn nested_map_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();

        let outer = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("outer".into())))
            .unwrap();
        c.bind_empty_map().unwrap();

        let inner_scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("inner".into())))
            .unwrap();
        c.bind_empty_map().unwrap();

        let leaf_scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("name".into())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::from("Ada")).unwrap();
        c.end_scope(leaf_scope).unwrap();

        c.end_scope(inner_scope).unwrap();
        c.end_scope(outer).unwrap();
        c.finish()
    }

    fn scalar_array_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();
        let outer = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("items".into())))
            .unwrap();
        c.bind_empty_array().unwrap();

        for value in [1_i64, 2] {
            let elem_scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
                .unwrap();
            c.bind_primitive(PrimitiveValue::Integer(value.into()))
                .unwrap();
            c.end_scope(elem_scope).unwrap();
        }

        c.end_scope(outer).unwrap();
        c.finish()
    }

    fn array_of_partial_maps_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();
        let outer = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("items".into())))
            .unwrap();
        c.bind_empty_array().unwrap();

        let elem_scope = c.begin_scope();
        c.navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
            .unwrap();
        c.bind_empty_partial_map().unwrap();
        let map_scope = c.begin_scope();
        c.navigate(PathSegment::HoleKey(Some("x".parse().unwrap())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::Integer(1.into())).unwrap();
        c.end_scope(map_scope).unwrap();
        c.end_scope(elem_scope).unwrap();

        c.end_scope(outer).unwrap();
        c.finish()
    }

    fn scalar_with_extension_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();

        let outer = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("name".into())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::from("Alice")).unwrap();

        let ext_scope = c.begin_scope();
        c.navigate(PathSegment::Extension("meta".parse().unwrap()))
            .unwrap();
        c.bind_empty_map().unwrap();
        let meta_scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("alpha".into())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::Integer(1.into())).unwrap();
        c.end_scope(meta_scope).unwrap();
        c.end_scope(ext_scope).unwrap();

        c.end_scope(outer).unwrap();
        c.finish()
    }

    fn root_extension_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();
        c.set_extension("meta", true).unwrap();
        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("name".into())))
            .unwrap();
        c.bind_primitive(PrimitiveValue::from("Alice")).unwrap();
        c.end_scope(scope).unwrap();
        c.finish()
    }

    #[test]
    fn auto_scalar_produces_inline_binding() {
        let doc = scalar_doc();
        let plan = LayoutPlan::auto(doc).unwrap();
        let src = plan.emit();
        let root = src.root_source();
        assert_eq!(root.bindings.len(), 1);
        assert!(matches!(
            root.bindings[0].bind,
            crate::source::BindSource::Value(_)
        ));
        assert!(root.sections.is_empty());
    }

    #[test]
    fn auto_array_of_maps_emits_array_of_sections() {
        let doc = array_of_maps_doc();
        let plan = LayoutPlan::auto(doc).unwrap();
        let src = plan.emit();
        let root = src.root_source();

        assert_eq!(root.sections.len(), 2, "expected two `@ items[]` sections");
        for section in &root.sections {
            let last = section.path.last().unwrap();
            assert_eq!(
                last.array,
                Some(ArrayIndexKind::Push),
                "expected push marker `[]` on array section"
            );
        }
    }

    #[test]
    fn flat_array_of_maps_uses_binding_blocks() {
        let doc = array_of_maps_doc();
        let plan = LayoutPlan::flat(doc).unwrap();
        let src = plan.emit();
        let root = src.root_source();

        assert_eq!(root.sections.len(), 0);
        assert_eq!(root.bindings.len(), 2);
        for b in &root.bindings {
            assert!(matches!(b.bind, crate::source::BindSource::Block(_)));
            assert_eq!(b.path.last().unwrap().array, Some(ArrayIndexKind::Push));
        }
    }

    #[test]
    fn set_form_on_array_rejected() {
        let doc = array_of_maps_doc();
        let mut b = LayoutPlan::builder(doc);
        let id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("items".into()))])
            .unwrap();
        let err = b.set_form(id, Form::Section).unwrap_err();
        assert!(matches!(err, PlanError::FormOnArrayNode(_)));
    }

    #[test]
    fn set_array_form_on_non_array_rejected() {
        let doc = scalar_doc();
        let mut b = LayoutPlan::builder(doc);
        let id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("name".into()))])
            .unwrap();
        let err = b.set_array_form(id, ArrayForm::Inline).unwrap_err();
        assert!(matches!(err, PlanError::ArrayFormOnNonArray(_)));
    }

    #[test]
    fn per_element_flatten_rejected() {
        let doc = array_of_maps_doc();
        let mut b = LayoutPlan::builder(doc);
        let id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("items".into()))])
            .unwrap();
        let err = b
            .set_array_form(id, ArrayForm::PerElement(Form::Flatten))
            .unwrap_err();
        assert!(matches!(err, PlanError::IncompatibleArrayForm { .. }));
    }

    #[test]
    fn sectioned_nested_maps_rejected_in_items_context() {
        let err = LayoutPlan::sectioned(nested_map_doc()).unwrap_err();
        assert!(matches!(err, PlanError::SectionInForbiddenContext(_)));
    }

    #[test]
    fn value_block_forms_rejected_for_maps() {
        let doc = nested_map_doc();
        let mut b = LayoutPlan::builder(doc);
        let outer_id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("outer".into()))])
            .unwrap();

        let err = b.set_form(outer_id, Form::BindingValueBlock).unwrap_err();
        assert!(matches!(err, PlanError::IncompatibleForm { .. }));

        let err = b.set_form(outer_id, Form::SectionValueBlock).unwrap_err();
        assert!(matches!(err, PlanError::IncompatibleForm { .. }));
    }

    #[test]
    fn scalar_arrays_support_section_value_block_elements() {
        let doc = scalar_array_doc();
        let mut b = LayoutPlan::builder(doc);
        let items_id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("items".into()))])
            .unwrap();
        b.set_array_form(items_id, ArrayForm::PerElement(Form::SectionValueBlock))
            .unwrap();

        let src = b.build().unwrap().emit();
        let root = src.root_source();
        assert_eq!(root.sections.len(), 2);
        for section in &root.sections {
            match &section.body {
                crate::source::SectionBody::Items { value, bindings } => {
                    assert!(value.is_some());
                    assert!(bindings.is_empty());
                }
                other => panic!("expected items body, got {other:?}"),
            }
        }
    }

    #[test]
    fn per_element_inline_rejected_for_partial_map_elements() {
        let doc = array_of_partial_maps_doc();
        let mut b = LayoutPlan::builder(doc);
        let items_id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("items".into()))])
            .unwrap();
        let err = b
            .set_array_form(items_id, ArrayForm::PerElement(Form::Inline))
            .unwrap_err();
        assert!(matches!(
            err,
            PlanError::IncompatibleArrayForm {
                reason: ArrayFormReason::ElementIncompatibleForm {
                    kind: ValueKind::PartialMap,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn inline_extensions_inherit_section_context() {
        let doc = scalar_with_extension_doc();
        let mut b = LayoutPlan::builder(doc);
        b.set_form_at(
            &[PathSegment::Value(ObjectKey::String("name".into()))],
            Form::Inline,
        )
        .unwrap();
        b.set_form_at(
            &[
                PathSegment::Value(ObjectKey::String("name".into())),
                PathSegment::Extension("meta".parse().unwrap()),
            ],
            Form::Section,
        )
        .unwrap();

        let src = b.build().unwrap().emit();
        let root = src.root_source();
        assert_eq!(root.bindings.len(), 1);
        assert_eq!(root.sections.len(), 1);
    }

    #[test]
    fn per_element_section_block_roundtrips_path_with_push_marker() {
        let doc = array_of_maps_doc();
        let mut b = LayoutPlan::builder(doc);
        let id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("items".into()))])
            .unwrap();
        b.set_array_form(id, ArrayForm::PerElement(Form::SectionBlock))
            .unwrap();
        let plan = b.build().unwrap();
        let src = plan.emit();
        let root = src.root_source();
        assert_eq!(root.sections.len(), 2);
        for section in &root.sections {
            assert!(matches!(section.body, crate::source::SectionBody::Block(_)));
            assert_eq!(
                section.path.last().unwrap().array,
                Some(ArrayIndexKind::Push)
            );
        }
    }

    #[test]
    fn path_not_found_error() {
        let doc = scalar_doc();
        let b = LayoutPlan::builder(doc);
        let err = b
            .node_at(&[PathSegment::Value(ObjectKey::String("missing".into()))])
            .unwrap_err();
        assert!(matches!(err, PlanError::PathNotFound { .. }));
    }

    #[test]
    fn order_on_array_rejected() {
        let doc = array_of_maps_doc();
        let mut b = LayoutPlan::builder(doc);
        let items_id = b
            .node_at(&[PathSegment::Value(ObjectKey::String("items".into()))])
            .unwrap();
        let err = b.order(items_id, vec![]).unwrap_err();
        assert!(matches!(err, PlanError::OrderOnArrayNode(_)));
    }

    #[test]
    fn ordering_extensions_is_rejected() {
        let doc = root_extension_doc();
        let mut b = LayoutPlan::builder(doc);
        let err = b
            .order_at(&[], vec![PathSegment::Extension("meta".parse().unwrap())])
            .unwrap_err();
        assert!(matches!(err, PlanError::OrderExtensionChild { .. }));
    }

    #[test]
    fn set_form_on_root_rejected() {
        let doc = scalar_doc();
        let root = doc.get_root_id();
        let mut b = LayoutPlan::builder(doc);
        let err = b.set_form(root, Form::Section).unwrap_err();
        assert!(matches!(err, PlanError::FormOnRoot(_)));
    }
}
