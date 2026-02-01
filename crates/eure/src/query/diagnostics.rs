//! Diagnostic collection queries.
//!
//! This module provides focused diagnostic queries following SRP:
//! - `get_parse_diagnostics`: Parse errors for a file
//! - `get_document_construction_diagnostics`: Document construction errors for a file
//! - `get_schema_conversion_diagnostics`: Schema conversion errors for a file
//! - `get_validation_diagnostics`: Validation errors for a file
//! - `get_file_diagnostics`: Composition of all diagnostics for a file
//! - `collect_diagnostic_targets`: All files needing diagnostics
//! - `collect_schema_files`: Local schema files referenced by open documents
//! - `get_all_diagnostics`: All diagnostics for all targets (CLI use)

use indexmap::{IndexMap, IndexSet};
use std::sync::Arc;

use eure_parol::EureParseError;
use query_flow::{Db, QueryError, QueryResultExt, query};

use crate::query::error::EureQueryError;
use crate::query::parse::{ParseCst, ParseDocument};
use crate::report::{ErrorReport, ErrorReports};

use super::assets::{OpenDocuments, OpenDocumentsList, TextFile};
use super::report::WithErrorReports;
use super::schema::{
    DocumentToSchemaQuery, GetSchemaExtension, GetSchemaExtensionDiagnostics, ResolveSchema,
    ValidateAgainstSchema,
};

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// A diagnostic message with source location.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticMessage {
    /// The file this diagnostic belongs to.
    pub file: TextFile,
    /// Start byte offset in the source.
    pub start: usize,
    /// End byte offset in the source.
    pub end: usize,
    /// The diagnostic message.
    pub message: String,
    /// Severity of the diagnostic.
    pub severity: DiagnosticSeverity,
}

// =============================================================================
// Focused Diagnostic Queries
// =============================================================================

/// Parse errors only. File-scoped.
///
/// Returns parse errors for the given file.
#[query(debug = "{Self}({file})")]
pub fn get_parse_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<Vec<DiagnosticMessage>, QueryError> {
    let parsed = db.query(ParseCst::new(file.clone()))?;

    match &parsed.error {
        Some(error) => Ok(parse_error_to_diagnostics(error, file)),
        None => Ok(vec![]),
    }
}

/// Schema conversion errors. File-scoped.
///
/// Tries to convert file as a schema. Returns conversion errors if any.
/// Returns empty if parsing failed (parse errors reported separately).
#[query(debug = "{Self}({file})")]
pub fn get_schema_conversion_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<Vec<DiagnosticMessage>, QueryError> {
    let parsed = db.query(ParseCst::new(file.clone()))?;
    if parsed.error.is_some() {
        return Ok(vec![]);
    }

    match db
        .query(WithErrorReports::new(DocumentToSchemaQuery::new(
            file.clone(),
        )))
        .downcast_err::<ErrorReports>()?
    {
        Ok(_) => Ok(vec![]),
        Err(reports) => Ok(reports
            .get()
            .iter()
            .map(error_report_to_diagnostic)
            .collect()),
    }
}

/// Document construction errors. File-scoped.
///
/// Returns document construction errors (like duplicate keys, invalid binding targets).
/// Returns empty if parsing failed (parse errors reported separately).
#[query(debug = "{Self}({file})")]
pub fn get_document_construction_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<Vec<DiagnosticMessage>, QueryError> {
    // Check parse errors first - skip if CST is invalid
    let parsed = db.query(ParseCst::new(file.clone()))?;
    if parsed.error.is_some() {
        return Ok(vec![]); // Parse errors reported separately
    }

    // Try to construct document from valid CST
    match db
        .query(WithErrorReports::new(ParseDocument::new(file.clone())))
        .downcast_err::<ErrorReports>()?
    {
        Err(e) => {
            // Document construction errors - convert to diagnostics
            let file_reports: Vec<_> = e
                .get()
                .iter()
                .filter(|r| r.primary_origin.file == file)
                .collect();
            Ok(file_reports
                .iter()
                .map(|r| error_report_to_diagnostic(r))
                .collect())
        }
        Ok(_) => {
            // Document construction succeeded
            Ok(vec![])
        }
    }
}

