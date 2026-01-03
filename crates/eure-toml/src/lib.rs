//! TOML conversion support for Eure format.
//!
//! This crate provides conversion from TOML documents to Eure's [`SourceDocument`],
//! preserving comments and section ordering.
//!
//! # Example
//!
//! ```
//! use eure_toml::to_source_document;
//!
//! let toml_str = r#"
//! [server]
//! host = "localhost"
//! port = 8080
//! "#;
//!
//! let source_doc = to_source_document(toml_str).unwrap();
//! ```

mod error;
mod query;

pub use error::TomlToEureError;
pub use query::{TomlToEureDocument, TomlToEureSource};

use std::collections::HashSet;

use eure_document::document::NodeId;
use eure_document::document::constructor::{DocumentConstructor, Scope};
use eure_document::identifier::Identifier;
use eure_document::path::PathSegment;
use eure_document::source::{
    ArrayElementLayout, Comment, Layout, LayoutItem, SectionBody, SourceDocument, SourceKey,
    SourcePathSegment,
};
use eure_document::text::{Language, Text};
use eure_document::value::ObjectKey;
use eure_document::value::PrimitiveValue;
use num_bigint::BigInt;
use toml_parser::decoder::Encoding;
use toml_parser::decoder::ScalarKind;
use toml_parser::parser::EventReceiver;
use toml_parser::{ErrorSink, ParseError, Source, Span};

/// Convert a TOML string to a SourceDocument.
///
/// This preserves:
/// - Comments (converted from `#` to `//`)
/// - Section ordering (including interleaved `[table]` and `[[array]]` sections)
/// - All TOML values
pub fn to_source_document(toml_str: &str) -> Result<SourceDocument, TomlToEureError> {
    let source = Source::new(toml_str);
    let tokens: Vec<_> = source.lex().collect();

    let mut converter = TomlParserConverter::new(source);
    let mut errors = ErrorCollector::new();

    toml_parser::parser::parse_document(&tokens, &mut converter, &mut errors);

    if let Some(err) = errors.first_error() {
        return Err(err);
    }

    converter.finish()
}

/// Error collector for toml_parser
struct ErrorCollector {
    errors: Vec<TomlToEureError>,
}

impl ErrorCollector {
    fn new() -> Self {
        Self { errors: Vec::new() }
    }

    fn first_error(&self) -> Option<TomlToEureError> {
        self.errors.first().cloned()
    }
}

impl ErrorSink for ErrorCollector {
    fn report_error(&mut self, error: ParseError) {
        self.errors.push(TomlToEureError::ParseError {
            message: format!("{:?}", error),
        });
    }
}

/// State for tracking current parsing context
#[derive(Debug, Clone)]
enum ValueContext {
    /// At the root document level
    Root,
    /// Inside a [table] section
    StdTable {
        /// Path segments for this section
        path: Vec<SourcePathSegment>,
        /// Items collected for this section
        items: Vec<LayoutItem>,
        /// Scope for the DocumentConstructor
        scope: Scope,
    },
    /// Inside a [[array_table]] section
    ArrayTable {
        /// Path segments for this section
        path: Vec<SourcePathSegment>,
        /// Items collected for this section
        items: Vec<LayoutItem>,
        /// Scope for the DocumentConstructor
        scope: Scope,
    },
    /// Inside an inline table { }
    InlineTable {
        /// Scope for the DocumentConstructor
        scope: Scope,
        /// Binding path for this inline table (if it's a value)
        binding_path: Vec<SourcePathSegment>,
    },
    /// Inside an array [ ]
    Array {
        /// Scope for the DocumentConstructor
        scope: Scope,
        /// Current element index
        element_index: usize,
        /// Binding path for this array (if it's a value)
        binding_path: Vec<SourcePathSegment>,
        /// Collected element layout info (comments before each element)
        element_layouts: Vec<ArrayElementLayout>,
        /// Pending comments for the current element (not yet committed)
        pending_element_comments: Vec<Comment>,
        /// Pending trailing comment for the previous element (same line)
        pending_trailing_comment: Option<String>,
        /// Whether the array spans multiple lines in source
        is_multiline: bool,
    },
}

