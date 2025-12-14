use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use eure::{
    document::{DocumentConstructionError, EureDocument, OriginMap},
    error::format_parse_error_color,
    parol::EureParseError,
    tree::Cst,
    value::{Language, Text},
};
use eure_schema::SchemaDocument;
use eure_schema::convert::SchemaSourceMap;

use crate::parser::{CompletionsScenario, DiagnosticsScenario, InputUnionTagMode};

// ============================================================================
// Meta Schema Loader
// ============================================================================

static META_SCHEMA_TEXT: &str = include_str!("../../assets/schemas/eure-schema.schema.eure");

struct MetaSchema {
    cst: Cst,
    origins: OriginMap,
    schema: SchemaDocument,
    source_map: SchemaSourceMap,
}

static META_SCHEMA: LazyLock<MetaSchema> = LazyLock::new(|| {
    let cst = eure::parol::parse(META_SCHEMA_TEXT).expect("Meta schema should parse");
    let (doc, origins) = eure::document::cst_to_document_and_origin_map(META_SCHEMA_TEXT, &cst)
        .expect("Meta schema should construct");
    let (schema, source_map) =
        eure_schema::convert::document_to_schema(&doc).expect("Meta schema should convert");
    MetaSchema {
        cst,
        origins,
        schema,
        source_map,
    }
});

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
        expected: serde_json::Value,
        actual: serde_json::Value,
    },
    /// JSON to Eure conversion output mismatch
    JsonToEureMismatch {
        expected_debug: String,
        actual_debug: String,
    },
    /// JSON to Eure conversion error
    JsonToEureConversionError { message: String },
    /// JSON parse error
    JsonParseError { message: String },
    /// JSON Schema output mismatch
    JsonSchemaMismatch {
        expected: serde_json::Value,
        actual: serde_json::Value,
    },
    /// Schema conversion error
    SchemaConversionError { message: String },
    /// JSON Schema conversion error
    JsonSchemaConversionError { message: String },
    /// JSON serialization error
    JsonSerializationError { message: String },
    /// Eure to JSON conversion error
    EureToJsonConversionError { message: String },
    /// Schema validation failed when it should have passed
    SchemaValidationFailed { errors: Vec<String> },
    /// Expected validation to fail but it passed
    ExpectedValidationToFail { expected_errors: Vec<String> },
    /// Expected error not found in actual errors
    ExpectedErrorNotFound {
        expected: String,
        actual_errors: Vec<String>,
    },
    /// Expected JSON Schema conversion to fail but it succeeded
    ExpectedJsonSchemaConversionToFail { expected_errors: Vec<String> },
    /// Preprocessing error (parse error, file read error, etc.)
    PreprocessingError { message: String },
    /// Scenario is not yet implemented
    Unimplemented { scenario_name: String },
    /// Formatter output mismatch
    FormattingMismatch {
        input: String,
        expected: String,
        actual: String,
    },
    /// Formatting error
    FormattingError { message: String },
    /// TOML parse error
    TomlParseError { message: String },
    /// TOML to Eure conversion error
    TomlToEureError { message: String },
    /// TOML to Eure document mismatch
    TomlToEureDocumentMismatch {
        expected_debug: String,
        actual_debug: String,
    },
}

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

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
        }
    }
}

impl std::error::Error for ScenarioError {}

use crate::parser::CaseData;

/// A single test case with its data and path
pub struct Case {
    pub path: PathBuf,
    /// Case name: "" for default (root-level), or the name from @ cases.<name>
    pub name: String,
    pub data: CaseData,
}

/// Configuration for running test cases
#[derive(Debug, Clone, Default)]
pub struct RunConfig {
    /// Enable trace output for debugging
    pub trace: bool,
}

/// Result of running a single scenario
#[derive(Debug, Clone)]
pub enum ScenarioResult {
    Passed,
    Failed { error: String },
}

impl ScenarioResult {
    pub fn is_passed(&self) -> bool {
        matches!(self, ScenarioResult::Passed)
    }
}

/// Named scenario with its result
#[derive(Debug, Clone)]
pub struct NamedScenarioResult {
    pub name: String,
    pub result: ScenarioResult,
}

/// Result of running all scenarios in a test case
#[derive(Debug, Clone)]
pub struct CaseResult {
    pub scenarios: Vec<NamedScenarioResult>,
}

impl CaseResult {
    pub fn passed_count(&self) -> usize {
        self.scenarios
            .iter()
            .filter(|s| s.result.is_passed())
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.scenarios.len()
    }

    pub fn all_passed(&self) -> bool {
        self.scenarios.iter().all(|s| s.result.is_passed())
    }

    pub fn failed_scenarios(&self) -> Vec<&NamedScenarioResult> {
        self.scenarios
            .iter()
            .filter(|s| !s.result.is_passed())
            .collect()
    }
}

