//! TOML conversion support for Eure format.
//!
//! This crate provides conversion from TOML documents to Eure's [`SourceDocument`],
//! preserving section ordering.
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

use eure_document::document::constructor::{DocumentConstructor, Scope};
use eure_document::identifier::Identifier;
use eure_document::path::{ArrayIndexKind, PathSegment};
use eure_document::source::{
    ArrayElementSource, BindSource, BindingSource, Comment, EureSource, SectionBody,
    SourceDocument, SourceKey, SourcePathSegment, Trivia,
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
        /// Trivia (comments/blank lines) before this section
        trivia_before: Vec<Trivia>,
        /// Path segments for this section
        path: Vec<SourcePathSegment>,
        /// Bindings collected for this section
        bindings: Vec<BindingSource>,
        /// Scope for the DocumentConstructor
        scope: Scope,
    },
    /// Inside a [[array_table]] section
    ArrayTable {
        /// Trivia (comments/blank lines) before this section
        trivia_before: Vec<Trivia>,
        /// Path segments for this section
        path: Vec<SourcePathSegment>,
        /// Bindings collected for this section
        bindings: Vec<BindingSource>,
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
        /// Per-element trivia collected during parsing
        element_sources: Vec<ArrayElementSource>,
        /// Pending trivia for the next element
        element_pending_trivia: Vec<Trivia>,
        /// Whether the array was multi-line in the original TOML
        is_multiline: bool,
        /// Span end of the last element (for trailing comment detection)
        last_element_span_end: Option<usize>,
    },
}

/// Main converter from TOML to SourceDocument
struct TomlParserConverter<'a> {
    /// The source TOML string
    source: Source<'a>,
    /// Document constructor for building EureDocument
    constructor: DocumentConstructor,
    /// Arena for EureSource blocks
    sources: Vec<EureSource>,

    /// Stack of parsing contexts
    context_stack: Vec<ValueContext>,

    /// Current key path being built (for dotted keys like `a.b.c`)
    current_keys: Vec<(String, Option<Encoding>)>,
    /// Whether we're currently parsing a key (before `=`)
    parsing_key: bool,

    /// Pending trivia to attach to the next item
    pending_trivia: Vec<Trivia>,
    /// Flag to track blank lines (consecutive newlines)
    saw_newline: bool,

    /// Array nodes that should be formatted multi-line
    multiline_arrays: std::collections::HashSet<eure_document::document::NodeId>,
}

impl<'a> TomlParserConverter<'a> {
    fn new(source: Source<'a>) -> Self {
        // Create root EureSource
        let sources = vec![EureSource::default()];
        Self {
            source,
            constructor: DocumentConstructor::new(),
            sources,
            context_stack: vec![ValueContext::Root],
            current_keys: Vec::new(),
            parsing_key: false,
            pending_trivia: Vec::new(),
            saw_newline: false,
            multiline_arrays: std::collections::HashSet::new(),
        }
    }

    fn finish(mut self) -> Result<SourceDocument, TomlToEureError> {
        // Close any remaining sections
        self.close_current_section();
        // Any remaining pending trivia becomes trailing trivia of the root source
        if !self.pending_trivia.is_empty() {
            self.sources[0].trailing_trivia = std::mem::take(&mut self.pending_trivia);
        }
        let mut source_doc = SourceDocument::new(self.constructor.finish(), self.sources);
        source_doc.multiline_arrays = self.multiline_arrays;
        Ok(source_doc)
    }

    fn current_context(&self) -> &ValueContext {
        self.context_stack.last().unwrap()
    }

    fn current_context_mut(&mut self) -> &mut ValueContext {
        self.context_stack.last_mut().unwrap()
    }