/// Validation errors. File-scoped to document.
///
/// Returns validation errors when checking document against its schema.
/// Returns empty if:
/// - Parse errors exist (reported separately via `get_parse_diagnostics`)
/// - Document construction errors exist (reported separately via `get_document_construction_diagnostics`)
/// - No schema is configured
///
/// When schema has conversion errors:
/// - Emits a warning in the document at the $schema value span (if explicit)
/// - Or at span (0, 1) if schema is discovered implicitly (workspace config)
#[query(debug = "{Self}({doc_file})")]
pub fn get_validation_diagnostics(
    db: &impl Db,
    doc_file: TextFile,
) -> Result<Vec<DiagnosticMessage>, QueryError> {
    // Check parse errors first
    let parsed = db.query(ParseCst::new(doc_file.clone()))?;
    if parsed.error.is_some() {
        return Ok(vec![]); // Parse errors reported separately
    }

    // Check document construction errors - skip validation if doc can't be constructed
    let doc_construction_diags =
        db.query(GetDocumentConstructionDiagnostics::new(doc_file.clone()))?;
    if !doc_construction_diags.is_empty() {
        return Ok(vec![]); // Document construction errors reported separately
    }

    let mut diagnostics = Vec::new();

    // Schema extension errors ($schema wrong type)
    let schema_ext_errors = db.query(GetSchemaExtensionDiagnostics::new(doc_file.clone()))?;
    diagnostics.extend(schema_ext_errors.iter().map(error_report_to_diagnostic));

    // Resolve schema to check if it exists and has errors
    let resolved_schema = match db
        .query(ResolveSchema::new(doc_file.clone()))
        .downcast_err::<EureQueryError>()?
    {
        Ok(arc) => match arc.as_ref() {
            Some(schema) => schema.clone(),
            None => return Ok(diagnostics), // No schema, no validation
        },
        Err(e) => {
            // Error resolving schema - report at $schema span if available
            if let Some(ext) = db
                .query(GetSchemaExtension::new(doc_file.clone()))?
                .as_ref()
                && let EureQueryError::InvalidUrl { url, reason } = e.get()
            {
                let start = ext.origin.span.start as usize;
                let end = ext.origin.span.end as usize;

                diagnostics.push(DiagnosticMessage {
                    file: doc_file.clone(),
                    start,
                    end,
                    message: format!("Invalid URL '{}': {}", url, reason),
                    severity: DiagnosticSeverity::Error,
                });
                return Ok(diagnostics);
            }
            return Err(e.into());
        }
    };

    // Check if schema can be converted (detect schema errors)
    // WithErrorReports converts ConversionError to ErrorReports
    // EureQueryError (ContentNotFound, HostNotAllowed, etc.) propagates as-is
    let schema_result = db.query(WithErrorReports::new(DocumentToSchemaQuery::new(
        resolved_schema.file.clone(),
    )));
    match schema_result {
        Ok(_) => {
            // Schema is valid, proceed with validation
            match db
                .query(ValidateAgainstSchema::new(doc_file.clone()))
                .downcast_err::<ErrorReports>()?
            {
                Ok(reports) => {
                    // Filter to only include diagnostics for this file
                    let file_reports: Vec<_> = reports
                        .iter()
                        .filter(|r| r.primary_origin.file == doc_file)
                        .collect();
                    diagnostics.extend(file_reports.iter().map(|r| error_report_to_diagnostic(r)));
                }
                Err(e) => {
                    let file_reports: Vec<_> = e
                        .get()
                        .iter()
                        .filter(|r| r.primary_origin.file == doc_file)
                        .collect();
                    diagnostics.extend(file_reports.iter().map(|r| error_report_to_diagnostic(r)));
                }
            }
        }
        Err(QueryError::UserError(e)) => {
            // Schema has errors - emit a warning/error in the document
            // Location depends on whether schema was explicit ($schema) or implicit (config)
            let (start, end) = if let Some(origin) = &resolved_schema.origin {
                // Explicit: use the $schema value span
                (origin.span.start as usize, origin.span.end as usize)
            } else {
                // Implicit: use file start
                (0, 1)
            };

            // Check for EureQueryError (system errors) first
            if let Some(query_err) = e.downcast_ref::<EureQueryError>() {
                match query_err {
                    EureQueryError::ContentNotFound(text_file) => {
                        diagnostics.push(DiagnosticMessage {
                            file: doc_file.clone(),
                            start,
                            end,
                            message: format!("Failed to load schema file: {}", text_file),
                            severity: DiagnosticSeverity::Error,
                        });
                    }
                    EureQueryError::HostNotAllowed { url, host } => {
                        diagnostics.push(DiagnosticMessage {
                            file: doc_file.clone(),
                            start,
                            end,
                            message: format!(
                                "Remote host not allowed: {} (URL: {}). If you trust this host, add it to security.allowed-hosts in Eure.eure",
                                host, url
                            ),
                            severity: DiagnosticSeverity::Error,
                        });
                    }
                    EureQueryError::InvalidUrl { url, reason } => {
                        diagnostics.push(DiagnosticMessage {
                            file: doc_file.clone(),
                            start,
                            end,
                            message: format!("Invalid URL '{}': {}", url, reason),
                            severity: DiagnosticSeverity::Error,
                        });
                    }
                    _ => {
                        // Other EureQueryError variants
                        diagnostics.push(DiagnosticMessage {
                            file: doc_file.clone(),
                            start,
                            end,
                            message: format!("Schema error: {}", query_err),
                            severity: DiagnosticSeverity::Error,
                        });
                    }
                }
            } else if e.downcast_ref::<ErrorReports>().is_some() {
                // Schema has conversion errors (ConversionError converted to ErrorReports)
                diagnostics.push(DiagnosticMessage {
                    file: doc_file.clone(),
                    start,
                    end,
                    message: "Schema has errors, validation skipped".to_string(),
                    severity: DiagnosticSeverity::Warning,
                });
            } else {
                // Other unexpected error
                diagnostics.push(DiagnosticMessage {
                    file: doc_file.clone(),
                    start,
                    end,
                    message: format!("Schema error: {}", e),
                    severity: DiagnosticSeverity::Error,
                });
            }
        }
        Err(other) => return Err(other),
    }

    Ok(diagnostics)
}

