//! Source-level document representation for programmatic construction and formatting.
//!
//! This module provides types for representing Eure source structure with layout metadata,
//! while actual values are referenced via [`NodeId`] into an [`EureDocument`].
//!
//! # Design
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │              SourceDocument                  │
//! │  ┌─────────────────┐  ┌──────────────────┐  │
//! │  │  EureDocument   │  │     Layout       │  │
//! │  │  ┌───────────┐  │  │                  │  │
//! │  │  │ NodeId(0) │◄─┼──┼─ Binding.node    │  │
//! │  │  │ NodeId(1) │◄─┼──┼─ Binding.node    │  │
//! │  │  │ NodeId(2) │◄─┼──┼─ Binding.node    │  │
//! │  │  └───────────┘  │  │                  │  │
//! │  └─────────────────┘  └──────────────────┘  │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! - **EureDocument**: Holds semantic data (values)
//! - **Layout**: Holds presentation metadata (comments, ordering, section structure)
//!
//! # Example
//!
//! ```ignore
//! // Convert from TOML, preserving comments and section ordering
//! let source = eure_toml::to_source_document(&toml_doc);
//!
//! // Modify values (layout is preserved)
//! let node = source.find_binding(&["server", "port"]).unwrap();
//! source.document.node_mut(node).set_primitive(8080.into());
//!
//! // Format to Eure string
//! let output = eure_fmt::format_source(&source, &config);
//! ```

use std::collections::HashSet;

use crate::document::{EureDocument, NodeId};
use crate::prelude_internal::*;

/// A document with layout/presentation metadata.
///
/// Combines semantic data ([`EureDocument`]) with presentation information ([`Layout`])
/// for round-trip conversions from formats like TOML, preserving comments and ordering.
#[derive(Debug, Clone)]
pub struct SourceDocument {
    /// The semantic data (values, structure)
    pub document: EureDocument,
    /// The presentation layout (comments, ordering, sections)
    pub layout: Layout,
}

impl SourceDocument {
    /// Create a new source document with the given document and layout.
    pub fn new(document: EureDocument, layout: Layout) -> Self {
        Self { document, layout }
    }

    /// Create an empty source document.
    pub fn empty() -> Self {
        Self {
            document: EureDocument::new_empty(),
            layout: Layout::new(),
        }
    }
}

/// Layout information describing how to render the document.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// Top-level items in order
    pub items: Vec<LayoutItem>,
    /// Nodes that should be formatted with multiple lines
    pub multiline_nodes: HashSet<NodeId>,
}

impl Layout {
    /// Create an empty layout.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            multiline_nodes: HashSet::new(),
        }
    }

    /// Add an item to the layout.
    pub fn push(&mut self, item: LayoutItem) {
        self.items.push(item);
    }
}

/// An item in the layout.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutItem {
    /// A comment (line or block)
    Comment(Comment),

    /// A blank line for visual separation
    BlankLine,

    /// A key-value binding: `path.to.key = <value from NodeId>`
    Binding {
        /// Path to the binding target
        path: SourcePath,
        /// Reference to the value node in EureDocument
        node: NodeId,
        /// Optional trailing comment: `key = value // comment`
        trailing_comment: Option<String>,
    },

    /// A section header: `@ path.to.section`
    Section {
        /// Path to the section
        path: SourcePath,
        /// Optional trailing comment: `@ section // comment`
        trailing_comment: Option<String>,
        /// Section body
        body: SectionBody,
    },

    /// An array binding with per-element layout information.
    ///
    /// Used when an array has comments between elements that need to be preserved.
    /// ```eure
    /// items = [
    ///   // First item
    ///   "one",
    ///   // Second item
    ///   "two",
    /// ]
    /// ```
    ArrayBinding {
        /// Path to the binding target
        path: SourcePath,
        /// Reference to the array node in EureDocument
        node: NodeId,
        /// Per-element layout information (comments before each element)
        elements: Vec<ArrayElementLayout>,
        /// Optional trailing comment
        trailing_comment: Option<String>,
    },
}

