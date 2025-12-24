//! Error reporting for eure-mark using eure's ErrorReports system

use eure::document::OriginMap;
use eure::report::{ErrorReport, ErrorReports, FileId, FileRegistry, Origin};
use eure::tree::Cst;
use eure_tree::tree::InputSpan;

use crate::check::CheckResult;
use crate::error::ReferenceError;

/// Context for converting eure-mark errors to ErrorReports
pub struct EumdReportContext<'a> {
    /// File ID for the eumd document
    pub file: FileId,
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
    let span = if let (Some(node_id), Some(offset), Some(len)) =
        (error.node_id, error.offset, error.len)
    {
        // Get the span of the node (code block)
        if let Some(node_span) = ctx.origins.get_node_span(node_id, ctx.cst) {
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

    let origin = Origin::new(ctx.file, span);
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

/// Format check errors to a string using annotate-snippets
pub fn format_check_errors(
    result: &CheckResult,
    source: &str,
    path: &str,
    cst: &Cst,
    origins: &OriginMap,
    styled: bool,
) -> String {
    let mut files = FileRegistry::new();
    let file = files.register(path, source);
    let ctx = EumdReportContext { file, cst, origins };
    let reports = report_check_errors(result, &ctx);
    eure::report::format_error_reports(&reports, &files, styled)
}

/// Format check errors without ANSI colors (for testing)
pub fn format_check_errors_plain(
    result: &CheckResult,
    source: &str,
    path: &str,
    cst: &Cst,
    origins: &OriginMap,
) -> String {
    format_check_errors(result, source, path, cst, origins, false)
}
