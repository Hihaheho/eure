//! Unified error reporting IR for Eure.
//!
//! This module provides a standardized `ErrorReport` structure that can represent
//! errors from all Eure crates (parser, document, schema, etc.) and be rendered
//! to multiple outputs (CLI via annotate-snippets, LSP Diagnostics).

use std::borrow::Cow;

use annotate_snippets::renderer::DecorStyle as AnnotateDecorStyle;
use eure_document::document::NodeId;
use eure_document::path::EurePath;
use eure_document::value::ObjectKey;
use eure_parol::EureParseError;
use eure_parol::error::{ParseErrorEntry, ParseErrorKind};
use eure_schema::SchemaNodeId;
use eure_schema::convert::ConversionError;
use eure_schema::validate::ValidationError;
use eure_tree::prelude::{Cst, CstNodeId};
use eure_tree::tree::InputSpan;
use query_flow::{Db, QueryError};
use thisisplural::Plural;

use crate::document::OriginMap;
use crate::query::{
    DecorStyle, DecorStyleKey, DocumentToSchemaQuery, ParseCst, ParseDocument, TextFile,
    TextFileContent, ValidCst,
};

// ============================================================================
// Origin
// ============================================================================

/// A location in source code with semantic context hints.
#[derive(Debug, Clone, PartialEq)]
pub struct Origin {
    /// The file containing this location.
    pub file: TextFile,
    /// Byte range within the file.
    pub span: InputSpan,
    /// Optional semantic hints for this location.
    pub hints: OriginHints,
    /// True if this span couldn't be resolved and falls back to a default.
    pub is_fallback: bool,
}

impl Origin {
    /// Create an origin with just file and span (no hints).
    pub fn new(file: TextFile, span: InputSpan) -> Self {
        Self {
            file,
            span,
            hints: OriginHints::default(),
            is_fallback: false,
        }
    }

    /// Create an origin with full hints.
    pub fn with_hints(file: TextFile, span: InputSpan, hints: OriginHints) -> Self {
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
#[derive(Debug, Clone, Default, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Default, Plural, PartialEq)]
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

impl std::fmt::Display for ErrorReports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, report) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", report.title)?;
        }
        Ok(())
    }
}

impl std::error::Error for ErrorReports {}

// ============================================================================
// Element
// ============================================================================

/// An element within an error report.
#[derive(Debug, Clone, PartialEq)]
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
// Conversion Functions
// ============================================================================

/// Convert a parse error to ErrorReports.
pub fn report_parse_error(error: &EureParseError, file: TextFile) -> ErrorReports {
    error
        .entries
        .iter()
        .map(|entry| report_parse_entry(entry, file.clone()))
        .collect()
}

fn report_parse_entry(entry: &ParseErrorEntry, file: TextFile) -> ErrorReport {
    // FIXME: Fallback to EMPTY span when entry.span is None.
    // This silently reports errors at file start without indicating the location is uncertain.
    // Should set is_fallback flag on Origin when span is missing.
    let span = entry.span.unwrap_or(InputSpan::EMPTY);
    let origin = Origin::new(file.clone(), span);

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
        // FIXME: Fallback to EMPTY span for source errors without span information.
        // Nested error locations are lost when source.span is None.
        let source_span = source.span.unwrap_or(InputSpan::EMPTY);
        report = report.with_element(Element::Labelled {
            label: "caused by".into(),
            element: Box::new(Element::Annotation {
                origin: Origin::new(file.clone(), source_span),
                kind: AnnotationKind::Secondary,
                label: source.message.clone().into(),
            }),
        });
    }

    report
}

/// Convert schema validation errors to ErrorReports.
pub fn report_schema_validation_errors(
    db: &impl Db,
    file: TextFile,
    schema_file: TextFile,
    errors: &[ValidationError],
) -> Result<ErrorReports, QueryError> {
    errors
        .iter()
        .map(|error| report_validation_error(db, error, file.clone(), schema_file.clone()))
        .collect::<Result<ErrorReports, QueryError>>()
}

