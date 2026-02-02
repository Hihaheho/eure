use std::sync::Arc;

use eure_env::ConfigError;
use eure_parol::EureParseError;
use eure_schema::convert::ConversionError;
use query_flow::{Cachable, Db, Query, QueryError, QueryResultExt as _, query};

use super::assets::{OpenDocuments, OpenDocumentsList, TextFile};
use super::error::{EureQueryError, FileError};
use super::parse::{ParseCst, ParseDocument};
use super::schema::{
    DocumentToSchemaQuery, GetSchemaExtension, GetSchemaExtensionDiagnostics, ResolveSchema,
    ValidateAgainstSchema,
};
use crate::document::DocumentConstructionErrorWithOriginMap;
use crate::report::{
    ErrorReport, ErrorReports, Origin, format_error_reports, report_config_error,
    report_conversion_error, report_document_error, report_parse_error, report_schema_load_error,
};

/// Wraps a query and converts all user errors to `ErrorReports`.
///
/// This handles:
/// - `ErrorReports` - passed through as-is
/// - `FileError<ConversionError>` - converted with source location from error's file
/// - `FileError<ConfigError>` - converted with source location from error's file
/// - `FileError<EureParseError>` - converted with source location from error's file
/// - `EureQueryError` - propagated as-is (system/environment errors)
///
/// Other user errors propagate unchanged. Queries that produce errors should
/// wrap them in `FileError<T>` to enable conversion to `ErrorReports`.
///
/// System errors (Suspend, Cycle, etc.) are propagated unchanged.
/// Use `downcast_err::<ErrorReports>()` on the result to handle user errors.
#[query(debug = "{Self}({query:?})")]
pub fn with_error_reports<T>(db: &impl Db, query: T) -> Result<Arc<T::Output>, QueryError>
where
    T: Query + std::fmt::Debug + Cachable,
    T::Output: PartialEq,
{
    let result = db.query(query);

    // Try ErrorReports first - already in the right format
    match result.downcast_err::<ErrorReports>() {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(e.into()),
        Err(original) => {
            // Try FileError<ConversionError>
            if let Some(error) = original.downcast_ref::<FileError<ConversionError>>()
                && let (Ok(cst), Ok(parsed)) = (
                    db.query(ParseCst::new(error.file.clone())),
                    db.query(ParseDocument::new(error.file.clone())),
                )
            {
                let report = report_conversion_error(
                    &error.kind,
                    error.file.clone(),
                    &cst.cst,
                    &parsed.origins,
                );
                return Err(ErrorReports::from(vec![report]).into());
            }

            // Try FileError<ConfigError>
            if let Some(error) = original.downcast_ref::<FileError<ConfigError>>()
                && let (Ok(cst), Ok(parsed)) = (
                    db.query(ParseCst::new(error.file.clone())),
                    db.query(ParseDocument::new(error.file.clone())),
                )
            {
                let reports =
                    report_config_error(&error.kind, error.file.clone(), &cst.cst, &parsed.origins);
                return Err(reports.into());
            }

            // Try FileError<EureParseError>
            if let Some(error) = original.downcast_ref::<FileError<EureParseError>>() {
                let reports = report_parse_error(&error.kind, error.file.clone());
                return Err(reports.into());
            }

            // Try FileError<Box<DocumentConstructionErrorWithOriginMap>>
            if let Some(error) =
                original.downcast_ref::<FileError<Box<DocumentConstructionErrorWithOriginMap>>>()
                && let Ok(cst) = db.query(ParseCst::new(error.file.clone()))
            {
                let report = report_document_error(
                    &error.kind.error,
                    error.file.clone(),
                    &cst.cst,
                    &error.kind.partial_origins,
                );
                return Err(ErrorReports::from(vec![report]).into());
            }

            // EureQueryError (ContentNotFound, HostNotAllowed, etc.) propagate as-is
            // These are system/environment errors, not user source errors
            if original.downcast_ref::<EureQueryError>().is_some() {
                return Err(original);
            }

            // Other errors propagate unchanged
            // Queries should wrap errors in FileError<T> if they need conversion
            Err(original)
        }
    }
}

