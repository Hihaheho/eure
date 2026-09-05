//! Path-based navigation over a [`SchemaDocument`].
//!
//! Editor tooling (completion, hover) needs to answer "which schema node
//! describes the document node at this path?" without having a complete
//! [`EureDocument`](eure_document::document::EureDocument) at hand: while the
//! user is typing, the document usually does not parse.
//!
//! [`SchemaNavigator`] walks a list of [`PathSegment`]s against the schema
//! only. References are dereferenced transparently. Unions are resolved with
//! an explicit variant when the caller knows one (from a sibling `$variant`
//! binding); otherwise every variant becomes a candidate, so a position inside
//! an untagged union yields several candidate nodes instead of none.

use std::collections::HashSet;

use eure_document::identifier::Identifier;
use eure_document::parse::variant_path::VariantPath;
use eure_document::path::PathSegment;
use eure_document::value::ObjectKey;
use indexmap::IndexMap;

use crate::{
    RecordFieldSchema, SchemaDocument, SchemaNodeContent, SchemaNodeId, UnknownFieldsPolicy,
};

/// Explicit variant selection for a union located at a path prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantHint {
    /// Number of path segments consumed when the union is reached.
    pub prefix_len: usize,
    /// Variant path (possibly nested, e.g. `ok.some.left`).
    pub variant: VariantPath,
}

/// Navigates a [`SchemaDocument`] along document paths.
#[derive(Debug, Clone, Copy)]
pub struct SchemaNavigator<'a> {
    schema: &'a SchemaDocument,
}

impl<'a> SchemaNavigator<'a> {
    pub fn new(schema: &'a SchemaDocument) -> Self {
        Self { schema }
    }

