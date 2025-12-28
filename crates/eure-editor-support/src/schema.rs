//! Schema conversion and validation queries.

use std::sync::Arc;

use eure::document::OriginMap;
use eure::report::ErrorReports;
use eure_schema::SchemaDocument;
use eure_schema::convert::{ConversionError, document_to_schema};
use eure_schema::validate::{ValidationError, validate};
use eure_tree::prelude::Cst;
use query_flow::query;

use crate::assets::TextFile;
use crate::config::{ParseDocument, ParsedDocument};

/// Validated schema with the SchemaDocument.
#[derive(Clone, PartialEq)]
pub struct ValidatedSchema {
    pub schema: Arc<SchemaDocument>,
    pub parsed: ParsedDocument,
}

/// Convert document to SchemaDocument.
///
/// Returns `None` if parsing failed.
/// Returns `UserError` if schema conversion fails.
#[query]
pub fn document_to_schema_query(
    ctx: &mut query_flow::QueryContext,
    file: TextFile,
) -> Result<Option<ValidatedSchema>, query_flow::QueryError> {
    let result = ctx.query(ParseDocument::new(file.clone()))?;
    let parsed = match &*result {
        None => return Ok(None),
        Some(p) => p.clone(),
    };

    match document_to_schema(&parsed.doc) {
        Ok((schema, _source_map)) => Ok(Some(ValidatedSchema {
            schema: Arc::new(schema),
            parsed,
        })),
        Err(e) => Err(report_schema_conversion_error(&e, &parsed))?,
    }
}

/// Error span with source location for display.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorSpan {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// Validate document against schema.
///
/// Returns empty vec if either document or schema parsing failed.
#[query]
pub fn validate_against_schema(
    ctx: &mut query_flow::QueryContext,
    doc_file: TextFile,
    schema_file: TextFile,
) -> Result<Vec<ErrorSpan>, query_flow::QueryError> {
    let doc_result = ctx.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = match &*doc_result {
        None => return Ok(vec![]),
        Some(p) => p,
    };

    let schema_result = ctx.query(DocumentToSchemaQuery::new(schema_file.clone()))?;
    let validated_schema = match &*schema_result {
        None => return Ok(vec![]),
        Some(s) => s,
    };

    let result = validate(&doc_parsed.doc, &validated_schema.schema);
    let spans = result
        .errors
        .iter()
        .map(|e| validation_error_to_span(e, &doc_parsed.cst, &doc_parsed.origins))
        .collect();
    Ok(spans)
}

/// Validate schema against meta-schema.
///
/// Returns empty vec if either schema parsing failed.
#[query]
pub fn validate_against_meta_schema(
    ctx: &mut query_flow::QueryContext,
    schema_file: TextFile,
    meta_schema_file: TextFile,
) -> Result<Vec<ErrorSpan>, query_flow::QueryError> {
    let schema_result = ctx.query(ParseDocument::new(schema_file.clone()))?;
    let schema_parsed = match &*schema_result {
        None => return Ok(vec![]),
        Some(p) => p,
    };

    let meta_result = ctx.query(DocumentToSchemaQuery::new(meta_schema_file.clone()))?;
    let meta_schema = match &*meta_result {
        None => return Ok(vec![]),
        Some(s) => s,
    };

    let result = validate(&schema_parsed.doc, &meta_schema.schema);
    let spans = result
        .errors
        .iter()
        .map(|e| validation_error_to_span(e, &schema_parsed.cst, &schema_parsed.origins))
        .collect();
    Ok(spans)
}

/// Convert a validation error to an ErrorSpan using the origin map.
fn validation_error_to_span(error: &ValidationError, cst: &Cst, origins: &OriginMap) -> ErrorSpan {
    let message = error.to_string();
    let (node_id, _schema_node_id) = error.node_ids();

    let span = origins.get_node_span(node_id, cst);
    match span {
        Some(s) => ErrorSpan {
            start: s.start as usize,
            end: s.end as usize,
            message,
        },
        None => ErrorSpan {
            start: 0,
            end: 1,
            message,
        },
    }
}

/// Convert schema conversion error to ErrorReports.
fn report_schema_conversion_error(
    error: &ConversionError,
    parsed: &ParsedDocument,
) -> ErrorReports {
    use eure::report::{DocumentReportContext, ErrorReport, FileRegistry, report_conversion_error};

    let mut files = FileRegistry::new();
    let file_id = files.register("schema.eure", &*parsed.source);
    let ctx = DocumentReportContext {
        file: file_id,
        cst: &parsed.cst,
        origins: &parsed.origins,
    };
    let report = report_conversion_error(error, &ctx);

    ErrorReports::from(vec![ErrorReport::error(
        error.to_string(),
        report.primary_origin,
    )])
}