/// Main converter from TOML to SourceDocument
struct TomlParserConverter<'a> {
    /// The source TOML string
    source: Source<'a>,
    /// Document constructor for building EureDocument
    constructor: DocumentConstructor,
    /// Layout for preserving source structure
    layout: Layout,

    /// Stack of parsing contexts
    context_stack: Vec<ValueContext>,

    /// Current key path being built (for dotted keys like `a.b.c`)
    current_keys: Vec<(String, Option<Encoding>)>,
    /// Whether we're currently parsing a key (before `=`)
    parsing_key: bool,

    /// Pending comments to attach to next item
    pending_comments: Vec<Comment>,

    /// Count of consecutive newlines (for blank line detection)
    pending_newline_count: usize,

    /// End offset of last value (for inline comment detection)
    last_value_end: Option<usize>,

    /// Nodes that should be formatted multi-line
    multiline_nodes: HashSet<NodeId>,
}

impl<'a> TomlParserConverter<'a> {
    fn new(source: Source<'a>) -> Self {
        Self {
            source,
            constructor: DocumentConstructor::new(),
            layout: Layout::new(),
            context_stack: vec![ValueContext::Root],
            current_keys: Vec::new(),
            parsing_key: false,
            pending_comments: Vec::new(),
            pending_newline_count: 0,
            last_value_end: None,
            multiline_nodes: HashSet::new(),
        }
    }

    fn finish(mut self) -> Result<SourceDocument, TomlToEureError> {
        // Close any remaining sections
        self.close_current_section();
        // Transfer multiline node info to layout
        self.layout.multiline_nodes = self.multiline_nodes;
        Ok(SourceDocument::new(self.constructor.finish(), self.layout))
    }

    fn current_context(&self) -> &ValueContext {
        self.context_stack.last().unwrap()
    }

    fn current_context_mut(&mut self) -> &mut ValueContext {
        self.context_stack.last_mut().unwrap()
    }

    /// Flush pending blank lines to the current context.
    /// Two or more consecutive newlines = one blank line.
    fn flush_blank_lines(&mut self) {
        if self.pending_newline_count > 1 {
            match self.context_stack.last_mut() {
                Some(ValueContext::StdTable { items, .. })
                | Some(ValueContext::ArrayTable { items, .. }) => {
                    items.push(LayoutItem::BlankLine);
                }
                Some(ValueContext::Root) | None => {
                    self.layout.push(LayoutItem::BlankLine);
                }
                Some(ValueContext::InlineTable { .. }) | Some(ValueContext::Array { .. }) => {
                    // Inline structures don't track layout items
                }
            }
        }
        self.pending_newline_count = 0;
    }

    /// Close the current section and add it to layout
    fn close_current_section(&mut self) {
        if let Some(context) = self.context_stack.pop() {
            match context {
                ValueContext::StdTable { path, items, scope } => {
                    self.constructor.end_scope(scope).expect("scope mismatch");

                    // Add pending comments first
                    for comment in self.pending_comments.drain(..) {
                        self.layout.push(LayoutItem::Comment(comment));
                    }

                    self.layout.push(LayoutItem::Section {
                        path,
                        trailing_comment: None,
                        body: SectionBody::Items(items),
                    });
                }
                ValueContext::ArrayTable { path, items, scope } => {
                    self.constructor.end_scope(scope).expect("scope mismatch");

                    // Add pending comments first
                    for comment in self.pending_comments.drain(..) {
                        self.layout.push(LayoutItem::Comment(comment));
                    }

                    self.layout.push(LayoutItem::Section {
                        path,
                        trailing_comment: None,
                        body: SectionBody::Items(items),
                    });
                }
                ValueContext::Root => {
                    // Don't pop root, push it back
                    self.context_stack.push(ValueContext::Root);
                }
                _ => {}
            }
        }
    }