    pub fn schema(&self) -> &'a SchemaDocument {
        self.schema
    }

    /// Resolve `path` starting from the schema root.
    ///
    /// Returns every concrete (non-reference, non-union) schema node that may
    /// describe the document node at `path`. The list is empty when the path
    /// does not exist in the schema.
    pub fn resolve(&self, path: &[PathSegment], hints: &[VariantHint]) -> Vec<SchemaNodeId> {
        self.resolve_from(self.schema.root, path, hints)
    }

    /// Resolve `path` relative to `start`.
    pub fn resolve_from(
        &self,
        start: SchemaNodeId,
        path: &[PathSegment],
        hints: &[VariantHint],
    ) -> Vec<SchemaNodeId> {
        let mut candidates = self.concretize(start, hint_at(hints, 0));
        for (index, segment) in path.iter().enumerate() {
            let hint = hint_at(hints, index + 1);
            let mut next = Vec::new();
            for candidate in candidates {
                if let Some(stepped) = self.step(candidate, segment) {
                    for concrete in self.concretize(stepped, hint) {
                        if !next.contains(&concrete) {
                            next.push(concrete);
                        }
                    }
                }
            }
            if next.is_empty() {
                return next;
            }
            candidates = next;
        }
        candidates
    }

    /// Resolve `path` but stop at unions instead of expanding them.
    ///
    /// Used to enumerate variant names: the returned nodes are references-free
    /// but may be `Union` nodes.
    pub fn resolve_to_union(
        &self,
        path: &[PathSegment],
        hints: &[VariantHint],
    ) -> Vec<SchemaNodeId> {
        let Some((last, prefix)) = path.split_last() else {
            return self
                .deref_references(self.schema.root)
                .into_iter()
                .collect();
        };
        let parents = self.resolve(prefix, hints);
        let mut result = Vec::new();
        for parent in parents {
            if let Some(stepped) = self.step(parent, last)
                && let Some(node) = self.deref_references(stepped)
                && !result.contains(&node)
            {
                result.push(node);
            }
        }
        result
    }

    /// Descend from `id` through the variants named by `path`.
    ///
    /// Returns the reference-free nodes reached, which are typically nested
    /// unions. Used to complete the tail of a dotted variant path.
    pub fn descend_variants(&self, id: SchemaNodeId, path: &VariantPath) -> Vec<SchemaNodeId> {
        let mut current = self.deref_references(id).into_iter().collect::<Vec<_>>();
        for name in path.segments() {
            let mut next = Vec::new();
            for node in current {
                if let SchemaNodeContent::Union(union) = &self.schema.node(node).content
                    && let Some(&variant) = union.variants.get(name.as_ref())
                    && let Some(variant) = self.deref_references(variant)
                    && !next.contains(&variant)
                {
                    next.push(variant);
                }
            }
            current = next;
        }
        current
    }

    /// Follow references until a non-reference node is reached.
    ///
    /// Returns `None` for undefined references and reference cycles.
    pub fn deref_references(&self, mut id: SchemaNodeId) -> Option<SchemaNodeId> {
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(id) {
                return None;
            }
            match &self.schema.node(id).content {
                SchemaNodeContent::Reference(reference) => {
                    id = self.schema.resolve_reference(reference)?;
                }
                _ => return Some(id),
            }
        }
    }

    /// Expand `id` into concrete nodes: references are dereferenced and unions
    /// are replaced by their variants (or the hinted variant only).
    pub fn concretize(&self, id: SchemaNodeId, hint: Option<&VariantPath>) -> Vec<SchemaNodeId> {
        let mut out = Vec::new();
        let mut visited = HashSet::new();
        self.concretize_into(id, hint, &mut out, &mut visited);
        out
    }

    fn concretize_into(
        &self,
        id: SchemaNodeId,
        hint: Option<&VariantPath>,
        out: &mut Vec<SchemaNodeId>,
        visited: &mut HashSet<SchemaNodeId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        match &self.schema.node(id).content {
            SchemaNodeContent::Reference(reference) => {
                if let Some(target) = self.schema.resolve_reference(reference) {
                    self.concretize_into(target, hint, out, visited);
                }
            }
            SchemaNodeContent::Union(union) => match hint {
                Some(variant_path) => {
                    let Some(first) = variant_path.first() else {
                        return;
                    };
                    if let Some(&variant_id) = union.variants.get(first.as_ref()) {
                        let rest = variant_path.rest();
                        self.concretize_into(variant_id, rest.as_ref(), out, visited);
                    }
                }
                None => {
                    for &variant_id in union.variants.values() {
                        self.concretize_into(variant_id, None, out, visited);
                    }
                }
            },
            _ => {
                if !out.contains(&id) {
                    out.push(id);
                }
            }
        }
    }

    /// Step from a concrete node to the child described by `segment`.
    ///
    /// The result may be a reference or union; callers usually pass it through
    /// [`Self::concretize`].
    pub fn step(&self, from: SchemaNodeId, segment: &PathSegment) -> Option<SchemaNodeId> {
        let node = self.schema.node(from);
        match segment {
            PathSegment::Ident(ident) => self.step_string_key(from, ident.as_ref()),
            PathSegment::Value(ObjectKey::String(name)) => self.step_string_key(from, name),
            PathSegment::Value(_) | PathSegment::PartialValue(_) => match &node.content {
                SchemaNodeContent::Map(map) => Some(map.value),
                _ => None,
            },
            PathSegment::ArrayIndex(_) => match &node.content {
                SchemaNodeContent::Array(array) => Some(array.item),
                _ => None,
            },
            PathSegment::TupleIndex(index) => match &node.content {
                SchemaNodeContent::Tuple(tuple) => tuple.elements.get(*index as usize).copied(),
                _ => None,
            },
            PathSegment::Extension(name) => node.ext_types.get(name).map(|ext| ext.schema),
            PathSegment::HoleKey(_) => None,
        }
    }

    fn step_string_key(&self, from: SchemaNodeId, name: &str) -> Option<SchemaNodeId> {
        match &self.schema.node(from).content {
            SchemaNodeContent::Record(record) => {
                if let Some(field) = self.record_fields(from).get(name) {
                    return Some(field.schema);
                }
                match &record.unknown_fields {
                    UnknownFieldsPolicy::Schema(id) => Some(*id),
                    UnknownFieldsPolicy::Deny | UnknownFieldsPolicy::Allow => None,
                }
            }
            SchemaNodeContent::Map(map) => Some(map.value),
            _ => None,
        }
    }

    /// All fields of a record, including fields contributed by `flatten`
    /// targets, in declaration order. Own fields take precedence over
    /// flattened ones with the same name.
    pub fn record_fields(&self, id: SchemaNodeId) -> IndexMap<String, &'a RecordFieldSchema> {
        let mut fields = IndexMap::new();
        let mut visited = HashSet::new();
        self.collect_record_fields(id, &mut fields, &mut visited);
        fields
    }

    fn collect_record_fields(
        &self,
        id: SchemaNodeId,
        fields: &mut IndexMap<String, &'a RecordFieldSchema>,
        visited: &mut HashSet<SchemaNodeId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        let SchemaNodeContent::Record(record) = &self.schema.node(id).content else {
            return;
        };
        for (name, field) in &record.properties {
            fields.entry(name.clone()).or_insert(field);
        }
        for &flatten_id in &record.flatten {
            for concrete in self.concretize(flatten_id, None) {
                self.collect_record_fields(concrete, fields, visited);
            }
        }
    }

    /// Extension names accepted on the node, as declared by `$ext-type`.
    pub fn extension_names(&self, id: SchemaNodeId) -> impl Iterator<Item = &'a Identifier> {
        self.schema.node(id).ext_types.keys()
    }
}