// =============================================================================
// Composition Queries
// =============================================================================

/// Get all diagnostics for a single file.
///
/// Includes:
/// - Parse diagnostics
/// - Document construction diagnostics
/// - Validation diagnostics
/// - Schema conversion diagnostics (if this file is referenced as a schema)
#[query(debug = "{Self}({file})")]
pub fn get_file_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<Vec<DiagnosticMessage>, QueryError> {
    let mut diagnostics = Vec::new();

    // Parse diagnostics
    let parse_diags = db.query(GetParseDiagnostics::new(file.clone()))?;
    diagnostics.extend(parse_diags.iter().cloned());

    // Document construction diagnostics
    let doc_construction_diags = db.query(GetDocumentConstructionDiagnostics::new(file.clone()))?;
    diagnostics.extend(doc_construction_diags.iter().cloned());

    // Validation diagnostics
    let validation_diags = db.query(GetValidationDiagnostics::new(file.clone()))?;
    diagnostics.extend(validation_diags.iter().cloned());

    // Schema conversion if this file is a schema
    let schema_files = db.query(CollectSchemaFiles::new())?;
    if schema_files.contains(&file) {
        let schema_diags = db.query(GetSchemaConversionDiagnostics::new(file.clone()))?;
        diagnostics.extend(schema_diags.iter().cloned());
    }

    Ok(diagnostics)
}

// =============================================================================
// Collection Queries
// =============================================================================

/// Collect all diagnostic targets: open documents + referenced schema files.
///
/// This is the primary query for determining which files need diagnostics.
/// LSP and CLI should poll this to discover all relevant files.
#[query]
pub fn collect_diagnostic_targets(db: &impl Db) -> Result<IndexSet<TextFile>, QueryError> {
    let mut targets = IndexSet::new();

    // 1. Open documents
    let open_docs: Arc<OpenDocumentsList> = db.asset(OpenDocuments)?;
    targets.extend(open_docs.0.iter().cloned());

    // 2. Schema files referenced by open documents
    let schema_files = db.query(CollectSchemaFiles::new())?;
    targets.extend(schema_files.iter().cloned());

    // TODO: Files from workspace config

    Ok(targets)
}

/// Collect all local schema files referenced by open documents.
///
/// Discovers schemas from $schema extensions in documents.
/// Only includes local files (not remote URLs) that exist - remote schemas are not diagnosed.
/// Tolerates parse/construction errors in documents - files with errors are skipped.
/// Non-existent schema files are not included (errors are reported at $schema location instead).
#[query]
pub fn collect_schema_files(db: &impl Db) -> Result<IndexSet<TextFile>, QueryError> {
    use crate::query::error::EureQueryError;

    // Use open documents directly to avoid circular dependency with CollectDiagnosticTargets
    let open_docs: Arc<OpenDocumentsList> = db.asset(OpenDocuments)?;
    let mut schemas = IndexSet::new();

    for file in open_docs.0.iter() {
        // ResolveSchema may fail for files with parse/construction errors - that's ok, skip them
        let resolved = match db.query(ResolveSchema::new(file.clone())) {
            Ok(r) => r,
            Err(QueryError::UserError(_)) => continue, // Parse/construction errors - skip this file
            Err(e) => return Err(e),                   // System errors - propagate
        };

        let Some(resolved) = resolved.as_ref().as_ref() else {
            continue; // No schema reference
        };

        // Only include local files (not remote URLs)
        if !resolved.file.is_local() {
            continue;
        }

        // Only include files that actually have content
        // Missing files are reported as errors at $schema location via ValidateAgainstSchema
        if db
            .asset(resolved.file.clone())
            .downcast_err::<EureQueryError>()?
            .is_err()
        {
            continue;
        }
        schemas.insert(resolved.file.clone());
    }

    Ok(schemas)
}

