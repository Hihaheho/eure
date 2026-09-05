//! Schema-based hover.
//!
//! Shares the cursor-site walk with completion (see [`super::completion`]):
//! [`find_site`] reports the key or value under the cursor as an
//! [`Anchor`], the document's schema is loaded, and [`hover_markdown`]
//! describes the schema node at the anchor's path.
//!
//! What is shown, in order:
//!
//! Diagnostics at the cursor are shown first, followed by schema information:
//!
//! 1. A signature line: the document path and its type summary, with
//!    `(optional)` for optional record fields and extensions.
//! 2. Deprecation, the `$description`, the `$default` and `$examples`.
//! 3. The structure behind the type: record fields, union variants, or the
//!    constraints of a primitive/container type.
//!
//! `$variant` is special-cased: hovering the key lists the variants of the
//! union it selects; hovering the value describes the selected variant.
//!
//! Without a schema (or at a path the schema does not know) only the path
//! is shown, which is still useful inside deeply nested sections.

use eure_document::parse::variant_path::VariantPath;
use eure_document::path::PathSegment;
use eure_document::value::ObjectKey;
use eure_schema::navigate::{SchemaNavigator, VariantHint, hint_at};
use eure_schema::{
    FloatPrecision, SchemaDocument, SchemaNode, SchemaNodeContent, SchemaNodeId,
    UnknownFieldsPolicy,
};
use eure_tree::tree::InputSpan;
use query_flow::{Db, QueryError};

use super::assets::TextFile;
use super::completion::{Anchor, AnchorKind, ValueStyle, find_site, load_schema};
use super::diagnostics::{DiagnosticSeverity, GetFileDiagnostics};
use super::parse::ParseCst;
use super::summary::{description_text, render_literal, type_summary};

/// Editor-agnostic hover result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Markdown content.
    pub contents: String,
    /// Range where the diagnostic and/or schema information applies.
    pub span: InputSpan,
}

/// Hover information for `file` at byte offset `offset`.
///
/// Diagnostics at the cursor precede schema information. Works on documents
/// that do not parse, including positions with only a diagnostic to show.
///
/// Like `get_completions`, this is a plain function over `Db` rather than a
/// query so that per-cursor results are not memoized.
pub fn get_hover(db: &impl Db, file: &TextFile, offset: u32) -> Result<Option<Hover>, QueryError> {
    let diagnostics = db.query(GetFileDiagnostics::new(file.clone()))?;
    let mut diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            &d.file == file
                && d.start <= offset as usize
                && ((offset as usize) < d.end || d.start == d.end && d.start == offset as usize)
        })
        .collect();
    diagnostics.sort_by_key(|d| match d.severity {
        DiagnosticSeverity::Error => 0,
        DiagnosticSeverity::Warning => 1,
        DiagnosticSeverity::Info => 2,
        DiagnosticSeverity::Hint => 3,
    });
    let mut sections = Vec::new();
    let mut span: Option<InputSpan> = None;
    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => "Error",
            DiagnosticSeverity::Warning => "Warning",
            DiagnosticSeverity::Info => "Information",
            DiagnosticSeverity::Hint => "Hint",
        };
        // Diagnostic messages are plain text, not schema-authored Markdown.
        let message: String = diagnostic.message.chars().fold(String::new(), |mut s, c| {
            if matches!(
                c,
                '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '#' | '~'
            ) {
                s.push('\\');
            }
            s.push(c);
            s
        });
        sections.push(format!("**{severity}**\n\n{message}"));
        let diagnostic_span = InputSpan {
            start: diagnostic.start as u32,
            end: diagnostic.end as u32,
        };
        span = Some(intersect_span(span, diagnostic_span));
    }

    let parsed = db.query(ParseCst::new(file.clone()))?;
    let source = db.asset(file.clone())?;
    if let Some(site) = find_site(source.get(), &parsed.cst, offset)
        && let Some(anchor) = site.anchor
    {
        let schema = load_schema(db, file)?;
        sections.push(hover_markdown(&anchor, &site.hints, schema.as_deref()));
        span = Some(intersect_span(span, anchor.span));
    }
    Ok(span.map(|span| Hover {
        contents: sections.join("\n\n---\n\n"),
        span,
    }))
}

