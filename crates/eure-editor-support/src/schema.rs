//! Schema conversion and validation queries.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eure::document::OriginMap;
use eure::report::ErrorReports;
use eure_schema::SchemaDocument;
use eure_schema::convert::{ConversionError, document_to_schema};
use eure_schema::validate::{ValidationError, validate};
use eure_tree::prelude::Cst;
use query_flow::query;

use crate::assets::TextFile;
use crate::config::{GetConfig, ParseDocument, ParsedDocument};

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
    db: &impl Db,
    file: TextFile,
) -> Result<Option<ValidatedSchema>, query_flow::QueryError> {
    let result = db.query(ParseDocument::new(file.clone()))?;
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
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
) -> Result<Vec<ErrorSpan>, query_flow::QueryError> {
    let doc_result = db.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = match &*doc_result {
        None => return Ok(vec![]),
        Some(p) => p,
    };

    let schema_result = db.query(DocumentToSchemaQuery::new(schema_file.clone()))?;
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
    db: &impl Db,
    schema_file: TextFile,
    meta_schema_file: TextFile,
) -> Result<Vec<ErrorSpan>, query_flow::QueryError> {
    let schema_result = db.query(ParseDocument::new(schema_file.clone()))?;
    let schema_parsed = match &*schema_result {
        None => return Ok(vec![]),
        Some(p) => p,
    };

    let meta_result = db.query(DocumentToSchemaQuery::new(meta_schema_file.clone()))?;
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

// =============================================================================
// Schema Resolution Queries
// =============================================================================

/// Extract the `$schema` extension value from a document's root node.
///
/// Returns `None` if:
/// - The file cannot be parsed
/// - The document has no `$schema` extension
/// - The `$schema` value is not a valid string
#[query]
pub fn get_schema_extension(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<String>, query_flow::QueryError> {
    let result = db.query(ParseDocument::new(file.clone()))?;
    let parsed = match &*result {
        None => return Ok(None),
        Some(p) => p,
    };

    let root_id = parsed.doc.get_root_id();
    let root_ctx = parsed.doc.parse_context(root_id);

    // Try to get $schema extension as a string
    match root_ctx.parse_ext_optional::<String>("schema") {
        Ok(Some(schema_path)) => Ok(Some(schema_path)),
        Ok(None) => Ok(None),
        Err(_) => Ok(None), // Invalid type, diagnostics handled by get_schema_extension_diagnostics
    }
}

/// Check for schema extension errors (e.g., wrong type).
///
/// Returns diagnostics if `$schema` exists but is not a valid string.
#[query]
pub fn get_schema_extension_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<Vec<ErrorSpan>, query_flow::QueryError> {
    let result = db.query(ParseDocument::new(file.clone()))?;
    let parsed = match &*result {
        None => return Ok(vec![]),
        Some(p) => p,
    };

    let root_id = parsed.doc.get_root_id();
    let root_ctx = parsed.doc.parse_context(root_id);

    // Check if $schema extension exists
    let Some(schema_ctx) = root_ctx.ext_optional("schema") else {
        return Ok(vec![]);
    };

    // Try to parse as string
    if root_ctx.parse_ext_optional::<String>("schema").is_ok() {
        return Ok(vec![]);
    }

    // $schema exists but has wrong type - generate diagnostic
    let node_id = schema_ctx.node_id();
    let span = parsed.origins.get_node_span(node_id, &parsed.cst);
    let (start, end) = span
        .map(|s| (s.start as usize, s.end as usize))
        .unwrap_or((0, 1));

    Ok(vec![ErrorSpan {
        start,
        end,
        message: "$schema must be a string path to a schema file".to_string(),
    }])
}

/// Resolve the schema file for a document.
///
/// Priority order:
/// 1. `$schema` extension in the document itself
/// 2. Workspace config (`Eure.eure`) schema mappings
/// 3. File name heuristics (e.g., `*.schema.eure` uses meta-schema)
///
/// Returns `None` if no schema can be determined.
#[query]
pub fn resolve_schema(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<TextFile>, query_flow::QueryError> {
    // 1. Check $schema extension in the document
    if let Some(schema_path) = db.query(GetSchemaExtension::new(file.clone()))?.as_ref() {
        let resolved = resolve_relative_path(&file.path, schema_path);
        return Ok(Some(TextFile::from_path(resolved)));
    }

    // 2. Check workspace config
    if let Some(config) = db.query(GetConfig::new(file.clone()))?.as_ref() {
        // Get config directory from workspace
        let workspace_ids = db.list_asset_keys::<crate::assets::WorkspaceId>();
        if let Some(workspace_id) = workspace_ids.into_iter().next() {
            let workspace: std::sync::Arc<crate::assets::Workspace> =
                db.asset(workspace_id)?.suspend()?;
            if let Some(config_dir) = workspace.config_path.parent()
                && let Some(schema_path) = config.schema_for_path(&file.path, config_dir)
            {
                return Ok(Some(TextFile::from_path(schema_path)));
            }
        }
    }

    // 3. File name heuristics
    let path_str = file.path.to_string_lossy();
    if path_str.ends_with(".schema.eure") {
        // Schema files are validated against the meta-schema
        return Ok(Some(meta_schema_file()));
    }

    Ok(None)
}

/// Get the built-in meta-schema file.
fn meta_schema_file() -> TextFile {
    // The meta-schema is bundled with the application
    TextFile::from_path(PathBuf::from("$eure/meta-schema.eure"))
}

/// Resolve a relative path against a base file path.
fn resolve_relative_path(base: &Path, relative: &str) -> PathBuf {
    if let Some(parent) = base.parent() {
        parent.join(relative)
    } else {
        PathBuf::from(relative)
    }
}

// =============================================================================
// Validation Helper Functions
// =============================================================================

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
