//! Unified error reporting IR for Eure.
//!
//! This module provides a standardized `ErrorReport` structure that can represent
//! errors from all Eure crates (parser, document, schema, etc.) and be rendered
//! to multiple outputs (CLI via annotate-snippets, LSP Diagnostics).

use std::borrow::Cow;

use eure_document::document::NodeId;
use eure_document::path::EurePath;
use eure_parol::EureParseError;
use eure_parol::error::{ParseErrorEntry, ParseErrorKind};
use eure_schema::SchemaNodeId;
use eure_schema::convert::{ConversionError, SchemaSourceMap};
use eure_schema::validate::ValidationError;
use eure_tree::prelude::{Cst, CstNodeId};
use eure_tree::tree::InputSpan;
use thisisplural::Plural;

use crate::document::{DocumentConstructionError, OriginMap};

// ============================================================================
// File Registry
// ============================================================================

/// Opaque identifier for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

/// Information about a single source file.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Display path (may be virtual like "<stdin>" or "untitled:1").
    pub path: String,
    /// Source content.
    pub source: String,
}

/// Registry mapping FileIds to file information.
#[derive(Debug, Default)]
pub struct FileRegistry {
    files: Vec<FileInfo>,
}

impl FileRegistry {
    /// Create a new empty file registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a file and return its FileId.
    pub fn register(&mut self, path: impl Into<String>, source: impl Into<String>) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(FileInfo {
            path: path.into(),
            source: source.into(),
        });
        id
    }

    /// Get file information by FileId.
    pub fn get(&self, id: FileId) -> Option<&FileInfo> {
        self.files.get(id.0 as usize)
    }
}

// ============================================================================
// Origin
// ============================================================================

/// A location in source code with semantic context hints.
#[derive(Debug, Clone)]
pub struct Origin {
    /// The file containing this location.
    pub file: FileId,
    /// Byte range within the file.
    pub span: InputSpan,
    /// Optional semantic hints for this location.
    pub hints: OriginHints,
    /// True if this span couldn't be resolved and falls back to a default.
    pub is_fallback: bool,
}

impl Origin {
    /// Create an origin with just file and span (no hints).
    pub fn new(file: FileId, span: InputSpan) -> Self {
        Self {
            file,
            span,
            hints: OriginHints::default(),
            is_fallback: false,
        }
    }

    /// Create an origin with full hints.
    pub fn with_hints(file: FileId, span: InputSpan, hints: OriginHints) -> Self {
        Self {
            file,
            span,
            hints,
            is_fallback: false,
        }
    }

    /// Mark this origin as a fallback (span couldn't be resolved).
    #[must_use]
    pub fn as_fallback(mut self) -> Self {
        self.is_fallback = true;
        self
    }
}

/// Semantic context hints that aid in navigation and understanding.
#[derive(Debug, Clone, Default)]
pub struct OriginHints {
    /// Logical path into the document (e.g., ".user.name").
    pub path: Option<EurePath>,
    /// CST node ID (for IDE features like syntax-aware selection).
    pub cst: Option<CstNodeId>,
    /// Document node ID (for value-level navigation).
    pub doc: Option<NodeId>,
    /// Schema node ID (for type-level information).
    pub schema: Option<SchemaNodeId>,
}

impl OriginHints {
    /// Set the document path hint.
    pub fn with_path(mut self, path: EurePath) -> Self {
        self.path = Some(path);
        self
    }

    /// Set the CST node hint.
    pub fn with_cst(mut self, cst: CstNodeId) -> Self {
        self.cst = Some(cst);
        self
    }

    /// Set the document node hint.
    pub fn with_doc(mut self, doc: NodeId) -> Self {
        self.doc = Some(doc);
        self
    }

    /// Set the schema node hint.
    pub fn with_schema(mut self, schema: SchemaNodeId) -> Self {
        self.schema = Some(schema);
        self
    }
}

// ============================================================================
// Severity and AnnotationKind
// ============================================================================

/// Severity level of an error report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Hard error that prevents further processing.
    Error,
    /// Warning that may indicate a problem.
    Warning,
    /// Informational note.
    Note,
    /// Hint for improvement (lowest severity).
    Hint,
}

