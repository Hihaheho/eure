//! Formatter for SourceDocument.
//!
//! This module provides formatting functionality for `SourceDocument`,
//! which is used when converting from other formats (TOML, JSON, etc.) to Eure.
//!
//! The implementation builds a `Doc` IR that integrates with eure-fmt's
//! pretty-printing infrastructure.

use crate::config::FormatConfig;
use crate::doc::Doc;
use crate::printer::Printer;

use eure_document::document::node::{NodeArray, NodeMap, NodeTuple, NodeValue};
use eure_document::document::{EureDocument, NodeId};
use eure_document::source::{
    ArrayElementSource, BindSource, BindingSource, Comment, EureSource, SectionBody, SectionSource,
    SourceDocument, SourceId, SourceKey, SourcePathSegment, StringStyle, Trivia,
};
use eure_document::text::{Language, SyntaxHint};
use eure_document::value::{ObjectKey, PrimitiveValue};

/// Build a Doc IR from a SourceDocument.
///
/// This produces a document IR that can be printed with the pretty-printer.
pub fn build_source_doc(source: &SourceDocument) -> Doc {
    SourceDocBuilder::new(source).build()
}

/// Format a SourceDocument to Eure source string.
///
/// This produces output that can be parsed back to an equivalent EureDocument.
/// Comments and section ordering from the source structure are preserved.
pub fn format_source_document(source: &SourceDocument) -> String {
    let doc = build_source_doc(source);
    Printer::new(FormatConfig::default()).print(&doc)
}

struct SourceDocBuilder<'a> {
    source: &'a SourceDocument,
}

impl<'a> SourceDocBuilder<'a> {
    fn new(source: &'a SourceDocument) -> Self {
        Self { source }
    }

    fn doc(&self) -> &EureDocument {
        &self.source.document
    }

    fn get_source(&self, id: SourceId) -> &EureSource {
        self.source.source(id)
    }

    fn build(&self) -> Doc {
        self.build_eure_source(self.source.root_source())
    }

    fn build_eure_source(&self, eure: &EureSource) -> Doc {
        let mut parts = Vec::new();

        // Leading trivia
        if !eure.leading_trivia.is_empty() {
            parts.push(self.build_trivia(&eure.leading_trivia));
        }

        // Value binding (if present)
        if let Some(node_id) = eure.value {
            parts.push(
                Doc::text("= ")
                    .concat(self.build_value(node_id))
                    .concat(Doc::hardline()),
            );
        }

        // Bindings
        for binding in &eure.bindings {
            parts.push(self.build_binding(binding));
        }

        // Sections
        for section in &eure.sections {
            parts.push(self.build_section(section));
        }

        // Trailing trivia
        if !eure.trailing_trivia.is_empty() {
            parts.push(self.build_trivia(&eure.trailing_trivia));
        }

        Doc::concat_all(parts)
    }

    fn build_trivia(&self, trivia: &[Trivia]) -> Doc {
        let mut parts = Vec::new();
        for item in trivia {
            match item {
                Trivia::Comment(comment) => {
                    parts.push(self.build_comment(comment));
                }
                Trivia::BlankLine => {
                    parts.push(Doc::hardline());
                }
            }
        }
        Doc::concat_all(parts)
    }

    fn build_comment(&self, comment: &Comment) -> Doc {
        match comment {
            Comment::Line(s) => {
                if s.is_empty() {
                    Doc::text("//").concat(Doc::hardline())
                } else {
                    Doc::text("// ")
                        .concat(Doc::text(s.clone()))
                        .concat(Doc::hardline())
                }
            }
            Comment::Block(s) => {
                if s.is_empty() {
                    Doc::text("/**/").concat(Doc::hardline())
                } else {
                    Doc::text("/* ")
                        .concat(Doc::text(s.clone()))
                        .concat(Doc::text(" */"))
                        .concat(Doc::hardline())
                }
            }
        }
    }

    /// Build a comment for array elements (no trailing hardline, caller controls line breaks).
    fn build_array_comment(&self, comment: &Comment) -> Doc {
        match comment {
            Comment::Line(s) => {
                if s.is_empty() {
                    Doc::text("//")
                } else {
                    Doc::text("// ").concat(Doc::text(s.clone()))
                }
            }
            Comment::Block(s) => {
                if s.is_empty() {
                    Doc::text("/**/")
                } else {
                    Doc::text("/* ")
                        .concat(Doc::text(s.clone()))
                        .concat(Doc::text(" */"))
                }
            }
        }
    }

