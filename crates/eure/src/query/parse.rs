use std::sync::Arc;

use eure_document::parse::FromEure;
use eure_parol::{EureParseError, ParseResult, parse_tolerant};
use eure_tree::prelude::Cst;
use query_flow::{Db, QueryError, QueryOutput, query};

use crate::document::{EureDocument, OriginMap, cst_to_document_and_origin_map};
use crate::report::IntoErrorReports;

use super::assets::TextFile;
use super::error::FileError;

/// Result of tolerant parsing - always returns CST, optionally with error.
#[derive(Clone, PartialEq)]
pub struct ParsedCst {
    pub cst: Cst,
    pub error: Option<EureParseError>,
}

/// Step 1: Parse text content to CST (tolerant).
///
/// Always succeeds and returns a (possibly partial) CST.
/// Parse errors are included in the result for downstream processing.
#[query(debug = "{Self}({file})")]
pub fn parse_cst(db: &impl Db, file: TextFile) -> Result<ParsedCst, QueryError> {
    let text = db.asset(file.clone())?;
    let parsed = match parse_tolerant(text.get()) {
        ParseResult::Ok(cst) => ParsedCst { cst, error: None },
        ParseResult::ErrWithCst { cst, error } => ParsedCst {
            cst,
            error: Some(error),
        },
    };
    Ok(parsed)
}

#[query(debug = "{Self}({file})")]
pub fn valid_cst(db: &impl Db, file: TextFile) -> Result<Cst, QueryError> {
    let parsed = db.query(ParseCst::new(file.clone()))?;
    if let Some(error) = &parsed.error {
        return Err(FileError {
            file,
            kind: error.clone(),
        }
        .into());
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
#[query(debug = "{Self}({file})")]
pub fn parse_document(db: &impl Db, file: TextFile) -> Result<ParsedDocument, QueryError> {
    // Get CST from previous step
    let cst = db.query(ValidCst::new(file.clone()))?;
    let source = db.asset(file.clone())?;

    // Build document
    match cst_to_document_and_origin_map(source.get(), &cst) {
        Ok((doc, origins)) => Ok(ParsedDocument {
            doc: Arc::new(doc),
            origins: Arc::new(origins),
        }),
        Err(e) => Err(FileError { file, kind: e })?,
    }
}

#[query(debug = "{Self}({file})")]
pub fn parse_eure<T: for<'doc> FromEure<'doc> + QueryOutput>(
    db: &impl Db,
    file: TextFile,
) -> Result<T, QueryError>
where
    for<'doc> <T as FromEure<'doc>>::Error: IntoErrorReports,
{
    let parsed = db.query(ParseDocument::new(file.clone()))?;
    match parsed.doc.parse(parsed.doc.get_root_id()) {
        Ok(v) => Ok(v),
        Err(e) => Err(e.to_error_reports(db, file)?.into()),
    }
}