/// A runnable scenario with its name
pub enum Scenario<'a> {
    Normalization(NormalizationScenario<'a>),
    EureToJson(EureToJsonScenario<'a>),
    JsonToEure(JsonToEureScenario<'a>),
    TomlToEureDocument(TomlToEureDocumentScenario<'a>),
    TomlToJson(TomlToJsonScenario<'a>),
    SchemaValidation(SchemaValidationScenario<'a>),
    SchemaErrorValidation(SchemaErrorValidationScenario<'a>),
    MetaSchema(MetaSchemaScenario<'a>),
    EureSchemaToJsonSchema(EureSchemaToJsonSchemaScenario<'a>),
    EureSchemaToJsonSchemaError(EureSchemaToJsonSchemaErrorScenario<'a>),
    Formatting(FormattingScenario<'a>),
    Completions(&'a CompletionsScenario),
    Diagnostics(&'a DiagnosticsScenario),
}

impl Scenario<'_> {
    pub fn name(&self) -> String {
        match self {
            Scenario::Normalization(_) => "normalization".to_string(),
            Scenario::EureToJson(s) => format!("eure_to_json({})", s.source),
            Scenario::JsonToEure(s) => format!("json_to_eure({})", s.source),
            Scenario::TomlToEureDocument(_) => "toml_to_eure_document".to_string(),
            Scenario::TomlToJson(_) => "toml_to_json".to_string(),
            Scenario::SchemaValidation(_) => "schema_validation".to_string(),
            Scenario::SchemaErrorValidation(_) => "schema_error_validation".to_string(),
            Scenario::MetaSchema(_) => "meta_schema".to_string(),
            Scenario::EureSchemaToJsonSchema(_) => "eure_schema_to_json_schema".to_string(),
            Scenario::EureSchemaToJsonSchemaError(_) => {
                "eure_schema_to_json_schema_error".to_string()
            }
            Scenario::Formatting(_) => "formatting".to_string(),
            Scenario::Completions(_) => "completions".to_string(),
            Scenario::Diagnostics(_) => "diagnostics".to_string(),
        }
    }

    pub fn run(&self) -> Result<(), ScenarioError> {
        match self {
            Scenario::Normalization(s) => s.run(),
            Scenario::EureToJson(s) => s.run(),
            Scenario::JsonToEure(s) => s.run(),
            Scenario::TomlToEureDocument(s) => s.run(),
            Scenario::TomlToJson(s) => s.run(),
            Scenario::SchemaValidation(s) => s.run(),
            Scenario::SchemaErrorValidation(s) => s.run(),
            Scenario::MetaSchema(s) => s.run(),
            Scenario::EureSchemaToJsonSchema(s) => s.run(),
            Scenario::EureSchemaToJsonSchemaError(s) => s.run(),
            Scenario::Formatting(s) => s.run(),
            Scenario::Completions(s) => s.run(),
            Scenario::Diagnostics(s) => s.run(),
        }
    }
}

#[derive(Default)]
pub struct PreprocessedCase {
    pub input_eure: Option<PreprocessedEure>,
    pub input_toml: Option<PreprocessedToml>,
    pub input_json: Option<serde_json::Value>,
    pub normalized: Option<PreprocessedEure>,
    pub output_json: Option<serde_json::Value>,
    pub schema: Option<PreprocessedEure>,
    pub schema_errors: Vec<String>,
    pub meta_schema_errors: Vec<String>,
    pub output_json_schema: Option<serde_json::Value>,
    pub json_schema_errors: Vec<String>,
    /// Union tag mode for validation (default: eure)
    pub input_union_tag_mode: InputUnionTagMode,
    // Formatter testing fields - expected formatted outputs
    pub formatted_input: Option<String>,
    pub formatted_normalized: Option<String>,
    // Editor scenarios
    pub completions_scenario: Option<CompletionsScenario>,
    pub diagnostics_scenario: Option<DiagnosticsScenario>,
}

pub enum PreprocessedEure {
    Ok {
        input: String,
        cst: Cst,
        doc: EureDocument,
        origins: eure::document::OriginMap,
    },
    ErrParol {
        input: String,
        error: EureParseError,
    },
    ErrDocument {
        input: String,
        cst: Cst,
        error: DocumentConstructionError,
    },
    ErrFileRead {
        path: PathBuf,
        error: std::io::Error,
    },
}

impl PreprocessedEure {
    /// Short status string for summary display
    pub fn status(&self) -> String {
        match self {
            PreprocessedEure::Ok { .. } => "OK".to_string(),
            PreprocessedEure::ErrParol { .. } => "PARSE_ERROR".to_string(),
            PreprocessedEure::ErrFileRead { path, .. } => {
                format!("FILE_READ_ERROR({})", path.display())
            }
            PreprocessedEure::ErrDocument { .. } => "DOC_ERROR".to_string(),
        }
    }

    /// Detailed error message for failure reporting
    pub fn detailed_error(&self) -> Option<String> {
        match self {
            PreprocessedEure::Ok { .. } => None,
            PreprocessedEure::ErrParol { input, error } => {
                Some(format_parse_error_color(error, input, "<test>"))
            }
            PreprocessedEure::ErrFileRead { path, error } => {
                Some(format!("Failed to read '{}': {}", path.display(), error))
            }
            PreprocessedEure::ErrDocument { input, cst, error } => {
                // Get node_id and node_data for better debugging
                let node_info = match error {
                    DocumentConstructionError::CstError(cst_error) => {
                        use eure::tree::CstConstructError;
                        match cst_error {
                            CstConstructError::UnexpectedExtraNode { node } => {
                                let data = cst.node_data(*node);
                                Some(format!("node_id={}, data={:?}", node, data))
                            }
                            CstConstructError::UnexpectedNode {
                                node,
                                data,
                                expected_kind,
                            } => Some(format!(
                                "node_id={}, expected={:?}, got={:?}",
                                node, expected_kind, data
                            )),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                let msg = if let Some(info) = node_info {
                    format!("{} [{}]", error, info)
                } else if let Some(span) = error.span(cst) {
                    let start = span.start as usize;
                    let end = span.end as usize;
                    if start < input.len() && end <= input.len() && start <= end {
                        let snippet = &input[start..end];
                        format!("{} at {}..{}: {:?}", error, start, end, snippet)
                    } else {
                        format!("{} at {}..{} (invalid span)", error, start, end)
                    }
                } else {
                    format!("{}", error)
                };
                Some(msg)
            }
        }
    }

    pub fn input(&self) -> &str {
        match self {
            PreprocessedEure::Ok { input, .. } => input,
            PreprocessedEure::ErrParol { input, .. } => input,
            PreprocessedEure::ErrDocument { input, .. } => input,
            PreprocessedEure::ErrFileRead { path, .. } => path.to_str().unwrap_or("<invalid path>"),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, PreprocessedEure::Ok { .. })
    }

    pub fn cst(&self) -> eros::Result<&Cst> {
        match self {
            PreprocessedEure::Ok { cst, .. } => Ok(cst),
            PreprocessedEure::ErrDocument { cst, .. } => Ok(cst),
            PreprocessedEure::ErrParol { error, .. } => Err(eros::traced!("{}", error)),
            PreprocessedEure::ErrFileRead { path, error } => Err(eros::traced!(
                "Failed to read file '{}': {}",
                path.display(),
                error
            )),
        }
    }

    pub fn doc(&self) -> eros::Result<&EureDocument> {
        match self {
            PreprocessedEure::Ok { doc, .. } => Ok(doc),
            PreprocessedEure::ErrParol { error, .. } => Err(eros::traced!("{}", error)),
            PreprocessedEure::ErrDocument { error, .. } => Err(eros::traced!("{}", error.clone())),
            PreprocessedEure::ErrFileRead { path, error } => Err(eros::traced!(
                "Failed to read file '{}': {}",
                path.display(),
                error
            )),
        }
    }

    pub fn origins(&self) -> eros::Result<&eure::document::OriginMap> {
        match self {
            PreprocessedEure::Ok { origins, .. } => Ok(origins),
            PreprocessedEure::ErrParol { error, .. } => Err(eros::traced!("{}", error)),
            PreprocessedEure::ErrDocument { error, .. } => Err(eros::traced!("{}", error.clone())),
            PreprocessedEure::ErrFileRead { path, error } => Err(eros::traced!(
                "Failed to read file '{}': {}",
                path.display(),
                error
            )),
        }
    }
}

/// Preprocessed TOML input ready for testing
pub enum PreprocessedToml {
    Ok {
        input: String,
        source_doc: eure_document::source::SourceDocument,
    },
    ErrParse {
        input: String,
        error: String,
    },
    ErrConvert {
        input: String,
        error: eure_toml::TomlToEureError,
    },
}

impl Default for PreprocessedToml {
    fn default() -> Self {
        PreprocessedToml::ErrParse {
            input: String::new(),
            error: "No TOML input provided".to_string(),
        }
    }
}

impl PreprocessedToml {
    pub fn status(&self) -> String {
        match self {
            PreprocessedToml::Ok { .. } => "OK".to_string(),
            PreprocessedToml::ErrParse { .. } => "PARSE_ERROR".to_string(),
            PreprocessedToml::ErrConvert { .. } => "CONVERT_ERROR".to_string(),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, PreprocessedToml::Ok { .. })
    }

    pub fn source_doc(&self) -> Result<&eure_document::source::SourceDocument, ScenarioError> {
        match self {
            PreprocessedToml::Ok { source_doc, .. } => Ok(source_doc),
            PreprocessedToml::ErrParse { error, .. } => Err(ScenarioError::TomlParseError {
                message: error.clone(),
            }),
            PreprocessedToml::ErrConvert { error, .. } => Err(ScenarioError::TomlToEureError {
                message: error.to_string(),
            }),
        }
    }

    pub fn doc(&self) -> Result<&EureDocument, ScenarioError> {
        self.source_doc().map(|s| &s.document)
    }
}

impl Case {
    /// Create a Case from a CaseData with path and name
    pub fn new(path: PathBuf, name: String, data: CaseData) -> Self {
        Self { path, name, data }
    }

    /// Check if this case is marked as unimplemented
    pub fn is_unimplemented(&self) -> bool {
        self.data.unimplemented.is_some()
    }

    /// Get the unimplemented reason if any
    pub fn unimplemented_reason(&self) -> Option<&str> {
        self.data.unimplemented.as_deref()
    }

    fn preprocess_eure(code: &Text) -> PreprocessedEure {
        // Check if language is "path" - load file from workspace root
        let input = if let Language::Other(lang) = &code.language {
            if lang == "path" {
                let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
                let file_path = workspace_root.join(code.content.trim());
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => content,
                    Err(e) => {
                        return PreprocessedEure::ErrFileRead {
                            path: file_path,
                            error: e,
                        };
                    }
                }
            } else {
                code.content.clone()
            }
        } else {
            code.content.clone()
        };

        match eure::parol::parse(&input) {
            Ok(cst) => match eure::document::cst_to_document_and_origin_map(&input, &cst) {
                Ok((doc, origins)) => PreprocessedEure::Ok {
                    input,
                    cst,
                    doc,
                    origins,
                },
                Err(e) => PreprocessedEure::ErrDocument {
                    input,
                    cst,
                    error: e,
                },
            },
            Err(e) => PreprocessedEure::ErrParol { input, error: e },
        }
    }

    fn preprocess_toml(code: &Text) -> PreprocessedToml {
        let input = code.content.clone();

        // Parse TOML
        let toml_doc: toml_edit::DocumentMut = match input.parse() {
            Ok(doc) => doc,
            Err(e) => {
                return PreprocessedToml::ErrParse {
                    input,
                    error: e.to_string(),
                };
            }
        };

        // Convert to SourceDocument
        match eure_toml::to_source_document(&toml_doc) {
            Ok(source_doc) => PreprocessedToml::Ok { input, source_doc },
            Err(e) => PreprocessedToml::ErrConvert { input, error: e },
        }
    }
    pub fn preprocess(&self) -> Result<PreprocessedCase, PreprocessError> {
        let input_eure = self.data.input_eure.as_ref().map(Self::preprocess_eure);
        let input_toml = self.data.input_toml.as_ref().map(Self::preprocess_toml);
        let normalized = self.data.normalized.as_ref().map(Self::preprocess_eure);

        // Expand JsonInput into separate input_json and output_json
        let (input_json, output_json) = if let Some(ref json_input) = self.data.json_input {
            let input = json_input
                .input_json()
                .map(|text| {
                    serde_json::from_str(text.as_str()).map_err(|source| {
                        PreprocessError::JsonParseError {
                            field: "json (as input)",
                            source,
                        }
                    })
                })
                .transpose()?;

            let output = json_input
                .output_json()
                .map(|text| {
                    serde_json::from_str(text.as_str()).map_err(|source| {
                        PreprocessError::JsonParseError {
                            field: "json (as output)",
                            source,
                        }
                    })
                })
                .transpose()?;

            (input, output)
        } else {
            (None, None)
        };
        let schema = self.data.schema.as_ref().map(Self::preprocess_eure);
        let schema_errors = self
            .data
            .schema_errors
            .iter()
            .map(|e| e.as_str().to_string())
            .collect();
        let meta_schema_errors = self
            .data
            .meta_schema_errors
            .iter()
            .map(|e| e.as_str().to_string())
            .collect();
        let output_json_schema = self
            .data
            .output_json_schema
            .as_ref()
            .map(|code| {
                serde_json::from_str(code.as_str()).map_err(|source| {
                    PreprocessError::JsonParseError {
                        field: "output_json_schema",
                        source,
                    }
                })
            })
            .transpose()?;
        let json_schema_errors = self
            .data
            .json_schema_errors
            .iter()
            .map(|e| e.as_str().to_string())
            .collect();

        // Formatter testing fields
        let formatted_input = self
            .data
            .formatted_input
            .as_ref()
            .map(|text| text.as_str().to_string());
        let formatted_normalized = self
            .data
            .formatted_normalized
            .as_ref()
            .map(|text| text.as_str().to_string());

        Ok(PreprocessedCase {
            input_eure,
            input_toml,
            input_json,
            normalized,
            output_json,
            schema,
            schema_errors,
            meta_schema_errors,
            output_json_schema,
            json_schema_errors,
            input_union_tag_mode: self.data.input_union_tag_mode,
            formatted_input,
            formatted_normalized,
            completions_scenario: self.data.completions_scenario.clone(),
            diagnostics_scenario: self.data.diagnostics_scenario.clone(),
        })
    }
}

/// Error that can occur during case preprocessing
#[derive(Debug, thiserror::Error)]
pub enum PreprocessError {
    #[error("Failed to parse {field} as JSON: {source}")]
    JsonParseError {
        field: &'static str,
        #[source]
        source: serde_json::Error,
    },
}

pub struct NormalizationScenario<'a> {
    input: &'a PreprocessedEure,
    normalized: &'a PreprocessedEure,
}

impl NormalizationScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        let input_doc = self
            .input
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let normalized_doc =
            self.normalized
                .doc()
                .map_err(|e| ScenarioError::PreprocessingError {
                    message: format!("{}", e),
                })?;
        if input_doc != normalized_doc {
            return Err(ScenarioError::NormalizationMismatch {
                input_debug: format!("{:#?}", input_doc),
                normalized_debug: format!("{:#?}", normalized_doc),
            });
        }
        Ok(())
    }
}

pub struct FormattingScenario<'a> {
    input: &'a PreprocessedEure,
    expected: &'a str,
}

impl FormattingScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        // Get the CST from the preprocessed input
        let cst = self
            .input
            .cst()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;

        let input_str = self.input.input();

        // Format the input using eure-fmt with default config
        let config = eure_fmt::FormatConfig::default();
        let formatted = eure_fmt::format_cst(input_str, cst, &config);

        // Compare formatted output with expected
        if formatted != self.expected {
            return Err(ScenarioError::FormattingMismatch {
                input: input_str.to_string(),
                expected: self.expected.to_string(),
                actual: formatted,
            });
        }
        Ok(())
    }
}

pub struct EureToJsonScenario<'a> {
    input: &'a PreprocessedEure,
    output_json: &'a serde_json::Value,
    source: &'static str,
}

impl EureToJsonScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        let input_doc = self
            .input
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let actual = eure_json::document_to_value(input_doc, &eure_json::Config::default())
            .map_err(|e| ScenarioError::EureToJsonConversionError {
                message: format!("{}", e),
            })?;
        if actual != *self.output_json {
            return Err(ScenarioError::EureToJsonMismatch {
                expected: self.output_json.clone(),
                actual,
            });
        }
        Ok(())
    }
}

pub struct JsonToEureScenario<'a> {
    input_json: &'a serde_json::Value,
    expected: &'a PreprocessedEure,
    source: &'static str,
}

impl JsonToEureScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        // Get the expected document
        let expected_doc = self
            .expected
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;

        // Convert JSON to EureDocument
        let actual_doc =
            eure_json::value_to_document(self.input_json, &eure_json::Config::default()).map_err(
                |e| ScenarioError::JsonToEureConversionError {
                    message: format!("{}", e),
                },
            )?;

        // Compare documents
        if &actual_doc != expected_doc {
            return Err(ScenarioError::JsonToEureMismatch {
                expected_debug: format!("{:#?}", expected_doc),
                actual_debug: format!("{:#?}", actual_doc),
            });
        }
        Ok(())
    }
}

/// Scenario: TOML → EureDocument should equal input_eure → EureDocument
pub struct TomlToEureDocumentScenario<'a> {
    input_toml: &'a PreprocessedToml,
    input_eure: &'a PreprocessedEure,
}