    /// Close the current section and add it to sources
    fn close_current_section(&mut self) {
        if let Some(context) = self.context_stack.pop() {
            match context {
                ValueContext::StdTable {
                    trivia_before,
                    path,
                    bindings,
                    scope,
                } => {
                    self.constructor.end_scope(scope).expect("scope mismatch");

                    // Add section to root source
                    self.sources[0]
                        .sections
                        .push(eure_document::source::SectionSource {
                            trivia_before,
                            path,
                            body: SectionBody::Items {
                                value: None,
                                bindings,
                            },
                            trailing_comment: None,
                        });
                }
                ValueContext::ArrayTable {
                    trivia_before,
                    path,
                    bindings,
                    scope,
                } => {
                    self.constructor.end_scope(scope).expect("scope mismatch");

                    // Add section to root source
                    self.sources[0]
                        .sections
                        .push(eure_document::source::SectionSource {
                            trivia_before,
                            path,
                            body: SectionBody::Items {
                                value: None,
                                bindings,
                            },
                            trailing_comment: None,
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
                SourceKey::quoted(key.to_string()),
                PathSegment::Value(ObjectKey::String(key.to_string())),
            ),
        }
    }

    /// Create a SourcePathSegment from a SourceKey
    fn source_path_segment(&self, key: SourceKey) -> SourcePathSegment {
        SourcePathSegment { key, array: None }
    }

    /// Check if there's a newline between two byte positions in the source
    fn has_newline_between(&self, start: usize, end: usize) -> bool {
        if let Some(raw) = self.source.get(Span::new_unchecked(start, end)) {
            raw.as_str().contains('\n')
        } else {
            // If we can't get the slice, assume there's a newline to be safe
            true
        }
    }

    /// Navigate to the key path and bind a value
    fn bind_value(&mut self, value: PrimitiveValue) {
        self.constructor
            .bind_primitive(value)
            .expect("binding should succeed");
    }

    /// Add a binding to the current context
    fn add_binding(&mut self, path: Vec<SourcePathSegment>, node: eure_document::document::NodeId) {
        // Don't consume trivia when in inline contexts (it should go to the outer binding)
        match self.current_context() {
            ValueContext::InlineTable { .. } | ValueContext::Array { .. } => {
                // Inline structures don't track bindings in source
                return;
            }
            _ => {}
        }

        // Attach pending trivia to this binding
        let trivia_before = std::mem::take(&mut self.pending_trivia);
        let binding = BindingSource {
            trivia_before,
            path,
            bind: BindSource::Value(node),
            trailing_comment: None,
        };

        match self.current_context_mut() {
            ValueContext::Root => {
                self.sources[0].bindings.push(binding);
            }
            ValueContext::StdTable { bindings, .. } | ValueContext::ArrayTable { bindings, .. } => {
                bindings.push(binding);
            }
            ValueContext::InlineTable { .. } | ValueContext::Array { .. } => {
                // Already handled above
            }
        }
    }

    /// Add an array binding with per-element trivia to the current context
    fn add_array_binding(
        &mut self,
        path: Vec<SourcePathSegment>,
        node: eure_document::document::NodeId,
        elements: Vec<ArrayElementSource>,
    ) {
        // Don't consume trivia when in inline contexts (it should go to the outer binding)
        match self.current_context() {
            ValueContext::InlineTable { .. } | ValueContext::Array { .. } => {
                // Inline structures don't track bindings in source
                return;
            }
            _ => {}
        }

        // Attach pending trivia to this binding
        let trivia_before = std::mem::take(&mut self.pending_trivia);
        let binding = BindingSource {
            trivia_before,
            path,
            bind: BindSource::Array { node, elements },
            trailing_comment: None,
        };

        match self.current_context_mut() {
            ValueContext::Root => {
                self.sources[0].bindings.push(binding);
            }
            ValueContext::StdTable { bindings, .. } | ValueContext::ArrayTable { bindings, .. } => {
                bindings.push(binding);
            }
            ValueContext::InlineTable { .. } | ValueContext::Array { .. } => {
                // Already handled above
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
                PrimitiveValue::Text(Text::new(value.to_string(), Language::Other(lang.into())))
            }
        }
    }
}

impl<'a> EventReceiver for TomlParserConverter<'a> {
    fn std_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Reset newline tracking when we see new content
        self.saw_newline = false;

        // Close previous section if any
        self.close_current_section();

        // Reset key state
        self.current_keys.clear();
        self.parsing_key = true;
    }

    fn std_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Capture pending trivia for this section
        let trivia_before = std::mem::take(&mut self.pending_trivia);

        // Collect keys first to avoid borrow issues
        let keys = std::mem::take(&mut self.current_keys);

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
                SourceKey::String(s, _) => PathSegment::Value(ObjectKey::String(s.clone())),
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

        self.context_stack.push(ValueContext::StdTable {
            trivia_before,
            path,
            bindings: Vec::new(),
            scope,
        });

        self.parsing_key = false;
    }

    fn array_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Reset newline tracking when we see new content
        self.saw_newline = false;

        // Close previous section if any
        self.close_current_section();

        // Reset key state
        self.current_keys.clear();
        self.parsing_key = true;
    }

    fn array_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Capture pending trivia for this section
        let trivia_before = std::mem::take(&mut self.pending_trivia);

        // Build the path from collected keys with array marker
        let keys = std::mem::take(&mut self.current_keys);
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
                    .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
                    .expect("array navigation should succeed");
            }
        }

        // Ensure current position is a map
        if self.constructor.current_node().content.is_hole() {
            self.constructor
                .bind_empty_map()
                .expect("binding should succeed");
        }

        self.context_stack.push(ValueContext::ArrayTable {
            trivia_before,
            path,
            bindings: Vec::new(),
            scope,
        });

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
        if let Some(ValueContext::Array {
            element_index,
            element_pending_trivia,
            element_sources,
            ..
        }) = self.context_stack.last_mut()
        {
            self.constructor
                .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
                .expect("array navigation should succeed");

            // Capture pending trivia for this element
            let trivia = std::mem::take(element_pending_trivia);
            let idx = *element_index;
            element_sources.push(ArrayElementSource {
                trivia_before: trivia,
                index: idx,
                trailing_comment: None,
            });

            *element_index += 1;
            // Reset newline tracking - element newline shouldn't count as blank line
            self.saw_newline = false;
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
        // Handle pending trivia for this element from parent array
        if let Some(ValueContext::Array {
            element_index,
            element_pending_trivia,
            element_sources,
            ..
        }) = self.context_stack.last_mut()
        {
            self.constructor
                .navigate(PathSegment::ArrayIndex(ArrayIndexKind::Push))
                .expect("array navigation should succeed");
            let trivia = std::mem::take(element_pending_trivia);
            let idx = *element_index;
            *element_index += 1;
            // Create element source for this nested array element
            element_sources.push(ArrayElementSource {
                trivia_before: trivia,
                index: idx,
                trailing_comment: None,
            });
            // Reset newline tracking - element newline shouldn't count as blank line
            self.saw_newline = false;
        }

        self.constructor
            .bind_empty_array()
            .expect("binding should succeed");
        self.context_stack.push(ValueContext::Array {
            scope,
            element_index: 0,
            binding_path,
            element_sources: Vec::new(),
            element_pending_trivia: Vec::new(),
            is_multiline: false,
            last_element_span_end: None,
        });
        self.current_keys.clear();
        true
    }

    fn array_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // Reset newline tracking - newline after ] shouldn't count as blank line
        self.saw_newline = false;

        if let Some(ValueContext::Array {
            scope,