/// The body of a section.
#[derive(Debug, Clone, PartialEq)]
pub enum SectionBody {
    /// Items following the section header (newline-separated)
    /// ```eure
    /// @ section
    /// key1 = value1
    /// key2 = value2
    /// ```
    Items(Vec<LayoutItem>),

    /// Block syntax with braces
    /// ```eure
    /// @ section {
    ///     key1 = value1
    ///     key2 = value2
    /// }
    /// ```
    Block(Vec<LayoutItem>),
}

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
}

/// A key in source representation.
///
/// This determines how the key should be rendered in the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKey {
    /// Bare identifier: `foo`, `bar_baz`
    Ident(Identifier),

    /// Extension namespace: `$variant`, `$eure`
    Extension(Identifier),

    /// Quoted string key: `"hello world"`
    String(String),

    /// Integer key: `123`
    Integer(i64),

    /// Tuple key: `(1, "a")`
    Tuple(Vec<SourceKey>),

    /// Tuple index: `#0`, `#1`
    TupleIndex(u8),
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

/// Layout information for an array element.
///
/// Used to preserve comments that appear before array elements when converting
/// from formats like TOML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayElementLayout {
    /// Comments that appear before this element in the source
    pub comments_before: Vec<Comment>,
    /// Trailing comment on the same line as this element
    pub trailing_comment: Option<String>,
    /// The index of this element in the array (corresponds to NodeArray)
    pub index: usize,
}

// ============================================================================
// Builder helpers
// ============================================================================

impl LayoutItem {
    /// Create a line comment item.
    pub fn line_comment(s: impl Into<String>) -> Self {
        LayoutItem::Comment(Comment::Line(s.into()))
    }

    /// Create a block comment item.
    pub fn block_comment(s: impl Into<String>) -> Self {
        LayoutItem::Comment(Comment::Block(s.into()))
    }

    /// Create a binding item.
    pub fn binding(path: SourcePath, node: NodeId) -> Self {
        LayoutItem::Binding {
            path,
            node,
            trailing_comment: None,
        }
    }

    /// Create a binding item with trailing comment.
    pub fn binding_with_comment(
        path: SourcePath,
        node: NodeId,
        comment: impl Into<String>,
    ) -> Self {
        LayoutItem::Binding {
            path,
            node,
            trailing_comment: Some(comment.into()),
        }
    }

    /// Create a section item with items body.
    pub fn section(path: SourcePath, items: Vec<LayoutItem>) -> Self {
        LayoutItem::Section {
            path,
            trailing_comment: None,
            body: SectionBody::Items(items),
        }
    }

    /// Create a section item with block body.
    pub fn section_block(path: SourcePath, items: Vec<LayoutItem>) -> Self {
        LayoutItem::Section {
            path,
            trailing_comment: None,
            body: SectionBody::Block(items),
        }
    }

    /// Create a section item with trailing comment.
    pub fn section_with_comment(
        path: SourcePath,
        comment: impl Into<String>,
        items: Vec<LayoutItem>,
    ) -> Self {
        LayoutItem::Section {
            path,
            trailing_comment: Some(comment.into()),
            body: SectionBody::Items(items),
        }
    }

    /// Create an array binding item with per-element layout.
    pub fn array_binding(
        path: SourcePath,
        node: NodeId,
        elements: Vec<ArrayElementLayout>,
    ) -> Self {
        LayoutItem::ArrayBinding {
            path,
            node,
            elements,
            trailing_comment: None,
        }
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
    fn test_layout_item_binding() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked("foo"))];
        let actual = LayoutItem::binding(path.clone(), NodeId(0));
        let expected = LayoutItem::Binding {
            path,
            node: NodeId(0),
            trailing_comment: None,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_layout_item_section_with_comment() {
        let path = vec![SourcePathSegment::ident(Identifier::new_unchecked(
            "config",
        ))];
        let actual = LayoutItem::section_with_comment(path.clone(), "this is config", vec![]);
        let expected = LayoutItem::Section {
            path,
            trailing_comment: Some("this is config".into()),
            body: SectionBody::Items(vec![]),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_source_document_empty() {
        let doc = SourceDocument::empty();
        assert!(doc.layout.items.is_empty());
        assert!(doc.layout.multiline_nodes.is_empty());
    }
}