impl TomlToEureDocumentScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        // Get the expected document from input_eure
        let expected_doc =
            self.input_eure
                .doc()
                .map_err(|e| ScenarioError::PreprocessingError {
                    message: format!("{}", e),
                })?;

        // Get the actual document from TOML conversion
        let actual_doc = self.input_toml.doc()?;

        // Compare documents
        if actual_doc != expected_doc {
            return Err(ScenarioError::TomlToEureDocumentMismatch {
                expected_debug: format!("{:#?}", expected_doc),
                actual_debug: format!("{:#?}", actual_doc),
            });
        }
        Ok(())
    }
}

/// Scenario: TOML → JSON should equal input_eure → JSON
pub struct TomlToJsonScenario<'a> {
    input_toml: &'a PreprocessedToml,
    input_eure: &'a PreprocessedEure,
}

impl TomlToJsonScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        // Get the expected JSON from input_eure
        let eure_doc = self
            .input_eure
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let expected_json = eure_json::document_to_value(eure_doc, &eure_json::Config::default())
            .map_err(|e| ScenarioError::EureToJsonConversionError {
            message: format!("{}", e),
        })?;

        // Get the actual JSON from TOML conversion
        let toml_doc = self.input_toml.doc()?;
        let actual_json = eure_json::document_to_value(toml_doc, &eure_json::Config::default())
            .map_err(|e| ScenarioError::EureToJsonConversionError {
                message: format!("{}", e),
            })?;

        // Compare JSON values
        if actual_json != expected_json {
            return Err(ScenarioError::EureToJsonMismatch {
                expected: expected_json,
                actual: actual_json,
            });
        }
        Ok(())
    }
}

