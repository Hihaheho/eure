//! Error reporting for eure-mark using eure's ErrorReports system

use eure::document::OriginMap;
use eure::query::assets::TextFile;
use eure::query_flow::{Db, QueryError};
use eure::report::{ErrorReport, ErrorReports, Origin};
use eure::tree::Cst;
use eure_tree::tree::InputSpan;

use crate::check::CheckResult;
use crate::error::ReferenceError;

/// Context for converting eure-mark errors to ErrorReports
pub struct EumdReportContext<'a> {
    /// File for the eumd document
    pub file: TextFile,
    /// CST for span resolution
    pub cst: &'a Cst,
    /// Origin map with precise origins
    pub origins: &'a OriginMap,
}

/// Convert eure-mark check result to ErrorReports
pub fn report_check_errors(result: &CheckResult, ctx: &EumdReportContext<'_>) -> ErrorReports {
    result
        .errors
        .iter()
        .map(|error| report_reference_error(error, ctx))
        .collect()
}

fn report_reference_error(error: &ReferenceError, ctx: &EumdReportContext<'_>) -> ErrorReport {
    let title = format!(
        "Undefined !{}[{}] {}",
        error.ref_type, error.key, error.location
    );

    // Try to get span from NodeId if available
    // FIXME: Multiple fallback paths to EMPTY span without is_fallback flag:
    // 1. When node_id, offset, or len is None
    // 2. When get_value_span returns None
    // Both cases silently report errors at file start instead of indicating uncertainty.
    let span = if let (Some(node_id), Some(offset), Some(len)) =
        (error.node_id, error.offset, error.len)
    {
        // Get the span of the node (code block)
        if let Some(node_span) = ctx.origins.get_value_span(node_id, ctx.cst) {
            // For code blocks, we need to find where the actual content starts
            // The node_span includes the opening ``` and language tag
            // We'll calculate the content start by looking at the text
            let content_start = get_code_block_content_start(ctx, node_span);

            let start = content_start + offset;
            let end = start + len;
            InputSpan { start, end }
        } else {
            InputSpan::EMPTY
        }
    } else {
        InputSpan::EMPTY
    };

    let origin = Origin::new(ctx.file.clone(), span);
    ErrorReport::error(title, origin)
}

/// Get the byte offset where the code block content starts
fn get_code_block_content_start(_ctx: &EumdReportContext<'_>, node_span: InputSpan) -> u32 {
    // For code blocks, we need to skip past the opening ``` and language tag
    // The Text type in eure stores the content without the delimiters,
    // but the node span includes everything.
    //
    // For now, use the node span start which points to the content area.
    // The span resolution in OriginMap should give us a reasonable position.
    node_span.start
}

/// Format check errors to a string using annotate-snippets.
///
/// Returns `Err` with suspension if file content isn't loaded yet.
pub fn format_check_errors(
    db: &impl Db,
    result: &CheckResult,
    file: TextFile,
    cst: &Cst,
    origins: &OriginMap,
    styled: bool,
) -> Result<String, QueryError> {
    let ctx = EumdReportContext { file, cst, origins };
    let reports = report_check_errors(result, &ctx);
    eure::report::format_error_reports(db, &reports, styled)
}

/// Format check errors without ANSI colors (for testing).
///
/// Returns `Err` with suspension if file content isn't loaded yet.
pub fn format_check_errors_plain(
    db: &impl Db,
    result: &CheckResult,
    file: TextFile,
    cst: &Cst,
    origins: &OriginMap,
) -> Result<String, QueryError> {
    format_check_errors(db, result, file, cst, origins, false)
}