    /// Build a trailing comment (same line, no hardline at end).
    fn build_trailing_comment(&self, comment: &Comment) -> Doc {
        match comment {
            Comment::Line(s) => {
                if s.is_empty() {
                    Doc::text(" //")
                } else {
                    Doc::text(" // ").concat(Doc::text(s.clone()))
                }
            }
            Comment::Block(s) => {
                if s.is_empty() {
                    Doc::text(" /**/")
                } else {
                    Doc::text(" /* ")
                        .concat(Doc::text(s.clone()))
                        .concat(Doc::text(" */"))
                }
            }
        }
    }

    fn build_binding(&self, binding: &BindingSource) -> Doc {
        let mut parts = Vec::new();

        // Trivia before this binding
        if !binding.trivia_before.is_empty() {
            parts.push(self.build_trivia(&binding.trivia_before));
        }

        let path_doc = self.build_path(&binding.path);

        let body_doc = match &binding.bind {
            BindSource::Value(node_id) => Doc::text(" = ").concat(self.build_value(*node_id)),
            BindSource::Array { node, elements } => {
                Doc::text(" = ").concat(self.build_array_with_trivia(*node, elements))
            }
            BindSource::Block(source_id) => {
                let inner = self.build_eure_source(self.get_source(*source_id));
                Doc::text(" {")
                    .concat(Doc::hardline())
                    .concat(Doc::indent(inner))
                    .concat(Doc::text("}"))
            }
        };

        let mut doc = path_doc.concat(body_doc);

        if let Some(comment) = &binding.trailing_comment {
            doc = doc.concat(self.build_trailing_comment(comment));
        }

        parts.push(doc.concat(Doc::hardline()));
        Doc::concat_all(parts)
    }

    fn build_section(&self, section: &SectionSource) -> Doc {
        let mut parts = Vec::new();

        // Trivia before this section
        if !section.trivia_before.is_empty() {
            parts.push(self.build_trivia(&section.trivia_before));
        }

        let mut header = Doc::text("@ ").concat(self.build_path(&section.path));

        if let Some(comment) = &section.trailing_comment {
            header = header.concat(self.build_trailing_comment(comment));
        }

        let section_doc = match &section.body {
            SectionBody::Items { value, bindings } => {
                let mut body_parts = Vec::new();

                // Value binding (if present)
                if let Some(node_id) = value {
                    body_parts.push(
                        Doc::text("= ")
                            .concat(self.build_value(*node_id))
                            .concat(Doc::hardline()),
                    );
                }

                // Bindings in section
                for binding in bindings {
                    body_parts.push(self.build_binding(binding));
                }

                header
                    .concat(Doc::hardline())
                    .concat(Doc::concat_all(body_parts))
            }
            SectionBody::Block(source_id) => {
                let inner = self.build_eure_source(self.get_source(*source_id));
                header
                    .concat(Doc::text(" {"))
                    .concat(Doc::hardline())
                    .concat(Doc::indent(inner))
                    .concat(Doc::text("}"))
                    .concat(Doc::hardline())
            }
        };

        parts.push(section_doc);
        Doc::concat_all(parts)
    }

    fn build_path(&self, path: &[SourcePathSegment]) -> Doc {
        let mut result = Doc::Nil;
        for (i, segment) in path.iter().enumerate() {
            if i > 0 {
                result = result.concat(Doc::text("."));
            }
            result = result.concat(self.build_key(&segment.key));
            if let Some(index) = &segment.array {
                result = result.concat(Doc::text("["));
                if let Some(n) = index {
                    result = result.concat(Doc::text(n.to_string()));
                }
                result = result.concat(Doc::text("]"));
            }
        }
        result
    }