pub struct SchemaValidationScenario<'a> {
    input: &'a PreprocessedEure,
    schema: &'a PreprocessedEure,
    union_tag_mode: InputUnionTagMode,
}

impl SchemaValidationScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        let input_doc = self
            .input
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let input_cst = self
            .input
            .cst()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let input_origins =
            self.input
                .origins()
                .map_err(|e| ScenarioError::PreprocessingError {
                    message: format!("{}", e),
                })?;
        let schema_doc = self
            .schema
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let schema_cst = self
            .schema
            .cst()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let schema_origins =
            self.schema
                .origins()
                .map_err(|e| ScenarioError::PreprocessingError {
                    message: format!("{}", e),
                })?;

        // Convert schema document to SchemaDocument
        let (schema, schema_source_map) = eure_schema::convert::document_to_schema(schema_doc)
            .map_err(|e| ScenarioError::SchemaConversionError {
                message: format!("{}", e),
            })?;

        // Convert union tag mode
        let mode = match self.union_tag_mode {
            InputUnionTagMode::Eure => eure_schema::validate::UnionTagMode::Eure,
            InputUnionTagMode::Repr => eure_schema::validate::UnionTagMode::Repr,
        };

        // Validate document with specified mode
        let result = eure_schema::validate::validate_with_mode(input_doc, &schema, mode);

        if !result.is_valid {
            // Format errors with source spans (using V2 for precise key spans)
            let context = eure::error::SchemaErrorContext {
                doc_source: self.input.input(),
                doc_path: "<input>",
                doc_cst: input_cst,
                doc_origins: input_origins,
                schema_source: self.schema.input(),
                schema_path: "<schema>",
                schema_cst,
                schema_origins,
                schema_source_map: &schema_source_map,
            };
            let formatted_errors: Vec<String> = result
                .errors
                .iter()
                .map(|e| eure::error::format_schema_error(e, &context))
                .collect();
            return Err(ScenarioError::SchemaValidationFailed {
                errors: formatted_errors,
            });
        }

        Ok(())
    }
}

