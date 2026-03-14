//! Generic layout projection for Eure documents.
//!
//! `DocLayout` is a source-agnostic, declarative plan describing how document
//! paths should be projected to source constructs.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::document::node::NodeValue;
use crate::document::{EureDocument, NodeId};
use crate::identifier::Identifier;
use crate::parse::union::VARIANT;
use crate::parse::variant_path::VariantPath;
use crate::path::PathSegment;
use crate::source::{
    BindSource, BindingSource, EureSource, SectionBody, SectionSource, SourceDocument, SourceId,
    SourceKey, SourcePath, SourcePathSegment,
};
use crate::value::ObjectKey;

/// Preferred layout style for a document path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LayoutStyle {
    /// Automatically determine the best representation.
    #[default]
    Auto,
    /// Pass through; emit children at the current level with the path prefix.
    Passthrough,
    /// Create a new section (`@ a.b.c`).
    Section,
    /// Create a nested section (`@ a.b.c { ... }`).
    Nested,
    /// Bind value (`a.b.c = value`).
    Binding,
    /// Bind a block (`a.b.c { ... }`).
    SectionBinding,
    /// Section with root value binding (`@ a.b.c = value`).
    SectionRootBinding,
}

/// Declarative layout plan for projecting a document to source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocLayout {
    /// Exact-path style rules.
    pub style_rules: Vec<LayoutStyleRule>,
    /// Ordering directives by parent path.
    pub order_rules: Vec<LayoutOrderRule>,
    /// Fallback style when no style rule matches.
    pub fallback_style: LayoutStyle,
}

/// Style directive at a path with optional union constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutStyleRule {
    pub path: Vec<PathSegment>,
    pub style: LayoutStyle,
    pub variant_constraints: Vec<VariantConstraint>,
}

/// Ordering directive for direct children of a parent path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutOrderRule {
    pub parent_path: Vec<PathSegment>,
    pub child_order: Vec<PathSegment>,
    pub append_unlisted: bool,
}

/// Constraint for style applicability under a union branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantConstraint {
    /// Path to the union node whose variant selection constrains this rule.
    pub union_path: Vec<PathSegment>,
    /// Variant name that must be selected.
    pub variant: String,
    /// Whether this branch requires an explicit variant tag.
    pub requires_explicit_tag: bool,
}

impl DocLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_style_rule(&mut self, path: Vec<PathSegment>, style: LayoutStyle) {
        self.style_rules.push(LayoutStyleRule {
            path,
            style,
            variant_constraints: Vec::new(),
        });
    }

    pub fn add_style_rule_with_constraints(
        &mut self,
        path: Vec<PathSegment>,
        style: LayoutStyle,
        variant_constraints: Vec<VariantConstraint>,
    ) {
        self.style_rules.push(LayoutStyleRule {
            path,
            style,
            variant_constraints,
        });
    }

    pub fn add_order_rule(
        &mut self,
        parent_path: Vec<PathSegment>,
        child_order: Vec<PathSegment>,
        append_unlisted: bool,
    ) {
        self.order_rules.push(LayoutOrderRule {
            parent_path,
            child_order,
            append_unlisted,
        });
    }

    fn order_rule_for_parent(&self, path: &[PathSegment]) -> Option<&LayoutOrderRule> {
        self.order_rules
            .iter()
            .rev()
            .find(|rule| rule.parent_path == path)
    }

    fn style_for_path(&self, doc: &EureDocument, path: &[PathSegment]) -> LayoutStyle {
        let mut exact_style: Option<LayoutStyle> = None;
        let mut exact_conflict = false;

        let mut potential_style: Option<LayoutStyle> = None;
        let mut potential_conflict = false;

        for rule in self.style_rules.iter().filter(|rule| rule.path == path) {
            match rule_match_status(doc, rule) {
                RuleMatchStatus::NoMatch => {}
                RuleMatchStatus::Exact(style) => {
                    if let Some(existing) = exact_style {
                        if existing != style {
                            exact_conflict = true;
                        }
                    } else {
                        exact_style = Some(style);
                    }
                }
                RuleMatchStatus::Potential(style) => {
                    if let Some(existing) = potential_style {
                        if existing != style {
                            potential_conflict = true;
                        }
                    } else {
                        potential_style = Some(style);
                    }
                }
            }
        }

        if exact_conflict {
            return LayoutStyle::Auto;
        }
        if let Some(style) = exact_style {
            return style;
        }

        if potential_conflict {
            return LayoutStyle::Auto;
        }
        if let Some(style) = potential_style {
            return style;
        }

        self.fallback_style
    }
}