/// Kind of annotation marker on a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationKind {
    /// Primary annotation - the main location of the error.
    Primary,
    /// Secondary annotation - related context location.
    Secondary,
    /// Help annotation - suggested fix location.
    Help,
}

// ============================================================================
// ErrorReport
// ============================================================================

/// A complete error report with location and structured content.
#[derive(Debug, Clone)]
pub struct ErrorReport {
    /// Short title/summary of the error (e.g., "type mismatch").
    pub title: Cow<'static, str>,
    /// Severity level.
    pub severity: Severity,
    /// Primary origin where the error occurred.
    pub primary_origin: Origin,
    /// Optional error code (e.g., "E0001", "schema:type-mismatch").
    pub code: Option<Cow<'static, str>>,
    /// Structured content elements.
    pub elements: Vec<Element>,
}

impl ErrorReport {
    /// Create a new error report with Error severity.
    pub fn error(title: impl Into<Cow<'static, str>>, origin: Origin) -> Self {
        Self {
            title: title.into(),
            severity: Severity::Error,
            primary_origin: origin,
            code: None,
            elements: Vec::new(),
        }
    }

    /// Create a new error report with Warning severity.
    pub fn warning(title: impl Into<Cow<'static, str>>, origin: Origin) -> Self {
        Self {
            title: title.into(),
            severity: Severity::Warning,
            primary_origin: origin,
            code: None,
            elements: Vec::new(),
        }
    }

    /// Create a new error report with Note severity.
    pub fn note(title: impl Into<Cow<'static, str>>, origin: Origin) -> Self {
        Self {
            title: title.into(),
            severity: Severity::Note,
            primary_origin: origin,
            code: None,
            elements: Vec::new(),
        }
    }

    /// Create a new error report with Hint severity.
    pub fn hint(title: impl Into<Cow<'static, str>>, origin: Origin) -> Self {
        Self {
            title: title.into(),
            severity: Severity::Hint,
            primary_origin: origin,
            code: None,
            elements: Vec::new(),
        }
    }

    /// Set the error code.
    #[must_use]
    pub fn with_code(mut self, code: impl Into<Cow<'static, str>>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Add an element to the report.
    #[must_use]
    pub fn with_element(mut self, element: Element) -> Self {
        self.elements.push(element);
        self
    }

    /// Add an annotation element.
    #[must_use]
    pub fn with_annotation(
        self,
        origin: Origin,
        kind: AnnotationKind,
        label: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.with_element(Element::Annotation {
            origin,
            kind,
            label: label.into(),
        })
    }

    /// Add a note element.
    #[must_use]
    pub fn with_note(self, text: impl Into<Cow<'static, str>>) -> Self {
        self.with_element(Element::Note(text.into()))
    }

    /// Add a help element.
    #[must_use]
    pub fn with_help(self, text: impl Into<Cow<'static, str>>) -> Self {
        self.with_element(Element::Help(text.into()))
    }
}

// ============================================================================
// ErrorReports
// ============================================================================

/// A collection of error reports.
#[derive(Debug, Clone, Plural)]
pub struct ErrorReports(Vec<ErrorReport>);

impl ErrorReports {
    /// Sort reports by their primary origin span start position.
    pub fn sort_by_span(&mut self) {
        self.0.sort_by(|a, b| {
            a.primary_origin
                .span
                .start
                .cmp(&b.primary_origin.span.start)
        });
    }
}

// ============================================================================
// Element
// ============================================================================

/// An element within an error report.
#[derive(Debug, Clone)]
pub enum Element {
    /// A span annotation with a label.
    Annotation {
        origin: Origin,
        kind: AnnotationKind,
        label: Cow<'static, str>,
    },