pub struct SchemaErrorValidationScenario<'a> {
    input: &'a PreprocessedEure,
    schema: &'a PreprocessedEure,
    expected_errors: &'a [String],
    union_tag_mode: InputUnionTagMode,
}

impl SchemaErrorValidationScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        let input_doc = self
            .input
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let input_cst = self
            .input
            .cst()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let input_origins =
            self.input
                .origins()
                .map_err(|e| ScenarioError::PreprocessingError {
                    message: format!("{}", e),
                })?;
        let schema_doc = self
            .schema
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let schema_cst = self
            .schema
            .cst()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let schema_origins =
            self.schema
                .origins()
                .map_err(|e| ScenarioError::PreprocessingError {
                    message: format!("{}", e),
                })?;

        // Convert schema document to SchemaDocument
        let (schema, schema_source_map) = eure_schema::convert::document_to_schema(schema_doc)
            .map_err(|e| ScenarioError::SchemaConversionError {
                message: format!("{}", e),
            })?;

        // Convert union tag mode
        let mode = match self.union_tag_mode {
            InputUnionTagMode::Eure => eure_schema::validate::UnionTagMode::Eure,
            InputUnionTagMode::Repr => eure_schema::validate::UnionTagMode::Repr,
        };

        // Validate document with specified mode
        let result = eure_schema::validate::validate_with_mode(input_doc, &schema, mode);

        // Should have errors
        if result.is_valid {
            return Err(ScenarioError::ExpectedValidationToFail {
                expected_errors: self.expected_errors.to_vec(),
            });
        }

        // Format errors with source spans using plain text (no ANSI colors)
        // Using V2 for precise key spans
        let context = eure::error::SchemaErrorContext {
            doc_source: self.input.input(),
            doc_path: "<input>",
            doc_cst: input_cst,
            doc_origins: input_origins,
            schema_source: self.schema.input(),
            schema_path: "<schema>",
            schema_cst,
            schema_origins,
            schema_source_map: &schema_source_map,
        };
        let actual_errors: Vec<String> = result
            .errors
            .iter()
            .map(|e| eure::error::format_schema_error_plain(e, &context))
            .collect();

        // Normalize errors by trimming whitespace for comparison
        let actual_errors_trimmed: Vec<String> =
            actual_errors.iter().map(|e| e.trim().to_string()).collect();
        let expected_errors_trimmed: Vec<String> = self
            .expected_errors
            .iter()
            .map(|e| e.trim().to_string())
            .collect();

        // Check exact match between expected and actual errors
        if expected_errors_trimmed.len() != actual_errors_trimmed.len() {
            return Err(ScenarioError::ExpectedErrorNotFound {
                expected: format!(
                    "Expected {} errors, got {}",
                    expected_errors_trimmed.len(),
                    actual_errors_trimmed.len()
                ),
                actual_errors: actual_errors.clone(),
            });
        }

        for expected in &expected_errors_trimmed {
            let found = actual_errors_trimmed
                .iter()
                .any(|actual| actual == expected);
            if !found {
                return Err(ScenarioError::ExpectedErrorNotFound {
                    expected: expected.clone(),
                    actual_errors: actual_errors.clone(),
                });
            }
        }

        Ok(())
    }
}

pub struct EureSchemaToJsonSchemaScenario<'a> {
    schema: &'a PreprocessedEure,
    output_json_schema: &'a serde_json::Value,
}

impl EureSchemaToJsonSchemaScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        let schema_doc = self
            .schema
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;

        // Convert Eure document to SchemaDocument
        let (schema, _source_map) =
            eure_schema::convert::document_to_schema(schema_doc).map_err(|e| {
                ScenarioError::SchemaConversionError {
                    message: format!("{}", e),
                }
            })?;

        // Convert SchemaDocument to JSON Schema
        let json_schema = eure_json_schema::eure_to_json_schema(&schema).map_err(|e| {
            ScenarioError::JsonSchemaConversionError {
                message: format!("{}", e),
            }
        })?;

        // Serialize to serde_json::Value for comparison
        let actual_json: serde_json::Value = serde_json::to_value(&json_schema).map_err(|e| {
            ScenarioError::JsonSerializationError {
                message: e.to_string(),
            }
        })?;

        // Normalize both JSON values by sorting order-independent arrays
        let actual_normalized = normalize_json_schema(&actual_json);
        let expected_normalized = normalize_json_schema(self.output_json_schema);

        // Compare normalized outputs
        if actual_normalized != expected_normalized {
            return Err(ScenarioError::JsonSchemaMismatch {
                expected: expected_normalized,
                actual: actual_normalized,
            });
        }

        Ok(())
    }
}

/// Normalize a JSON Schema value by sorting arrays where order doesn't matter semantically.
/// This includes `required`, `oneOf`, `anyOf`, `allOf` arrays.
fn normalize_json_schema(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;

    match value {
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (key, val) in obj {
                let normalized_val = normalize_json_schema(val);
                // Sort arrays for these keys where order doesn't matter
                let final_val = if matches!(key.as_str(), "required" | "oneOf" | "anyOf") {
                    if let Value::Array(arr) = normalized_val {
                        let mut sorted: Vec<_> = arr.into_iter().collect();
                        sorted.sort_by(|a, b| {
                            let a_str = serde_json::to_string(a).unwrap_or_default();
                            let b_str = serde_json::to_string(b).unwrap_or_default();
                            a_str.cmp(&b_str)
                        });
                        Value::Array(sorted)
                    } else {
                        normalized_val
                    }
                } else {
                    normalized_val
                };
                new_obj.insert(key.clone(), final_val);
            }
            Value::Object(new_obj)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_json_schema).collect()),
        _ => value.clone(),
    }
}

pub struct EureSchemaToJsonSchemaErrorScenario<'a> {
    schema: &'a PreprocessedEure,
    expected_errors: &'a [String],
}

impl EureSchemaToJsonSchemaErrorScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        let schema_doc = self
            .schema
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;

        // Convert Eure document to SchemaDocument
        let (schema, _source_map) =
            eure_schema::convert::document_to_schema(schema_doc).map_err(|e| {
                ScenarioError::SchemaConversionError {
                    message: format!("{}", e),
                }
            })?;

        // Attempt to convert SchemaDocument to JSON Schema
        let result = eure_json_schema::eure_to_json_schema(&schema);

        // Should fail
        let error = match result {
            Ok(_) => {
                return Err(ScenarioError::ExpectedJsonSchemaConversionToFail {
                    expected_errors: self.expected_errors.to_vec(),
                });
            }
            Err(e) => e.to_string(),
        };

        // Check exact match: expect exactly one error that matches (trimmed for whitespace)
        if self.expected_errors.len() != 1 {
            return Err(ScenarioError::ExpectedErrorNotFound {
                expected: format!(
                    "Expected exactly 1 error specification, got {}",
                    self.expected_errors.len()
                ),
                actual_errors: vec![error.clone()],
            });
        }

        let expected = self.expected_errors[0].trim();
        let actual = error.trim();
        if actual != expected {
            return Err(ScenarioError::ExpectedErrorNotFound {
                expected: expected.to_string(),
                actual_errors: vec![error],
            });
        }

        Ok(())
    }
}

// ============================================================================
// Meta Schema Validation Scenario
// ============================================================================

/// Validates a schema against the meta-schema (eure-schema.schema.eure).
/// If expected_errors is empty, validation should pass.
/// If expected_errors is non-empty, validation should fail with those specific errors.
pub struct MetaSchemaScenario<'a> {
    schema: &'a PreprocessedEure,
    expected_errors: &'a [String],
}