// =============================================================================
// Global Query (CLI use)
// =============================================================================

/// Global diagnostics query for CLI use.
///
/// Returns all diagnostics for all targets (open docs + schema files).
/// For LSP, use per-file polling with `GetFileDiagnostics` instead.
#[query]
pub fn get_all_diagnostics(
    db: &impl Db,
) -> Result<IndexMap<TextFile, Vec<DiagnosticMessage>>, QueryError> {
    let all_files = db.query(CollectDiagnosticTargets::new())?;

    let mut result = IndexMap::new();
    for file in all_files.iter() {
        let diags = db.query(GetFileDiagnostics::new(file.clone()))?;
        result.insert(file.clone(), diags.as_ref().clone());
    }

    Ok(result)
}

// =============================================================================
// Legacy Compatibility (to be removed)
// =============================================================================

/// Collect all diagnostics for a document.
///
/// **DEPRECATED**: Use `GetFileDiagnostics` instead.
///
/// This includes:
/// - Parse errors (from tolerant parsing)
/// - Schema validation errors (if a schema is resolved)
///
/// Returns an empty vec if the file cannot be parsed.
#[query(debug = "{Self}({file})")]
pub fn get_diagnostics(db: &impl Db, file: TextFile) -> Result<Vec<DiagnosticMessage>, QueryError> {
    let mut diagnostics = Vec::new();

    let parse_diags = db.query(GetParseDiagnostics::new(file.clone()))?;
    diagnostics.extend(parse_diags.iter().cloned());

    let validation_diags = db.query(GetValidationDiagnostics::new(file.clone()))?;
    diagnostics.extend(validation_diags.iter().cloned());

    Ok(diagnostics)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert parse errors to diagnostic messages.
fn parse_error_to_diagnostics(error: &EureParseError, file: TextFile) -> Vec<DiagnosticMessage> {
    error
        .entries
        .iter()
        .map(|entry| {
            // FIXME: Fallback to file start (0, 1) when span is missing.
            // This causes errors to be reported at the wrong location.
            // Should propagate the missing span information or use a better heuristic.
            let (start, end) = entry
                .span
                .map(|s| (s.start as usize, s.end as usize))
                .unwrap_or((0, 1));

            DiagnosticMessage {
                file: file.clone(),
                start,
                end,
                message: entry.message.clone(),
                severity: DiagnosticSeverity::Error,
            }
        })
        .collect()
}

/// Convert an error report to a diagnostic message.
fn error_report_to_diagnostic(report: &ErrorReport) -> DiagnosticMessage {
    DiagnosticMessage {
        file: report.primary_origin.file.clone(),
        start: report.primary_origin.span.start as usize,
        end: report.primary_origin.span.end as usize,
        message: report.title.to_string(),
        severity: DiagnosticSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{TextFile, TextFileContent, build_runtime};
    use query_flow::DurabilityLevel;
    use std::path::PathBuf;

    #[test]
    fn test_diagnostic_severity_eq() {
        assert_eq!(DiagnosticSeverity::Error, DiagnosticSeverity::Error);
        assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_document_construction_diagnostics_duplicate_key() {
        let runtime = build_runtime();
        let file = TextFile::from_path(PathBuf::from("test.eure"));

        // Set up file with duplicate keys
        runtime.resolve_asset(
            file.clone(),
            TextFileContent("name = \"Alice\"\nname = \"Bob\"".to_string()),
            DurabilityLevel::Volatile,
        );

        // Get document construction diagnostics
        let diags = runtime
            .query(GetDocumentConstructionDiagnostics::new(file.clone()))
            .unwrap();

        // Should have exactly one diagnostic for the duplicate key
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].file, file);
        assert!(
            diags[0]
                .message
                .contains("Binding target already has a value")
        );
        assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn test_document_construction_diagnostics_valid_document() {
        let runtime = build_runtime();
        let file = TextFile::from_path(PathBuf::from("test.eure"));

        // Set up valid file
        runtime.resolve_asset(
            file.clone(),
            TextFileContent("name = \"Alice\"".to_string()),
            DurabilityLevel::Volatile,
        );

        // Get document construction diagnostics
        let diags = runtime
            .query(GetDocumentConstructionDiagnostics::new(file.clone()))
            .unwrap();

        // Should have no diagnostics
        assert_eq!(diags.len(), 0);
    }
}
