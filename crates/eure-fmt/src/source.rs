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
    ArrayElementLayout, Comment, Layout, LayoutItem, SectionBody, SourceDocument, SourceKey,
    SourcePathSegment,
};
use eure_document::text::{Language, SyntaxHint};
use eure_document::value::{ObjectKey, PrimitiveValue};

/// Build a Doc IR from a SourceDocument.
///
/// This produces a document IR that can be printed with the pretty-printer.
pub fn build_source_doc(source: &SourceDocument) -> Doc {
    SourceDocBuilder::new(&source.document, &source.layout).build()
}

/// Format a SourceDocument to Eure source string.
///
/// This produces output that can be parsed back to an equivalent EureDocument.
/// Comments and section ordering from the Layout are preserved.
pub fn format_source_document(source: &SourceDocument) -> String {
    let doc = build_source_doc(source);
    Printer::new(FormatConfig::default()).print(&doc)
}

struct SourceDocBuilder<'a> {
    doc: &'a EureDocument,
    layout: &'a Layout,
}

impl<'a> SourceDocBuilder<'a> {
    fn new(doc: &'a EureDocument, layout: &'a Layout) -> Self {
        Self { doc, layout }
    }

    fn build(&self) -> Doc {
        Doc::concat_all(self.layout.items.iter().map(|item| self.build_item(item)))
    }

    fn build_item(&self, item: &LayoutItem) -> Doc {
        match item {
            LayoutItem::Comment(comment) => self.build_comment(comment),
            LayoutItem::BlankLine => Doc::hardline(),
            LayoutItem::Binding {
                path,
                node,
                trailing_comment,
            } => self.build_binding(path, *node, trailing_comment.as_deref()),
            LayoutItem::Section {
                path,
                trailing_comment,
                body,
            } => self.build_section(path, trailing_comment.as_deref(), body),
            LayoutItem::ArrayBinding {
                path,
                node,
                elements,
                trailing_comment,
            } => self.build_array_binding(path, *node, elements, trailing_comment.as_deref()),
        }
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

    fn build_binding(
        &self,
        path: &[SourcePathSegment],
        node: NodeId,
        trailing_comment: Option<&str>,
    ) -> Doc {
        let mut doc = self
            .build_path(path)
            .concat(Doc::text(" = "))
            .concat(self.build_value(node));

        if let Some(comment) = trailing_comment {
            doc = doc.concat(Doc::text(" //"));
            if !comment.is_empty() {
                doc = doc.concat(Doc::text(" ")).concat(Doc::text(comment));
            }
        }

        doc.concat(Doc::hardline())
    }

    fn build_section(
        &self,
        path: &[SourcePathSegment],
        trailing_comment: Option<&str>,
        body: &SectionBody,
    ) -> Doc {
        let mut header = Doc::text("@ ").concat(self.build_path(path));

        if let Some(comment) = trailing_comment {
            header = header.concat(Doc::text(" //"));
            if !comment.is_empty() {
                header = header.concat(Doc::text(" ")).concat(Doc::text(comment));
            }
        }

        match body {
            SectionBody::Items(items) => {
                let items_doc = Doc::concat_all(items.iter().map(|item| self.build_item(item)));
                header.concat(Doc::hardline()).concat(items_doc)
            }
            SectionBody::Block(items) => {
                let items_doc = Doc::concat_all(items.iter().map(|item| self.build_item(item)));
                header
                    .concat(Doc::text(" {"))
                    .concat(Doc::hardline())
                    .concat(Doc::indent(items_doc))
                    .concat(Doc::text("}"))
                    .concat(Doc::hardline())
            }
        }
    }

    fn build_array_binding(
        &self,
        path: &[SourcePathSegment],
        node: NodeId,
        elements: &[ArrayElementLayout],
        trailing_comment: Option<&str>,
    ) -> Doc {
        let mut doc = self
            .build_path(path)
            .concat(Doc::text(" = "))
            .concat(self.build_array_with_comments(node, elements));

        if let Some(comment) = trailing_comment {
            doc = doc.concat(Doc::text(" //"));
            if !comment.is_empty() {
                doc = doc.concat(Doc::text(" ")).concat(Doc::text(comment));
            }
        }

        doc.concat(Doc::hardline())
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
            SourceKey::String(s) => Doc::text("\"")
                .concat(Doc::text(escape_string(s)))
                .concat(Doc::text("\"")),
            SourceKey::Integer(n) => Doc::text(n.to_string()),
            SourceKey::Tuple(keys) => {
                let inner = Doc::join(keys.iter().map(|k| self.build_key(k)), Doc::text(", "));
                Doc::text("(").concat(inner).concat(Doc::text(")"))
            }
            SourceKey::TupleIndex(n) => Doc::text("#").concat(Doc::text(n.to_string())),
        }
    }

    fn build_value(&self, node_id: NodeId) -> Doc {
        let node = self.doc.node(node_id);
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

        // Check if this array should be formatted multi-line
        if self.layout.multiline_nodes.contains(&node_id) {
            // Force multiline format
            let elements = Doc::concat_all(arr.iter().map(|&id| {
                Doc::hardline()
                    .concat(self.build_value(id))
                    .concat(Doc::text(","))
            }));

            Doc::text("[")
                .concat(Doc::indent(elements))
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

    fn build_array_with_comments(&self, node_id: NodeId, elements: &[ArrayElementLayout]) -> Doc {
        let node = self.doc.node(node_id);
        if let NodeValue::Array(arr) = &node.content {
            if arr.is_empty() {
                return Doc::text("[]");
            }

            let mut items = Vec::new();

            for (i, &child_id) in arr.iter().enumerate() {
                // Add comments before this element
                if let Some(el) = elements.iter().find(|e| e.index == i) {
                    for comment in &el.comments_before {
                        // Build comment without the trailing hardline (we'll add separators later)
                        let comment_doc = match comment {
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
                        };
                        items.push(comment_doc);
                    }
                }

                // Add the element value with trailing comma
                let mut elem_doc = self.build_value(child_id).concat(Doc::text(","));

                // Add trailing comment if present
                if let Some(el) = elements.iter().find(|e| e.index == i)
                    && let Some(ref trailing) = el.trailing_comment
                {
                    elem_doc = elem_doc.concat(Doc::text(" //"));
                    if !trailing.is_empty() {
                        elem_doc = elem_doc
                            .concat(Doc::text(" "))
                            .concat(Doc::text(trailing.clone()));
                    }
                }

                items.push(elem_doc);
            }

            // Join items with hardline, wrap in indent, add brackets
            let content = Doc::join(items, Doc::hardline());
            Doc::text("[")
                .concat(Doc::indent(Doc::hardline().concat(content)))
                .concat(Doc::hardline())
                .concat(Doc::text("]"))
        } else {
            // Fallback: format as regular value
            self.build_value(node_id)
        }
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