    /// Decode a key from span
    fn decode_key(&self, span: Span, encoding: Option<Encoding>) -> String {
        let raw = self.source.get(span).expect("valid span");
        let raw = toml_parser::Raw::new_unchecked(raw.as_str(), encoding, span);
        let mut output = String::new();
        let mut errors = ErrorCollector::new();
        raw.decode_key(&mut output, &mut errors);
        output
    }

    /// Decode a scalar value from span
    fn decode_scalar(&self, span: Span, encoding: Option<Encoding>) -> (ScalarKind, String) {
        let raw = self.source.get(span).expect("valid span");
        let raw = toml_parser::Raw::new_unchecked(raw.as_str(), encoding, span);
        let mut output = String::new();
        let mut errors = ErrorCollector::new();
        let kind = raw.decode_scalar(&mut output, &mut errors);
        (kind, output)
    }

    /// Parse a key string into SourceKey and PathSegment
    fn parse_key(&self, key: &str) -> (SourceKey, PathSegment) {
        match key.parse::<Identifier>() {
            Ok(id) => (SourceKey::Ident(id.clone()), PathSegment::Ident(id)),
            Err(_) => (
                SourceKey::String(key.to_string()),
                PathSegment::Value(ObjectKey::String(key.to_string())),
            ),
        }
    }

    /// Create a SourcePathSegment from a SourceKey
    fn source_path_segment(&self, key: SourceKey) -> SourcePathSegment {
        SourcePathSegment { key, array: None }
    }

    /// Navigate to the key path and bind a value
    fn bind_value(&mut self, value: PrimitiveValue) -> NodeId {
        let node_id = self.constructor.current_node_id();
        self.constructor
            .bind_primitive(value)
            .expect("binding should succeed");
        node_id
    }

    /// Add a binding to the current context
    fn add_binding(&mut self, path: Vec<SourcePathSegment>, node: NodeId) {
        // Flush blank lines before the binding
        self.flush_blank_lines();

        let binding = LayoutItem::Binding {
            path,
            node,
            trailing_comment: None,
        };

        // Drain pending comments first
        let comments: Vec<_> = self.pending_comments.drain(..).collect();

        match self.current_context_mut() {
            ValueContext::Root => {
                // Add pending comments first
                for comment in comments {
                    self.layout.push(LayoutItem::Comment(comment));
                }
                self.layout.push(binding);
            }
            ValueContext::StdTable { items, .. } | ValueContext::ArrayTable { items, .. } => {
                // Add pending comments first
                for comment in comments {
                    items.push(LayoutItem::Comment(comment));
                }
                items.push(binding);
            }
            ValueContext::InlineTable { .. } | ValueContext::Array { .. } => {
                // Inline structures don't track layout items
            }
        }
    }

    /// Add an array binding with per-element layout info to the current context
    fn add_array_binding(
        &mut self,
        path: Vec<SourcePathSegment>,
        node: NodeId,
        elements: Vec<ArrayElementLayout>,
    ) {
        // Flush blank lines before the binding
        self.flush_blank_lines();

        let binding = LayoutItem::ArrayBinding {
            path,
            node,
            elements,
            trailing_comment: None,
        };

        // Drain pending comments first
        let comments: Vec<_> = self.pending_comments.drain(..).collect();

        match self.current_context_mut() {
            ValueContext::Root => {
                // Add pending comments first
                for comment in comments {
                    self.layout.push(LayoutItem::Comment(comment));
                }
                self.layout.push(binding);
            }
            ValueContext::StdTable { items, .. } | ValueContext::ArrayTable { items, .. } => {
                // Add pending comments first
                for comment in comments {
                    items.push(LayoutItem::Comment(comment));
                }
                items.push(binding);
            }
            ValueContext::InlineTable { .. } | ValueContext::Array { .. } => {
                // Inline structures don't track layout items
            }
        }
    }