fn report_validation_error(
    db: &impl Db,
    error: &ValidationError,
    file: TextFile,
    schema_file: TextFile,
) -> Result<ErrorReport, QueryError> {
    let (doc_node_id, schema_node_id) = error.node_ids();

    // Query parsed document
    let doc = db.query(ParseDocument::new(file.clone()))?;

    // Query CST for span resolution
    let cst = db.query(ValidCst::new(file.clone()))?;

    // Query schema
    let schema = db.query(DocumentToSchemaQuery::new(schema_file.clone()))?;

    // Resolve span based on error type
    let resolved_span = match error {
        ValidationError::InvalidKeyType { key, node_id, .. } => {
            // Try to get precise key span first
            doc.origins
                .get_key_span(*node_id, key, &cst)
                .or_else(|| doc.origins.get_value_span(doc_node_id, &cst))
        }
        ValidationError::UnknownField { field, node_id, .. } => {
            // Try to get precise key span for the unknown field
            let object_key = ObjectKey::String(field.clone());
            doc.origins
                .get_key_span(*node_id, &object_key, &cst)
                .or_else(|| doc.origins.get_value_span(doc_node_id, &cst))
        }
        ValidationError::MissingRequiredField { .. } => {
            // Use definition span (the key) to show where the record is defined
            doc.origins
                .get_definition_span(doc_node_id, &cst)
                .or_else(|| doc.origins.get_value_span(doc_node_id, &cst))
        }
        _ => {
            // Standard node span resolution (value span)
            doc.origins.get_value_span(doc_node_id, &cst)
        }
    };

    // FIXME: While is_fallback is correctly set here, the EMPTY span still points to
    // file start which is misleading. Should consider using a wider range or the
    // parent node's span as a better fallback location.
    let (doc_span, is_fallback) = match resolved_span {
        Some(span) => (span, false),
        None => (InputSpan::EMPTY, true),
    };

    let doc_origin = Origin::with_hints(
        file.clone(),
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
    if let Some(schema_span) = resolve_schema_span(db, schema_node_id, schema_file.clone(), &schema)
    {
        let schema_origin = Origin::with_hints(
            schema_file.clone(),
            schema_span,
            OriginHints::default().with_schema(schema_node_id),
        );

        report = report.with_element(Element::Annotation {
            origin: schema_origin,
            kind: AnnotationKind::Secondary,
            label: "constraint defined here".into(),
        });
    }

    // Handle nested errors (e.g., NoVariantMatched with best match info)
    if let ValidationError::NoVariantMatched {
        best_match: Some(best),
        ..
    } = error
    {
        let nested = report_validation_error(db, &best.error, file, schema_file)?;
        report = report.with_element(Element::Nested {
            title: format!("most close variant '{}' failed with", best.variant_name).into(),
            children: nested.elements,
        });
    }

    Ok(report)
}

fn resolve_schema_span(
    db: &impl Db,
    schema_id: SchemaNodeId,
    schema_file: TextFile,
    schema: &crate::query::ValidatedSchema,
) -> Option<InputSpan> {
    // SchemaNodeId -> NodeId (from schema source map)
    let doc_node_id = schema.source_map.get(&schema_id)?;
    // Query CST for span resolution
    let cst = db.query(ParseCst::new(schema_file)).ok()?;
    // Use OriginMap for span resolution
    schema.parsed.origins.get_value_span(*doc_node_id, &cst.cst)
}

/// Convert a schema conversion error to an ErrorReport.
pub fn report_conversion_error(
    error: &ConversionError,
    file: TextFile,
    cst: &Cst,
    origins: &OriginMap,
) -> ErrorReport {
    match error {
        ConversionError::ParseError(parse_error) => {
            // ParseError contains a NodeId, resolve it
            // FIXME: Fallback to EMPTY span when node span resolution fails.
            // Should set is_fallback flag on Origin.
            let span = origins
                .get_value_span(parse_error.node_id, cst)
                .unwrap_or(InputSpan::EMPTY);

            let origin = Origin::with_hints(
                file,
                span,
                OriginHints::default().with_doc(parse_error.node_id),
            );

            ErrorReport::error(error.to_string(), origin)
        }
        _ => {
            // FIXME: All non-ParseError conversion errors get EMPTY span.
            // These errors report at file start with no attempt to provide location.
            // Should attempt to extract location info from the error or mark as fallback.
            let origin = Origin::new(file, InputSpan::EMPTY);
            ErrorReport::error(error.to_string(), origin)
        }
    }
}

// ============================================================================
// Rendering Functions
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;

use annotate_snippets::{AnnotationKind as SnippetAnnotation, Group, Level, Renderer, Snippet};

/// File info for rendering error reports.
struct FileInfo {
    path: String,
    source: String,
}

/// Rendering context that owns file contents so groups can borrow from it.
struct RenderContext {
    files: HashMap<TextFile, FileInfo>,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    fn prefetch(&mut self, db: &impl Db, file: &TextFile) -> Result<(), QueryError> {
        if self.files.contains_key(file) {
            return Ok(());
        }
        if let Some(file_info) = get_file_content(db, file)? {
            self.files.insert(file.clone(), file_info);
        }
        Ok(())
    }

    fn prefetch_for_element(&mut self, db: &impl Db, element: &Element) -> Result<(), QueryError> {
        match element {
            Element::Annotation { origin, .. } => self.prefetch(db, &origin.file)?,
            Element::Suggestion { origin, .. } => self.prefetch(db, &origin.file)?,
            Element::Labelled { element, .. } => self.prefetch_for_element(db, element)?,
            Element::Nested { children, .. } => {
                for child in children {
                    self.prefetch_for_element(db, child)?;
                }
            }
            Element::Note(_) | Element::Help(_) => {}
        }
        Ok(())
    }

    fn get(&self, file: &TextFile) -> Option<&FileInfo> {
        self.files.get(file)
    }
}

/// Get file content via query-flow asset.
///
/// Returns `Ok(Some(info))` if file exists, `Ok(None)` if file not found,
/// or propagates suspension error if content isn't loaded yet.
fn get_file_content(db: &impl Db, file: &TextFile) -> Result<Option<FileInfo>, QueryError> {
    let content: Arc<TextFileContent> = db.asset(file.clone())?;
    Ok(Some(FileInfo {
        path: file.to_string(),
        source: content.get().to_string(),
    }))
}

/// Get decor style from runtime asset, falling back to Unicode if not set.
fn get_decor_style(db: &impl Db) -> DecorStyle {
    db.asset(DecorStyleKey)
        .ok()
        .map(|arc| *arc)
        .unwrap_or_default() // Default to Unicode
}

/// Render an ErrorReport to a string using annotate-snippets.
///
/// Returns `Err` with suspension if file content isn't loaded yet.
pub fn format_error_report(
    db: &impl Db,
    report: &ErrorReport,
    styled: bool,
) -> Result<String, QueryError> {
    // Pre-fetch all file contents into a context that owns the data
    let mut ctx = RenderContext::new();
    ctx.prefetch(db, &report.primary_origin.file)?;
    for element in &report.elements {
        ctx.prefetch_for_element(db, element)?;
    }

    // Build groups that borrow from ctx
    let groups = build_snippet_groups(&ctx, report);

    let renderer = if styled {
        Renderer::styled()
    } else {
        Renderer::plain()
    };

    // Get decor style from asset
    let decor_style = get_decor_style(db);
    let annotate_decor_style = match decor_style {
        DecorStyle::Unicode => AnnotateDecorStyle::Unicode,
        DecorStyle::Ascii => AnnotateDecorStyle::Ascii,
    };

    Ok(renderer
        .decor_style(annotate_decor_style)
        .render(&groups)
        .to_string())
}

/// Render multiple ErrorReports to a string.
///
/// Returns `Err` with suspension if any file content isn't loaded yet.
pub fn format_error_reports(
    db: &impl Db,
    reports: &ErrorReports,
    styled: bool,
) -> Result<String, QueryError> {
    let mut results = Vec::new();
    for r in reports.iter() {
        results.push(format_error_report(db, r, styled)?);
    }
    Ok(results.join("\n"))
}

fn build_snippet_groups<'a>(ctx: &'a RenderContext, report: &'a ErrorReport) -> Vec<Group<'a>> {
    let mut groups = Vec::new();

    // Get primary file info
    let primary_file = ctx.get(&report.primary_origin.file);

    // Build primary group
    let level = match report.severity {
        Severity::Error => Level::ERROR,
        Severity::Warning => Level::WARNING,
        Severity::Note => Level::NOTE,
        Severity::Hint => Level::HELP,
    };

    if let Some(file_info) = primary_file {
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
        let primary_group = level.primary_title(report.title.as_ref()).element(
            Snippet::source("")
                .line_start(1)
                .annotation(SnippetAnnotation::Primary.span(0..0)),
        );
        groups.push(primary_group);
    }

    // Process elements
    for element in &report.elements {
        add_element_to_groups(ctx, element, &mut groups, primary_file);
    }

    groups
}

fn add_element_to_groups<'a>(
    ctx: &'a RenderContext,
    element: &'a Element,
    groups: &mut Vec<Group<'a>>,
    primary_file: Option<&'a FileInfo>,
) {
    match element {
        Element::Annotation {
            origin,
            kind,
            label,
        } => {
            if let Some(file_info) = ctx.get(&origin.file) {
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
            add_element_to_groups(ctx, element, groups, primary_file);
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
                add_element_to_groups(ctx, child, groups, primary_file);
            }
        }

        Element::Suggestion {
            origin,
            message,
            replacement: _,
        } => {
            // Suggestions are rendered as help with the origin highlighted
            if let Some(file_info) = ctx.get(&origin.file) {
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

// ============================================================================
// Config Error Reporting
// ============================================================================

/// Error comparator for query-flow that compares ErrorReports by value.
///
/// This comparator is designed for use with `QueryRuntimeBuilder::error_comparator`.
/// It enables early cutoff optimization by detecting when errors are semantically
/// equivalent, avoiding unnecessary downstream recomputation.
///
/// # Comparison Strategy
///
/// 1. If both errors can be downcast to `ErrorReports`, compare using `PartialEq`
/// 2. Otherwise, fall back to string comparison via `to_string()`
pub fn error_reports_comparator(a: &anyhow::Error, b: &anyhow::Error) -> bool {
    match (
        a.downcast_ref::<ErrorReports>(),
        b.downcast_ref::<ErrorReports>(),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => a.to_string() == b.to_string(),
    }
}

/// Convert a ConfigError to ErrorReports.
pub fn report_config_error(
    error: &eure_env::ConfigError,
    file: TextFile,
    cst: &Cst,
    origins: &OriginMap,
) -> ErrorReports {
    use eure_env::ConfigError;

    match error {
        // FIXME: IO errors have no location info, but EMPTY span is misleading.
        // Should mark as fallback or use a different reporting strategy for file-level errors.
        ConfigError::Io(e) => ErrorReports::from(vec![ErrorReport::error(
            e.to_string(),
            Origin::new(file, InputSpan::EMPTY),
        )]),
        ConfigError::Syntax(e) => report_parse_error(e, file),
        ConfigError::Parse(e) => {
            // FIXME: Fallback to EMPTY span when node span resolution fails.
            // Should set is_fallback flag on Origin when span is missing.
            let span = origins
                .get_value_span(e.node_id, cst)
                .unwrap_or(InputSpan::EMPTY);
            ErrorReports::from(vec![ErrorReport::error(
                e.to_string(),
                Origin::new(file, span),
            )])
        }
    }
}
