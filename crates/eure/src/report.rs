//! Unified error reporting IR for Eure.
//!
//! This module provides a standardized `ErrorReport` structure that can represent
//! errors from all Eure crates (parser, document, schema, etc.) and be rendered
//! to multiple outputs (CLI via annotate-snippets, LSP Diagnostics).

use std::borrow::Cow;

use annotate_snippets::renderer::DecorStyle as AnnotateDecorStyle;
use eure_document::document::NodeId;
use eure_document::parse::ParseError;
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

use crate::document::{DocumentConstructionError, OriginMap};
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
    /// Add a new error report to the collection.
    pub fn push(&mut self, report: ErrorReport) {
        self.0.push(report);
    }

    /// Remove and return the last error report.
    pub fn pop(&mut self) -> Option<ErrorReport> {
        self.0.pop()
    }

    /// Get a mutable reference to a report at a specific index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ErrorReport> {
        self.0.get_mut(index)
    }

    /// Replace a report at a specific index.
    pub fn replace(&mut self, index: usize, report: ErrorReport) {
        if index < self.0.len() {
            self.0[index] = report;
        }
    }

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
// IntoErrorReports Trait
// ============================================================================

/// Trait for errors that can be converted to ErrorReports.
///
/// This trait allows custom error types to be converted to `ErrorReports`
/// for rich error reporting with source locations.
pub trait IntoErrorReports {
    fn to_error_reports(&self, db: &impl Db, file: TextFile) -> Result<ErrorReports, QueryError>;
}

