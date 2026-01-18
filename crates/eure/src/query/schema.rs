//! Schema conversion and validation queries.

use std::path::Path;
use std::sync::Arc;

use eure_document::value::ObjectKey;
use eure_schema::SchemaDocument;
use eure_schema::convert::{ConversionError, SchemaSourceMap, document_to_schema};
pub use eure_schema::validate::UnionTagMode;
use eure_schema::validate::{ValidationError, validate, validate_with_mode};
use eure_tree::prelude::Cst;
use eure_tree::tree::InputSpan;
use query_flow::{Db, QueryError, query};

use crate::document::OriginMap;

use crate::report::{
    ErrorReport, ErrorReports, Origin, format_error_reports, report_schema_validation_errors,
};

use super::assets::TextFile;
use super::config::ResolveConfig;
use super::parse::{ParseCst, ParseDocument, ParsedDocument};

/// Validated schema with the SchemaDocument and source map.
#[derive(Clone, PartialEq)]
pub struct ValidatedSchema {
    pub schema: Arc<SchemaDocument>,
    pub source_map: Arc<SchemaSourceMap>,
    pub parsed: ParsedDocument,
}

/// Resolved $schema extension value with origin for error reporting.
#[derive(Clone, PartialEq)]
pub struct ResolvedSchemaExtension {
    /// The schema path string from $schema extension.
    pub path: String,
    /// Origin of the $schema value (for span in diagnostics).
    pub origin: Origin,
}

/// Resolved schema reference with origin for error reporting.
#[derive(Clone, PartialEq)]
pub struct ResolvedSchema {
    /// The resolved schema file.
    pub file: TextFile,
    /// Origin of the schema reference (None for heuristics like *.schema.eure).
    pub origin: Option<Origin>,
}

/// Convert document to SchemaDocument.
///
/// Returns `None` if parsing failed.
/// Returns `UserError` if schema conversion fails.
#[query(debug = "{Self}({file})")]
pub fn document_to_schema_query(
    db: &impl Db,
    file: TextFile,
) -> Result<ValidatedSchema, QueryError> {
    let parsed = db.query(ParseDocument::new(file.clone()))?;

    match document_to_schema(&parsed.doc) {
        Ok((schema, source_map)) => Ok(ValidatedSchema {
            schema: Arc::new(schema),
            source_map: Arc::new(source_map),
            parsed: parsed.as_ref().clone(),
        }),
        Err(e) => {
            let cst = db.query(ParseCst::new(file.clone()))?;
            Err(report_schema_conversion_error(&e, &parsed, &cst.cst, file))?
        }
    }
}

