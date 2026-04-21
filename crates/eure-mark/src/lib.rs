//! Parsing, validation, and rendering tools for Eure-authored content.

mod article;
mod check;
mod document;
mod error;
mod query;
mod reference;
mod report;

pub use article::*;
pub use check::{CheckResult, check_references, check_references_with_spans};
pub use document::*;
pub use error::*;
pub use query::{CheckEumdReferences, CheckEumdReferencesFormatted, ParseEumdDocument, ParsedEumd};
pub use reference::*;
pub use report::{
    EumdReportContext, format_check_errors, format_check_errors_plain, report_check_errors,
};
