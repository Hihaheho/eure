//! Query-flow queries for eure-mark.

use std::sync::Arc;

use eure::query::{ParseCst, ParseDocument, TextFile};
use eure::report::{ErrorReports, Origin};
use eure_document::document::EureDocument;
use eure_document::parse::ParseError;
use eure_tree::prelude::Cst;
use eure_tree::tree::InputSpan;
use query_flow::{Db, QueryError, query};

use crate::document::EumdDocument;
use crate::report::EumdReportContext;
use crate::{check_references_with_spans, report_check_errors};

/// Parsed EumdDocument with CST and OriginMap for error reporting.
#[derive(Clone, PartialEq)]
pub struct ParsedEumd {
    /// The parsed EumdDocument
    pub doc: Arc<EumdDocument>,
    /// The underlying EureDocument (for span resolution)
    pub eure_doc: Arc<EureDocument>,
    /// CST for error formatting
    pub cst: Arc<Cst>,
    /// Origin map for error formatting
    pub origins: Arc<eure::document::OriginMap>,
}

/// Parse an EumdDocument from a file.
///
/// This query combines:
/// - ParseDocument (which internally handles CST parsing)
/// - EumdDocument parsing from the document
///
/// Returns errors via ErrorReports if parsing fails.
#[query]
pub fn parse_eumd_document(db: &impl Db, file: TextFile) -> Result<ParsedEumd, QueryError> {
    // Parse the document
    let parsed_doc = db.query(ParseDocument::new(file.clone()))?;

    // Get CST for error formatting (cached from ParseDocument's internal call)
    let parsed_cst = db.query(ParseCst::new(file.clone()))?;

    // Parse EumdDocument from the document
    let root_id = parsed_doc.doc.get_root_id();
    let eumd_doc: EumdDocument = parsed_doc.doc.parse(root_id).map_err(|e: ParseError| {
        // Convert parse error to ErrorReports
        let span = parsed_doc
            .origins
            .get_node_span(e.node_id, &parsed_cst.cst)
            .unwrap_or(InputSpan::EMPTY);
        let origin = Origin::new(file.clone(), span);
        ErrorReports::from(vec![eure::report::ErrorReport::error(e.to_string(), origin)])
    })?;

    Ok(ParsedEumd {
        doc: Arc::new(eumd_doc),
        eure_doc: parsed_doc.doc.clone(),
        cst: parsed_cst.cst.clone(),
        origins: parsed_doc.origins.clone(),
    })
}

/// Check references in an EumdDocument and return errors.
///
/// This query combines:
/// - ParseEumdDocument
/// - Reference checking
///
/// Returns ErrorReports with any reference errors found.
#[query]
pub fn check_eumd_references(db: &impl Db, file: TextFile) -> Result<ErrorReports, QueryError> {
    let parsed = db.query(ParseEumdDocument::new(file.clone()))?;

    // Check references
    let result = check_references_with_spans(&parsed.doc, &parsed.eure_doc);

    if result.is_ok() {
        Ok(ErrorReports::new())
    } else {
        // Convert check errors to ErrorReports
        let ctx = EumdReportContext {
            file,
            cst: &parsed.cst,
            origins: &parsed.origins,
        };
        Ok(report_check_errors(&result, &ctx))
    }
}