// Keep the returned range inside every contributing range so clients do not
// reuse diagnostic content when moving outside the diagnostic's location.
fn intersect_span(previous: Option<InputSpan>, next: InputSpan) -> InputSpan {
    match previous {
        Some(previous) => InputSpan {
            start: previous.start.max(next.start),
            end: previous.end.min(next.end),
        },
        None => next,
    }
}

/// Markdown describing `anchor` under `schema`.
///
/// `schema` is `None` when the document has no schema; only the path is
/// shown in that case.
pub fn hover_markdown(
    anchor: &Anchor,
    hints: &[VariantHint],
    schema: Option<&SchemaDocument>,
) -> String {
    let path = &anchor.path.0;
    let title = anchor.path.to_string();
    let Some(schema) = schema else {
        return code_block(&title);
    };
    let nav = SchemaNavigator::new(schema);

    let sections = match path.split_last() {
        Some((PathSegment::Extension(ext), parent)) if ext.as_ref() == "variant" => {
            variant_sections(&nav, &title, parent, hints, anchor.kind)
        }
        _ => node_sections(&nav, &title, path, hints),
    };
    if sections.is_empty() {
        code_block(&title)
    } else {
        sections.join("\n\n---\n\n")
    }
}

/// A schema node that may describe the hovered path, before references and
/// unions are expanded, so that the signature can name the referenced type.
struct Target {
    node: SchemaNodeId,
    /// The path is an optional field or extension of its parent.
    optional: bool,
}

/// One section per schema node that may describe `path` (several inside an
/// untagged union).
fn node_sections(
    nav: &SchemaNavigator<'_>,
    title: &str,
    path: &[PathSegment],
    hints: &[VariantHint],
) -> Vec<String> {
    let targets: Vec<Target> = match path.split_last() {
        None => vec![Target {
            node: nav.schema().root,
            optional: false,
        }],
        Some((last, parent)) => {
            let mut targets: Vec<Target> = Vec::new();
            for parent_node in nav.resolve(parent, hints) {
                let Some(node) = nav.step(parent_node, last) else {
                    continue;
                };
                if targets.iter().any(|t| t.node == node) {
                    continue;
                }
                targets.push(Target {
                    node,
                    optional: is_optional(nav, parent_node, last),
                });
            }
            targets
        }
    };

    let hint = hint_at(hints, path.len());
    let mut sections: Vec<String> = Vec::new();
    for target in targets {
        let section = render_target(nav, title, &target, hint);
        if !sections.contains(&section) {
            sections.push(section);
        }
    }
    sections
}

/// Whether `segment` names an optional field or extension of `parent`.
fn is_optional(nav: &SchemaNavigator<'_>, parent: SchemaNodeId, segment: &PathSegment) -> bool {
    match segment {
        PathSegment::Ident(ident) => nav
            .record_fields(parent)
            .get(ident.as_ref())
            .is_some_and(|field| field.optional),
        PathSegment::Value(ObjectKey::String(name)) => nav
            .record_fields(parent)
            .get(name.as_str())
            .is_some_and(|field| field.optional),
        PathSegment::Extension(name) => nav
            .schema()
            .node(parent)
            .ext_types
            .get(name)
            .is_some_and(|ext| ext.optional),
        _ => false,
    }
}

