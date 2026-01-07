use std::sync::Arc;

use eure::query::TextFile;
use query_flow::{Db, QueryError};

pub mod completions;
pub mod diagnostics;
pub mod eumd_error_validation;
pub mod eure_schema_to_json_schema;
pub mod eure_schema_to_json_schema_error;
pub mod eure_to_json;
pub mod format_schema;
pub mod formatting;
pub mod json_to_eure;
pub mod meta_schema;
pub mod normalization;
pub mod schema_conversion_error;
pub mod schema_error_validation;
pub mod schema_roundtrip;
pub mod schema_validation;
pub mod toml_to_eure_document;
pub mod toml_to_eure_source;
pub mod toml_to_json;

pub trait Scenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError>;
}

/// Error type for scenario execution failures.
/// Each variant represents a specific failure mode that can occur during test scenario execution.
#[derive(Debug, Clone)]
pub enum ScenarioError {
    /// Documents are not equal after normalization
    NormalizationMismatch {
        input_debug: String,
        normalized_debug: String,
    },
    /// Eure to JSON conversion output mismatch
    EureToJsonMismatch {
        expected: Box<serde_json::Value>,
        actual: Arc<serde_json::Value>,
    },
    /// JSON to Eure conversion output mismatch
    JsonToEureMismatch {
        expected_debug: String,
        actual_debug: String,
    },
    /// JSON to Eure conversion error
    JsonToEureConversionError {
        message: String,
    },
    /// JSON parse error
    JsonParseError {
        message: String,
    },
    /// JSON Schema output mismatch
    JsonSchemaMismatch {
        expected: Box<serde_json::Value>,
        actual: Box<serde_json::Value>,
    },
    /// Schema conversion error
    SchemaConversionError {
        message: String,
    },
    /// Schema conversion error mismatch (expected vs actual)
    SchemaConversionMismatch {
        expected: String,
        actual: String,
    },
    /// JSON Schema conversion error
    JsonSchemaConversionError {
        message: String,
    },
    /// JSON serialization error
    JsonSerializationError {
        message: String,
    },
    /// Eure to JSON conversion error
    EureToJsonConversionError {
        message: String,
    },
    /// Schema validation failed when it should have passed
    SchemaValidationFailed {
        errors: Vec<String>,
    },
    /// Schema validation errors mismatch (expected vs actual)
    SchemaValidationMismatch {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    /// Expected validation to fail but it passed
    ExpectedValidationToFail {
        expected_errors: Vec<String>,
    },
    /// Expected error not found in actual errors
    ExpectedErrorNotFound {
        expected: String,
        actual_errors: Vec<String>,
    },
    /// Expected JSON Schema conversion to fail but it succeeded
    ExpectedJsonSchemaConversionToFail {
        expected_errors: Vec<String>,
    },
    /// Expected schema conversion (document_to_schema) to fail but it succeeded
    ExpectedSchemaConversionToFail {
        expected_errors: Vec<String>,
    },
    /// Format schema mismatch (format_schema output != expected formatted_schema)
    FormatSchemaMismatch {
        expected: String,
        actual: String,
    },
    /// Schema roundtrip mismatch (format_schema output != original schema)
    SchemaRoundtripMismatch {
        expected: String,
        actual: String,
    },
    /// Schema roundtrip document mismatch (parsed documents differ)
    SchemaRoundtripDocumentMismatch {
        original_source: String,
        formatted_source: String,
        original_doc: String,
        formatted_doc: String,
    },
    /// Preprocessing error (parse error, file read error, etc.)
    PreprocessingError {
        message: String,
    },
    /// Scenario is not yet implemented
    Unimplemented {
        scenario_name: String,
    },
    /// Formatter output mismatch
    FormattingMismatch {
        input: String,
        expected: String,
        actual: String,
    },
    /// Formatting error
    FormattingError {
        message: String,
    },
    /// TOML parse error
    TomlParseError {
        message: String,
    },
    /// TOML to Eure conversion error
    TomlToEureError {
        message: String,
    },
    /// TOML to Eure document mismatch
    TomlToEureDocumentMismatch {
        expected_debug: String,
        actual_debug: String,
    },
    /// TOML to Eure source mismatch
    TomlToEureSourceMismatch {
        expected: String,
        actual: String,
    },
    /// Diagnostics mismatch (expected vs actual)
    DiagnosticsMismatch {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    /// Diagnostic span position mismatch
    SpanMismatch {
        diagnostic_index: usize,
        field: String,
        expected: i64,
        actual: i64,
    },
    /// Span string not found in editor content
    SpanStringNotFound {
        diagnostic_index: usize,
        span: String,
    },
    /// Span string found multiple times in editor content (ambiguous)
    SpanStringAmbiguous {
        diagnostic_index: usize,
        span: String,
        occurrences: usize,
    },
    /// Query error
    QueryError(QueryError),
    FileNotFound(TextFile),
    FileReadError {
        file: TextFile,
        error: String,
    },
}

impl From<QueryError> for ScenarioError {
    fn from(error: QueryError) -> Self {
        ScenarioError::QueryError(error)
    }
}

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioError::FileNotFound(file) => {
                write!(f, "File not found: {}", file)
            }
            ScenarioError::NormalizationMismatch {
                input_debug,
                normalized_debug,
            } => {
                write!(
                    f,
                    "Normalization mismatch.\nInput:\n{}\nNormalized:\n{}",
                    input_debug, normalized_debug
                )
            }
            ScenarioError::EureToJsonMismatch { expected, actual } => {
                write!(
                    f,
                    "Eure to JSON mismatch.\nExpected:\n{}\nActual:\n{}",
                    serde_json::to_string_pretty(expected)
                        .unwrap_or_else(|_| format!("{:?}", expected)),
                    serde_json::to_string_pretty(actual)
                        .unwrap_or_else(|_| format!("{:?}", actual))
                )
            }
            ScenarioError::JsonToEureMismatch {
                expected_debug,
                actual_debug,
            } => {
                write!(
                    f,
                    "JSON to Eure mismatch.\nExpected:\n{}\nActual:\n{}",
                    expected_debug, actual_debug
                )
            }
            ScenarioError::JsonToEureConversionError { message } => {
                write!(f, "JSON to Eure conversion error: {}", message)
            }
            ScenarioError::JsonParseError { message } => {
                write!(f, "JSON parse error: {}", message)
            }
            ScenarioError::JsonSchemaMismatch { expected, actual } => {
                write!(
                    f,
                    "JSON Schema output mismatch.\nExpected:\n{}\nActual:\n{}",
                    serde_json::to_string_pretty(expected)
                        .unwrap_or_else(|_| format!("{:?}", expected)),
                    serde_json::to_string_pretty(actual)
                        .unwrap_or_else(|_| format!("{:?}", actual))
                )
            }
            ScenarioError::SchemaConversionError { message } => {
                write!(f, "Schema conversion error: {}", message)
            }
            ScenarioError::SchemaConversionMismatch { expected, actual } => {
                writeln!(f, "Schema conversion error mismatch.")?;
                writeln!(f, "\n--- Expected ---")?;
                writeln!(f, "{}", expected)?;
                writeln!(f, "\n--- Actual ---")?;
                writeln!(f, "{}", actual)
            }
            ScenarioError::JsonSchemaConversionError { message } => {
                write!(f, "JSON Schema conversion error: {}", message)
            }
            ScenarioError::JsonSerializationError { message } => {
                write!(f, "JSON serialization error: {}", message)
            }
            ScenarioError::EureToJsonConversionError { message } => {
                write!(f, "Eure to JSON conversion error: {}", message)
            }
            ScenarioError::SchemaValidationFailed { errors } => {
                write!(f, "Schema validation failed:\n{}", errors.join("\n"))
            }
            ScenarioError::SchemaValidationMismatch { expected, actual } => {
                writeln!(f, "Schema validation errors mismatch.")?;
                writeln!(f, "\n--- Expected ({}) ---", expected.len())?;
                for error in expected {
                    writeln!(f, "{}", error)?;
                }
                writeln!(f, "\n--- Actual ({}) ---", actual.len())?;
                for error in actual {
                    writeln!(f, "{}", error)?;
                }
                Ok(())
            }
            ScenarioError::ExpectedValidationToFail { expected_errors } => {
                writeln!(f, "Expected validation to fail, but validation passed.")?;
                writeln!(f, "Expected errors ({}):", expected_errors.len())?;
                for (i, error) in expected_errors.iter().enumerate() {
                    writeln!(f, "\n--- Expected Error {} ---", i + 1)?;
                    write!(f, "{}", error)?;
                }
                Ok(())
            }
            ScenarioError::ExpectedErrorNotFound {
                expected,
                actual_errors,
            } => format_expected_error_not_found(f, expected, actual_errors),
            ScenarioError::ExpectedJsonSchemaConversionToFail { expected_errors } => {
                writeln!(
                    f,
                    "Expected JSON Schema conversion to fail, but conversion succeeded."
                )?;
                writeln!(f, "Expected errors ({}):", expected_errors.len())?;
                for (i, error) in expected_errors.iter().enumerate() {
                    writeln!(f, "\n--- Expected Error {} ---", i + 1)?;
                    write!(f, "{}", error)?;
                }
                Ok(())
            }
            ScenarioError::ExpectedSchemaConversionToFail { expected_errors } => {
                writeln!(
                    f,
                    "Expected schema conversion to fail, but conversion succeeded."
                )?;
                writeln!(f, "Expected errors ({}):", expected_errors.len())?;
                for (i, error) in expected_errors.iter().enumerate() {
                    writeln!(f, "\n--- Expected Error {} ---", i + 1)?;
                    write!(f, "{}", error)?;
                }
                Ok(())
            }
            ScenarioError::FormatSchemaMismatch { expected, actual } => {
                use similar::{ChangeTag, TextDiff};
                writeln!(f, "Format schema mismatch.")?;
                let diff = TextDiff::from_lines(expected, actual);
                let changes: Vec<_> = diff
                    .iter_all_changes()
                    .filter(|c| c.tag() != ChangeTag::Equal)
                    .collect();
                if changes.is_empty() {
                    writeln!(f, "No visible line differences. Checking char-by-char...")?;
                    let expected_bytes = expected.as_bytes();
                    let actual_bytes = actual.as_bytes();
                    writeln!(
                        f,
                        "Expected length: {}, Actual length: {}",
                        expected_bytes.len(),
                        actual_bytes.len()
                    )?;
                    for (i, (e, a)) in expected_bytes.iter().zip(actual_bytes.iter()).enumerate() {
                        if e != a {
                            writeln!(
                                f,
                                "First diff at byte {}: expected {:02x} ({:?}), actual {:02x} ({:?})",
                                i,
                                e,
                                char::from(*e),
                                a,
                                char::from(*a)
                            )?;
                            let start = i.saturating_sub(20);
                            let end = (i + 20).min(expected_bytes.len()).min(actual_bytes.len());
                            writeln!(f, "Context expected: {:?}", &expected[start..end])?;
                            writeln!(f, "Context actual:   {:?}", &actual[start..end])?;
                            break;
                        }
                    }
                    if expected_bytes.len() != actual_bytes.len() {
                        let min_len = expected_bytes.len().min(actual_bytes.len());
                        writeln!(f, "Length mismatch after byte {}", min_len)?;
                    }
                } else {
                    writeln!(f, "Diff (expected → actual):")?;
                    for change in diff.iter_all_changes() {
                        let sign = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        write!(f, "{}{}", sign, change)?;
                    }
                }
                Ok(())
            }
            ScenarioError::SchemaRoundtripMismatch { expected, actual } => {
                use similar::{ChangeTag, TextDiff};
                writeln!(f, "Schema roundtrip mismatch.")?;
                let diff = TextDiff::from_lines(expected, actual);
                let changes: Vec<_> = diff
                    .iter_all_changes()
                    .filter(|c| c.tag() != ChangeTag::Equal)
                    .collect();
                if changes.is_empty() {
                    writeln!(f, "No visible line differences. Checking char-by-char...")?;
                    let expected_bytes = expected.as_bytes();
                    let actual_bytes = actual.as_bytes();
                    writeln!(
                        f,
                        "Expected length: {}, Actual length: {}",
                        expected_bytes.len(),
                        actual_bytes.len()
                    )?;
                    for (i, (e, a)) in expected_bytes.iter().zip(actual_bytes.iter()).enumerate() {
                        if e != a {
                            writeln!(
                                f,
                                "First diff at byte {}: expected {:02x} ({:?}), actual {:02x} ({:?})",
                                i,
                                e,
                                char::from(*e),
                                a,
                                char::from(*a)
                            )?;
                            let start = i.saturating_sub(20);
                            let end = (i + 20).min(expected_bytes.len()).min(actual_bytes.len());
                            writeln!(f, "Context expected: {:?}", &expected[start..end])?;
                            writeln!(f, "Context actual:   {:?}", &actual[start..end])?;
                            break;
                        }
                    }
                    if expected_bytes.len() != actual_bytes.len() {
                        let min_len = expected_bytes.len().min(actual_bytes.len());
                        writeln!(f, "Length mismatch after byte {}", min_len)?;
                    }
                } else {
                    writeln!(f, "Diff (expected → actual):")?;
                    for change in diff.iter_all_changes() {
                        let sign = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        write!(f, "{}{}", sign, change)?;
                    }
                }
                Ok(())
            }
            ScenarioError::SchemaRoundtripDocumentMismatch {
                original_source,
                formatted_source,
                original_doc,
                formatted_doc,
            } => {
                writeln!(f, "Schema roundtrip document mismatch.")?;
                writeln!(
                    f,
                    "Documents parsed from original and formatted sources differ."
                )?;
                writeln!(f, "\nOriginal source:\n{}", original_source)?;
                writeln!(f, "\nFormatted source:\n{}", formatted_source)?;
                writeln!(f, "\nOriginal doc:\n{}", original_doc)?;
                writeln!(f, "\nFormatted doc:\n{}", formatted_doc)?;
                Ok(())
            }
            ScenarioError::PreprocessingError { message } => {
                write!(f, "Preprocessing error: {}", message)
            }
            ScenarioError::Unimplemented { scenario_name } => {
                write!(f, "Scenario '{}' is not yet implemented", scenario_name)
            }
            ScenarioError::FormattingMismatch {
                input,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Formatting mismatch.\nInput:\n{}\n\nExpected:\n{}\n\nActual:\n{}",
                    input, expected, actual
                )
            }
            ScenarioError::FormattingError { message } => {
                write!(f, "Formatting error: {}", message)
            }
            ScenarioError::TomlParseError { message } => {
                write!(f, "TOML parse error: {}", message)
            }
            ScenarioError::TomlToEureError { message } => {
                write!(f, "TOML to Eure conversion error: {}", message)
            }
            ScenarioError::TomlToEureDocumentMismatch {
                expected_debug,
                actual_debug,
            } => {
                write!(
                    f,
                    "TOML to Eure document mismatch.\nExpected:\n{}\nActual:\n{}",
                    expected_debug, actual_debug
                )
            }
            ScenarioError::TomlToEureSourceMismatch { expected, actual } => {
                use similar::{ChangeTag, TextDiff};
                writeln!(f, "TOML to Eure source mismatch.")?;
                let diff = TextDiff::from_lines(expected, actual);
                let changes: Vec<_> = diff
                    .iter_all_changes()
                    .filter(|c| c.tag() != ChangeTag::Equal)
                    .collect();
                if changes.is_empty() {
                    // No visible line differences - likely whitespace issue
                    writeln!(f, "No visible line differences. Checking char-by-char...")?;
                    let expected_bytes = expected.as_bytes();
                    let actual_bytes = actual.as_bytes();
                    writeln!(
                        f,
                        "Expected length: {}, Actual length: {}",
                        expected_bytes.len(),
                        actual_bytes.len()
                    )?;
                    for (i, (e, a)) in expected_bytes.iter().zip(actual_bytes.iter()).enumerate() {
                        if e != a {
                            writeln!(
                                f,
                                "First diff at byte {}: expected {:02x} ({:?}), actual {:02x} ({:?})",
                                i,
                                e,
                                char::from(*e),
                                a,
                                char::from(*a)
                            )?;
                            // Show context
                            let start = i.saturating_sub(20);
                            let end = (i + 20).min(expected_bytes.len()).min(actual_bytes.len());
                            writeln!(f, "Context expected: {:?}", &expected[start..end])?;
                            writeln!(f, "Context actual:   {:?}", &actual[start..end])?;
                            break;
                        }
                    }
                    if expected_bytes.len() != actual_bytes.len() {
                        let min_len = expected_bytes.len().min(actual_bytes.len());
                        writeln!(f, "Length mismatch after byte {}", min_len)?;
                    }
                } else {
                    writeln!(f, "Diff (expected → actual):")?;
                    for change in diff.iter_all_changes() {
                        let sign = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        write!(f, "{}{}", sign, change)?;
                    }
                }
                Ok(())
            }
            ScenarioError::DiagnosticsMismatch { expected, actual } => {
                writeln!(f, "Diagnostics mismatch.")?;
                writeln!(f, "\n--- Expected ({}) ---", expected.len())?;
                for diag in expected {
                    writeln!(f, "{}", diag)?;
                }
                writeln!(f, "\n--- Actual ({}) ---", actual.len())?;
                for diag in actual {
                    writeln!(f, "{}", diag)?;
                }
                Ok(())
            }
            ScenarioError::SpanMismatch {
                diagnostic_index,
                field,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Diagnostic span mismatch at index {}.\nField '{}': expected {}, got {}",
                    diagnostic_index, field, expected, actual
                )
            }
            ScenarioError::SpanStringNotFound {
                diagnostic_index,
                span,
            } => {
                write!(
                    f,
                    "Diagnostic span string not found at index {}.\nSpan: {:?}",
                    diagnostic_index, span
                )
            }
            ScenarioError::SpanStringAmbiguous {
                diagnostic_index,
                span,
                occurrences,
            } => {
                write!(
                    f,
                    "Diagnostic span string is ambiguous at index {}.\nSpan {:?} found {} times (must be unique)",
                    diagnostic_index, span, occurrences
                )
            }
            ScenarioError::QueryError(error) => {
                write!(f, "Query error: {}", error)
            }
            ScenarioError::FileReadError { file, error } => {
                write!(f, "File read error: {}: {}", file, error)
            }
        }
    }
}