    fn build_key(&self, key: &SourceKey) -> Doc {
        match key {
            SourceKey::Ident(s) => Doc::text(s.as_ref()),
            SourceKey::Extension(s) => Doc::text("$").concat(Doc::text(s.as_ref())),
            SourceKey::String(s, style) => match style {
                StringStyle::Quoted => Doc::text("\"")
                    .concat(Doc::text(escape_string(s)))
                    .concat(Doc::text("\"")),
                StringStyle::Backtick => Doc::text("`")
                    .concat(Doc::text(s.clone()))
                    .concat(Doc::text("`")),
            },
            SourceKey::Integer(n) => Doc::text(n.to_string()),
            SourceKey::Tuple(keys) => {
                let inner = Doc::join(keys.iter().map(|k| self.build_key(k)), Doc::text(", "));
                Doc::text("(").concat(inner).concat(Doc::text(")"))
            }
            SourceKey::TupleIndex(n) => Doc::text("#").concat(Doc::text(n.to_string())),
        }
    }

    fn build_value(&self, node_id: NodeId) -> Doc {
        let node = self.doc().node(node_id);
        match &node.content {
            NodeValue::Hole(_) => Doc::text("null"),
            NodeValue::Primitive(prim) => self.build_primitive(prim),
            NodeValue::Array(arr) => self.build_array(node_id, arr),
            NodeValue::Tuple(tuple) => self.build_tuple(tuple),
            NodeValue::Map(map) => self.build_map(map),
        }
    }

    fn build_primitive(&self, prim: &PrimitiveValue) -> Doc {
        match prim {
            PrimitiveValue::Null => Doc::text("null"),
            PrimitiveValue::Bool(b) => Doc::text(if *b { "true" } else { "false" }),
            PrimitiveValue::Integer(n) => Doc::text(n.to_string()),
            PrimitiveValue::F64(f) => self.build_f64(*f),
            PrimitiveValue::F32(f) => self.build_f32(*f),
            PrimitiveValue::Text(text) => self.build_text(text),
        }
    }