    /// Plain text note (rendered as "note: ...").
    Note(Cow<'static, str>),

    /// Help suggestion (rendered as "help: ...").
    Help(Cow<'static, str>),

    /// A labelled sub-element (e.g., "caused by: ...").
    Labelled {
        label: Cow<'static, str>,
        element: Box<Element>,
    },

    /// Nested group of related elements (e.g., union variant errors).
    Nested {
        title: Cow<'static, str>,
        children: Vec<Element>,
    },

    /// A suggested fix with replacement text.
    Suggestion {
        origin: Origin,
        message: Cow<'static, str>,
        replacement: Cow<'static, str>,
    },
}

// ============================================================================
// Conversion Contexts
// ============================================================================

/// Context for converting document-related errors to ErrorReports.
pub struct DocumentReportContext<'a> {
    /// File ID for the document.
    pub file: FileId,
    /// CST for span resolution.
    pub cst: &'a Cst,
    /// Origin map with precise key origins.
    pub origins: &'a OriginMap,
}

/// Context for converting schema validation errors to ErrorReports.
/// References both document and schema sources.
pub struct SchemaReportContext<'a> {
    /// Document context.
    pub doc: DocumentReportContext<'a>,
    /// Schema file ID.
    pub schema_file: FileId,
    /// Schema CST.
    pub schema_cst: &'a Cst,
    /// Schema OriginMap.
    pub schema_origins: &'a OriginMap,
    /// SchemaNodeId -> NodeId mapping.
    pub schema_source_map: &'a SchemaSourceMap,
}

// ============================================================================
// Conversion Functions
// ============================================================================

/// Convert a parse error to ErrorReports.
pub fn report_parse_error(error: &EureParseError, file: FileId) -> ErrorReports {
    error
        .entries
        .iter()
        .map(|entry| report_parse_entry(entry, file))
        .collect()
}

fn report_parse_entry(entry: &ParseErrorEntry, file: FileId) -> ErrorReport {
    let span = entry.span.unwrap_or(InputSpan::EMPTY);
    let origin = Origin::new(file, span);

    let mut report = ErrorReport::error(entry.message.clone(), origin);

    // Add expected tokens as help
    if let ParseErrorKind::SyntaxError {
        expected_tokens, ..
    } = &entry.kind
        && !expected_tokens.is_empty()
    {
        let expected = expected_tokens.join(", ");
        report = report.with_help(format!("expected one of: {}", expected));
    }

    // Recursively add source errors as nested elements
    for source in &entry.source {
        let source_span = source.span.unwrap_or(InputSpan::EMPTY);
        report = report.with_element(Element::Labelled {
            label: "caused by".into(),
            element: Box::new(Element::Annotation {
                origin: Origin::new(file, source_span),
                kind: AnnotationKind::Secondary,
                label: source.message.clone().into(),
            }),
        });
    }

    report
}

/// Convert a document construction error to an ErrorReport.
pub fn report_document_error(
    error: &DocumentConstructionError,
    ctx: &DocumentReportContext<'_>,
) -> ErrorReport {
    report_document_error_simple(error, ctx.file, ctx.cst)
}

/// Convert a document construction error to an ErrorReport (simplified API).
pub fn report_document_error_simple(
    error: &DocumentConstructionError,
    file: FileId,
    cst: &Cst,
) -> ErrorReport {
    let span = error.span(cst).unwrap_or(InputSpan::EMPTY);
    let origin = Origin::new(file, span);

    ErrorReport::error(error.to_string(), origin)
}

/// Convert schema validation errors to ErrorReports.
pub fn report_schema_validation_errors(
    errors: &[ValidationError],
    ctx: &SchemaReportContext<'_>,
) -> ErrorReports {
    errors
        .iter()
        .map(|error| report_validation_error(error, ctx))
        .collect()
}

fn report_validation_error(error: &ValidationError, ctx: &SchemaReportContext<'_>) -> ErrorReport {
    let (doc_node_id, schema_node_id) = error.node_ids();

    // For InvalidKeyType, try to get precise key span
    let resolved_span = if let ValidationError::InvalidKeyType { key, node_id, .. } = error {
        // Try to get precise key span first
        ctx.doc
            .origins
            .get_key_span(*node_id, key, ctx.doc.cst)
            .or_else(|| ctx.doc.origins.get_node_span(doc_node_id, ctx.doc.cst))
    } else {
        // Standard node span resolution
        ctx.doc.origins.get_node_span(doc_node_id, ctx.doc.cst)
    };

    let (doc_span, is_fallback) = match resolved_span {
        Some(span) => (span, false),
        None => (InputSpan::EMPTY, true),
    };

    let doc_origin = Origin::with_hints(
        ctx.doc.file,
        doc_span,
        OriginHints::default().with_doc(doc_node_id),
    );
    let doc_origin = if is_fallback {
        doc_origin.as_fallback()
    } else {
        doc_origin
    };

    let mut report = ErrorReport::error(error.to_string(), doc_origin);

    // Add schema location as secondary annotation
    if let Some(schema_span) = resolve_schema_span(schema_node_id, ctx) {
        let schema_origin = Origin::with_hints(
            ctx.schema_file,
            schema_span,
            OriginHints::default().with_schema(schema_node_id),
        );

        report = report.with_element(Element::Annotation {
            origin: schema_origin,
            kind: AnnotationKind::Secondary,
            label: "constraint defined here".into(),
        });
    }

    // Handle nested errors (e.g., NoVariantMatched)
    if let ValidationError::NoVariantMatched { variant_errors, .. } = error {
        for (variant_name, variant_error) in variant_errors {
            let nested = report_validation_error(variant_error, ctx);
            report = report.with_element(Element::Nested {
                title: format!("variant '{}' did not match", variant_name).into(),
                children: nested.elements,
            });
        }
    }

    report
}

fn resolve_schema_span(
    schema_id: SchemaNodeId,
    ctx: &SchemaReportContext<'_>,
) -> Option<InputSpan> {
    // SchemaNodeId -> NodeId (from schema source map)
    let doc_node_id = ctx.schema_source_map.get(&schema_id)?;
    // Use OriginMap for span resolution
    ctx.schema_origins
        .get_node_span(*doc_node_id, ctx.schema_cst)
}

/// Convert a schema conversion error to an ErrorReport.
pub fn report_conversion_error(
    error: &ConversionError,
    ctx: &DocumentReportContext<'_>,
) -> ErrorReport {
    match error {
        ConversionError::ParseError(parse_error) => {
            // ParseError contains a NodeId, resolve it
            let span = ctx
                .origins
                .get_node_span(parse_error.node_id, ctx.cst)
                .unwrap_or(InputSpan::EMPTY);

            let origin = Origin::with_hints(
                ctx.file,
                span,
                OriginHints::default().with_doc(parse_error.node_id),
            );

            ErrorReport::error(error.to_string(), origin)
        }
        _ => {
            // Other conversion errors may not have precise spans
            let origin = Origin::new(ctx.file, InputSpan::EMPTY);
            ErrorReport::error(error.to_string(), origin)
        }
    }
}

// ============================================================================
// Rendering Functions
// ============================================================================

use annotate_snippets::{AnnotationKind as SnippetAnnotation, Group, Level, Renderer, Snippet};

/// Render an ErrorReport to a string using annotate-snippets.
pub fn format_error_report(report: &ErrorReport, files: &FileRegistry, styled: bool) -> String {
    let groups = build_snippet_groups(report, files);

    let renderer = if styled {
        Renderer::styled()
    } else {
        Renderer::plain()
    };

    renderer.render(&groups).to_string()
}

/// Render multiple ErrorReports to a string.
pub fn format_error_reports(reports: &ErrorReports, files: &FileRegistry, styled: bool) -> String {
    reports
        .iter()
        .map(|r| format_error_report(r, files, styled))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_snippet_groups<'a>(report: &'a ErrorReport, files: &'a FileRegistry) -> Vec<Group<'a>> {
    let mut groups = Vec::new();

    // Get primary file info
    let file_info = files.get(report.primary_origin.file);

    // Build primary group
    let level = match report.severity {
        Severity::Error => Level::ERROR,
        Severity::Warning => Level::WARNING,
        Severity::Note => Level::NOTE,
        Severity::Hint => Level::HELP,
    };

    if let Some(file_info) = file_info {
        let span = &report.primary_origin.span;

        // Clamp span to valid range
        let span_start = (span.start as usize).min(file_info.source.len());
        let span_end = (span.end as usize)
            .min(file_info.source.len())
            .max(span_start);

        let primary_snippet = Snippet::source(&file_info.source)
            .line_start(1)
            .path(&file_info.path)
            .annotation(
                SnippetAnnotation::Primary
                    .span(span_start..span_end)
                    .label(report.title.as_ref()),
            );

        let primary_group = level
            .primary_title(report.title.as_ref())
            .element(primary_snippet);
        groups.push(primary_group);
    } else {
        // No file info - create a minimal group with just the title
        // We need a snippet with annotation to create a Group
        let primary_group = level.primary_title(report.title.as_ref()).element(
            Snippet::source("")
                .line_start(1)
                .annotation(SnippetAnnotation::Primary.span(0..0)),
        );
        groups.push(primary_group);
    }

    // Process elements
    for element in &report.elements {
        add_element_to_groups(element, files, &mut groups, file_info);
    }

    groups
}

fn add_element_to_groups<'a>(
    element: &'a Element,
    files: &'a FileRegistry,
    groups: &mut Vec<Group<'a>>,
    primary_file: Option<&'a FileInfo>,
) {
    match element {
        Element::Annotation {
            origin,
            kind,
            label,
        } => {
            if let Some(file_info) = files.get(origin.file) {
                let snippet_kind = match kind {
                    AnnotationKind::Primary => SnippetAnnotation::Primary,
                    AnnotationKind::Secondary | AnnotationKind::Help => SnippetAnnotation::Context,
                };

                let span = &origin.span;
                let span_start = (span.start as usize).min(file_info.source.len());
                let span_end = (span.end as usize)
                    .min(file_info.source.len())
                    .max(span_start);

                let group = Level::NOTE.primary_title(label.as_ref()).element(
                    Snippet::source(&file_info.source)
                        .line_start(1)
                        .path(&file_info.path)
                        .annotation(
                            snippet_kind
                                .span(span_start..span_end)
                                .label(label.as_ref()),
                        ),
                );
                groups.push(group);
            }
        }

        Element::Note(text) => {
            // For text-only notes, create a minimal group with the primary file as context
            if let Some(file_info) = primary_file {
                let group = Level::NOTE.primary_title(text.as_ref()).element(
                    Snippet::source(&file_info.source)
                        .line_start(1)
                        .path(&file_info.path)
                        .annotation(SnippetAnnotation::Context.span(0..0)),
                );
                groups.push(group);
            }
        }

        Element::Help(text) => {
            // For text-only help, create a minimal group with the primary file as context
            if let Some(file_info) = primary_file {
                let group = Level::HELP.primary_title(text.as_ref()).element(
                    Snippet::source(&file_info.source)
                        .line_start(1)
                        .path(&file_info.path)
                        .annotation(SnippetAnnotation::Context.span(0..0)),
                );
                groups.push(group);
            }
        }

        Element::Labelled { label, element } => {
            // Add the label as a NOTE, then recurse into the inner element
            if let Some(file_info) = primary_file {
                let group = Level::NOTE.primary_title(label.as_ref()).element(
                    Snippet::source(&file_info.source)
                        .line_start(1)
                        .path(&file_info.path)
                        .annotation(SnippetAnnotation::Context.span(0..0)),
                );
                groups.push(group);
            }
            add_element_to_groups(element, files, groups, primary_file);
        }

        Element::Nested { title, children } => {
            // Add title as a NOTE group before children
            if let Some(file_info) = primary_file {
                let group = Level::NOTE.primary_title(title.as_ref()).element(
                    Snippet::source(&file_info.source)
                        .line_start(1)
                        .path(&file_info.path)
                        .annotation(SnippetAnnotation::Context.span(0..0)),
                );
                groups.push(group);
            }
            // Recurse into children
            for child in children {
                add_element_to_groups(child, files, groups, primary_file);
            }
        }

        Element::Suggestion {
            origin,
            message,
            replacement: _,
        } => {
            // Suggestions are rendered as help with the origin highlighted
            if let Some(file_info) = files.get(origin.file) {
                let span = &origin.span;
                let span_start = (span.start as usize).min(file_info.source.len());
                let span_end = (span.end as usize)
                    .min(file_info.source.len())
                    .max(span_start);

                let group = Level::HELP.primary_title(message.as_ref()).element(
                    Snippet::source(&file_info.source)
                        .line_start(1)
                        .path(&file_info.path)
                        .annotation(
                            SnippetAnnotation::Context
                                .span(span_start..span_end)
                                .label(message.as_ref()),
                        ),
                );
                groups.push(group);
            }
        }
    }
}
