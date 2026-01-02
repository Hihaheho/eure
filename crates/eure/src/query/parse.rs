use std::sync::Arc;

use eure_parol::{EureParseError, ParseResult, parse, parse_tolerant};
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
    pub cst: Arc<Cst>,
    pub error: Option<EureParseError>,
}

pub fn read_text_file(db: &impl Db, file: TextFile) -> Result<String, QueryError> {
    let content: Arc<TextFileContent> = db.asset(file.clone())?.suspend()?;
    match content.as_ref() {
        TextFileContent::NotFound => Err(EureQueryError::ContentNotFound(file).into()),
        TextFileContent::Content(text) => Ok(text.clone()),
    }
}

/// Step 1: Parse text content to CST (tolerant).
///
/// Always succeeds and returns a (possibly partial) CST.
/// Parse errors are included in the result for downstream processing.
#[query]
pub fn parse_cst(db: &impl Db, file: TextFile) -> Result<ParsedCst, QueryError> {
    let text = read_text_file(db, file.clone())?;
    let parsed = match parse_tolerant(&text) {
        ParseResult::Ok(cst) => ParsedCst {
            cst: Arc::new(cst),
            error: None,
        },
        ParseResult::ErrWithCst { cst, error } => ParsedCst {
            cst: Arc::new(cst),
            error: Some(error),
        },
    };
    Ok(parsed)
}

#[query]
pub fn valid_cst(db: &impl Db, file: TextFile) -> Result<Cst, QueryError> {
    let text = read_text_file(db, file.clone())?;
    let cst = parse(&text).map_err(EureQueryError::ParseError)?;
    Ok(cst)
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
    match cst_to_document_and_origin_map(&source, &cst) {
        Ok((doc, origins)) => Ok(ParsedDocument {
            doc: Arc::new(doc),
            origins: Arc::new(origins),
        }),
        Err(e) => Err(ErrorReports::from(vec![report_document_error(
            &e, file, &cst,
        )]))?,
    }
}

/// Convert a document construction error to an ErrorReport.
fn report_document_error(
    error: &DocumentConstructionError,
    file: TextFile,
    cst: &Cst,
) -> ErrorReport {
    let span = error.span(cst).unwrap_or(InputSpan::EMPTY);
    ErrorReport::error(error.to_string(), Origin::new(file, span))
}