    fn build_f64(&self, f: f64) -> Doc {
        if f.is_nan() {
            Doc::text("nan")
        } else if f.is_infinite() {
            if f.is_sign_positive() {
                Doc::text("inf")
            } else {
                Doc::text("-inf")
            }
        } else {
            let s = f.to_string();
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                Doc::text(s).concat(Doc::text(".0"))
            } else {
                Doc::text(s)
            }
        }
    }

    fn build_f32(&self, f: f32) -> Doc {
        if f.is_nan() {
            Doc::text("nan")
        } else if f.is_infinite() {
            if f.is_sign_positive() {
                Doc::text("inf")
            } else {
                Doc::text("-inf")
            }
        } else {
            Doc::text(f.to_string())
        }
    }

    fn build_text(&self, text: &eure_document::text::Text) -> Doc {
        let is_block = matches!(
            text.syntax_hint,
            Some(SyntaxHint::Block)
                | Some(SyntaxHint::Block3)
                | Some(SyntaxHint::Block4)
                | Some(SyntaxHint::Block5)
                | Some(SyntaxHint::Block6)
        );

        if is_block {
            self.build_block_text(text)
        } else {
            self.build_inline_text(text)
        }
    }

    fn build_block_text(&self, text: &eure_document::text::Text) -> Doc {
        let backticks = match text.syntax_hint {
            Some(SyntaxHint::Block6) => "``````",
            Some(SyntaxHint::Block5) => "`````",
            Some(SyntaxHint::Block4) => "````",
            _ => "```",
        };

        let mut doc = Doc::text(backticks);

        if let Language::Other(lang) = &text.language {
            doc = doc.concat(Doc::text(lang.clone()));
        }

        // Note: Block content with internal newlines - we output it literally
        // The printer doesn't wrap this content since it's already formatted
        doc = doc.concat(Doc::text("\n"));
        doc = doc.concat(Doc::text(text.content.clone()));
        if !text.content.ends_with('\n') {
            doc = doc.concat(Doc::text("\n"));
        }
        doc.concat(Doc::text(backticks))
    }

    fn build_inline_text(&self, text: &eure_document::text::Text) -> Doc {
        match &text.language {
            Language::Plaintext => Doc::text("\"")
                .concat(Doc::text(escape_string(&text.content)))
                .concat(Doc::text("\"")),
            Language::Implicit => Doc::text("`")
                .concat(Doc::text(text.content.clone()))
                .concat(Doc::text("`")),
            Language::Other(lang) => Doc::text(lang.clone())
                .concat(Doc::text("`"))
                .concat(Doc::text(text.content.clone()))
                .concat(Doc::text("`")),
        }
    }

    fn build_array(&self, node_id: NodeId, arr: &NodeArray) -> Doc {
        if arr.is_empty() {
            return Doc::text("[]");
        }

        // Check if this array should be forced multi-line (from source tracking)
        let force_multiline = self.source.is_multiline_array(node_id);

        if force_multiline {
            // Force multi-line: use hardline, no group
            let elements = Doc::join(
                arr.iter()
                    .map(|&id| self.build_value(id).concat(Doc::text(","))),
                Doc::hardline(),
            );

            Doc::text("[")
                .concat(Doc::indent(Doc::hardline().concat(elements)))
                .concat(Doc::hardline())
                .concat(Doc::text("]"))
        } else {
            // Smart breaking with Group
            let elements = Doc::join(
                arr.iter().map(|&id| self.build_value(id)),
                Doc::text(",").concat(Doc::line()),
            );

            Doc::group(
                Doc::text("[")
                    .concat(Doc::softline())
                    .concat(Doc::indent(elements))
                    .concat(Doc::softline())
                    .concat(Doc::text("]")),
            )
        }
    }

    /// Build an array with per-element trivia (comments/blank lines).
    fn build_array_with_trivia(&self, node_id: NodeId, elements: &[ArrayElementSource]) -> Doc {
        let arr = match &self.doc().node(node_id).content {
            NodeValue::Array(arr) => arr,
            _ => return self.build_value(node_id), // Fallback if not an array
        };

        if elements.is_empty() {
            // Fallback to regular array formatting
            return self.build_array(node_id, arr);
        }

        // Build inner content without per-item indent wrapping
        let mut inner_parts = Vec::new();

        for (i, elem_source) in elements.iter().enumerate() {
            // Trivia before element (comments, blank lines)
            for trivia in &elem_source.trivia_before {
                match trivia {
                    Trivia::Comment(comment) => {
                        // Build comment without trailing hardline (we control line breaks)
                        inner_parts.push(self.build_array_comment(comment));
                        inner_parts.push(Doc::hardline());
                    }
                    Trivia::BlankLine => {
                        inner_parts.push(Doc::hardline());
                    }
                }
            }

            // Element value
            let value_id = arr.get(elem_source.index).unwrap();
            let mut elem_doc = self.build_value(value_id);

            // Always add trailing comma for Eure style
            elem_doc = elem_doc.concat(Doc::text(","));

            // Trailing comment
            if let Some(comment) = &elem_source.trailing_comment {
                elem_doc = elem_doc.concat(self.build_trailing_comment(comment));
            }

            inner_parts.push(elem_doc);
            // Add newline after each element except the last
            if i < elements.len() - 1 {
                inner_parts.push(Doc::hardline());
            }
        }

        // Wrap all content in indent block
        // - Hardline inside Indent for proper indentation of first element
        // - Hardline after Indent for closing bracket at column 0
        Doc::text("[")
            .concat(Doc::indent(
                Doc::hardline().concat(Doc::concat_all(inner_parts)),
            ))
            .concat(Doc::hardline())
            .concat(Doc::text("]"))
    }

    fn build_tuple(&self, tuple: &NodeTuple) -> Doc {
        if tuple.is_empty() {
            return Doc::text("()");
        }

        let elements = Doc::join(
            tuple.iter().map(|&id| self.build_value(id)),
            Doc::text(", "),
        );

        Doc::text("(").concat(elements).concat(Doc::text(")"))
    }

    fn build_map(&self, map: &NodeMap) -> Doc {
        if map.is_empty() {
            return Doc::text("{}");
        }

        let entries = Doc::join(
            map.iter().map(|(key, &child_id)| {
                self.build_object_key(key)
                    .concat(Doc::text(" => "))
                    .concat(self.build_value(child_id))
            }),
            Doc::text(", "),
        );

        Doc::text("{ ").concat(entries).concat(Doc::text(" }"))
    }

    fn build_object_key(&self, key: &ObjectKey) -> Doc {
        match key {
            ObjectKey::String(s) => Doc::text(s.clone()),
            ObjectKey::Number(n) => Doc::text(n.to_string()),
            ObjectKey::Tuple(keys) => {
                let inner = Doc::join(
                    keys.iter().map(|k| self.build_object_key(k)),
                    Doc::text(", "),
                );
                Doc::text("(").concat(inner).concat(Doc::text(")"))
            }
        }
    }
}

/// Escape a string for Eure output
fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}
