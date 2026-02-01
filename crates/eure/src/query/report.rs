use std::sync::Arc;

use eure_env::ConfigError;
use eure_parol::EureParseError;
use eure_schema::convert::ConversionError;
use query_flow::{Cachable, Db, Query, QueryError, QueryResultExt as _, query};

use super::error::{EureQueryError, FileError};
use super::parse::{ParseCst, ParseDocument};
use crate::document::DocumentConstructionErrorWithOriginMap;
use crate::report::{
    ErrorReports, format_error_reports, report_config_error, report_conversion_error,
    report_document_error, report_parse_error,
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
