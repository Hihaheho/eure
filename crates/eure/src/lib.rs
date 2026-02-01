pub mod document;
pub mod query;
pub mod report;
pub mod tree;
pub mod value;

use std::path::PathBuf;
use std::sync::Arc;

use query_flow::DurabilityLevel;

use crate::query::{ParseEure, TextFile, TextFileContent, WithFormattedError, build_runtime};
use crate::report::IntoErrorReports;

pub use eure_document::data_model;
pub use eure_document::eure;
pub use eure_document::parse::FromEure;
pub use eure_macros::{BuildSchema, FromEure};
pub use eure_parol as parol;
pub use eure_schema::{BuildSchema as BuildSchemaTrait, SchemaBuilder, SchemaDocument};
pub use query_flow;

/// Parse Eure content directly into a typed value.
///
/// This is a convenience function that creates a temporary query runtime,
/// parses the content, and returns the result. Useful for simple parsing
/// scenarios where you don't need the full query infrastructure.
///
/// # Arguments
/// * `content` - The Eure source content to parse
/// * `path` - The path to use for error reporting
///
/// # Returns
/// * `Ok(T)` - The parsed value
/// * `Err(String)` - A formatted error message if parsing fails
pub fn parse_content<T>(content: &str, path: PathBuf) -> Result<T, String>
where
    T: for<'doc> FromEure<'doc> + Send + Sync + 'static,
    for<'doc> <T as FromEure<'doc>>::Error: IntoErrorReports,
{
    let runtime = build_runtime();
    let file = TextFile::from_path(path);
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(content.to_string()),
        DurabilityLevel::Volatile,
    );
    let result = runtime
        .query(WithFormattedError::new(
            ParseEure::<NoCompare<T>>::new(file),
            false,
        ))
        .expect("query should succeed");
    drop(runtime);
    Arc::into_inner(result)
        .expect("This should be singleton since runtime dropped")
        .map(|arc| {
            Arc::into_inner(arc)
                .expect("this should be singleton since runtime dropped")
                .0
        })
}

struct NoCompare<T>(T);

impl<'doc, T: FromEure<'doc>> FromEure<'doc> for NoCompare<T> {
    type Error = T::Error;

    fn parse(ctx: &eure_document::parse::ParseContext<'doc>) -> Result<Self, Self::Error> {
        T::parse(ctx).map(NoCompare)
    }
}

impl<T> PartialEq for NoCompare<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::EureDocument;

    #[test]
    fn test_parse_content_success() {
        let content = r#"
            name = "Alice"
            age = 30
        "#;
        let result: Result<EureDocument, String> =
            parse_content(content, PathBuf::from("test.eure"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_content_parse_error() {
        // Invalid syntax - unclosed brace
        let content = r#"
            foo {
                bar = 1
        "#;
        let result: Result<EureDocument, String> =
            parse_content(content, PathBuf::from("test.eure"));
        // This should return Err, not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_content_from_eure_error() {
        // Valid syntax but wrong type for FromEure
        let content = r#"
            name = "Alice"
        "#;
        // Try to parse as i32 which should fail
        let result: Result<i32, String> = parse_content(content, PathBuf::from("test.eure"));
        assert!(result.is_err());
    }
}