    /// Convert a scalar value to PrimitiveValue
    fn scalar_to_primitive(
        &self,
        kind: ScalarKind,
        value: &str,
        encoding: Option<Encoding>,
    ) -> PrimitiveValue {
        match kind {
            ScalarKind::String => {
                // Check if this is a multi-line string (TOML """ or ''')
                let is_multiline = matches!(
                    encoding,
                    Some(Encoding::MlBasicString) | Some(Encoding::MlLiteralString)
                );

                if is_multiline {
                    // Use block text for multi-line strings
                    // Determine appropriate block level based on content
                    use eure_document::text::SyntaxHint;

                    let mut content = value.to_string();
                    if !content.ends_with('\n') {
                        content.push('\n');
                    }

                    // Find the minimum block level needed
                    let syntax_hint = if content.contains("``````") {
                        // Content has 6 backticks, can't safely delimit
                        // Use Block6 and hope for the best
                        SyntaxHint::Block6
                    } else if content.contains("`````") {
                        SyntaxHint::Block6
                    } else if content.contains("````") {
                        SyntaxHint::Block5
                    } else if content.contains("```") {
                        SyntaxHint::Block4
                    } else {
                        SyntaxHint::Block3
                    };

                    PrimitiveValue::Text(Text {
                        content,
                        language: Language::Implicit,
                        syntax_hint: Some(syntax_hint),
                    })
                } else {
                    // Use plaintext for single-line strings
                    let text = Text::plaintext(value.to_string());
                    PrimitiveValue::Text(text)
                }
            }
            ScalarKind::Boolean(b) => PrimitiveValue::Bool(b),
            ScalarKind::Integer(_radix) => {
                // Parse the integer, handling underscores
                let clean: String = value.chars().filter(|c| *c != '_').collect();
                let parsed = if clean.starts_with("0x") || clean.starts_with("0X") {
                    i64::from_str_radix(&clean[2..], 16)
                } else if clean.starts_with("0o") || clean.starts_with("0O") {
                    i64::from_str_radix(&clean[2..], 8)
                } else if clean.starts_with("0b") || clean.starts_with("0B") {
                    i64::from_str_radix(&clean[2..], 2)
                } else {
                    clean.parse::<i64>()
                };
                match parsed {
                    Ok(n) => PrimitiveValue::Integer(BigInt::from(n)),
                    Err(_) => {
                        // i64 overflow: try parsing as BigInt for very large numbers
                        let n = clean.parse::<BigInt>().unwrap_or_else(|e| {
                            panic!("TOML parser validated integer '{clean}' failed to parse: {e}")
                        });
                        PrimitiveValue::Integer(n)
                    }
                }
            }
            ScalarKind::Float => {
                let clean: String = value.chars().filter(|c| *c != '_').collect();
                if clean == "inf" || clean == "+inf" {
                    PrimitiveValue::F64(f64::INFINITY)
                } else if clean == "-inf" {
                    PrimitiveValue::F64(f64::NEG_INFINITY)
                } else if clean == "nan" || clean == "+nan" || clean == "-nan" {
                    PrimitiveValue::F64(f64::NAN)
                } else {
                    let f = clean.parse::<f64>().unwrap_or_else(|e| {
                        panic!("TOML parser validated float '{clean}' failed to parse: {e}")
                    });
                    PrimitiveValue::F64(f)
                }
            }
            ScalarKind::DateTime => {
                // Determine the datetime type and create appropriate Text with language tag
                let lang = if value.contains('T') || value.contains(' ') {
                    // Has date and time component (datetime)
                    "datetime"
                } else if value.contains(':') {
                    // Time only
                    "time"
                } else {
                    // Date only
                    "date"
                };
                PrimitiveValue::Text(Text::new(
                    value.to_string(),
                    Language::Other(lang.to_string()),
                ))
            }
        }
    }
}

impl<'a> EventReceiver for TomlParserConverter<'a> {
    fn std_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Close previous section if any
        self.close_current_section();