fn render_target(
    nav: &SchemaNavigator<'_>,
    title: &str,
    target: &Target,
    hint: Option<&VariantPath>,
) -> String {
    let schema = nav.schema();
    let mut signature = format!("{title}: {}", type_summary(nav, target.node));
    if target.optional {
        signature.push_str(" (optional)");
    }
    let mut paragraphs = vec![code_block(&signature)];

    // Nodes whose structure is shown. An untagged union is presented as its
    // variant list; a hinted one as the selected variant.
    let is_union =
        |id: SchemaNodeId| matches!(schema.node(id).content, SchemaNodeContent::Union(_));
    let detail_nodes: Vec<SchemaNodeId> = match (nav.deref_references(target.node), hint) {
        (None, _) => Vec::new(),
        (Some(id), None) if is_union(id) => vec![id],
        (Some(id), Some(hint)) if is_union(id) => {
            paragraphs.push(format!("Variant: `{hint}`"));
            nav.concretize(target.node, Some(hint))
        }
        (Some(_), _) => nav.concretize(target.node, None),
    };

    // Metadata: the field's own node first, then the type definition behind it.
    let nodes: Vec<&SchemaNode> = std::iter::once(target.node)
        .chain(detail_nodes.iter().copied())
        .collect::<indexmap::IndexSet<_>>()
        .into_iter()
        .map(|id| schema.node(id))
        .collect();
    paragraphs.extend(metadata_paragraphs(&nodes));

    for id in detail_nodes {
        paragraphs.extend(detail_paragraphs(nav, id));
    }
    paragraphs.join("\n\n")
}

/// Sections for the `$variant` extension at `parent`.
fn variant_sections(
    nav: &SchemaNavigator<'_>,
    title: &str,
    parent: &[PathSegment],
    hints: &[VariantHint],
    kind: AnchorKind,
) -> Vec<String> {
    let schema = nav.schema();
    let unions: Vec<SchemaNodeId> = nav
        .resolve_to_union(parent, hints)
        .into_iter()
        .filter(|&id| matches!(schema.node(id).content, SchemaNodeContent::Union(_)))
        .collect();
    let selected = hint_at(hints, parent.len());

    let mut sections: Vec<String> = Vec::new();

    // Hovering the value: describe the selected variant.
    if kind == AnchorKind::Value
        && let Some(selected) = selected
    {
        for &union in &unions {
            for variant in nav.descend_variants(union, selected) {
                let mut paragraphs = vec![code_block(&format!("{title}: {selected}"))];
                paragraphs.extend(metadata_paragraphs(&[schema.node(variant)]));
                for id in nav.concretize(variant, None) {
                    paragraphs.extend(detail_paragraphs(nav, id));
                }
                let section = paragraphs.join("\n\n");
                if !sections.contains(&section) {
                    sections.push(section);
                }
            }
        }
        if !sections.is_empty() {
            return sections;
        }
    }

    // Hovering the key (or an unknown variant name): list the variants.
    for union in unions {
        let mut paragraphs = vec![code_block(title)];
        paragraphs.extend(metadata_paragraphs(&[schema.node(union)]));
        paragraphs.extend(detail_paragraphs(nav, union));
        let section = paragraphs.join("\n\n");
        if !sections.contains(&section) {
            sections.push(section);
        }
    }
    sections
}

/// Deprecation, description, default and examples. The first node that
/// carries each piece of metadata wins.
fn metadata_paragraphs(nodes: &[&SchemaNode]) -> Vec<String> {
    let mut paragraphs = Vec::new();
    if nodes.iter().any(|node| node.metadata.deprecated) {
        paragraphs.push("**Deprecated**".to_string());
    }
    if let Some(description) = nodes
        .iter()
        .find_map(|node| description_text(&node.metadata.description))
    {
        let description = description.trim();
        if !description.is_empty() {
            paragraphs.push(description.to_string());
        }
    }
    if let Some(default) = nodes
        .iter()
        .find_map(|node| node.metadata.default.as_ref())
        .and_then(|doc| render_literal(doc, ValueStyle::Bind))
    {
        paragraphs.push(format!("Default: `{default}`"));
    }
    if let Some(examples) = nodes
        .iter()
        .find_map(|node| node.metadata.examples.as_ref())
    {
        let rendered: Vec<String> = examples
            .iter()
            .filter_map(|doc| render_literal(doc, ValueStyle::Bind))
            .map(|example| format!("`{example}`"))
            .collect();
        if !rendered.is_empty() {
            paragraphs.push(format!("Examples: {}", rendered.join(", ")));
        }
    }
    paragraphs
}