impl std::error::Error for ScenarioError {}

/// Format helper for displaying expected error not found with actual errors list
fn format_expected_error_not_found(
    f: &mut std::fmt::Formatter<'_>,
    expected: &str,
    actual_errors: &[String],
) -> std::fmt::Result {
    writeln!(f, "Expected error containing '{}' not found.", expected)?;
    writeln!(f, "Actual errors ({}):", actual_errors.len())?;
    for (i, error) in actual_errors.iter().enumerate() {
        writeln!(f, "\n--- Error {} ---", i + 1)?;
        write!(f, "{}", error)?;
    }
    Ok(())
}

/// Compare error lists from query results with expected errors.
/// - If actual is empty but expected is not, returns ExpectedValidationToFail
/// - If lists don't match, returns SchemaValidationMismatch
/// - Returns Ok(()) if lists match exactly
///
/// Note: Trailing whitespace on each line is normalized for comparison,
/// as test case files may have trailing whitespace stripped differently
/// than the formatted error output.
pub fn compare_error_lists(
    actual: &Arc<Vec<String>>,
    expected: Vec<String>,
) -> Result<(), ScenarioError> {
    if actual.is_empty() && !expected.is_empty() {
        return Err(ScenarioError::ExpectedValidationToFail {
            expected_errors: expected,
        });
    }

    // Normalize trailing whitespace on each line for comparison
    let normalize = |s: &str| -> String {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let actual_normalized: Vec<String> = actual.iter().map(|s| normalize(s)).collect();
    let expected_normalized: Vec<String> = expected.iter().map(|s| normalize(s)).collect();

    if actual_normalized != expected_normalized {
        return Err(ScenarioError::SchemaValidationMismatch {
            expected,
            actual: actual.as_ref().clone(),
        });
    }
    Ok(())
}
