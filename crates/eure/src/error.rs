//! Error formatting utilities for Eure.

use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet};
use eure_parol::EureParseError;
use eure_tree::tree::LineNumbers;

/// Format a parse error with source context using annotate-snippets.
///
/// # Arguments
/// * `error` - The parse error to format
/// * `input` - The source input that was being parsed
/// * `path` - The file path (for display purposes)
///
/// # Returns
/// A formatted error string suitable for terminal output
pub fn format_parse_error(error: &EureParseError, input: &str, path: &str) -> String {
    if let Some(span) = &error.span {
        let line_numbers = LineNumbers::new(input);
        let start_info = line_numbers.get_char_info(span.start);

        // Build the annotated snippet
        let report = Level::ERROR.primary_title(&error.message).element(
            Snippet::source(input).line_start(1).path(path).annotation(
                AnnotationKind::Primary
                    .span(span.start as usize..span.end as usize)
                    .label(&error.message),
            ),
        );

        // Add line/column info to the output
        let rendered = Renderer::styled().render(&[report]).to_string();
        format!(
            "at line {}, column {}\n{}",
            start_info.line_number + 1,
            start_info.column_number + 1,
            rendered
        )
    } else {
        format!("error: {}\n  --> {}\n", error.message, path)
    }
}