impl MetaSchemaScenario<'_> {
    pub fn run(&self) -> Result<(), ScenarioError> {
        let schema_doc = self
            .schema
            .doc()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let schema_cst = self
            .schema
            .cst()
            .map_err(|e| ScenarioError::PreprocessingError {
                message: format!("{}", e),
            })?;
        let schema_origins =
            self.schema
                .origins()
                .map_err(|e| ScenarioError::PreprocessingError {
                    message: format!("{}", e),
                })?;

        // Validate the schema document against the meta-schema
        let meta = &*META_SCHEMA;
        let result = eure_schema::validate::validate(schema_doc, &meta.schema);

        let context = eure::error::SchemaErrorContext {
            doc_source: self.schema.input(),
            doc_path: "<schema>",
            doc_cst: schema_cst,
            doc_origins: schema_origins,
            schema_source: META_SCHEMA_TEXT,
            schema_path: "<meta-schema>",
            schema_cst: &meta.cst,
            schema_origins: &meta.origins,
            schema_source_map: &meta.source_map,
        };

        if self.expected_errors.is_empty() {
            // No errors expected - validation should pass
            if !result.is_valid {
                let formatted_errors: Vec<String> = result
                    .errors
                    .iter()
                    .map(|e| eure::error::format_schema_error(e, &context))
                    .collect();
                return Err(ScenarioError::SchemaValidationFailed {
                    errors: formatted_errors,
                });
            }
        } else {
            // Errors expected - validation should fail
            if result.is_valid {
                return Err(ScenarioError::ExpectedValidationToFail {
                    expected_errors: self.expected_errors.to_vec(),
                });
            }

            // Format errors with source spans using plain text (no ANSI colors)
            let actual_errors: Vec<String> = result
                .errors
                .iter()
                .map(|e| eure::error::format_schema_error_plain(e, &context))
                .collect();

            // Normalize errors by trimming whitespace for comparison
            let actual_errors_trimmed: Vec<String> =
                actual_errors.iter().map(|e| e.trim().to_string()).collect();
            let expected_errors_trimmed: Vec<String> = self
                .expected_errors
                .iter()
                .map(|e| e.trim().to_string())
                .collect();

            // Check exact match between expected and actual errors
            if expected_errors_trimmed.len() != actual_errors_trimmed.len() {
                return Err(ScenarioError::ExpectedErrorNotFound {
                    expected: format!(
                        "Expected {} errors, got {}",
                        expected_errors_trimmed.len(),
                        actual_errors_trimmed.len()
                    ),
                    actual_errors: actual_errors.clone(),
                });
            }

            for expected in &expected_errors_trimmed {
                let found = actual_errors_trimmed
                    .iter()
                    .any(|actual| actual == expected);
                if !found {
                    return Err(ScenarioError::ExpectedErrorNotFound {
                        expected: expected.clone(),
                        actual_errors: actual_errors.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

impl PreprocessedCase {
    /// Returns all scenarios that this case will run.
    /// This is the single source of truth for scenario collection.
    pub fn scenarios(&self) -> Vec<Scenario<'_>> {
        let mut scenarios = Vec::new();

        // Normalization scenario
        if let (Some(input), Some(normalized)) = (&self.input_eure, &self.normalized) {
            scenarios.push(Scenario::Normalization(NormalizationScenario {
                input,
                normalized,
            }));
        }

        // Eure-to-JSON scenarios
        if let (Some(input), Some(output_json)) = (&self.input_eure, &self.output_json) {
            scenarios.push(Scenario::EureToJson(EureToJsonScenario {
                input,
                output_json,
                source: "input_eure",
            }));
        }
        if let (Some(normalized), Some(output_json)) = (&self.normalized, &self.output_json) {
            scenarios.push(Scenario::EureToJson(EureToJsonScenario {
                input: normalized,
                output_json,
                source: "normalized",
            }));
        }

        // JSON-to-Eure scenarios
        if let (Some(input_json), Some(input_eure)) = (&self.input_json, &self.input_eure) {
            scenarios.push(Scenario::JsonToEure(JsonToEureScenario {
                input_json,
                expected: input_eure,
                source: "input_eure",
            }));
        }
        if let (Some(input_json), Some(normalized)) = (&self.input_json, &self.normalized) {
            scenarios.push(Scenario::JsonToEure(JsonToEureScenario {
                input_json,
                expected: normalized,
                source: "normalized",
            }));
        }

        // TOML-to-Eure scenarios
        if let (Some(input_toml), Some(input_eure)) = (&self.input_toml, &self.input_eure) {
            // TOML → EureDocument should equal input_eure → EureDocument
            scenarios.push(Scenario::TomlToEureDocument(TomlToEureDocumentScenario {
                input_toml,
                input_eure,
            }));
            // TOML → JSON should equal input_eure → JSON
            scenarios.push(Scenario::TomlToJson(TomlToJsonScenario {
                input_toml,
                input_eure,
            }));
        }

        // Schema validation scenarios
        if let (Some(input), Some(schema)) = (&self.input_eure, &self.schema) {
            if self.schema_errors.is_empty() {
                // No expected errors - validation should pass
                scenarios.push(Scenario::SchemaValidation(SchemaValidationScenario {
                    input,
                    schema,
                    union_tag_mode: self.input_union_tag_mode,
                }));
            } else {
                // Expected errors - validation should fail with specific errors
                scenarios.push(Scenario::SchemaErrorValidation(
                    SchemaErrorValidationScenario {
                        input,
                        schema,
                        expected_errors: &self.schema_errors,
                        union_tag_mode: self.input_union_tag_mode,
                    },
                ));
            }
        }

        // Meta schema validation scenario
        // When schema is present but input_eure is not, validate schema against meta-schema
        if let Some(schema) = &self.schema
            && self.input_eure.is_none()
        {
            scenarios.push(Scenario::MetaSchema(MetaSchemaScenario {
                schema,
                expected_errors: &self.meta_schema_errors,
            }));
        }

        // Eure Schema to JSON Schema conversion scenarios
        if let Some(schema) = &self.schema {
            if let Some(output_json_schema) = &self.output_json_schema {
                // Success case - conversion should produce expected JSON Schema
                scenarios.push(Scenario::EureSchemaToJsonSchema(
                    EureSchemaToJsonSchemaScenario {
                        schema,
                        output_json_schema,
                    },
                ));
            } else if !self.json_schema_errors.is_empty() {
                // Error case - conversion should fail with expected errors
                scenarios.push(Scenario::EureSchemaToJsonSchemaError(
                    EureSchemaToJsonSchemaErrorScenario {
                        schema,
                        expected_errors: &self.json_schema_errors,
                    },
                ));
            }
        }

        // Formatting scenarios
        if let (Some(input), Some(expected)) = (&self.input_eure, &self.formatted_input) {
            scenarios.push(Scenario::Formatting(FormattingScenario { input, expected }));
        }
        if let (Some(input), Some(expected)) = (&self.normalized, &self.formatted_normalized) {
            scenarios.push(Scenario::Formatting(FormattingScenario { input, expected }));
        }

        // Editor scenarios
        if let Some(completions_scenario) = &self.completions_scenario {
            scenarios.push(Scenario::Completions(completions_scenario));
        }
        if let Some(diagnostics_scenario) = &self.diagnostics_scenario {
            scenarios.push(Scenario::Diagnostics(diagnostics_scenario));
        }

        scenarios
    }

    /// Run all scenarios and return structured results.
    /// This does not panic on assertion failures - it captures them as failed scenarios.
    pub fn run_all(&self, config: &RunConfig) -> CaseResult {
        if config.trace {
            self.print_trace_header();
        }

        let scenarios = self.scenarios();
        if config.trace {
            eprintln!("\n--- Running {} scenarios ---", scenarios.len());
        }

        let results = scenarios
            .into_iter()
            .enumerate()
            .map(|(i, scenario)| {
                let name = scenario.name();
                if config.trace {
                    eprintln!("Running scenario {}: {}", i + 1, name);
                }
                let result = match scenario.run() {
                    Ok(()) => {
                        if config.trace {
                            eprintln!("✓ Scenario {} passed", i + 1);
                        }
                        ScenarioResult::Passed
                    }
                    Err(e) => ScenarioResult::Failed {
                        error: e.to_string(),
                    },
                };
                NamedScenarioResult { name, result }
            })
            .collect();

        if config.trace {
            eprintln!("=== End Debug Trace ===\n");
        }

        CaseResult { scenarios: results }
    }

    fn print_trace_header(&self) {
        eprintln!("\n=== PreprocessedCase Debug Trace ===");
        if let Some(ref input_eure) = self.input_eure {
            eprintln!("input_eure: {}", input_eure.status());
            if !input_eure.is_ok() {
                eprintln!("--- input_eure source ---");
                eprintln!("{}", input_eure.input());
                eprintln!("--- end source ---");
            }
        } else {
            eprintln!("input_eure: None");
        }
        if let Some(ref normalized) = self.normalized {
            eprintln!("normalized: {}", normalized.status());
            if !normalized.is_ok() {
                eprintln!("--- normalized source ---");
                eprintln!("{}", normalized.input());
                eprintln!("--- end source ---");
            }
        } else {
            eprintln!("normalized: None");
        }
        if let Some(ref schema) = self.schema {
            eprintln!("schema: {}", schema.status());
            if !schema.is_ok() {
                eprintln!("--- schema source ---");
                eprintln!("{}", schema.input());
                eprintln!("--- end source ---");
            }
        } else {
            eprintln!("schema: None");
        }
        eprintln!(
            "output_json: {}",
            if self.output_json.is_some() {
                "Some"
            } else {
                "None"
            }
        );
    }

    /// Legacy method that returns Result for backwards compatibility.
    pub fn run(&self, config: &RunConfig) -> eros::Result<()> {
        if config.trace {
            self.print_trace_header();
        }

        let scenarios = self.scenarios();
        if config.trace {
            eprintln!("\n--- Running {} scenarios ---", scenarios.len());
        }

        for (i, scenario) in scenarios.iter().enumerate() {
            if config.trace {
                eprintln!("Running scenario {}: {}", i + 1, scenario.name());
            }
            scenario.run()?;
            if config.trace {
                eprintln!("✓ Scenario {} passed", i + 1);
            }
        }

        if config.trace {
            eprintln!("=== End Debug Trace ===\n");
        }

        Ok(())
    }

    /// Returns the number of scenarios this case will run
    pub fn scenario_count(&self) -> usize {
        self.scenarios().len()
    }

    /// Returns a status summary for error reporting
    pub fn status_summary(&self) -> String {
        let mut lines = Vec::new();

        if let Some(ref input_eure) = self.input_eure
            && let Some(err) = input_eure.detailed_error()
        {
            lines.push(format!("input_eure error:\n{}", err));
        }
        if let Some(ref normalized) = self.normalized
            && let Some(err) = normalized.detailed_error()
        {
            lines.push(format!("normalized error:\n{}", err));
        }
        if let Some(ref schema) = self.schema
            && let Some(err) = schema.detailed_error()
        {
            lines.push(format!("schema error:\n{}", err));
        }

        lines.join("\n\n")
    }

    pub fn normalization_scenario(&self) -> Option<NormalizationScenario<'_>> {
        match (&self.input_eure, &self.normalized) {
            (Some(input), Some(normalized)) => Some(NormalizationScenario { input, normalized }),
            _ => None,
        }
    }

    pub fn eure_to_json_scenario(&self) -> Vec<EureToJsonScenario<'_>> {
        let mut scenarios = Vec::new();
        if let (Some(input), Some(output_json)) = (&self.input_eure, &self.output_json) {
            scenarios.push(EureToJsonScenario {
                input,
                output_json,
                source: "input_eure",
            });
        }
        if let (Some(normalized), Some(output_json)) = (&self.normalized, &self.output_json) {
            scenarios.push(EureToJsonScenario {
                input: normalized,
                output_json,
                source: "normalized",
            });
        }

        scenarios
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a PreprocessedEure from a simple Eure string
    fn preprocess(code: &str) -> PreprocessedEure {
        let input = code.to_string();
        match eure::parol::parse(code) {
            Ok(cst) => match eure::document::cst_to_document_and_origin_map(code, &cst) {
                Ok((doc, origins)) => PreprocessedEure::Ok {
                    input,
                    cst,
                    doc,
                    origins,
                },
                Err(e) => PreprocessedEure::ErrDocument {
                    input,
                    cst,
                    error: e,
                },
            },
            Err(e) => PreprocessedEure::ErrParol { input, error: e },
        }
    }

    #[test]
    fn scenarios_all_fields_present() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            normalized: Some(preprocess("= { a => 1 }")),
            output_json: Some(serde_json::json!({"a": 1})),
            ..Default::default()
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 3);

        let names: Vec<_> = scenarios.iter().map(|s| s.name()).collect();
        assert_eq!(names[0], "normalization");
        assert_eq!(names[1], "eure_to_json(input_eure)");
        assert_eq!(names[2], "eure_to_json(normalized)");
    }

    #[test]
    fn scenarios_input_and_normalized_only() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            normalized: Some(preprocess("= { a => 1 }")),
            ..Default::default()
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].name(), "normalization");
    }

    #[test]
    fn scenarios_input_and_json_only() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            output_json: Some(serde_json::json!({"a": 1})),
            ..Default::default()
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].name(), "eure_to_json(input_eure)");
    }

    #[test]
    fn scenarios_normalized_and_json_only() {
        let case = PreprocessedCase {
            normalized: Some(preprocess("= { a => 1 }")),
            output_json: Some(serde_json::json!({"a": 1})),
            ..Default::default()
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].name(), "eure_to_json(normalized)");
    }

    #[test]
    fn scenarios_input_only() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            ..Default::default()
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenarios_normalized_only() {
        let case = PreprocessedCase {
            normalized: Some(preprocess("= { a => 1 }")),
            ..Default::default()
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenarios_json_only() {
        let case = PreprocessedCase {
            output_json: Some(serde_json::json!({"a": 1})),
            ..Default::default()
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenarios_empty() {
        let case = PreprocessedCase::default();

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenario_count_matches_scenarios_len() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            normalized: Some(preprocess("= { a => 1 }")),
            output_json: Some(serde_json::json!({"a": 1})),
            ..Default::default()
        };

        assert_eq!(case.scenario_count(), case.scenarios().len());
        assert_eq!(case.scenario_count(), 3);
    }
}
