//! Query integration for `eure-lint`.

use eure_lint::{Diagnostic, Severity as LintSeverity, lint_default};
use query_flow::{Db, QueryError, query};

use crate::report::{Element, ErrorReport, ErrorReports, Origin};

use super::{ParseCst, TextFile};

/// Run built-in lint rules and convert their findings to Eure error reports.
#[query(debug = "{Self}({file})")]
pub fn get_lint_reports(db: &impl Db, file: TextFile) -> Result<ErrorReports, QueryError> {
    let parsed = db.query(ParseCst::new(file.clone()))?;
    if parsed.error.is_some() {
        return Ok(ErrorReports::new());
    }
    let source = db.asset(file.clone())?;

    Ok(lint_default(source.get(), &parsed.cst)
        .into_iter()
        .map(|diagnostic| lint_report(file.clone(), diagnostic))
        .collect())
}

fn lint_report(file: TextFile, diagnostic: Diagnostic) -> ErrorReport {
    let origin = Origin::new(file.clone(), diagnostic.span);
    let mut report = match diagnostic.severity {
        LintSeverity::Error => ErrorReport::error(diagnostic.message, origin),
        LintSeverity::Warning => ErrorReport::warning(diagnostic.message, origin),
        LintSeverity::Hint => ErrorReport::hint(diagnostic.message, origin),
    }
    .with_code(diagnostic.rule.as_str());

    if let Some(help) = diagnostic.help {
        report = report.with_help(help);
    }
    if let Some(fix) = diagnostic.fix {
        for edit in fix.edits {
            report = report.with_element(Element::Suggestion {
                origin: Origin::new(file.clone(), edit.span),
                message: fix.message.clone().into(),
                replacement: edit.replacement.into(),
            });
        }
    }
    report
}
