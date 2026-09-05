//! Turn a [`CompletionSite`] plus a schema into completion items.

use eure_document::document::EureDocument;
use eure_document::document::node::NodeValue;
use eure_document::identifier::Identifier;
use eure_document::parse::variant_path::VariantPath;
use eure_document::path::PathSegment;
use eure_document::value::PrimitiveValue;
use eure_schema::navigate::{SchemaNavigator, VariantHint};
use eure_schema::{Description, SchemaDocument, SchemaNodeContent, SchemaNodeId};
use eure_tree::tree::InputSpan;
use indexmap::IndexMap;

use super::site::{CompletionSite, SiteKind, ValueStyle};

/// What a completion item stands for. Editor front-ends map this to their own
/// icon/kind vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    /// A record field name.
    Field,
    /// An extension name (`$variant`, `$ext-type`-declared extensions).
    Extension,
    /// A union variant name.
    Variant,
    /// A literal value (`true`, `"development"`, ...).
    Value,
}

impl CompletionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Field => "field",
            Self::Extension => "extension",
            Self::Variant => "variant",
            Self::Value => "value",
        }
    }
}

/// Editor-agnostic completion item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// Text shown and inserted.
    pub label: String,
    pub kind: CompletionKind,
    /// Short type summary (`text`, `[integer]`, `$types.user (optional)`).
    pub detail: Option<String>,
    /// Longer documentation from the schema `$description`, as markdown.
    pub documentation: Option<String>,
    pub deprecated: bool,
    /// Span in the source that inserting the item replaces.
    pub replace: InputSpan,
}

/// Build completion items for `site`.
///
/// `schema` is `None` when the document has no schema; only syntactic value
/// keywords are offered in that case.
pub fn completion_items(
    site: &CompletionSite,
    schema: Option<&SchemaDocument>,
) -> Vec<CompletionItem> {
    let mut items: IndexMap<String, CompletionItem> = IndexMap::new();
    let mut push = |item: CompletionItem| {
        if item.label.starts_with(site.partial.as_str()) {
            items.entry(item.label.clone()).or_insert(item);
        }
    };

    match &site.kind {
        SiteKind::Key { parent, used } => {
            let Some(schema) = schema else {
                return Vec::new();
            };
            let nav = SchemaNavigator::new(schema);
            for candidate in nav.resolve(&parent.0, &site.hints) {
                key_items(&nav, candidate, used, site.replace, &mut push);
            }
            // Untagged union at this position: `$variant` selects a variant.
            if nav
                .resolve_to_union(&parent.0, &site.hints)
                .into_iter()
                .any(|id| matches!(schema.node(id).content, SchemaNodeContent::Union(_)))
                && !site.hints.iter().any(|h| h.prefix_len == parent.0.len())
            {
                push(CompletionItem {
                    label: "$variant".to_string(),
                    kind: CompletionKind::Extension,
                    detail: Some("union variant".to_string()),
                    documentation: None,
                    deprecated: false,
                    replace: site.replace,
                });
            }
        }
        SiteKind::Value { path, style } => match path.0.split_last() {
            Some((PathSegment::Extension(ext), parent)) if ext.as_ref() == "variant" => {
                if let Some(schema) = schema {
                    variant_items(
                        &SchemaNavigator::new(schema),
                        parent,
                        &site.hints,
                        &site.partial,
                        site.replace,
                        &mut push,
                    );
                }
            }
            _ => match schema {
                Some(schema) => {
                    let nav = SchemaNavigator::new(schema);
                    for candidate in nav.resolve(&path.0, &site.hints) {
                        value_items(&nav, candidate, *style, site.replace, &mut push);
                    }
                }
                None => {
                    if *style == ValueStyle::Bind {
                        for keyword in ["true", "false", "null"] {
                            push(keyword_item(keyword, site.replace));
                        }
                    }
                }
            },
        },
    }

    items.into_values().collect()
}

fn key_items(
    nav: &SchemaNavigator<'_>,
    node: SchemaNodeId,
    used: &[String],
    replace: InputSpan,
    push: &mut impl FnMut(CompletionItem),
) {
    let schema = nav.schema();
    for (name, field) in nav.record_fields(node) {
        if used.contains(&name) {
            continue;
        }
        let field_node = schema.node(field.schema);
        let mut detail = type_summary(nav, field.schema);
        if field.optional {
            detail.push_str(" (optional)");
        }
        if let Some(default) = field_node
            .metadata
            .default
            .as_ref()
            .and_then(|doc| render_literal(doc, ValueStyle::Bind))
        {
            detail.push_str(&format!(" = {default}"));
        }
        push(CompletionItem {
            label: key_label(&name),
            kind: CompletionKind::Field,
            detail: Some(detail),
            documentation: description(&field_node.metadata.description),
            deprecated: field_node.metadata.deprecated,
            replace,
        });
    }
    for ext in nav.extension_names(node) {
        let Some(ext_schema) = schema.node(node).ext_types.get(ext) else {
            continue;
        };
        push(CompletionItem {
            label: format!("${ext}"),
            kind: CompletionKind::Extension,
            detail: Some(type_summary(nav, ext_schema.schema)),
            documentation: description(&schema.node(ext_schema.schema).metadata.description),
            deprecated: false,
            replace,
        });
    }
}

