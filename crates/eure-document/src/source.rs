//! Source-level document representation for programmatic construction and formatting.
//!
//! This module provides types for representing Eure source structure as an AST,
//! while actual values are referenced via [`NodeId`] into an [`EureDocument`].
//!
//! The structure directly mirrors the Eure grammar from `eure.par`:
//!
//! ```text
//! Eure: [ ValueBinding ] { Binding } { Section } ;
//! Binding: Keys BindingRhs ;
//!   BindingRhs: ValueBinding | SectionBinding | TextBinding ;
//! Section: At Keys SectionBody ;
//!   SectionBody: [ ValueBinding ] { Binding } | Begin Eure End ;
//! ```
//!
//! # Design
//!
//! ```text
//! SourceDocument
//! ├── EureDocument (semantic data)
//! └── sources: Vec<EureSource> (arena)
//!     └── EureSource
//!         ├── leading_trivia: Vec<Trivia>
//!         ├── value: Option<NodeId>
//!         ├── bindings: Vec<BindingSource>
//!         │   └── trivia_before: Vec<Trivia>
//!         ├── sections: Vec<SectionSource>
//!         │   └── trivia_before: Vec<Trivia>
//!         └── trailing_trivia: Vec<Trivia>
//! ```
//!
//! Trivia (comments and blank lines) is preserved for round-trip formatting.

use std::collections::HashSet;

use crate::document::{EureDocument, NodeId};
use crate::prelude_internal::*;

// ============================================================================
// Core AST Types (mirrors grammar)
// ============================================================================

/// Index into the sources arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub usize);

/// A source-level Eure document/block.
///
/// Mirrors grammar: `Eure: [ ValueBinding ] { Binding } { Section } ;`
#[derive(Debug, Clone, Default)]
pub struct EureSource {
    /// Comments/blank lines before the first item (value, binding, or section)
    pub leading_trivia: Vec<Trivia>,
    /// Optional initial value binding: `[ ValueBinding ]`
    pub value: Option<NodeId>,
    /// Bindings in order: `{ Binding }`
    pub bindings: Vec<BindingSource>,
    /// Sections in order: `{ Section }`
    pub sections: Vec<SectionSource>,
    /// Comments/blank lines after the last item
    pub trailing_trivia: Vec<Trivia>,
}

/// A binding statement: path followed by value or block.
///
/// Mirrors grammar: `Binding: Keys BindingRhs ;`
#[derive(Debug, Clone)]
pub struct BindingSource {
    /// Comments/blank lines before this binding
    pub trivia_before: Vec<Trivia>,
    /// The path (Keys)
    pub path: SourcePath,
    /// The binding body (BindingRhs)
    pub bind: BindSource,
    /// Optional trailing comment (same line)
    pub trailing_comment: Option<Comment>,
}

/// The right-hand side of a binding.
///
/// Mirrors grammar: `BindingRhs: ValueBinding | SectionBinding | TextBinding ;`
#[derive(Debug, Clone)]
pub enum BindSource {
    /// Pattern #1: `path = value` (ValueBinding or TextBinding)
    Value(NodeId),
    /// Pattern #1b: `path = [array with element trivia]`
    ///
    /// Used when an array has comments between elements that need to be preserved.
    Array {
        /// Reference to the array node in EureDocument
        node: NodeId,
        /// Per-element layout information (comments before each element)
        elements: Vec<ArrayElementSource>,
    },
    /// Pattern #2/#3: `path { eure }` (SectionBinding -> nested EureSource)
    Block(SourceId),
}

/// Layout information for an array element.
///
/// Used to preserve comments that appear before array elements when converting
/// from formats like TOML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayElementSource {
    /// Trivia (comments/blank lines) before this element
    pub trivia_before: Vec<Trivia>,
    /// The index of this element in the NodeArray
    pub index: usize,
    /// Trailing comment on the same line as this element
    pub trailing_comment: Option<Comment>,
}