/// Project a runtime document to source using a generic layout plan.
pub fn project_with_layout(doc: &EureDocument, layout: &DocLayout) -> SourceDocument {
    let projected = LayoutBuilder::new(doc, layout).build();
    projected.into_source_document(doc.clone())
}

#[derive(Debug, Clone)]
struct ProjectedDoc {
    root: ProjectedBlock,
}

#[derive(Debug, Clone)]
struct ProjectedBlock {
    value: Option<NodeId>,
    bindings: Vec<ProjectedBinding>,
    sections: Vec<ProjectedSection>,
}

#[derive(Debug, Clone)]
struct ProjectedBinding {
    path: Vec<PathSegment>,
    body: ProjectedBindingBody,
}

#[derive(Debug, Clone)]
enum ProjectedBindingBody {
    Value(NodeId),
    Block(ProjectedBlock),
}

#[derive(Debug, Clone)]
struct ProjectedSection {
    path: Vec<PathSegment>,
    body: ProjectedSectionBody,
}

#[derive(Debug, Clone)]
enum ProjectedSectionBody {
    Items {
        value: Option<NodeId>,
        bindings: Vec<ProjectedBinding>,
    },
    Block(ProjectedBlock),
}

impl ProjectedDoc {
    fn into_source_document(self, doc: EureDocument) -> SourceDocument {
        let mut sources = Vec::new();
        let root_id = build_source_block(&self.root, &mut sources);
        debug_assert_eq!(root_id.0, 0, "root source must be index 0");
        SourceDocument::new(doc, sources)
    }
}

struct LayoutBuilder<'a> {
    doc: &'a EureDocument,
    layout: &'a DocLayout,
}

struct EntryContext<'a> {
    bindings: &'a mut Vec<ProjectedBinding>,
    sections: &'a mut Vec<ProjectedSection>,
    node_path: &'a [PathSegment],
    path_prefix: &'a [PathSegment],
    allow_sections: bool,
}

#[derive(Debug, Clone)]
struct ChildEntry {
    segment: PathSegment,
    node_id: NodeId,
}

impl<'a> LayoutBuilder<'a> {
    fn new(doc: &'a EureDocument, layout: &'a DocLayout) -> Self {
        Self { doc, layout }
    }

    fn build(&self) -> ProjectedDoc {
        let root_id = self.doc.get_root_id();
        let root_node = self.doc.node(root_id);
        let root_is_map = matches!(root_node.content, NodeValue::Map(_));
        let value = if root_is_map { None } else { Some(root_id) };

        let (bindings, sections) = self.build_entries(root_id, &[], &[], root_is_map, true);

        ProjectedDoc {
            root: ProjectedBlock {
                value,
                bindings,
                sections,
            },
        }
    }

    fn build_block(&self, node_id: NodeId, node_path: &[PathSegment]) -> ProjectedBlock {
        let node_is_map = matches!(self.doc.node(node_id).content, NodeValue::Map(_));
        let (bindings, sections) = self.build_entries(node_id, node_path, &[], node_is_map, true);
        ProjectedBlock {
            value: None,
            bindings,
            sections,
        }
    }