/// Structure of a concrete node: record fields, union variants, or the
/// constraints of a primitive/container type.
fn detail_paragraphs(nav: &SchemaNavigator<'_>, id: SchemaNodeId) -> Vec<String> {
    let schema = nav.schema();
    let node = schema.node(id);
    match &node.content {
        SchemaNodeContent::Record(record) => {
            let mut lines = Vec::new();
            for (name, field) in nav.record_fields(id) {
                let mut line = format!("- `{name}`: {}", type_summary(nav, field.schema));
                if field.optional {
                    line.push_str(" (optional)");
                }
                lines.push(line);
            }
            for (name, ext) in &node.ext_types {
                let mut line = format!("- `${name}`: {}", type_summary(nav, ext.schema));
                if ext.optional {
                    line.push_str(" (optional)");
                }
                lines.push(line);
            }
            match &record.unknown_fields {
                UnknownFieldsPolicy::Deny => {}
                UnknownFieldsPolicy::Allow => lines.push("- *other fields*: any".to_string()),
                UnknownFieldsPolicy::Schema(id) => {
                    lines.push(format!("- *other fields*: {}", type_summary(nav, *id)));
                }
            }
            list_paragraph("Fields", lines)
        }
        SchemaNodeContent::Union(union) => {
            let lines = union
                .variants
                .iter()
                .map(|(name, &variant)| {
                    let mut line = format!("- `{name}`: {}", type_summary(nav, variant));
                    if let Some(description) =
                        description_text(&schema.node(variant).metadata.description)
                            .and_then(|text| text.lines().next())
                            .map(str::trim)
                            .filter(|line| !line.is_empty())
                    {
                        line.push_str(&format!(" — {description}"));
                    }
                    line
                })
                .collect();
            list_paragraph("Variants", lines)
        }
        SchemaNodeContent::Text(text) => {
            let mut lines = Vec::new();
            if let Some(min) = text.min_length {
                lines.push(format!("- min-length: `{min}`"));
            }
            if let Some(max) = text.max_length {
                lines.push(format!("- max-length: `{max}`"));
            }
            if let Some(pattern) = &text.pattern {
                lines.push(format!("- pattern: `{}`", pattern.as_str()));
            }
            list_paragraph("Constraints", lines)
        }
        SchemaNodeContent::Integer(integer) => {
            let mut lines = Vec::new();
            if let Some(range) = integer.range_display() {
                lines.push(format!("- range: `{range}`"));
            }
            if let Some(multiple_of) = &integer.multiple_of {
                lines.push(format!("- multiple-of: `{multiple_of}`"));
            }
            list_paragraph("Constraints", lines)
        }
        SchemaNodeContent::Float(float) => {
            let mut lines = Vec::new();
            if let Some(range) = float.range_display() {
                lines.push(format!("- range: `{range}`"));
            }
            if let Some(multiple_of) = float.multiple_of {
                lines.push(format!("- multiple-of: `{multiple_of}`"));
            }
            if float.precision == FloatPrecision::F32 {
                lines.push("- precision: `f32`".to_string());
            }
            list_paragraph("Constraints", lines)
        }
        SchemaNodeContent::Array(array) => {
            let mut lines = Vec::new();
            if let Some(min) = array.min_length {
                lines.push(format!("- min-length: `{min}`"));
            }
            if let Some(max) = array.max_length {
                lines.push(format!("- max-length: `{max}`"));
            }
            if array.unique {
                lines.push("- unique: `true`".to_string());
            }
            if let Some(contains) = array.contains {
                lines.push(format!("- contains: {}", type_summary(nav, contains)));
            }
            list_paragraph("Constraints", lines)
        }
        SchemaNodeContent::Map(map) => {
            let mut lines = Vec::new();
            if let Some(min) = map.min_size {
                lines.push(format!("- min-size: `{min}`"));
            }
            list_paragraph("Constraints", lines)
        }
        SchemaNodeContent::Any
        | SchemaNodeContent::Boolean
        | SchemaNodeContent::Null
        | SchemaNodeContent::Literal(_)
        | SchemaNodeContent::Tuple(_)
        | SchemaNodeContent::Reference(_) => Vec::new(),
    }
}