/// A section statement: `@ path` followed by body.
///
/// Mirrors grammar: `Section: At Keys SectionBody ;`
#[derive(Debug, Clone)]
pub struct SectionSource {
    /// Comments/blank lines before this section
    pub trivia_before: Vec<Trivia>,
    /// The path (Keys)
    pub path: SourcePath,
    /// The section body (SectionBody)
    pub body: SectionBody,
    /// Optional trailing comment (same line)
    pub trailing_comment: Option<Comment>,
}

/// The body of a section.
///
/// Mirrors grammar: `SectionBody: [ ValueBinding ] { Binding } | Begin Eure End ;`
#[derive(Debug, Clone)]
pub enum SectionBody {
    /// Pattern #4: `@ section` (items follow) - `[ ValueBinding ] { Binding }`
    Items {
        /// Optional initial value binding
        value: Option<NodeId>,
        /// Bindings in the section
        bindings: Vec<BindingSource>,
    },
    /// Pattern #5/#6: `@ section { eure }` - `Begin Eure End`
    Block(SourceId),
}

// ============================================================================
// Source Document
// ============================================================================

/// A document with source structure metadata.
///
/// Combines semantic data ([`EureDocument`]) with source AST information
/// for round-trip conversions, preserving the exact source structure.
#[derive(Debug, Clone)]
pub struct SourceDocument {
    /// The semantic data (values, structure)
    pub document: EureDocument,
    /// Arena of all EureSource blocks
    pub sources: Vec<EureSource>,
    /// Root source index (always 0)
    pub root: SourceId,
    /// Array nodes that should be formatted multi-line (even without trivia)
    pub multiline_arrays: HashSet<NodeId>,
}

impl SourceDocument {
    /// Create a new source document with the given document and sources.
    #[must_use]
    pub fn new(document: EureDocument, sources: Vec<EureSource>) -> Self {
        Self {
            document,
            sources,
            root: SourceId(0),
            multiline_arrays: HashSet::new(),
        }
    }

    /// Create an empty source document.
    pub fn empty() -> Self {
        Self {
            document: EureDocument::new_empty(),
            sources: vec![EureSource::default()],
            root: SourceId(0),
            multiline_arrays: HashSet::new(),
        }
    }

    /// Mark an array node as needing multi-line formatting.
    pub fn mark_multiline_array(&mut self, node_id: NodeId) {
        self.multiline_arrays.insert(node_id);
    }

    /// Check if an array node should be formatted multi-line.
    pub fn is_multiline_array(&self, node_id: NodeId) -> bool {
        self.multiline_arrays.contains(&node_id)
    }

    /// Get a reference to the document.
    pub fn document(&self) -> &EureDocument {
        &self.document
    }

    /// Get a mutable reference to the document.
    pub fn document_mut(&mut self) -> &mut EureDocument {
        &mut self.document
    }

    /// Get the root EureSource.
    pub fn root_source(&self) -> &EureSource {
        &self.sources[self.root.0]
    }

    /// Get a reference to an EureSource by ID.
    pub fn source(&self, id: SourceId) -> &EureSource {
        &self.sources[id.0]
    }

    /// Get a mutable reference to an EureSource by ID.
    pub fn source_mut(&mut self, id: SourceId) -> &mut EureSource {
        &mut self.sources[id.0]
    }
}

// ============================================================================
// Path Types
// ============================================================================

/// A path in source representation.
pub type SourcePath = Vec<SourcePathSegment>;

/// A segment in a source path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePathSegment {
    /// The key part of the segment
    pub key: SourceKey,
    /// Optional array marker:
    /// - `None` = no marker
    /// - `Some(None)` = `[]` (push to array)
    /// - `Some(Some(n))` = `[n]` (index into array)
    pub array: Option<Option<usize>>,
}

impl SourcePathSegment {
    /// Create a simple identifier segment without array marker.
    pub fn ident(name: Identifier) -> Self {
        Self {
            key: SourceKey::Ident(name),
            array: None,
        }
    }