    fn build_entries(
        &self,
        node_id: NodeId,
        node_path: &[PathSegment],
        path_prefix: &[PathSegment],
        emit_map_fields: bool,
        allow_sections: bool,
    ) -> (Vec<ProjectedBinding>, Vec<ProjectedSection>) {
        let mut bindings = Vec::new();
        let mut sections = Vec::new();
        let mut ctx = EntryContext {
            bindings: &mut bindings,
            sections: &mut sections,
            node_path,
            path_prefix,
            allow_sections,
        };

        let node = self.doc.node(node_id);
        let mut children = Vec::new();

        for (ident, &child_id) in node.extensions.iter() {
            children.push(ChildEntry {
                segment: PathSegment::Extension(ident.clone()),
                node_id: child_id,
            });
        }

        if emit_map_fields && let NodeValue::Map(map) = &node.content {
            for (key, &child_id) in map.iter() {
                children.push(ChildEntry {
                    segment: PathSegment::Value(key.clone()),
                    node_id: child_id,
                });
            }
        }

        let children = self.order_children(node_path, children);
        for child in children {
            self.append_child_entries(&mut ctx, child.segment, child.node_id);
        }

        (bindings, sections)
    }

    fn order_children(
        &self,
        parent_path: &[PathSegment],
        children: Vec<ChildEntry>,
    ) -> Vec<ChildEntry> {
        let Some(rule) = self.layout.order_rule_for_parent(parent_path) else {
            return children;
        };

        if !rule.append_unlisted {
            let mut out = children;
            let listed: Vec<(usize, ChildEntry)> = out
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    rule.child_order
                        .iter()
                        .any(|segment| segment == &entry.segment)
                })
                .map(|(idx, entry)| (idx, entry.clone()))
                .collect();

            let mut sorted = listed
                .iter()
                .map(|(_, entry)| entry.clone())
                .collect::<Vec<_>>();
            sorted.sort_by_key(|entry| {
                rule.child_order
                    .iter()
                    .position(|segment| segment == &entry.segment)
                    .unwrap_or(usize::MAX)
            });

            for ((idx, _), replacement) in listed.into_iter().zip(sorted.into_iter()) {
                out[idx] = replacement;
            }