fn hint_at(hints: &[VariantHint], prefix_len: usize) -> Option<&VariantPath> {
    hints
        .iter()
        .find(|hint| hint.prefix_len == prefix_len)
        .map(|hint| &hint.variant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArraySchema, RecordSchema, TextSchema, TypeReference, UnionSchema};
    use eure_document::path::ArrayIndexKind;
    use indexmap::IndexSet;

    fn ident(s: &str) -> PathSegment {
        PathSegment::Ident(s.parse().unwrap())
    }

    fn field(schema: SchemaNodeId) -> RecordFieldSchema {
        RecordFieldSchema {
            schema,
            optional: false,
            binding_style: None,
            field_codegen: Default::default(),
        }
    }

    /// root = { items: [ $types.item ] }, item = union { a: { x: text }, b: { y: text } }
    fn fixture() -> SchemaDocument {
        let mut schema = SchemaDocument::new();
        let text = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let mut a = RecordSchema::default();
        a.properties.insert("x".into(), field(text));
        let a = schema.create_node(SchemaNodeContent::Record(a));
        let mut b = RecordSchema::default();
        b.properties.insert("y".into(), field(text));
        let b = schema.create_node(SchemaNodeContent::Record(b));
        let mut variants = IndexMap::new();
        variants.insert("a".to_string(), a);
        variants.insert("b".to_string(), b);
        let union = schema.create_node(SchemaNodeContent::Union(UnionSchema {
            variants,
            unambiguous: IndexSet::new(),
            interop: Default::default(),
            deny_untagged: IndexSet::new(),
        }));
        schema.register_type("item".parse().unwrap(), union);
        let reference = schema.create_node(SchemaNodeContent::Reference(TypeReference::Named {
            namespace: None,
            name: "item".parse().unwrap(),
        }));
        let array = schema.create_node(SchemaNodeContent::Array(ArraySchema {
            item: reference,
            min_length: None,
            max_length: None,
            unique: false,
            contains: None,
            binding_style: None,
        }));
        let mut root = RecordSchema::default();
        root.properties.insert("items".into(), field(array));
        schema.node_mut(schema.root).content = SchemaNodeContent::Record(root);
        schema
    }

    #[test]
    fn untagged_union_expands_to_all_variants() {
        let schema = fixture();
        let nav = SchemaNavigator::new(&schema);
        let path = [
            ident("items"),
            PathSegment::ArrayIndex(ArrayIndexKind::Push),
        ];
        let resolved = nav.resolve(&path, &[]);
        assert_eq!(resolved.len(), 2);
        let names: Vec<_> = resolved
            .iter()
            .flat_map(|id| nav.record_fields(*id).into_keys())
            .collect();
        assert_eq!(names, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn variant_hint_selects_single_variant() {
        let schema = fixture();
        let nav = SchemaNavigator::new(&schema);
        let path = [
            ident("items"),
            PathSegment::ArrayIndex(ArrayIndexKind::Push),
        ];
        let hints = [VariantHint {
            prefix_len: 2,
            variant: VariantPath::parse("b").unwrap(),
        }];
        let resolved = nav.resolve(&path, &hints);
        assert_eq!(resolved.len(), 1);
        let names: Vec<_> = nav.record_fields(resolved[0]).into_keys().collect();
        assert_eq!(names, vec!["y".to_string()]);
    }

    #[test]
    fn resolve_to_union_keeps_union_node() {
        let schema = fixture();
        let nav = SchemaNavigator::new(&schema);
        let path = [
            ident("items"),
            PathSegment::ArrayIndex(ArrayIndexKind::Push),
        ];
        let unions = nav.resolve_to_union(&path, &[]);
        assert_eq!(unions.len(), 1);
        assert!(matches!(
            schema.node(unions[0]).content,
            SchemaNodeContent::Union(_)
        ));
    }

    #[test]
    fn unknown_path_yields_nothing() {
        let schema = fixture();
        let nav = SchemaNavigator::new(&schema);
        assert!(nav.resolve(&[ident("missing")], &[]).is_empty());
    }
}