    /// Create an extension segment without array marker.
    pub fn extension(name: Identifier) -> Self {
        Self {
            key: SourceKey::Extension(name),
            array: None,
        }
    }

    /// Create a segment with array push marker (`[]`).
    pub fn with_array_push(mut self) -> Self {
        self.array = Some(None);
        self
    }

    /// Create a segment with array index marker (`[n]`).
    pub fn with_array_index(mut self, index: usize) -> Self {
        self.array = Some(Some(index));
        self
    }

    /// Create a quoted string segment without array marker.
    pub fn quoted_string(s: impl Into<String>) -> Self {
        Self {
            key: SourceKey::quoted(s),
            array: None,
        }
    }

    /// Create a literal string segment (single-quoted) without array marker.
    pub fn literal_string(s: impl Into<String>) -> Self {
        Self {
            key: SourceKey::literal(s),
            array: None,
        }
    }
}

/// Syntax style for string keys (for round-trip formatting).
///
/// This preserves whether a string key was written with quotes, single quotes, or delimiters,
/// similar to how `SyntaxHint` preserves code block formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StringStyle {
    /// Quoted string: `"..."`
    #[default]
    Quoted,
    /// Literal string (single-quoted): `'...'`
    /// Content is taken literally, no escape processing
    Literal,
    /// Delimited literal string: `<'...'>`, `<<'...'>>`, `<<<'...'>>>`
    /// The u8 indicates the delimiter level (1, 2, or 3)
    /// Content is taken literally, no escape processing
    DelimitedLitStr(u8),
    /// Delimited code: `<`...`>`, `<<`...`>>`, `<<<`...`>>>`
    /// The u8 indicates the delimiter level (1, 2, or 3)
    DelimitedCode(u8),
}

/// A key in source representation.
///
/// This determines how the key should be rendered in the output.
#[derive(Debug, Clone)]
pub enum SourceKey {
    /// Bare identifier: `foo`, `bar_baz`
    Ident(Identifier),

    /// Extension namespace: `$variant`, `$eure`
    Extension(Identifier),

    /// String key with syntax style hint.
    /// - `StringStyle::Quoted`: `"hello world"`
    /// - `StringStyle::Literal`: `'hello world'`
    ///
    /// Note: `PartialEq` ignores the style - only content matters for equality.
    String(String, StringStyle),

    /// Integer key: `123`
    Integer(i64),

    /// Tuple key: `(1, "a")`
    Tuple(Vec<SourceKey>),

    /// Tuple index: `#0`, `#1`
    TupleIndex(u8),
}

impl PartialEq for SourceKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ident(a), Self::Ident(b)) => a == b,
            (Self::Extension(a), Self::Extension(b)) => a == b,
            // String equality ignores style hint - only content matters
            (Self::String(a, _), Self::String(b, _)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Tuple(a), Self::Tuple(b)) => a == b,
            (Self::TupleIndex(a), Self::TupleIndex(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for SourceKey {}

impl SourceKey {
    /// Create a quoted string key: `"..."`
    pub fn quoted(s: impl Into<String>) -> Self {
        SourceKey::String(s.into(), StringStyle::Quoted)
    }

    /// Create a literal string key (single-quoted): `'...'`
    pub fn literal(s: impl Into<String>) -> Self {
        SourceKey::String(s.into(), StringStyle::Literal)
    }

    /// Create a delimited literal string key: `<'...'>`, `<<'...'>>`, `<<<'...'>>>`
    pub fn delimited_lit_str(s: impl Into<String>, level: u8) -> Self {
        SourceKey::String(s.into(), StringStyle::DelimitedLitStr(level))
    }

    /// Create a delimited code key: `<`...`>`, `<<`...`>>`, `<<<`...`>>>`
    pub fn delimited_code(s: impl Into<String>, level: u8) -> Self {
        SourceKey::String(s.into(), StringStyle::DelimitedCode(level))
    }
}

impl From<Identifier> for SourceKey {
    fn from(id: Identifier) -> Self {
        SourceKey::Ident(id)
    }
}

impl From<i64> for SourceKey {
    fn from(n: i64) -> Self {
        SourceKey::Integer(n)
    }
}

// ============================================================================
// Comment and Trivia Types
// ============================================================================

/// A comment in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comment {
    /// Line comment: `// comment`
    Line(String),
    /// Block comment: `/* comment */`
    Block(String),
}

impl Comment {
    /// Create a line comment.
    pub fn line(s: impl Into<String>) -> Self {
        Comment::Line(s.into())
    }

    /// Create a block comment.
    pub fn block(s: impl Into<String>) -> Self {
        Comment::Block(s.into())
    }

    /// Get the comment text content.
    pub fn text(&self) -> &str {
        match self {
            Comment::Line(s) | Comment::Block(s) => s,
        }
    }
}

/// Trivia: comments and blank lines that appear between statements.
///
/// Trivia is used to preserve whitespace and comments for round-trip formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trivia {
    /// A comment (line or block)
    Comment(Comment),
    /// A blank line (empty line separating statements)
    BlankLine,
}