            return out;
        }

        let mut remaining = children;
        let mut ordered = Vec::new();

        for segment in &rule.child_order {
            if let Some(pos) = remaining.iter().position(|entry| &entry.segment == segment) {
                ordered.push(remaining.remove(pos));
            }
        }

        ordered.extend(remaining);

        ordered
    }

    fn append_child_entries(
        &self,
        ctx: &mut EntryContext<'_>,
        child_seg: PathSegment,
        child_id: NodeId,
    ) {
        let child_node_path = concat_path(ctx.node_path, &child_seg);
        let child_print_path = concat_path(ctx.path_prefix, &child_seg);

        let style = self.layout.style_for_path(self.doc, &child_node_path);

        if style == LayoutStyle::Passthrough {
            let child_is_map = matches!(self.doc.node(child_id).content, NodeValue::Map(_));
            let (b, s) = self.build_entries(
                child_id,
                &child_node_path,
                &child_print_path,
                child_is_map,
                ctx.allow_sections,
            );
            ctx.bindings.extend(b);
            ctx.sections.extend(s);
            return;
        }

        let child_is_map = matches!(self.doc.node(child_id).content, NodeValue::Map(_));
        let mut style = self.normalize_style(style, child_is_map, ctx.allow_sections);

        if style == LayoutStyle::Binding
            && child_is_map
            && self.inline_binding_hides_descendant_entries(child_id, &child_node_path)
        {
            style = LayoutStyle::SectionBinding;
        }

        if style == LayoutStyle::Section && self.has_section_entries(child_id, &child_node_path) {
            style = LayoutStyle::Nested;
        }

        match style {
            LayoutStyle::Binding | LayoutStyle::Auto | LayoutStyle::Passthrough => {
                ctx.bindings.push(ProjectedBinding {
                    path: child_print_path.clone(),
                    body: ProjectedBindingBody::Value(child_id),
                });

                let (b, s) = self.build_entries(
                    child_id,
                    &child_node_path,
                    &child_print_path,
                    false,
                    ctx.allow_sections,
                );
                ctx.bindings.extend(b);
                ctx.sections.extend(s);
            }
            LayoutStyle::SectionBinding => {
                if child_is_map {
                    let block = self.build_block(child_id, &child_node_path);
                    ctx.bindings.push(ProjectedBinding {
                        path: child_print_path,
                        body: ProjectedBindingBody::Block(block),
                    });
                } else {
                    ctx.bindings.push(ProjectedBinding {
                        path: child_print_path.clone(),
                        body: ProjectedBindingBody::Value(child_id),
                    });
                    let (b, s) = self.build_entries(
                        child_id,
                        &child_node_path,
                        &child_print_path,
                        false,
                        ctx.allow_sections,
                    );
                    ctx.bindings.extend(b);
                    ctx.sections.extend(s);
                }
            }
            LayoutStyle::Section => {
                let (child_bindings, child_sections) =
                    self.build_entries(child_id, &child_node_path, &[], child_is_map, false);
                debug_assert!(child_sections.is_empty());
                ctx.sections.push(ProjectedSection {
                    path: child_print_path,
                    body: ProjectedSectionBody::Items {
                        value: None,
                        bindings: child_bindings,
                    },
                });
            }
            LayoutStyle::SectionRootBinding => {
                if child_is_map {
                    let (child_bindings, child_sections) =
                        self.build_entries(child_id, &child_node_path, &[], true, false);
                    debug_assert!(child_sections.is_empty());
                    ctx.sections.push(ProjectedSection {
                        path: child_print_path,
                        body: ProjectedSectionBody::Items {
                            value: None,
                            bindings: child_bindings,
                        },
                    });
                } else {
                    let (child_bindings, child_sections) =
                        self.build_entries(child_id, &child_node_path, &[], false, false);
                    debug_assert!(child_sections.is_empty());
                    ctx.sections.push(ProjectedSection {
                        path: child_print_path,
                        body: ProjectedSectionBody::Items {
                            value: Some(child_id),
                            bindings: child_bindings,
                        },
                    });
                }
            }
            LayoutStyle::Nested => {
                if child_is_map {
                    let block = self.build_block(child_id, &child_node_path);
                    ctx.sections.push(ProjectedSection {
                        path: child_print_path,
                        body: ProjectedSectionBody::Block(block),
                    });
                } else {
                    ctx.bindings.push(ProjectedBinding {
                        path: child_print_path.clone(),
                        body: ProjectedBindingBody::Value(child_id),
                    });
                    let (b, s) = self.build_entries(
                        child_id,
                        &child_node_path,
                        &child_print_path,
                        false,
                        ctx.allow_sections,
                    );
                    ctx.bindings.extend(b);
                    ctx.sections.extend(s);
                }
            }
        }
    }

    fn normalize_style(
        &self,
        style: LayoutStyle,
        node_is_map: bool,
        allow_sections: bool,
    ) -> LayoutStyle {
        let mut style = match style {
            LayoutStyle::Auto | LayoutStyle::Passthrough => LayoutStyle::Binding,
            other => other,
        };

        if !node_is_map {
            style = match style {
                LayoutStyle::SectionRootBinding => LayoutStyle::SectionRootBinding,
                _ => LayoutStyle::Binding,
            };
        } else if style == LayoutStyle::SectionRootBinding {
            style = LayoutStyle::Section;
        }

        if !allow_sections
            && matches!(
                style,
                LayoutStyle::Section | LayoutStyle::Nested | LayoutStyle::SectionRootBinding
            )
        {
            style = if node_is_map {
                LayoutStyle::SectionBinding
            } else {
                LayoutStyle::Binding
            };
        }

        style
    }

    fn inline_binding_hides_descendant_entries(
        &self,
        node_id: NodeId,
        node_path: &[PathSegment],
    ) -> bool {
        let NodeValue::Map(map) = &self.doc.node(node_id).content else {
            return false;
        };

        for (key, &child_id) in map.iter() {
            let child_node_path = concat_path(node_path, &PathSegment::Value(key.clone()));
            if self.subtree_has_deferred_entries(child_id, &child_node_path) {
                return true;
            }
        }

        false
    }

    fn subtree_has_deferred_entries(&self, node_id: NodeId, node_path: &[PathSegment]) -> bool {
        let node = self.doc.node(node_id);
        if !node.extensions.is_empty() {
            return true;
        }

        if self.has_section_entries(node_id, node_path) {
            return true;
        }

        let NodeValue::Map(map) = &node.content else {
            return false;
        };

        for (key, &child_id) in map.iter() {
            let child_node_path = concat_path(node_path, &PathSegment::Value(key.clone()));
            if self.subtree_has_deferred_entries(child_id, &child_node_path) {
                return true;
            }
        }

        false
    }

    fn has_section_entries(&self, node_id: NodeId, node_path: &[PathSegment]) -> bool {
        let node = self.doc.node(node_id);
        let node_is_map = matches!(node.content, NodeValue::Map(_));

        for (ident, &child_id) in node.extensions.iter() {
            let seg = PathSegment::Extension(ident.clone());
            let child_node_path = concat_path(node_path, &seg);
            if self.child_is_section(child_id, &child_node_path) {
                return true;
            }
        }

        if node_is_map && let NodeValue::Map(map) = &node.content {
            for (key, &child_id) in map.iter() {
                let seg = PathSegment::Value(key.clone());
                let child_node_path = concat_path(node_path, &seg);
                if self.child_is_section(child_id, &child_node_path) {
                    return true;
                }
            }
        }

        false
    }

    fn child_is_section(&self, child_id: NodeId, child_node_path: &[PathSegment]) -> bool {
        let style = self.layout.style_for_path(self.doc, child_node_path);
        if style == LayoutStyle::Passthrough {
            return self.has_section_entries(child_id, child_node_path);
        }

        let child_is_map = matches!(self.doc.node(child_id).content, NodeValue::Map(_));
        let style = self.normalize_style(style, child_is_map, true);
        matches!(
            style,
            LayoutStyle::Section | LayoutStyle::Nested | LayoutStyle::SectionRootBinding
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleMatchStatus {
    NoMatch,
    Exact(LayoutStyle),
    Potential(LayoutStyle),
}

fn rule_match_status(doc: &EureDocument, rule: &LayoutStyleRule) -> RuleMatchStatus {
    if rule.variant_constraints.is_empty() {
        return RuleMatchStatus::Exact(rule.style);
    }

    let mut inherited_variant: Option<VariantPath> = None;
    let mut uncertain = false;

    for constraint in &rule.variant_constraints {
        let Some(union_node_id) = node_id_at_path(doc, &constraint.union_path) else {
            return RuleMatchStatus::NoMatch;
        };

        let explicit_variant = if let Some(vp) = inherited_variant.as_ref()
            && !vp.is_empty()
        {
            Some(vp.clone())
        } else {
            match parse_variant_extension(doc, union_node_id) {
                Ok(path) => path,
                Err(()) => return RuleMatchStatus::NoMatch,
            }
        };

        match explicit_variant {
            Some(vp) if !vp.is_empty() => {
                let Some(first) = vp.first() else {
                    return RuleMatchStatus::NoMatch;
                };
                if first.as_ref() != constraint.variant {
                    return RuleMatchStatus::NoMatch;
                }
                inherited_variant = vp.rest();
            }
            _ => {
                inherited_variant = None;
                if constraint.requires_explicit_tag {
                    return RuleMatchStatus::NoMatch;
                }
                uncertain = true;
            }
        }
    }

    if uncertain {
        RuleMatchStatus::Potential(rule.style)
    } else {
        RuleMatchStatus::Exact(rule.style)
    }
}

fn parse_variant_extension(doc: &EureDocument, node_id: NodeId) -> Result<Option<VariantPath>, ()> {
    let node = doc.node(node_id);
    let Some(&variant_node_id) = node.extensions.get(&VARIANT) else {
        return Ok(None);
    };

    let Ok(variant_text) = doc.parse::<&str>(variant_node_id) else {
        return Err(());
    };

    VariantPath::parse(variant_text).map(Some).map_err(|_| ())
}

fn node_id_at_path(doc: &EureDocument, path: &[PathSegment]) -> Option<NodeId> {
    let mut current = doc.get_root_id();
    for segment in path {
        current = child_node_id(doc, current, segment)?;
    }
    Some(current)
}

fn build_source_block(block: &ProjectedBlock, sources: &mut Vec<EureSource>) -> SourceId {
    let id = SourceId(sources.len());
    sources.push(EureSource::default());
    let mut eure = EureSource {
        value: block.value,
        ..Default::default()
    };

    for binding in &block.bindings {
        let path = to_source_path(&binding.path);
        let bind = match &binding.body {
            ProjectedBindingBody::Value(node_id) => BindSource::Value(*node_id),
            ProjectedBindingBody::Block(inner) => {
                let inner_id = build_source_block(inner, sources);
                BindSource::Block(inner_id)
            }
        };
        eure.bindings.push(BindingSource {
            trivia_before: Vec::new(),
            path,
            bind,
            trailing_comment: None,
        });
    }

    for section in &block.sections {
        let path = to_source_path(&section.path);
        let body = match &section.body {
            ProjectedSectionBody::Items { value, bindings } => {
                let mut items = Vec::new();
                for binding in bindings {
                    let path = to_source_path(&binding.path);
                    let bind = match &binding.body {
                        ProjectedBindingBody::Value(node_id) => BindSource::Value(*node_id),
                        ProjectedBindingBody::Block(inner) => {
                            let inner_id = build_source_block(inner, sources);
                            BindSource::Block(inner_id)
                        }
                    };
                    items.push(BindingSource {
                        trivia_before: Vec::new(),
                        path,
                        bind,
                        trailing_comment: None,
                    });
                }
                SectionBody::Items {
                    value: *value,
                    bindings: items,
                }
            }
            ProjectedSectionBody::Block(inner) => {
                let inner_id = build_source_block(inner, sources);
                SectionBody::Block(inner_id)
            }
        };
        eure.sections.push(SectionSource {
            trivia_before: Vec::new(),
            path,
            body,
            trailing_comment: None,
        });
    }

    sources[id.0] = eure;
    id
}

fn to_source_path(path: &[PathSegment]) -> SourcePath {
    let mut out: Vec<SourcePathSegment> = Vec::new();
    for seg in path {
        match seg {
            PathSegment::Ident(id) => out.push(SourcePathSegment::ident(id.clone())),
            PathSegment::Extension(id) => out.push(SourcePathSegment::extension(id.clone())),
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

fn concat_path(prefix: &[PathSegment], seg: &PathSegment) -> Vec<PathSegment> {
    let mut out = Vec::with_capacity(prefix.len() + 1);
    out.extend_from_slice(prefix);
    out.push(seg.clone());
    out
}

fn child_node_id(doc: &EureDocument, parent_id: NodeId, segment: &PathSegment) -> Option<NodeId> {
    let parent = doc.node(parent_id);
    match segment {
        PathSegment::Extension(ext) => parent.extensions.get(ext).copied(),
        PathSegment::Ident(ident) => match &parent.content {
            NodeValue::Map(map) => map.get(&ObjectKey::String(ident.to_string())).copied(),
            _ => None,
        },
        PathSegment::Value(key) => match &parent.content {
            NodeValue::Map(map) => map.get(key).copied(),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::constructor::DocumentConstructor;
    use crate::value::ObjectKey;
    use alloc::vec;

    fn make_doc() -> EureDocument {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();

        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("a".to_string())))
            .unwrap();
        c.bind_empty_map().unwrap();
        let inner = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("x".to_string())))
            .unwrap();
        c.bind_primitive(crate::value::PrimitiveValue::Integer(1.into()))
            .unwrap();
        c.end_scope(inner).unwrap();
        c.end_scope(scope).unwrap();

        let scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("b".to_string())))
            .unwrap();
        c.bind_empty_map().unwrap();
        c.end_scope(scope).unwrap();

        c.finish()
    }

    #[test]
    fn applies_section_binding_rule() {
        let doc = make_doc();
        let mut layout = DocLayout::new();
        layout.add_style_rule(
            vec![PathSegment::Value(ObjectKey::String("a".to_string()))],
            LayoutStyle::SectionBinding,
        );

        let source = project_with_layout(&doc, &layout);
        let root = source.root_source();

        assert_eq!(root.bindings.len(), 2);
        assert!(matches!(root.bindings[0].bind, BindSource::Block(_)));
    }

    #[test]
    fn applies_order_rule() {
        let doc = make_doc();
        let mut layout = DocLayout::new();
        layout.add_order_rule(
            Vec::new(),
            vec![PathSegment::Value(ObjectKey::String("b".to_string()))],
            true,
        );

        let source = project_with_layout(&doc, &layout);
        let root = source.root_source();

        let first = &root.bindings[0].path[0];
        assert!(matches!(
            first,
            SourcePathSegment {
                key: SourceKey::Ident(id),
                ..
            } if id.as_ref() == "b"
        ));
    }

    #[test]
    fn passthrough_flattens_parent_binding() {
        let doc = make_doc();
        let mut layout = DocLayout::new();
        layout.add_style_rule(
            vec![PathSegment::Value(ObjectKey::String("a".to_string()))],
            LayoutStyle::Passthrough,
        );

        let source = project_with_layout(&doc, &layout);
        let root = source.root_source();

        assert!(!root.bindings.iter().any(|b| {
            matches!(
                b.path.as_slice(),
                [SourcePathSegment {
                    key: SourceKey::Ident(id),
                    ..
                }] if id.as_ref() == "a"
            )
        }));
        assert!(root.bindings.iter().any(|b| {
            matches!(
                b.path.as_slice(),
                [
                    SourcePathSegment {
                        key: SourceKey::Ident(first),
                        ..
                    },
                    SourcePathSegment {
                        key: SourceKey::Ident(second),
                        ..
                    }
                ] if first.as_ref() == "a" && second.as_ref() == "x"
            )
        }));
    }

    #[test]
    fn auto_promotes_inline_map_when_nested_extensions_would_be_lost() {
        let mut c = DocumentConstructor::new();
        c.bind_empty_map().unwrap();

        let outer_scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("outer".to_string())))
            .unwrap();
        c.bind_empty_map().unwrap();

        let inner_scope = c.begin_scope();
        c.navigate(PathSegment::Value(ObjectKey::String("inner".to_string())))
            .unwrap();
        c.bind_primitive(crate::value::PrimitiveValue::Integer(1.into()))
            .unwrap();
        c.set_extension("flag", true).unwrap();
        c.end_scope(inner_scope).unwrap();
        c.end_scope(outer_scope).unwrap();

        let doc = c.finish();
        let source = project_with_layout(&doc, &DocLayout::new());
        let root = source.root_source();

        assert_eq!(root.bindings.len(), 1);
        let BindSource::Block(outer_block_id) = &root.bindings[0].bind else {
            panic!("expected outer map to be promoted to a block binding");
        };
        let outer_block = source.source(*outer_block_id);
        assert_eq!(outer_block.bindings.len(), 2);
    }
}