/// Try to convert document to SchemaDocument and return formatted error if conversion fails.
///
/// Returns `Some(formatted_error)` if conversion fails.
/// Returns `None` if parsing failed or conversion succeeds.
#[query(debug = "{Self}({file})")]
pub fn get_schema_conversion_error_formatted(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<String>, QueryError> {
    match db.query(DocumentToSchemaQuery::new(file.clone())) {
        Ok(_) => Ok(None), // Schema conversion succeeded
        Err(QueryError::UserError(e)) => {
            // Try to downcast to ErrorReports
            if let Some(reports) = e.downcast_ref::<ErrorReports>() {
                Ok(Some(format_error_reports(db, reports, false)?))
            } else {
                // Can't downcast, re-propagate the error
                Err(QueryError::UserError(e))
            }
        }
        Err(other) => Err(other), // Re-propagate system errors
    }
}

/// Validate document against schema.
///
/// Resolves the schema internally from the document's $schema extension,
/// workspace config, or file name heuristics.
///
/// Returns empty reports if no schema is found or parsing failed.
/// Returns error report with proper origin if schema file is not found.
#[query(debug = "{Self}({doc_file})")]
pub fn validate_against_schema(
    db: &impl Db,
    doc_file: TextFile,
) -> Result<ErrorReports, QueryError> {
    // Resolve schema internally
    let Some(schema) = db
        .query(ResolveSchema::new(doc_file.clone()))?
        .as_ref()
        .clone()
    else {
        return Ok(ErrorReports::new());
    };

    // Parse document
    let doc_result = db.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = doc_result.as_ref().clone();

    // Load and convert schema - handle errors appropriately
    let schema_result = match db.query(DocumentToSchemaQuery::new(schema.file.clone())) {
        Ok(result) => result,
        Err(QueryError::UserError(e)) => {
            // Schema conversion errors are returned as ErrorReports with proper location
            if let Some(reports) = e.downcast_ref::<ErrorReports>() {
                return Ok(reports.clone());
            }
            // Other errors (file not found, network, etc.) should be reported at $schema origin
            if let Some(origin) = &schema.origin {
                return Ok(ErrorReports::from(vec![ErrorReport::error(
                    format!("Failed to load schema: {}", e),
                    origin.clone(),
                )]));
            }
            return Err(QueryError::UserError(e));
        }
        Err(other) => return Err(other),
    };

    let result = validate(&doc_parsed.doc, &schema_result.schema);

    report_schema_validation_errors(db, doc_file, schema.file, &result.errors)
}

/// Validate document against an explicitly provided schema file.
///
/// Use this when you have a specific schema file to validate against
/// (e.g., from workspace config). For automatic schema resolution,
/// use `validate_against_schema` instead.
///
/// Returns empty reports if either document or schema parsing failed.
#[query(debug = "{Self}({doc_file}, {schema_file})")]
pub fn validate_against_explicit_schema(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
) -> Result<ErrorReports, QueryError> {
    let doc_result = db.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = doc_result.as_ref().clone();

    let schema_result = db.query(DocumentToSchemaQuery::new(schema_file.clone()))?;

    let result = validate(&doc_parsed.doc, &schema_result.schema);

    report_schema_validation_errors(db, doc_file, schema_file, &result.errors)
}

/// Validate document against an explicitly provided schema file with specified union tag mode.
///
/// Use this when you have a specific schema file and need to control union tag handling.
///
/// Returns empty reports if either document or schema parsing failed.
#[query(debug = "{Self}({doc_file}, {schema_file}, {mode:?})")]
pub fn validate_against_explicit_schema_with_mode(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
    mode: UnionTagMode,
) -> Result<ErrorReports, QueryError> {
    let doc_result = db.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = doc_result.as_ref().clone();

    let schema_result = db.query(DocumentToSchemaQuery::new(schema_file.clone()))?;

    let result = validate_with_mode(&doc_parsed.doc, &schema_result.schema, mode);

    report_schema_validation_errors(db, doc_file, schema_file, &result.errors)
}

/// Validate document against an explicitly provided schema and return formatted error strings.
///
/// Use this when you have a specific schema file to validate against.
///
/// Returns empty vec if either document or schema parsing failed.
/// Returns formatted error messages suitable for display.
#[query(debug = "{Self}({doc_file}, {schema_file})")]
pub fn get_validation_errors_formatted_explicit(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
) -> Result<Vec<String>, QueryError> {
    let reports = db.query(ValidateAgainstExplicitSchema::new(doc_file, schema_file))?;

    // Format each error report individually
    let mut formatted = Vec::new();
    for report in reports.iter() {
        let single_report = ErrorReports::from(vec![report.clone()]);
        formatted.push(format_error_reports(db, &single_report, false)?);
    }

    Ok(formatted)
}

/// Validate document against an explicitly provided schema with specified union tag mode
/// and return formatted error strings.
///
/// Use this when you have a specific schema file and need to control union tag handling.
///
/// Returns empty vec if either document or schema parsing failed.
/// Returns formatted error messages suitable for display.
#[query(debug = "{Self}({doc_file}, {schema_file}, {mode:?})")]
pub fn get_validation_errors_formatted_explicit_with_mode(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: TextFile,
    mode: UnionTagMode,
) -> Result<Vec<String>, QueryError> {
    let reports = db.query(ValidateAgainstExplicitSchemaWithMode::new(
        doc_file,
        schema_file,
        mode,
    ))?;

    // Format each error report individually
    let mut formatted = Vec::new();
    for report in reports.iter() {
        let single_report = ErrorReports::from(vec![report.clone()]);
        formatted.push(format_error_reports(db, &single_report, false)?);
    }

    Ok(formatted)
}

/// Validate document against schema and return formatted error strings.
///
/// Resolves the schema internally from the document's $schema extension,
/// workspace config, or file name heuristics.
///
/// Returns empty vec if no schema is found or parsing failed.
/// Returns formatted error messages suitable for display.
#[query(debug = "{Self}({doc_file})")]
pub fn get_validation_errors_formatted(
    db: &impl Db,
    doc_file: TextFile,
) -> Result<Vec<String>, QueryError> {
    let reports = db.query(ValidateAgainstSchema::new(doc_file))?;

    // Format each error report individually
    let mut formatted = Vec::new();
    for report in reports.iter() {
        let single_report = ErrorReports::from(vec![report.clone()]);
        formatted.push(format_error_reports(db, &single_report, false)?);
    }

    Ok(formatted)
}

/// Validate document against schema with specified union tag mode.
///
/// Resolves the schema internally from the document's $schema extension,
/// workspace config, or file name heuristics.
///
/// Returns empty reports if no schema is found or parsing failed.
/// Returns error report with proper origin if schema file is not found.
#[query(debug = "{Self}({doc_file}, {mode:?})")]
pub fn validate_against_schema_with_mode(
    db: &impl Db,
    doc_file: TextFile,
    mode: UnionTagMode,
) -> Result<ErrorReports, QueryError> {
    // Resolve schema internally
    let Some(schema) = db
        .query(ResolveSchema::new(doc_file.clone()))?
        .as_ref()
        .clone()
    else {
        return Ok(ErrorReports::new());
    };

    // Parse document
    let doc_result = db.query(ParseDocument::new(doc_file.clone()))?;
    let doc_parsed = doc_result.as_ref().clone();

    // Load and convert schema - handle errors appropriately
    let schema_result = match db.query(DocumentToSchemaQuery::new(schema.file.clone())) {
        Ok(result) => result,
        Err(QueryError::UserError(e)) => {
            // Schema conversion errors are returned as ErrorReports with proper location
            if let Some(reports) = e.downcast_ref::<ErrorReports>() {
                return Ok(reports.clone());
            }
            // Other errors (file not found, network, etc.) should be reported at $schema origin
            if let Some(origin) = &schema.origin {
                return Ok(ErrorReports::from(vec![ErrorReport::error(
                    format!("Failed to load schema: {}", e),
                    origin.clone(),
                )]));
            }
            return Err(QueryError::UserError(e));
        }
        Err(other) => return Err(other),
    };

    let result = validate_with_mode(&doc_parsed.doc, &schema_result.schema, mode);

    report_schema_validation_errors(db, doc_file, schema.file, &result.errors)
}

/// Validate document against schema with specified union tag mode and return formatted error strings.
///
/// Resolves the schema internally from the document's $schema extension,
/// workspace config, or file name heuristics.
///
/// Returns empty vec if no schema is found or parsing failed.
/// Returns formatted error messages suitable for display.
#[query(debug = "{Self}({doc_file}, {mode:?})")]
pub fn get_validation_errors_formatted_with_mode(
    db: &impl Db,
    doc_file: TextFile,
    mode: UnionTagMode,
) -> Result<Vec<String>, QueryError> {
    let reports = db.query(ValidateAgainstSchemaWithMode::new(doc_file, mode))?;

    // Format each error report individually
    let mut formatted = Vec::new();
    for report in reports.iter() {
        let single_report = ErrorReports::from(vec![report.clone()]);
        formatted.push(format_error_reports(db, &single_report, false)?);
    }

    Ok(formatted)
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
///
/// Returns `ResolvedSchemaExtension` with the path and origin for error reporting.
#[query(debug = "{Self}({file})")]
pub fn get_schema_extension(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<ResolvedSchemaExtension>, QueryError> {
    let parsed = db.query(ParseDocument::new(file.clone()))?;

    let root_id = parsed.doc.get_root_id();
    let root_ctx = parsed.doc.parse_context(root_id);

    // Check if $schema extension exists
    let Some(schema_ctx) = root_ctx.ext_optional("schema") else {
        return Ok(None);
    };

    // Try to get $schema extension as a string
    let Ok(Some(schema_path)) = root_ctx.parse_ext_optional::<String>("schema") else {
        return Ok(None); // Invalid type, diagnostics handled by get_schema_extension_diagnostics
    };

    // Get the span for the $schema value
    let node_id = schema_ctx.node_id();
    let cst = db.query(ParseCst::new(file.clone()))?;
    let span = parsed
        .origins
        .get_value_span(node_id, &cst.cst)
        .unwrap_or(InputSpan::EMPTY);

    let origin = Origin::new(file, span);

    Ok(Some(ResolvedSchemaExtension {
        path: schema_path,
        origin,
    }))
}

/// Check for schema extension errors (e.g., wrong type).
///
/// Returns diagnostics if `$schema` exists but is not a valid string.
#[query(debug = "{Self}({file})")]
pub fn get_schema_extension_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<ErrorReports, QueryError> {
    let result = db.query(ParseDocument::new(file.clone()))?;
    let parsed = result.as_ref().clone();

    let root_id = parsed.doc.get_root_id();
    let root_ctx = parsed.doc.parse_context(root_id);

    // Check if $schema extension exists
    let Some(schema_ctx) = root_ctx.ext_optional("schema") else {
        return Ok(ErrorReports::new());
    };

    // Try to parse as string
    if root_ctx.parse_ext_optional::<String>("schema").is_ok() {
        return Ok(ErrorReports::new());
    }

    // $schema exists but has wrong type - generate diagnostic
    let node_id = schema_ctx.node_id();
    let cst = db.query(ParseCst::new(file.clone()))?;
    let span = parsed.origins.get_value_span(node_id, &cst.cst);

    // FIXME: Fallback span (0, 1) points to file start instead of the actual $schema value.
    // The is_fallback flag is set, but the span itself is misleading.
    // Should find the actual span of the $schema extension key or value.
    let origin = crate::report::Origin {
        file,
        span: span.unwrap_or(eure_tree::tree::InputSpan { start: 0, end: 1 }),
        hints: Default::default(),
        is_fallback: span.is_none(),
    };

    Ok(ErrorReports::from(vec![ErrorReport::error(
        "$schema must be a string path to a schema file",
        origin,
    )]))
}

/// Resolve the schema file for a document.
///
/// Priority order:
/// 1. `$schema` extension in the document itself
/// 2. Workspace config (`Eure.eure`) schema mappings
/// 3. File name heuristics (e.g., `*.schema.eure` uses meta-schema)
///
/// Returns `None` if no schema can be determined.
/// Returns `ResolvedSchema` with the file and origin for error reporting.
#[query(debug = "{Self}({file})")]
pub fn resolve_schema(db: &impl Db, file: TextFile) -> Result<Option<ResolvedSchema>, QueryError> {
    // 1. Check $schema extension in the document
    if let Some(ext) = db.query(GetSchemaExtension::new(file.clone()))?.as_ref() {
        // Resolve relative to the document's directory (only for local files)
        if let Some(base_path) = file.as_local_path() {
            let base_dir = base_path.parent().unwrap_or(Path::new("."));
            return Ok(Some(ResolvedSchema {
                file: TextFile::resolve(&ext.path, base_dir)?,
                origin: Some(ext.origin.clone()),
            }));
        }
        // For remote files, only absolute URLs are supported
        if ext.path.starts_with("https://") {
            return Ok(Some(ResolvedSchema {
                file: TextFile::parse(&ext.path)?,
                origin: Some(ext.origin.clone()),
            }));
        }
    }

    // 2. Check workspace config (only for local files)
    if let Some(file_path) = file.as_local_path()
        && let Some(resolved) = db.query(ResolveConfig::new(file.clone()))?.as_ref()
        && let Some(schema_path) = resolved
            .config
            .schema_for_path(file_path, &resolved.config_dir)
    {
        return Ok(Some(ResolvedSchema {
            file: TextFile::resolve(&schema_path, &resolved.config_dir)?,
            origin: None, // Config-based resolution has no specific origin
        }));
    }

    // 3. File name heuristics (works for both local and remote)
    if file.ends_with(".schema.eure") {
        // Schema files are validated against the meta-schema
        return Ok(Some(ResolvedSchema {
            file: meta_schema_file(),
            origin: None, // Heuristic-based resolution has no specific origin
        }));
    }

    Ok(None)
}

/// Get the built-in meta-schema file.
fn meta_schema_file() -> TextFile {
    // The meta-schema is bundled with the application
    TextFile::parse(concat!(
        "https://eure.dev/v",
        env!("CARGO_PKG_VERSION"),
        "/schemas/eure-schema.schema.eure"
    ))
    .expect("hardcoded meta-schema URL is valid")
}

// =============================================================================
// Validation Error Span Resolution
// =============================================================================

/// Resolve the document span for a validation error.
///
/// Handles error-specific span resolution:
/// - `UnknownField`: Use key span for the unknown field name
/// - `MissingRequiredField`: Use key span if the field exists elsewhere, otherwise node span
/// - `InvalidKeyType`: Use key span for the invalid key
/// - Others: Use node span
pub fn resolve_validation_error_span(
    error: &ValidationError,
    origins: &OriginMap,
    cst: &Cst,
) -> Option<InputSpan> {
    let (node_id, _schema_node_id) = error.node_ids();

    match error {
        // For UnknownField, try to get the precise key span
        ValidationError::UnknownField { field, node_id, .. } => {
            let key = ObjectKey::String(field.clone());
            origins
                .get_key_span(*node_id, &key, cst)
                .or_else(|| origins.get_value_span(*node_id, cst))
        }

        // For InvalidKeyType, use the key span
        ValidationError::InvalidKeyType { key, node_id, .. } => origins
            .get_key_span(*node_id, key, cst)
            .or_else(|| origins.get_value_span(*node_id, cst)),

        // For MissingRequiredField, the node_id is the parent map
        // We can't point to the missing field, so use the parent span
        ValidationError::MissingRequiredField { .. } => origins.get_value_span(node_id, cst),

        // For all other errors, use the standard node span
        _ => origins.get_value_span(node_id, cst),
    }
}

/// Convert schema conversion error to ErrorReports.
fn report_schema_conversion_error(
    error: &ConversionError,
    parsed: &ParsedDocument,
    cst: &Cst,
    file: TextFile,
) -> ErrorReports {
    // FIXME: Fallback to EMPTY span when span resolution fails or for non-ParseError errors.
    // This reports errors at the file start instead of the actual error location.
    // Non-ParseError cases should attempt to provide better location information.
    let span = match error {
        ConversionError::ParseError(parse_error) => parsed
            .origins
            .get_value_span(parse_error.node_id, cst)
            .unwrap_or(InputSpan::EMPTY),
        _ => InputSpan::EMPTY,
    };

    let origin = Origin::new(file, span);
    ErrorReports::from(vec![ErrorReport::error(error.to_string(), origin)])
}