impl Trivia {
    /// Create a line comment trivia.
    pub fn line_comment(s: impl Into<String>) -> Self {
        Trivia::Comment(Comment::Line(s.into()))
    }

    /// Create a block comment trivia.
    pub fn block_comment(s: impl Into<String>) -> Self {
        Trivia::Comment(Comment::Block(s.into()))
    }

    /// Create a blank line trivia.
    pub fn blank_line() -> Self {
        Trivia::BlankLine
    }
}

impl From<Comment> for Trivia {
    fn from(comment: Comment) -> Self {
        Trivia::Comment(comment)
    }
}

// ============================================================================
// Builder Helpers
// ============================================================================

impl EureSource {
    /// Create an empty EureSource.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding to this source.
    pub fn push_binding(&mut self, binding: BindingSource) {
        self.bindings.push(binding);
    }

    /// Add a section to this source.
    pub fn push_section(&mut self, section: SectionSource) {
        self.sections.push(section);
    }
}

impl BindingSource {
    /// Create a value binding: `path = value`
    pub fn value(path: SourcePath, node: NodeId) -> Self {
        Self {
            trivia_before: Vec::new(),
            path,
            bind: BindSource::Value(node),
            trailing_comment: None,
        }
    }

    /// Create a block binding: `path { eure }`
    pub fn block(path: SourcePath, source_id: SourceId) -> Self {
        Self {
            trivia_before: Vec::new(),
            path,
            bind: BindSource::Block(source_id),
            trailing_comment: None,
        }
    }

    /// Add a trailing comment.
    pub fn with_trailing_comment(mut self, comment: Comment) -> Self {
        self.trailing_comment = Some(comment);
        self
    }

    /// Add trivia before this binding.
    pub fn with_trivia(mut self, trivia: Vec<Trivia>) -> Self {
        self.trivia_before = trivia;
        self
    }

    /// Create an array binding with per-element layout: `path = [...]`
    pub fn array(path: SourcePath, node: NodeId, elements: Vec<ArrayElementSource>) -> Self {
        Self {
            trivia_before: Vec::new(),
            path,
            bind: BindSource::Array { node, elements },
            trailing_comment: None,
        }
    }
}

impl SectionSource {
    /// Create a section with items body: `@ path` (items follow)
    pub fn items(path: SourcePath, value: Option<NodeId>, bindings: Vec<BindingSource>) -> Self {
        Self {
            trivia_before: Vec::new(),
            path,
            body: SectionBody::Items { value, bindings },
            trailing_comment: None,
        }
    }

    /// Create a section with block body: `@ path { eure }`
    pub fn block(path: SourcePath, source_id: SourceId) -> Self {
        Self {
            trivia_before: Vec::new(),
            path,
            body: SectionBody::Block(source_id),
            trailing_comment: None,
        }
    }