impl IntoErrorReports for ParseError {
    fn to_error_reports(&self, db: &impl Db, file: TextFile) -> Result<ErrorReports, QueryError> {
        let parsed_cst = db.query(ParseCst::new(file.clone()))?;
        let parsed = db.query(ParseDocument::new(file.clone()))?;
        Ok(report_from_eure_parse_error(
            self,
            file,
            &parsed_cst.cst,
            &parsed.origins,
        ))
    }
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

/// Convert a document construction error to an ErrorReport.
/// Uses OriginMap for precise key span resolution when available.
pub fn report_document_error(
    error: &DocumentConstructionError,
    file: TextFile,
    cst: &Cst,
    origins: &OriginMap,
) -> ErrorReport {
    // Use span_with_origin_map for precise key spans, fallback to regular span
    let span = error
        .span_with_origin_map(cst, origins)
        .or_else(|| error.span(cst))
        .unwrap_or(InputSpan::EMPTY);
    ErrorReport::error(error.to_string(), Origin::new(file, span))
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
///
/// For `NoVariantMatched` errors, this expands all errors from the best matching
/// variant into separate reports, giving users complete visibility into what went wrong.
pub fn report_schema_validation_errors(
    db: &impl Db,
    file: TextFile,
    schema_file: TextFile,
    errors: &[ValidationError],
) -> Result<ErrorReports, QueryError> {
    let mut reports = ErrorReports::default();
    for error in errors {
        expand_validation_error(db, error, file.clone(), schema_file.clone(), &mut reports)?;
    }
    Ok(reports)
}

/// Expand a validation error into reports, recursively handling NoVariantMatched.
fn expand_validation_error(
    db: &impl Db,
    error: &ValidationError,
    file: TextFile,
    schema_file: TextFile,
    reports: &mut ErrorReports,
) -> Result<(), QueryError> {
    expand_validation_error_inner(db, error, file, schema_file, reports, None)?;
    Ok(())
}

/// Inner recursive function that tracks variant context for nested unions.
/// Returns the final accumulated variant path for use in context notes.
fn expand_validation_error_inner(
    db: &impl Db,
    error: &ValidationError,
    file: TextFile,
    schema_file: TextFile,
    reports: &mut ErrorReports,
    variant_context: Option<(String, &eure_document::path::EurePath)>,
) -> Result<Option<String>, QueryError> {
    // For NoVariantMatched with a best match, expand all errors from that variant
    if let ValidationError::NoVariantMatched {
        path,
        best_match: Some(best),
        ..
    } = error
    {
        // Build accumulated variant path: for nested unions, append variant names with '.'
        let variant_path = match &variant_context {
            Some((existing_path, _)) => format!("{}.{}", existing_path, best.variant_name),
            None => best.variant_name.clone(),
        };
        // Use outermost path for context
        let outer_path = match &variant_context {
            Some((_, outer_path)) => *outer_path,
            None => path,
        };

        let mut final_variant_path = None;
        for (i, inner_error) in best.all_errors.iter().enumerate() {
            // Recursively expand inner errors (handles nested NoVariantMatched)
            let start_len = reports.len();
            let inner_path = expand_validation_error_inner(
                db,
                inner_error,
                file.clone(),
                schema_file.clone(),
                reports,
                Some((variant_path.clone(), outer_path)),
            )?;

            // Track the final accumulated variant path from the first error
            if i == 0 {
                final_variant_path = inner_path;
            }

            // Add variant context note to the first report from this expansion
            // We modify the report at start_len (the first one added), not the last
            if reports.len() > start_len && i == 0 && variant_context.is_none() {
                // Only add full note to first error of outermost union
                // Use the final accumulated variant path from recursive calls
                let note_variant_path = final_variant_path.as_deref().unwrap_or(&variant_path);
                if let Some(first_report) = reports.get_mut(start_len).map(|r| r.clone()) {
                    let updated_report = add_variant_context_note(
                        db,
                        first_report,
                        note_variant_path,
                        outer_path,
                        schema_file.clone(),
                        error, // Pass the original error to traverse the variant chain
                    )?;
                    reports.replace(start_len, updated_report);
                }
            }
        }
        return Ok(final_variant_path.or(Some(variant_path)));
    }

    // For other errors, create a single report with variant context in message
    let final_path = variant_context.as_ref().map(|(name, _)| name.clone());
    let context_for_message = variant_context
        .as_ref()
        .map(|(name, path)| (name.as_str(), *path));
    reports.push(report_validation_error(
        db,
        error,
        file,
        schema_file,
        context_for_message,
    )?);
    Ok(final_path)
}

/// Collect variant schema IDs by traversing the BestVariantMatch chain.
/// Each nested NoVariantMatched error contains the variant_schema_id for that level.
fn collect_variant_schema_ids(error: &ValidationError) -> Vec<SchemaNodeId> {
    let mut ids = Vec::new();
    let mut current = error;
    while let ValidationError::NoVariantMatched {
        best_match: Some(best),
        ..
    } = current
    {
        ids.push(best.variant_schema_id);
        current = &best.error;
    }
    ids
}

/// Add a note about which variant was selected for union error context.
fn add_variant_context_note(
    db: &impl Db,
    report: ErrorReport,
    variant_name: &str,
    path: &eure_document::path::EurePath,
    schema_file: TextFile,
    error: &ValidationError,
) -> Result<ErrorReport, QueryError> {
    let schema = db.query(DocumentToSchemaQuery::new(schema_file.clone()))?;

    // Collect all existing schema spans from the report to avoid duplicates
    let existing_spans: Vec<InputSpan> = report
        .elements
        .iter()
        .filter_map(|e| {
            if let Element::Annotation { origin, .. } = e {
                Some(origin.span)
            } else {
                None
            }
        })
        .collect();

    let mut report = report;

    // Get variant schema IDs from the error chain (much simpler than walking schema hierarchy)
    let variant_schema_ids = collect_variant_schema_ids(error);

    // Collect spans for each variant, filtering duplicates
    let variant_spans: Vec<(InputSpan, SchemaNodeId)> = variant_schema_ids
        .iter()
        .filter_map(|&schema_id| {
            resolve_schema_definition_span(db, schema_id, schema_file.clone(), &schema)
                .filter(|span| !existing_spans.contains(span))
                .map(|span| (span, schema_id))
        })
        .collect();

    // Only add note and annotations if we have variant spans to show
    if !variant_spans.is_empty() {
        // Add concise note with first variant's span
        let note_message = format!(
            "based on nearest variant '{}' for union at path {}",
            variant_name, path
        );
        let (first_span, first_schema_id) = variant_spans[0];
        let origin = Origin::with_hints(
            schema_file.clone(),
            first_span,
            OriginHints::default().with_schema(first_schema_id),
        );
        report = report.with_element(Element::Annotation {
            origin,
            kind: AnnotationKind::Secondary,
            label: note_message.into(),
        });

        // Add "selected variant" annotations for remaining levels
        for (span, schema_id) in variant_spans.into_iter().skip(1) {
            let origin = Origin::with_hints(
                schema_file.clone(),
                span,
                OriginHints::default().with_schema(schema_id),
            );
            report = report.with_element(Element::Annotation {
                origin,
                kind: AnnotationKind::Secondary,
                label: "selected variant".into(),
            });
        }
    }

    Ok(report)
}

fn report_validation_error(
    db: &impl Db,
    error: &ValidationError,
    file: TextFile,
    schema_file: TextFile,
    variant_context: Option<(&str, &eure_document::path::EurePath)>,
) -> Result<ErrorReport, QueryError> {
    // For NoVariantMatched, use the deepest error for span resolution
    // to point to the actual error location instead of the outer union value
    let span_error = error.deepest_error();
    let (doc_node_id, schema_node_id) = span_error.node_ids();

    // Query parsed document
    let doc = db.query(ParseDocument::new(file.clone()))?;

    // Query CST for span resolution
    let cst = db.query(ValidCst::new(file.clone()))?;

    // Query schema
    let schema = db.query(DocumentToSchemaQuery::new(schema_file.clone()))?;

    // Resolve span based on error type (using span_error for deepest error location)
    let resolved_span = match span_error {
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
        ValidationError::FlattenMapKeyMismatch { key, node_id, .. } => {
            // Try to get precise key span for the mismatched key
            let object_key = ObjectKey::String(key.clone());
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

    // Build error message, optionally with variant context suffix
    let message = match variant_context {
        Some((variant_name, path)) => format!(
            "{} (based on nearest variant '{}' for union at path {})",
            error, variant_name, path
        ),
        None => error.to_string(),
    };
    let mut report = ErrorReport::error(message, doc_origin);

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

    // Note: Variant context notes for NoVariantMatched are added by
    // expand_validation_error/add_variant_context_note, not here.

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

/// Resolve schema definition span (where the key is, not the value).
/// Used for variant context notes to point to the variant NAME rather than value.
fn resolve_schema_definition_span(
    db: &impl Db,
    schema_id: SchemaNodeId,
    schema_file: TextFile,
    schema: &crate::query::ValidatedSchema,
) -> Option<InputSpan> {
    // SchemaNodeId -> NodeId (from schema source map)
    let doc_node_id = schema.source_map.get(&schema_id)?;
    // Query CST for span resolution
    let cst = db.query(ParseCst::new(schema_file)).ok()?;
    // Use definition span (key location) instead of value span
    schema
        .parsed
        .origins
        .get_definition_span(*doc_node_id, &cst.cst)
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

/// Convert a FromEure ParseError to ErrorReports.
///
/// This is the shared implementation used by both `IntoErrorReports` and `report_config_error`.
pub fn report_from_eure_parse_error(
    error: &ParseError,
    file: TextFile,
    cst: &Cst,
    origins: &OriginMap,
) -> ErrorReports {
    // FIXME: Fallback to EMPTY span when node span resolution fails.
    // Should set is_fallback flag on Origin when span is missing.
    let span = origins
        .get_value_span(error.node_id, cst)
        .unwrap_or(InputSpan::EMPTY);
    let origin = Origin::with_hints(file, span, OriginHints::default().with_doc(error.node_id));
    ErrorReports::from(vec![ErrorReport::error(error.to_string(), origin)])
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
        ConfigError::Parse(e) => report_from_eure_parse_error(e, file, cst, origins),
    }
}
