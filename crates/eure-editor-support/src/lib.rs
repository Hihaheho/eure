pub mod assets;
pub mod config;
pub mod errors;
pub mod schema;
pub mod semantic_token;

pub use query_flow;

use eure::report::ErrorReports;

/// Error comparator for query-flow that compares ErrorReports by value.
///
/// This comparator is designed for use with `QueryRuntimeBuilder::error_comparator`.
/// It enables early cutoff optimization by detecting when errors are semantically
/// equivalent, avoiding unnecessary downstream recomputation.
///
/// # Comparison Strategy
///
/// 1. If both errors can be downcast to `ErrorReports`, compare using `PartialEq`
/// 2. Otherwise, fall back to string comparison via `to_string()`
pub fn error_reports_comparator(a: &anyhow::Error, b: &anyhow::Error) -> bool {
    match (
        a.downcast_ref::<ErrorReports>(),
        b.downcast_ref::<ErrorReports>(),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => a.to_string() == b.to_string(),
    }
}