    /// Add a trailing comment.
    pub fn with_trailing_comment(mut self, comment: Comment) -> Self {
        self.trailing_comment = Some(comment);
        self
    }

    /// Add trivia before this section.
    pub fn with_trivia(mut self, trivia: Vec<Trivia>) -> Self {
        self.trivia_before = trivia;
        self
    }
}

impl ArrayElementSource {
    /// Create an array element source.
    pub fn new(index: usize) -> Self {
        Self {
            trivia_before: Vec::new(),
            index,
            trailing_comment: None,
        }
    }

    /// Add trivia before this element.
    pub fn with_trivia(mut self, trivia: Vec<Trivia>) -> Self {
        self.trivia_before = trivia;
        self
    }

    /// Add a trailing comment.
    pub fn with_trailing_comment(mut self, comment: Comment) -> Self {
        self.trailing_comment = Some(comment);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_path_segment_ident() {
        let actual = SourcePathSegment::ident(Identifier::new_unchecked("foo"));
        let expected = SourcePathSegment {
            key: SourceKey::Ident(Identifier::new_unchecked("foo")),
            array: None,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_source_path_segment_with_array_push() {
        let actual = SourcePathSegment::ident(Identifier::new_unchecked("items")).with_array_push();
        let expected = SourcePathSegment {
            key: SourceKey::Ident(Identifier::new_unchecked("items")),
            array: Some(None),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_source_path_segment_with_array_index() {
        let actual =
            SourcePathSegment::ident(Identifier::new_unchecked("items")).with_array_index(0);
        let expected = SourcePathSegment {
            key: SourceKey::Ident(Identifier::new_unchecked("items")),
            array: Some(Some(0)),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_binding_source_value() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked("foo"))];
        let binding = BindingSource::value(path.clone(), NodeId(1));
        assert_eq!(binding.path, path);
        assert!(matches!(binding.bind, BindSource::Value(NodeId(1))));
        assert!(binding.trivia_before.is_empty());
    }

    #[test]
    fn test_binding_source_block() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked("user"))];
        let binding = BindingSource::block(path.clone(), SourceId(1));
        assert_eq!(binding.path, path);
        assert!(matches!(binding.bind, BindSource::Block(SourceId(1))));
        assert!(binding.trivia_before.is_empty());
    }

    #[test]
    fn test_binding_with_trivia() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked("foo"))];
        let trivia = vec![Trivia::BlankLine, Trivia::line_comment("comment")];
        let binding = BindingSource::value(path.clone(), NodeId(1)).with_trivia(trivia.clone());
        assert_eq!(binding.trivia_before, trivia);
    }

    #[test]
    fn test_section_source_items() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked(
            "server",
        ))];
        let section = SectionSource::items(path.clone(), None, vec![]);
        assert_eq!(section.path, path);
        assert!(matches!(
            section.body,
            SectionBody::Items {
                value: None,
                bindings
            } if bindings.is_empty()
        ));
        assert!(section.trivia_before.is_empty());
    }

    #[test]
    fn test_section_source_block() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked(
            "config",
        ))];
        let section = SectionSource::block(path.clone(), SourceId(2));
        assert_eq!(section.path, path);
        assert!(matches!(section.body, SectionBody::Block(SourceId(2))));
        assert!(section.trivia_before.is_empty());
    }

    #[test]
    fn test_section_with_trivia() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked(
            "server",
        ))];
        let trivia = vec![Trivia::BlankLine];
        let section = SectionSource::items(path.clone(), None, vec![]).with_trivia(trivia.clone());
        assert_eq!(section.trivia_before, trivia);
    }

    #[test]
    fn test_source_document_empty() {
        let doc = SourceDocument::empty();
        assert_eq!(doc.sources.len(), 1);
        assert_eq!(doc.root, SourceId(0));
        assert!(doc.root_source().bindings.is_empty());
        assert!(doc.root_source().sections.is_empty());
    }
}
