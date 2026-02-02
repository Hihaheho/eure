//! Diagnostic collection queries.
//!
//! This module provides diagnostic queries:
//! - `get_file_diagnostics`: All diagnostics for a file (thin wrapper around GetFileErrorReports)
//! - `collect_diagnostic_targets`: All files needing diagnostics
//! - `collect_schema_files`: Local schema files referenced by open documents
//! - `get_all_diagnostics`: All diagnostics for all targets (CLI use)

use indexmap::{IndexMap, IndexSet};
use std::sync::Arc;

use query_flow::{Db, QueryError, QueryResultExt, query};

use crate::report::{ErrorReport, Severity};

use super::assets::{OpenDocuments, OpenDocumentsList, TextFile};
use super::report::GetFileErrorReports;
use super::schema::ResolveSchema;

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
// Composition Queries
// =============================================================================

/// Get all diagnostics for a single file.
///
/// Thin wrapper around `GetFileErrorReports` that converts to `DiagnosticMessage`.
#[query(debug = "{Self}({file})")]
pub fn get_file_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<Vec<DiagnosticMessage>, QueryError> {
    let reports = db.query(GetFileErrorReports::new(file))?;
    Ok(reports.iter().map(error_report_to_diagnostic).collect())
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
// Helper Functions
// =============================================================================

/// Convert an error report to a diagnostic message.
fn error_report_to_diagnostic(report: &ErrorReport) -> DiagnosticMessage {
    DiagnosticMessage {
        file: report.primary_origin.file.clone(),
        start: report.primary_origin.span.start as usize,
        end: report.primary_origin.span.end as usize,
        message: report.title.to_string(),
        severity: match report.severity {
            Severity::Error => DiagnosticSeverity::Error,
            Severity::Warning => DiagnosticSeverity::Warning,
            Severity::Note => DiagnosticSeverity::Info,
            Severity::Hint => DiagnosticSeverity::Hint,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_severity_eq() {
        assert_eq!(DiagnosticSeverity::Error, DiagnosticSeverity::Error);
        assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
    }
}