fn list_paragraph(heading: &str, lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }
    vec![format!("**{heading}**\n{}", lines.join("\n"))]
}

fn code_block(text: &str) -> String {
    format!("```eure\n{text}\n```")
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::path::{ArrayIndexKind, EurePath};

    fn anchor(kind: AnchorKind, segments: &[&str]) -> Anchor {
        Anchor {
            kind,
            path: EurePath(
                segments
                    .iter()
                    .map(|s| match *s {
                        "[]" => PathSegment::ArrayIndex(ArrayIndexKind::Push),
                        s if s.starts_with('$') => PathSegment::Extension(s[1..].parse().unwrap()),
                        s => PathSegment::Ident(s.parse().unwrap()),
                    })
                    .collect(),
            ),
            span: InputSpan::EMPTY,
        }
    }

    fn schema(source: &str) -> SchemaDocument {
        let doc = crate::document::parse_to_document(source, "<schema>").unwrap();
        eure_schema::convert::document_to_schema(&doc).unwrap().0
    }

    #[test]
    fn without_schema_only_the_path_is_shown() {
        let markdown = hover_markdown(&anchor(AnchorKind::Key, &["a", "[]", "b"]), &[], None);
        assert_eq!(markdown, "```eure\na[].b\n```");
    }

    #[test]
    fn unknown_path_falls_back_to_the_path() {
        let schema = schema("name = `text`");
        let markdown = hover_markdown(&anchor(AnchorKind::Key, &["missing"]), &[], Some(&schema));
        assert_eq!(markdown, "```eure\nmissing\n```");
    }

    #[test]
    fn field_hover_shows_type_metadata_and_constraints() {
        let schema = schema(
            "@ port\n$variant: integer\nrange = \"[1, 65535]\"\n$optional = true\n$description = \"Port to listen on.\"\n$default = 8080\n",
        );
        let markdown = hover_markdown(&anchor(AnchorKind::Key, &["port"]), &[], Some(&schema));
        assert_eq!(
            markdown,
            "```eure\nport: integer (optional)\n```\n\nPort to listen on.\n\nDefault: `8080`\n\n**Constraints**\n- range: `[1, 65535]`"
        );
    }

    #[test]
    fn reference_is_named_and_expanded() {
        let schema = schema(
            "$types.person {\n  name = `text`\n  age = `integer`\n  age.$optional = true\n}\nuser = `$types.person`\n",
        );
        let markdown = hover_markdown(&anchor(AnchorKind::Value, &["user"]), &[], Some(&schema));
        assert_eq!(
            markdown,
            "```eure\nuser: $types.person\n```\n\n**Fields**\n- `name`: text\n- `age`: integer (optional)"
        );
    }

    #[test]
    fn variant_key_lists_variants_and_value_describes_selected_one() {
        let schema = schema(
            "$types.action {\n  $variant: union\n  variants.say {\n    $description = \"Speak a line.\"\n    line = `text`\n  }\n  variants.wait {\n    ms = `integer`\n  }\n}\nactions = [`$types.action`]\n",
        );
        let hints = [VariantHint {
            prefix_len: 2,
            variant: VariantPath::parse("say").unwrap(),
        }];

        let key = hover_markdown(
            &anchor(AnchorKind::Key, &["actions", "[]", "$variant"]),
            &hints,
            Some(&schema),
        );
        assert_eq!(
            key,
            "```eure\nactions[].$variant\n```\n\n**Variants**\n- `say`: record — Speak a line.\n- `wait`: record"
        );

        let value = hover_markdown(
            &anchor(AnchorKind::Value, &["actions", "[]", "$variant"]),
            &hints,
            Some(&schema),
        );
        assert_eq!(
            value,
            "```eure\nactions[].$variant: say\n```\n\nSpeak a line.\n\n**Fields**\n- `line`: text"
        );
    }
}
