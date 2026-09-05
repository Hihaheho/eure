//! Human-readable summaries of schema nodes, shared by completion and hover.

use eure_document::document::EureDocument;
use eure_document::document::node::NodeValue;
use eure_document::value::PrimitiveValue;
use eure_schema::navigate::SchemaNavigator;
use eure_schema::{Description, SchemaNodeContent, SchemaNodeId};

use super::completion::ValueStyle;

/// One-line type description (`text`, `[integer]`, `$types.user`, ...).
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

/// Render a literal/default document as source text, when it is a primitive.
pub fn render_literal(doc: &EureDocument, style: ValueStyle) -> Option<String> {
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

/// `text` as a double-quoted Eure string literal.
pub fn quote(text: &str) -> String {
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

/// Text of a schema `$description`, as markdown.
pub fn description_text(description: &Option<Description>) -> Option<&str> {
    match description {
        Some(Description::String(s)) | Some(Description::Markdown(s)) => Some(s.as_str()),
        None => None,
    }
}