        // Reset key state
        self.current_keys.clear();
        self.parsing_key = true;
    }

    fn std_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Collect keys first to avoid borrow issues
        let keys: Vec<_> = self.current_keys.drain(..).collect();

        // Build the path from collected keys
        let path: Vec<SourcePathSegment> = keys
            .iter()
            .map(|(key, _)| {
                let (source_key, _) = self.parse_key(key);
                self.source_path_segment(source_key)
            })
            .collect();

        // Navigate to this path in the document
        let scope = self.constructor.begin_scope();

        // Navigate for each segment
        for seg in &path {
            let path_seg = match &seg.key {
                SourceKey::Ident(id) => PathSegment::Ident(id.clone()),
                SourceKey::String(s) => PathSegment::Value(ObjectKey::String(s.clone())),
                _ => continue,
            };
            self.constructor
                .navigate(path_seg)
                .expect("navigation should succeed");
        }

        // Ensure it's a map
        if self.constructor.current_node().content.is_hole() {
            self.constructor
                .bind_empty_map()
                .expect("binding should succeed");
        }

        // Flush blank lines and comments before the section
        self.flush_blank_lines();
        for comment in self.pending_comments.drain(..) {
            self.layout.push(LayoutItem::Comment(comment));
        }

        self.context_stack.push(ValueContext::StdTable {
            path,
            items: Vec::new(),
            scope,
        });

        // Reset newline count for the new section
        self.pending_newline_count = 0;
        self.parsing_key = false;
    }

    fn array_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Close previous section if any
        self.close_current_section();

        // Reset key state
        self.current_keys.clear();
        self.parsing_key = true;
    }

    fn array_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Build the path from collected keys with array marker
        let keys: Vec<_> = self.current_keys.drain(..).collect();
        let mut path: Vec<SourcePathSegment> = Vec::new();

        for (i, (key, _)) in keys.iter().enumerate() {
            let (source_key, _) = self.parse_key(key);
            let mut seg = self.source_path_segment(source_key);
            // Add array marker to last segment
            if i == keys.len() - 1 {
                seg = seg.with_array_push();
            }
            path.push(seg);
        }

        // Navigate to this path in the document
        let scope = self.constructor.begin_scope();

        for (i, (key, _)) in keys.iter().enumerate() {
            let (_, path_seg) = self.parse_key(key);
            self.constructor
                .navigate(path_seg)
                .expect("navigation should succeed");

            if i == keys.len() - 1 {
                // Last key - ensure it's an array and push new element
                if self.constructor.current_node().content.is_hole() {
                    self.constructor
                        .bind_empty_array()
                        .expect("binding should succeed");
                }
                self.constructor
                    .navigate(PathSegment::ArrayIndex(None))
                    .expect("array navigation should succeed");
            }
        }

        // Ensure current position is a map
        if self.constructor.current_node().content.is_hole() {
            self.constructor
                .bind_empty_map()
                .expect("binding should succeed");
        }

        // Flush blank lines and comments before the section
        self.flush_blank_lines();
        for comment in self.pending_comments.drain(..) {
            self.layout.push(LayoutItem::Comment(comment));
        }

        self.context_stack.push(ValueContext::ArrayTable {
            path,
            items: Vec::new(),
            scope,
        });

        // Reset newline count for the new section
        self.pending_newline_count = 0;
        self.parsing_key = false;
    }

    fn inline_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) -> bool {
        let scope = self.constructor.begin_scope();

        // Build binding path before clearing keys
        let binding_path: Vec<SourcePathSegment> = self
            .current_keys
            .iter()
            .map(|(key, _)| {
                let (source_key, _) = self.parse_key(key);
                self.source_path_segment(source_key)
            })
            .collect();

        // Navigate to the key path first
        for (key, _) in &self.current_keys {
            let (_, path_seg) = self.parse_key(key);
            self.constructor
                .navigate(path_seg)
                .expect("navigation should succeed");
        }

        // Check if we're in an array context (values don't have keys)
        if let Some(ValueContext::Array { element_index, .. }) = self.context_stack.last_mut() {
            self.constructor
                .navigate(PathSegment::ArrayIndex(None))
                .expect("array navigation should succeed");
            *element_index += 1;
        }

        self.constructor
            .bind_empty_map()
            .expect("binding should succeed");
        self.context_stack.push(ValueContext::InlineTable {
            scope,
            binding_path,
        });
        self.current_keys.clear();
        true
    }

    fn inline_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        if let Some(ValueContext::InlineTable {
            scope,
            binding_path,
        }) = self.context_stack.pop()
        {
            let node_id = self.constructor.current_node_id();
            self.constructor.end_scope(scope).expect("scope mismatch");

            // Add binding if we have a path
            if !binding_path.is_empty() {
                self.add_binding(binding_path, node_id);
            }
        }
    }

    fn array_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) -> bool {
        let scope = self.constructor.begin_scope();

        // Build binding path before clearing keys
        let binding_path: Vec<SourcePathSegment> = self
            .current_keys
            .iter()
            .map(|(key, _)| {
                let (source_key, _) = self.parse_key(key);
                self.source_path_segment(source_key)
            })
            .collect();

        // Navigate to the key path first
        for (key, _) in &self.current_keys {
            let (_, path_seg) = self.parse_key(key);
            self.constructor
                .navigate(path_seg)
                .expect("navigation should succeed");
        }

        // Check if we're in an array context (nested arrays)
        if let Some(ValueContext::Array { element_index, .. }) = self.context_stack.last_mut() {
            self.constructor
                .navigate(PathSegment::ArrayIndex(None))
                .expect("array navigation should succeed");
            *element_index += 1;
        }

        self.constructor
            .bind_empty_array()
            .expect("binding should succeed");
        self.context_stack.push(ValueContext::Array {
            scope,
            element_index: 0,
            binding_path,
            element_layouts: Vec::new(),
            pending_element_comments: Vec::new(),
            pending_trailing_comment: None,
            is_multiline: false,
        });
        self.current_keys.clear();
        true
    }

    fn array_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        if let Some(ValueContext::Array {
            scope,
            binding_path,
            element_layouts,
            pending_element_comments,
            pending_trailing_comment,
            element_index,
            is_multiline,
        }) = self.context_stack.pop()
        {
            let node_id = self.constructor.current_node_id();
            self.constructor.end_scope(scope).expect("scope mismatch");

            // Commit any remaining pending comments (for last element without trailing comma)
            let mut final_layouts = element_layouts;

            // Handle trailing comment for the last element
            if pending_trailing_comment.is_some() || !pending_element_comments.is_empty() {
                // Check if last element already has a layout entry
                if let Some(last) = final_layouts.last_mut() {
                    if last.index == element_index.saturating_sub(1) {
                        // Update existing entry with trailing comment
                        if pending_trailing_comment.is_some() {
                            last.trailing_comment = pending_trailing_comment;
                        }
                        // Add any pending comments as before-comments for next (non-existent) element
                        if !pending_element_comments.is_empty() {
                            final_layouts.push(ArrayElementLayout {
                                comments_before: pending_element_comments,
                                trailing_comment: None,
                                index: element_index,
                            });
                        }
                    } else {
                        // Add new entry for last element
                        final_layouts.push(ArrayElementLayout {
                            comments_before: pending_element_comments,
                            trailing_comment: pending_trailing_comment,
                            index: element_index.saturating_sub(1),
                        });
                    }
                } else if element_index > 0 {
                    // No entries yet, create one for last element
                    final_layouts.push(ArrayElementLayout {
                        comments_before: pending_element_comments,
                        trailing_comment: pending_trailing_comment,
                        index: element_index - 1,
                    });
                }
            }

            // Track multiline nodes for formatting
            if is_multiline {
                self.multiline_nodes.insert(node_id);
            }

            // Add binding if we have a path
            if !binding_path.is_empty() {
                // Use ArrayBinding if has comments OR is multiline
                if final_layouts.is_empty() && !is_multiline {
                    // No element comments and single line - use regular binding
                    self.add_binding(binding_path, node_id);
                } else {
                    // Has element comments or multiline - use ArrayBinding
                    self.add_array_binding(binding_path, node_id, final_layouts);
                }
            }
        }
    }

    fn simple_key(&mut self, span: Span, kind: Option<Encoding>, _error: &mut dyn ErrorSink) {
        let key = self.decode_key(span, kind);
        self.current_keys.push((key, kind));
    }

    fn key_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Dot separator between keys - keys are already being collected
    }

    fn key_val_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // = separator - now we'll receive the value
        self.parsing_key = false;
    }

    fn scalar(&mut self, span: Span, kind: Option<Encoding>, _error: &mut dyn ErrorSink) {
        let (scalar_kind, value) = self.decode_scalar(span, kind);
        let primitive = self.scalar_to_primitive(scalar_kind, &value, kind);

        // Build path from current_keys
        let path: Vec<SourcePathSegment> = self
            .current_keys
            .iter()
            .map(|(key, _)| {
                let (source_key, _) = self.parse_key(key);
                self.source_path_segment(source_key)
            })
            .collect();

        // Navigate to the path
        let scope = self.constructor.begin_scope();
        for (key, _) in &self.current_keys {
            let (_, path_seg) = self.parse_key(key);
            self.constructor
                .navigate(path_seg)
                .expect("navigation should succeed");
        }

        // Check if we're in an array context
        if let Some(ValueContext::Array {
            element_index,
            element_layouts,
            pending_element_comments,
            pending_trailing_comment,
            ..
        }) = self.context_stack.last_mut()
        {
            // Commit any pending trailing comment and before-comments for the previous element
            let trailing = std::mem::take(pending_trailing_comment);
            if trailing.is_some() || !pending_element_comments.is_empty() {
                let comments = std::mem::take(pending_element_comments);
                // Trailing comment goes to previous element (element_index - 1 if > 0)
                // Before comments go to current element
                if *element_index > 0 && trailing.is_some() {
                    // Update or create entry for previous element
                    if let Some(last) = element_layouts.last_mut() {
                        if last.index == *element_index - 1 {
                            last.trailing_comment = trailing;
                        } else {
                            element_layouts.push(ArrayElementLayout {
                                comments_before: Vec::new(),
                                trailing_comment: trailing,
                                index: *element_index - 1,
                            });
                        }
                    } else {
                        element_layouts.push(ArrayElementLayout {
                            comments_before: Vec::new(),
                            trailing_comment: trailing,
                            index: *element_index - 1,
                        });
                    }
                }
                // Add before-comments for current element
                if !comments.is_empty() {
                    element_layouts.push(ArrayElementLayout {
                        comments_before: comments,
                        trailing_comment: None,
                        index: *element_index,
                    });
                }
            }

            // Navigate to array index
            self.constructor
                .navigate(PathSegment::ArrayIndex(None))
                .expect("array navigation should succeed");
            *element_index += 1;
        }

        let node_id = self.bind_value(primitive);
        self.constructor.end_scope(scope).expect("scope mismatch");

        // Track value end position for inline comment detection
        self.last_value_end = Some(span.end());

        // Only add binding if we have a path (not in array context without keys)
        if !path.is_empty() {
            self.add_binding(path, node_id);
            self.current_keys.clear();
        }
    }

    fn value_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Comma separator - clear keys for next item in inline table
        if matches!(self.current_context(), ValueContext::InlineTable { .. }) {
            self.current_keys.clear();
        }
        // Note: For arrays, element comments are committed in scalar() before element_index is incremented
    }

    fn comment(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        let raw = self.source.get(span).expect("valid span");
        let text = raw.as_str();
        // Strip the leading # and whitespace
        let content = text.trim_start_matches('#').trim();
        let comment = Comment::Line(content.to_string());

        // Flush blank lines before this comment (no-op for arrays/inline tables)
        self.flush_blank_lines();

        // Check if this is an inline comment (same line as previous value)
        let is_inline = if let Some(last_end) = self.last_value_end {
            let comment_start = span.start();
            if comment_start > last_end {
                // Check if there's a newline between last value and this comment
                let between = self
                    .source
                    .get(Span::new_unchecked(last_end, comment_start));
                if let Some(between_raw) = between {
                    !between_raw.as_str().contains('\n')
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        // Add comment to appropriate context
        match self.context_stack.last_mut() {
            // If we're inside an array context, track as element comment
            Some(ValueContext::Array {
                pending_element_comments,
                pending_trailing_comment,
                ..
            }) => {
                if is_inline {
                    // This is a trailing comment for the current element
                    *pending_trailing_comment = Some(content.to_string());
                } else {
                    // This is a before-comment for the next element
                    pending_element_comments.push(comment);
                }
            }
            // If we're inside a section (table) context, add to section items
            Some(ValueContext::StdTable { items, .. })
            | Some(ValueContext::ArrayTable { items, .. }) => {
                items.push(LayoutItem::Comment(comment));
            }
            // Otherwise (root level or inline table), add to pending_comments
            _ => {
                self.pending_comments.push(comment);
            }
        }

        // Clear last_value_end after processing comment (to avoid false positives)
        self.last_value_end = None;
    }

    fn whitespace(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Ignore whitespace
    }

    fn newline(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // If inside an array, just mark it as multiline (don't count for blank lines)
        if let Some(ValueContext::Array { is_multiline, .. }) = self.context_stack.last_mut() {
            *is_multiline = true;
        } else {
            // Only count newlines outside arrays for blank line detection
            self.pending_newline_count += 1;
        }
    }

    fn error(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Errors are collected by ErrorCollector
    }
}

// Re-export formatting functions from eure-fmt
pub use eure_fmt::{build_source_doc, format_source_document};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key_value() {
        let toml = r#"key = "value""#;
        let result = to_source_document(toml);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        assert_eq!(source.layout.items.len(), 1);
    }

    #[test]
    fn test_section() {
        let toml = r#"
[server]
host = "localhost"
port = 8080
"#;
        let result = to_source_document(toml);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // Should have one section
        assert_eq!(source.layout.items.len(), 1);
    }

    #[test]
    fn test_array_of_tables() {
        let toml = r#"
[[items]]
name = "first"

[[items]]
name = "second"
"#;
        let result = to_source_document(toml);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // Should have two sections (one for each [[items]]) plus blank line
        let section_count = source
            .layout
            .items
            .iter()
            .filter(|i| matches!(i, LayoutItem::Section { .. }))
            .count();
        assert_eq!(section_count, 2);
    }

    #[test]
    fn test_comment_preservation() {
        let toml = r#"
# This is a comment
key = "value"
"#;
        let result = to_source_document(toml);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // Should have comment + binding
        assert!(!source.layout.items.is_empty());
    }

    #[test]
    fn test_interleaved_sections() {
        // With toml_parser, we should preserve the source order!
        let toml = r#"
[[example]]
name = "first"

[metadata.first]
description = "First example"

[[example]]
name = "second"

[metadata.second]
description = "Second example"
"#;
        let result = to_source_document(toml);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // toml_parser preserves order: [[example]], [metadata.first], [[example]], [metadata.second]
        let section_count = source
            .layout
            .items
            .iter()
            .filter(|i| matches!(i, LayoutItem::Section { .. }))
            .count();
        assert_eq!(section_count, 4);
    }

    #[test]
    fn test_quoted_string_key() {
        // Keys that are not valid identifiers should be converted to quoted strings
        let toml = r#""invalid key with spaces" = "value""#;
        let result = to_source_document(toml);
        assert!(result.is_ok());

        // Verify the source document uses a quoted string key
        let source_doc = result.unwrap();
        let formatted = format_source_document(&source_doc);
        assert!(
            formatted.contains(r#""invalid key with spaces""#),
            "Expected quoted key in output: {}",
            formatted
        );
    }

    #[test]
    fn test_numeric_key() {
        // Keys starting with numbers should be converted to quoted strings
        let toml = r#"[features]
2d = ["value"]"#;
        let result = to_source_document(toml);
        assert!(result.is_ok());

        let source_doc = result.unwrap();
        let formatted = format_source_document(&source_doc);
        assert!(
            formatted.contains(r#""2d""#),
            "Expected quoted key in output: {}",
            formatted
        );
    }
}
