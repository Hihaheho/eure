//! Error formatting utilities for Eure.

use annotate_snippets::{AnnotationKind, Group, Level, Renderer, Snippet};
use eure_parol::EureParseError;
use eure_parol::error::ParseErrorEntry;
use eure_schema::SchemaNodeId;
use eure_schema::convert::SchemaSourceMap;
use eure_schema::validate::ValidationError;
use eure_tree::prelude::Cst;

use crate::document::NodeOriginMap;

/// Format a parse error with source context using annotate-snippets.
///
/// # Arguments
/// * `error` - The parse error to format
/// * `input` - The source input that was being parsed
/// * `path` - The file path (for display purposes)
///
/// # Returns
/// A formatted error string suitable for terminal output
pub fn format_parse_error_color(error: &EureParseError, input: &str, path: &str) -> String {
    let mut reports = Vec::new();

    for entry in &error.entries {
        format_entry_recursive(entry, input, path, &mut reports);
    }

    if reports.is_empty() {
        return String::new();
    }

    Renderer::styled().render(&reports).to_string()
}

pub fn format_parse_error_plain(error: &EureParseError, input: &str, path: &str) -> String {
    let mut reports = Vec::new();

    for entry in &error.entries {
        format_entry_recursive(entry, input, path, &mut reports);
    }

    Renderer::plain().render(&reports).to_string()
}

/// Format a single parse error entry and its nested source errors recursively.
fn format_entry_recursive<'a>(
    entry: &'a ParseErrorEntry,
    input: &'a str,
    path: &'a str,
    reports: &mut Vec<Group<'a>>,
) {
    // Use the entire input as span if none is provided
    let span_range = entry
        .span
        .map(|s| s.start as usize..s.end as usize)
        .unwrap_or(0..input.len());

    let report = Level::ERROR.primary_title(&entry.message).element(
        Snippet::source(input).line_start(1).path(path).annotation(
            AnnotationKind::Primary
                .span(span_range)
                .label(&entry.message),
        ),
    );

    reports.push(report);

    // Recursively process nested source errors
    for source_entry in &entry.source {
        format_entry_recursive(source_entry, input, path, reports);
    }
}

/// Context for formatting schema validation errors with source spans.
///
/// This holds all the information needed to resolve spans for both the
/// document being validated and the schema defining the constraints.
pub struct SchemaErrorContext<'a> {
    /// Source text of the document being validated
    pub doc_source: &'a str,
    /// File path of the document (for display)
    pub doc_path: &'a str,
    /// CST of the document (for span resolution)
    pub doc_cst: &'a Cst,
    /// Origin mapping for document nodes (NodeId → NodeOrigin → span)
    pub doc_origins: &'a NodeOriginMap,
    /// Source text of the schema
    pub schema_source: &'a str,
    /// File path of the schema (for display)
    pub schema_path: &'a str,
    /// CST of the schema (for span resolution)
    pub schema_cst: &'a Cst,
    /// Origin mapping for schema document nodes
    pub schema_origins: &'a NodeOriginMap,
    /// Mapping from schema node IDs to document node IDs
    pub schema_source_map: &'a SchemaSourceMap,
}

/// Format a schema validation error with annotated source locations.
///
/// Shows both:
/// - The document location where the error occurred (primary)
/// - The schema location where the constraint is defined (secondary)
///
/// # Arguments
/// * `error` - The validation error to format
/// * `context` - Context containing source files, CSTs, and origin mappings
///
/// # Returns
/// A formatted error string suitable for terminal output
pub fn format_schema_error(error: &ValidationError, context: &SchemaErrorContext<'_>) -> String {
    let (doc_node_id, schema_node_id) = error.node_ids();
    let error_message = error.to_string();

    // Try to get document span
    let doc_span = doc_node_id.and_then(|node_id| {
        context
            .doc_origins
            .get(&node_id)
            .and_then(|origins| origins.first().and_then(|o| o.get_span(context.doc_cst)))
    });

    // Try to get schema span
    let schema_span = schema_node_id.and_then(|id| resolve_schema_span(id, context));

    // Build report based on what spans are available
    match (doc_span, schema_span) {
        (Some(doc_span), Some(schema_span)) => {
            // Both spans available - show both
            let report = Level::ERROR.primary_title(&error_message).element(
                Snippet::source(context.doc_source)
                    .line_start(1)
                    .path(context.doc_path)
                    .annotation(
                        AnnotationKind::Primary
                            .span(doc_span.start as usize..doc_span.end as usize)
                            .label(&error_message),
                    ),
            );
            let note = Level::NOTE
                .primary_title("constraint defined here")
                .element(
                    Snippet::source(context.schema_source)
                        .line_start(1)
                        .path(context.schema_path)
                        .annotation(
                            AnnotationKind::Context
                                .span(schema_span.start as usize..schema_span.end as usize)
                                .label("constraint defined here"),
                        ),
                );
            Renderer::styled().render(&[report, note]).to_string()
        }
        (Some(doc_span), None) => {
            // Only document span
            let report = Level::ERROR.primary_title(&error_message).element(
                Snippet::source(context.doc_source)
                    .line_start(1)
                    .path(context.doc_path)
                    .annotation(
                        AnnotationKind::Primary
                            .span(doc_span.start as usize..doc_span.end as usize)
                            .label(&error_message),
                    ),
            );
            Renderer::styled().render(&[report]).to_string()
        }
        (None, Some(schema_span)) => {
            // Only schema span
            let report = Level::ERROR.primary_title(&error_message).element(
                Snippet::source(context.schema_source)
                    .line_start(1)
                    .path(context.schema_path)
                    .annotation(
                        AnnotationKind::Primary
                            .span(schema_span.start as usize..schema_span.end as usize)
                            .label(&error_message),
                    ),
            );
            Renderer::styled().render(&[report]).to_string()
        }
        (None, None) => {
            // No spans available - just format the error
            format!("error: {}\n", error_message)
        }
    }
}

/// Resolve a schema node ID to an input span.
///
/// This goes through the chain: SchemaNodeId → NodeId → NodeOrigin → InputSpan
fn resolve_schema_span(
    schema_id: SchemaNodeId,
    context: &SchemaErrorContext<'_>,
) -> Option<eure_tree::tree::InputSpan> {
    // SchemaNodeId → NodeId (from schema source map)
    let doc_node_id = context.schema_source_map.get(&schema_id)?;
    // NodeId → NodeOrigins
    let origins = context.schema_origins.get(doc_node_id)?;
    // NodeOrigin → InputSpan
    origins.first().and_then(|o| o.get_span(context.schema_cst))
}
