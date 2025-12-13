//! Document IR for the Eure formatter.
//!
//! This module defines the intermediate representation used by the formatter.
//! The design follows Wadler's "A Prettier Printer" algorithm.

/// Intermediate representation for formatting.
///
/// A `Doc` describes how content should be laid out, with the actual
/// line-breaking decisions deferred to the printer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doc {
    /// Empty document
    Nil,

    /// Literal text (no line breaks allowed within)
    Text(String),

    /// A line break: becomes a single space if the enclosing group fits on one line,
    /// otherwise becomes a newline followed by current indentation.
    Line,

    /// A soft line break: becomes empty if the enclosing group fits on one line,
    /// otherwise becomes a newline followed by current indentation.
    SoftLine,

    /// Always a line break, regardless of whether the group fits.
    HardLine,

    /// Increase indentation for the nested document.
    Indent(Box<Doc>),

    /// Concatenate two documents.
    Concat(Box<Doc>, Box<Doc>),

    /// Try to fit the document on one line. If it doesn't fit within the
    /// max width, break at `Line` and `SoftLine` positions.
    Group(Box<Doc>),

    /// Conditional content based on whether the enclosing group breaks.
    /// - `flat`: used when the group fits on one line
    /// - `broken`: used when the group breaks across lines
    IfBreak { flat: Box<Doc>, broken: Box<Doc> },
}

impl Doc {
    /// Create a text document.
    pub fn text(s: impl Into<String>) -> Doc {
        let s = s.into();
        if s.is_empty() { Doc::Nil } else { Doc::Text(s) }
    }

    /// Create a line break (space when flat, newline when broken).
    pub fn line() -> Doc {
        Doc::Line
    }

    /// Create a soft line break (empty when flat, newline when broken).
    pub fn softline() -> Doc {
        Doc::SoftLine
    }

    /// Create a hard line break (always a newline).
    pub fn hardline() -> Doc {
        Doc::HardLine
    }

    /// Indent the given document.
    pub fn indent(doc: Doc) -> Doc {
        if matches!(doc, Doc::Nil) {
            Doc::Nil
        } else {
            Doc::Indent(Box::new(doc))
        }
    }

    /// Create a group that tries to fit on one line.
    pub fn group(doc: Doc) -> Doc {
        if matches!(doc, Doc::Nil) {
            Doc::Nil
        } else {
            Doc::Group(Box::new(doc))
        }
    }

    /// Concatenate two documents.
    pub fn concat(self, other: Doc) -> Doc {
        match (self, other) {
            (Doc::Nil, other) => other,
            (this, Doc::Nil) => this,
            (this, other) => Doc::Concat(Box::new(this), Box::new(other)),
        }
    }

    /// Conditional content based on break state.
    pub fn if_break(flat: Doc, broken: Doc) -> Doc {
        Doc::IfBreak {
            flat: Box::new(flat),
            broken: Box::new(broken),
        }
    }

    /// Join multiple documents with a separator.
    pub fn join(docs: impl IntoIterator<Item = Doc>, sep: Doc) -> Doc {
        let mut result = Doc::Nil;
        let mut first = true;
        for doc in docs {
            if first {
                first = false;
                result = doc;
            } else {
                result = result.concat(sep.clone()).concat(doc);
            }
        }
        result
    }

    /// Concatenate multiple documents.
    pub fn concat_all(docs: impl IntoIterator<Item = Doc>) -> Doc {
        let mut result = Doc::Nil;
        for doc in docs {
            result = result.concat(doc);
        }
        result
    }

    /// Wrap content with prefix and suffix.
    pub fn surround(prefix: Doc, content: Doc, suffix: Doc) -> Doc {
        prefix.concat(content).concat(suffix)
    }
}

/// Builder for constructing documents fluently.
#[derive(Debug, Default)]
pub struct DocBuilder {
    parts: Vec<Doc>,
}

impl DocBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    /// Add text.
    pub fn text(&mut self, s: impl Into<String>) -> &mut Self {
        self.parts.push(Doc::text(s));
        self
    }

    /// Add a line break.
    pub fn line(&mut self) -> &mut Self {
        self.parts.push(Doc::line());
        self
    }

    /// Add a soft line break.
    pub fn softline(&mut self) -> &mut Self {
        self.parts.push(Doc::softline());
        self
    }

    /// Add a hard line break.
    pub fn hardline(&mut self) -> &mut Self {
        self.parts.push(Doc::hardline());
        self
    }

    /// Add a document.
    pub fn push(&mut self, doc: Doc) -> &mut Self {
        self.parts.push(doc);
        self
    }

    /// Build the final document.
    pub fn build(self) -> Doc {
        Doc::concat_all(self.parts)
    }

    /// Build as a group.
    pub fn build_group(self) -> Doc {
        Doc::group(self.build())
    }

    /// Build with indentation.
    pub fn build_indent(self) -> Doc {
        Doc::indent(self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_empty() {
        assert_eq!(Doc::text(""), Doc::Nil);
    }

    #[test]
    fn test_text_non_empty() {
        assert_eq!(Doc::text("hello"), Doc::Text("hello".to_string()));
    }

    #[test]
    fn test_concat_nil() {
        let doc = Doc::text("a").concat(Doc::Nil);
        assert_eq!(doc, Doc::Text("a".to_string()));

        let doc = Doc::Nil.concat(Doc::text("b"));
        assert_eq!(doc, Doc::Text("b".to_string()));
    }

    #[test]
    fn test_join() {
        let docs = vec![Doc::text("a"), Doc::text("b"), Doc::text("c")];
        let joined = Doc::join(docs, Doc::text(", "));
        // Should produce: a, b, c (as nested concats)
        assert!(matches!(joined, Doc::Concat(_, _)));
    }

    #[test]
    fn test_builder() {
        let doc = {
            let mut b = DocBuilder::new();
            b.text("hello").line().text("world");
            b.build()
        };
        assert!(matches!(doc, Doc::Concat(_, _)));
    }
}