fn variant_items(
    nav: &SchemaNavigator<'_>,
    parent: &[PathSegment],
    hints: &[VariantHint],
    partial: &str,
    replace: InputSpan,
    push: &mut impl FnMut(CompletionItem),
) {
    // `ok.so|` completes the nested variant after descending through `ok`;
    // labels carry the completed prefix so they match the whole partial.
    let (prefix, done_path) = match partial.rfind('.') {
        Some(dot) => match VariantPath::parse(&partial[..dot]) {
            Ok(path) => (&partial[..=dot], Some(path)),
            Err(_) => return,
        },
        None => ("", None),
    };

    let mut unions: Vec<SchemaNodeId> = nav.resolve_to_union(parent, hints);
    if let Some(done_path) = &done_path {
        let mut next = Vec::new();
        for union in unions {
            for candidate in nav.descend_variants(union, done_path) {
                if !next.contains(&candidate) {
                    next.push(candidate);
                }
            }
        }
        unions = next;
    }

    for union in unions {
        let SchemaNodeContent::Union(union_schema) = &nav.schema().node(union).content else {
            continue;
        };
        for (name, &variant_id) in &union_schema.variants {
            let variant = nav.schema().node(variant_id);
            push(CompletionItem {
                label: format!("{prefix}{name}"),
                kind: CompletionKind::Variant,
                detail: Some(type_summary(nav, variant_id)),
                documentation: description(&variant.metadata.description),
                deprecated: variant.metadata.deprecated,
                replace,
            });
        }
    }
}

fn value_items(
    nav: &SchemaNavigator<'_>,
    node: SchemaNodeId,
    style: ValueStyle,
    replace: InputSpan,
    push: &mut impl FnMut(CompletionItem),
) {
    let schema_node = nav.schema().node(node);
    match (&schema_node.content, style) {
        (SchemaNodeContent::Boolean, ValueStyle::Bind) => {
            push(keyword_item("true", replace));
            push(keyword_item("false", replace));
        }
        (SchemaNodeContent::Null, ValueStyle::Bind) => push(keyword_item("null", replace)),
        (SchemaNodeContent::Any, ValueStyle::Bind) => {
            for keyword in ["true", "false", "null"] {
                push(keyword_item(keyword, replace));
            }
        }
        (SchemaNodeContent::Literal(doc), style) => {
            if let Some(label) = render_literal(doc, style) {
                push(CompletionItem {
                    label,
                    kind: CompletionKind::Value,
                    detail: Some("literal".to_string()),
                    documentation: description(&schema_node.metadata.description),
                    deprecated: schema_node.metadata.deprecated,
                    replace,
                });
            }
        }
        _ => {}
    }
    if let Some(default) = schema_node
        .metadata
        .default
        .as_ref()
        .and_then(|doc| render_literal(doc, style))
    {
        push(CompletionItem {
            label: default,
            kind: CompletionKind::Value,
            detail: Some("default".to_string()),
            documentation: None,
            deprecated: false,
            replace,
        });
    }
}

fn keyword_item(keyword: &str, replace: InputSpan) -> CompletionItem {
    CompletionItem {
        label: keyword.to_string(),
        kind: CompletionKind::Value,
        detail: None,
        documentation: None,
        deprecated: false,
        replace,
    }
}

/// Field name as it must be written in a key position.
fn key_label(name: &str) -> String {
    if name.parse::<Identifier>().is_ok() {
        name.to_string()
    } else {
        quote(name)
    }
}

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn description(description: &Option<Description>) -> Option<String> {
    match description {
        Some(Description::String(s)) | Some(Description::Markdown(s)) => Some(s.clone()),
        None => None,
    }
}

/// Render a literal/default document as source text, when it is a primitive.
fn render_literal(doc: &EureDocument, style: ValueStyle) -> Option<String> {
    let NodeValue::Primitive(primitive) = &doc.node(doc.get_root_id()).content else {
        return None;
    };
    match (primitive, style) {
        (PrimitiveValue::Text(text), ValueStyle::Text) => Some(text.content.clone()),
        (PrimitiveValue::Text(text), ValueStyle::Bind) => Some(quote(&text.content)),
        (_, ValueStyle::Text) => None,
        (PrimitiveValue::Null, _) => Some("null".to_string()),
        (PrimitiveValue::Bool(b), _) => Some(b.to_string()),
        (PrimitiveValue::Integer(i), _) => Some(i.to_string()),
        (PrimitiveValue::F32(f), _) => Some(f.to_string()),
        (PrimitiveValue::F64(f), _) => Some(f.to_string()),
    }
}

/// One-line type description for `detail`.
pub fn type_summary(nav: &SchemaNavigator<'_>, id: SchemaNodeId) -> String {
    let schema = nav.schema();
    match &schema.node(id).content {
        SchemaNodeContent::Any => "any".to_string(),
        SchemaNodeContent::Text(text) => match &text.language {
            Some(language) => format!("text.{language}"),
            None => "text".to_string(),
        },
        SchemaNodeContent::Integer(_) => "integer".to_string(),
        SchemaNodeContent::Float(_) => "float".to_string(),
        SchemaNodeContent::Boolean => "boolean".to_string(),
        SchemaNodeContent::Null => "null".to_string(),
        SchemaNodeContent::Literal(doc) => {
            render_literal(doc, ValueStyle::Bind).unwrap_or_else(|| "literal".to_string())
        }
        SchemaNodeContent::Array(array) => format!("[{}]", type_summary(nav, array.item)),
        SchemaNodeContent::Map(map) => format!(
            "{{ {} => {} }}",
            type_summary(nav, map.key),
            type_summary(nav, map.value)
        ),
        SchemaNodeContent::Record(_) => "record".to_string(),
        SchemaNodeContent::Tuple(tuple) => format!(
            "({})",
            tuple
                .elements
                .iter()
                .map(|&e| type_summary(nav, e))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        SchemaNodeContent::Union(union) => format!(
            "union {{ {} }}",
            union
                .variants
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ),
        SchemaNodeContent::Reference(reference) => {
            format!("$types.{}", schema.display_reference(reference))
        }
    }
}
