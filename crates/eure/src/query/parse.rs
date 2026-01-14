use std::sync::Arc;

use eure_parol::{EureParseError, ParseResult, parse_tolerant};
use eure_tree::prelude::Cst;
use query_flow::{Db, QueryError, query};

use crate::document::{
    DocumentConstructionError, EureDocument, OriginMap, cst_to_document_and_origin_map,
};
use crate::query::error::EureQueryError;
use crate::report::{ErrorReport, ErrorReports, Origin};
use eure_tree::tree::InputSpan;

use super::assets::{TextFile, TextFileContent};

/// Result of tolerant parsing - always returns CST, optionally with error.
#[derive(Clone, PartialEq)]
pub struct ParsedCst {
    pub cst: Cst,
    pub error: Option<EureParseError>,
}

pub fn read_text_file(db: &impl Db, file: TextFile) -> Result<Arc<TextFileContent>, QueryError> {
    db.asset(file.clone())
}

/// Step 1: Parse text content to CST (tolerant).
///
/// Always succeeds and returns a (possibly partial) CST.
/// Parse errors are included in the result for downstream processing.
#[query]
pub fn parse_cst(db: &impl Db, file: TextFile) -> Result<ParsedCst, QueryError> {
    let text = read_text_file(db, file.clone())?;
    let parsed = match parse_tolerant(text.get()) {
        ParseResult::Ok(cst) => ParsedCst { cst, error: None },
        ParseResult::ErrWithCst { cst, error } => ParsedCst {
            cst,
            error: Some(error),
        },
    };
    Ok(parsed)
}

#[query]
pub fn valid_cst(db: &impl Db, file: TextFile) -> Result<Cst, QueryError> {
    let parsed = db.query(ParseCst::new(file.clone()))?;
    if let Some(error) = &parsed.error {
        return Err(EureQueryError::ParseError(error.clone()).into());
    }
    Ok(parsed.cst.clone())
}

/// Parsed document with CST and OriginMap for error reporting.
#[derive(Clone, PartialEq)]
pub struct ParsedDocument {
    pub doc: Arc<EureDocument>,
    pub origins: Arc<OriginMap>,
}

/// Step 2: Build EureDocument from CST.
///
/// Returns `None` if file not found or if there was a parse error.
/// Returns `UserError` if document construction fails on a valid CST.
#[query]
pub fn parse_document(db: &impl Db, file: TextFile) -> Result<ParsedDocument, QueryError> {
    // Get CST from previous step
    let cst = db.query(ValidCst::new(file.clone()))?;
    let source = read_text_file(db, file.clone())?;

    // Build document
    match cst_to_document_and_origin_map(source.get(), &cst) {
        Ok((doc, origins)) => Ok(ParsedDocument {
            doc: Arc::new(doc),
            origins: Arc::new(origins),
        }),
        Err(e) => Err(ErrorReports::from(vec![report_document_error(
            &e.error,
            file,
            &cst,
            &e.partial_origins,
        )]))?,
    }
}

/// Convert a document construction error to an ErrorReport.
/// Uses OriginMap for precise key span resolution when available.
fn report_document_error(
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
