//! Error formatting utilities for Eure.
//!
//! This module provides functions that use the `report` module internally.

use eure_parol::EureParseError;
use eure_schema::convert::SchemaSourceMap;
use eure_schema::validate::ValidationError;
use eure_tree::prelude::Cst;

use crate::document::{DocumentConstructionError, OriginMap};
use crate::report::{
    DocumentReportContext, FileRegistry, SchemaReportContext, format_error_report,
    format_error_reports, report_document_error_simple, report_parse_error,
    report_schema_validation_errors,
};

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
    /// Origin map with precise key origins
    pub doc_origins: &'a OriginMap,
    /// Source text of the schema
    pub schema_source: &'a str,
    /// File path of the schema (for display)
    pub schema_path: &'a str,
    /// CST of the schema (for span resolution)
    pub schema_cst: &'a Cst,
    /// Origin map for schema
    pub schema_origins: &'a OriginMap,
    /// Mapping from schema node IDs to document node IDs
    pub schema_source_map: &'a SchemaSourceMap,
}

/// Format a parse error with source context using annotate-snippets.
pub fn format_parse_error_color(error: &EureParseError, input: &str, path: &str) -> String {
    let mut files = FileRegistry::new();
    let file_id = files.register(path, input);
    let mut reports = report_parse_error(error, file_id);
    reports.sort_by_span();
    format_error_reports(&reports, &files, true)
}

/// Format a parse error with source context (plain text, no colors).
pub fn format_parse_error_plain(error: &EureParseError, input: &str, path: &str) -> String {
    let mut files = FileRegistry::new();
    let file_id = files.register(path, input);
    let mut reports = report_parse_error(error, file_id);
    reports.sort_by_span();
    format_error_reports(&reports, &files, false)
}

/// Format a document construction error with source context using annotate-snippets.
pub fn format_document_error(
    error: &DocumentConstructionError,
    input: &str,
    path: &str,
    cst: &Cst,
) -> String {
    let mut files = FileRegistry::new();
    let file_id = files.register(path, input);
    let report = report_document_error_simple(error, file_id, cst);
    format_error_report(&report, &files, true)
}

/// Format a schema validation error with annotated source locations (with colors).
pub fn format_schema_error(error: &ValidationError, context: &SchemaErrorContext<'_>) -> String {
    format_schema_error_impl(error, context, true)
}

/// Format a schema validation error with annotated source locations (plain text).
pub fn format_schema_error_plain(
    error: &ValidationError,
    context: &SchemaErrorContext<'_>,
) -> String {
    format_schema_error_impl(error, context, false)
}

fn format_schema_error_impl(
    error: &ValidationError,
    context: &SchemaErrorContext<'_>,
    styled: bool,
) -> String {
    let mut files = FileRegistry::new();
    let doc_file_id = files.register(context.doc_path, context.doc_source);
    let schema_file_id = files.register(context.schema_path, context.schema_source);

    let report_ctx = SchemaReportContext {
        doc: DocumentReportContext {
            file: doc_file_id,
            cst: context.doc_cst,
            origins: context.doc_origins,
        },
        schema_file: schema_file_id,
        schema_cst: context.schema_cst,
        schema_origins: context.schema_origins,
        schema_source_map: context.schema_source_map,
    };

    let mut reports = report_schema_validation_errors(std::slice::from_ref(error), &report_ctx);
    reports.sort_by_span();
    format_error_reports(&reports, &files, styled)
}