/// Get all error reports for a single file.
///
/// Returns structured ErrorReports that can be formatted for display.
/// This is the underlying query used by `GetFileDiagnostics`.
///
/// Includes:
/// - Parse errors
/// - Document construction errors
/// - Schema extension errors
/// - Validation errors (if this is a document with a schema)
/// - Schema conversion errors (if this file is referenced as a schema)
#[query(debug = "{Self}({file})")]
pub fn get_file_error_reports(db: &impl Db, file: TextFile) -> Result<ErrorReports, QueryError> {
    let mut reports = ErrorReports::new();

    // 1. Parse errors
    let parsed = db.query(ParseCst::new(file.clone()))?;
    if let Some(error) = &parsed.error {
        reports.extend(report_parse_error(error, file.clone()));
        return Ok(reports); // Stop here if parse failed
    }

    // 2. Document construction errors
    let doc_result = db.query(WithErrorReports::new(ParseDocument::new(file.clone())));
    if let Err(e) = &doc_result {
        if let Some(r) = e.downcast_ref::<ErrorReports>() {
            reports.extend(r.clone());
        }
        return Ok(reports); // Stop here if doc construction failed
    }

    // 3. Schema extension errors ($schema wrong type)
    reports.extend(
        db.query(GetSchemaExtensionDiagnostics::new(file.clone()))?
            .iter()
            .cloned(),
    );

    // 4. Validation errors - check schema validity first, then validate
    let resolved = match db.query(ResolveSchema::new(file.clone())) {
        Ok(r) => r,
        Err(QueryError::UserError(e)) => {
            if let Some(qe) = e.downcast_ref::<EureQueryError>() {
                // Try to get the $schema span for better error positioning
                let schema_ext = db.query(GetSchemaExtension::new(file.clone()))?;
                let origin = schema_ext
                    .as_ref()
                    .as_ref()
                    .map(|ext| ext.origin.clone())
                    .unwrap_or_else(|| Origin::file_start(file.clone()));
                reports.push(report_schema_load_error(qe, origin));
            }
            return Ok(reports);
        }
        Err(e) => return Err(e),
    };

    if let Some(resolved) = resolved.as_ref() {
        let schema_origin = resolved
            .origin
            .clone()
            .unwrap_or_else(|| Origin::file_start(file.clone()));

        // Check if schema can be loaded and converted
        match db.query(WithErrorReports::new(DocumentToSchemaQuery::new(
            resolved.file.clone(),
        ))) {
            Ok(_) => {
                // Schema is valid, run validation
                let validation = db.query(ValidateAgainstSchema::new(file.clone()))?;
                reports.extend(validation.iter().cloned());
            }
            Err(QueryError::UserError(e)) => {
                // Schema failed - show appropriate error/warning
                if let Some(qe) = e.downcast_ref::<EureQueryError>() {
                    reports.push(report_schema_load_error(qe, schema_origin));
                } else if e.downcast_ref::<ErrorReports>().is_some() {
                    reports.push(ErrorReport::warning(
                        "Schema has errors, validation skipped",
                        schema_origin,
                    ));
                }
            }
            Err(e) => return Err(e),
        }
    }

    // 5. Schema conversion errors (only if this file is a schema)
    let schema_files = collect_schema_files(db)?;
    if schema_files.contains(&file)
        && let Err(e) = db.query(WithErrorReports::new(DocumentToSchemaQuery::new(
            file.clone(),
        )))
        && let Some(r) = e.downcast_ref::<ErrorReports>()
    {
        reports.extend(r.clone());
    }

    Ok(reports)
}

/// Collect all local schema files referenced by open documents.
/// This is a helper to avoid duplicating CollectSchemaFiles logic.
fn collect_schema_files(db: &impl Db) -> Result<indexmap::IndexSet<TextFile>, QueryError> {
    let open_docs: Arc<OpenDocumentsList> = db.asset(OpenDocuments)?;
    let mut schemas = indexmap::IndexSet::new();

    for file in open_docs.0.iter() {
        let resolved = match db.query(ResolveSchema::new(file.clone())) {
            Ok(r) => r,
            Err(QueryError::UserError(_)) => continue,
            Err(e) => return Err(e),
        };

        let Some(resolved) = resolved.as_ref().as_ref() else {
            continue;
        };

        if !resolved.file.is_local() {
            continue;
        }

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

#[query(debug = "{Self}({query:?})")]
pub fn with_formatted_error<T>(
    db: &impl Db,
    query: T,
    styled: bool,
) -> Result<Result<Arc<T::Output>, String>, query_flow::QueryError>
where
    T: Query + std::fmt::Debug + Cachable,
    T::Output: PartialEq,
{
    match db
        .query(WithErrorReports::new(query))
        .downcast_err::<ErrorReports>()?
    {
        Ok(output) => Ok(Ok(output)),
        Err(reports) => Ok(Err(format_error_reports(db, reports.get(), styled)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{OpenDocuments, OpenDocumentsList, TextFileContent, build_runtime};
    use query_flow::DurabilityLevel;
    use std::path::PathBuf;

    #[test]
    fn test_file_error_reports_duplicate_key() {
        let runtime = build_runtime();
        let file = TextFile::from_path(PathBuf::from("test.eure"));

        // Set up open documents list
        runtime.resolve_asset(
            OpenDocuments,
            OpenDocumentsList(vec![file.clone()]),
            DurabilityLevel::Volatile,
        );

        // Set up file with duplicate keys
        runtime.resolve_asset(
            file.clone(),
            TextFileContent("name = \"Alice\"\nname = \"Bob\"".to_string()),
            DurabilityLevel::Volatile,
        );

        // Get file error reports
        let reports = runtime
            .query(GetFileErrorReports::new(file.clone()))
            .unwrap();

        // Should have exactly one report for the duplicate key
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].primary_origin.file, file);
        assert!(
            reports[0]
                .title
                .contains("Binding target already has a value")
        );
    }

    #[test]
    fn test_file_error_reports_valid_document() {
        let runtime = build_runtime();
        let file = TextFile::from_path(PathBuf::from("test.eure"));

        // Set up open documents list
        runtime.resolve_asset(
            OpenDocuments,
            OpenDocumentsList(vec![file.clone()]),
            DurabilityLevel::Volatile,
        );

        // Set up valid file
        runtime.resolve_asset(
            file.clone(),
            TextFileContent("name = \"Alice\"".to_string()),
            DurabilityLevel::Volatile,
        );

        // Get file error reports
        let reports = runtime
            .query(GetFileErrorReports::new(file.clone()))
            .unwrap();

        // Should have no reports
        assert_eq!(reports.len(), 0);
    }
}
