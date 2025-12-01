use std::path::{Path, PathBuf};

use eure::{
    document::{DocumentConstructionError, EureDocument},
    error::format_parse_error,
    parol::EureParseError,
    tree::Cst,
    value::{Language, Text},
};

pub struct Case {
    pub path: PathBuf,
    pub input_eure: Option<Text>,
    pub normalized: Option<Text>,
    pub output_json: Option<Text>,
    /// Schema to validate input_eure against
    pub schema: Option<Text>,
    /// Expected validation errors (for error test cases)
    pub schema_errors: Vec<Text>,
    /// Expected JSON Schema output (for eure-schema to json-schema conversion)
    pub output_json_schema: Option<Text>,
    /// Expected conversion errors (for eure-schema to json-schema error test cases)
    pub json_schema_errors: Vec<Text>,
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
    SchemaValidation(SchemaValidationScenario<'a>),
    SchemaErrorValidation(SchemaErrorValidationScenario<'a>),
    EureSchemaToJsonSchema(EureSchemaToJsonSchemaScenario<'a>),
    EureSchemaToJsonSchemaError(EureSchemaToJsonSchemaErrorScenario<'a>),
}

impl Scenario<'_> {
    pub fn name(&self) -> String {
        match self {
            Scenario::Normalization(_) => "normalization".to_string(),
            Scenario::EureToJson(s) => format!("eure_to_json({})", s.source),
            Scenario::SchemaValidation(_) => "schema_validation".to_string(),
            Scenario::SchemaErrorValidation(_) => "schema_error_validation".to_string(),
            Scenario::EureSchemaToJsonSchema(_) => "eure_schema_to_json_schema".to_string(),
            Scenario::EureSchemaToJsonSchemaError(_) => {
                "eure_schema_to_json_schema_error".to_string()
            }
        }
    }

    pub fn run(&self) -> eros::Result<()> {
        match self {
            Scenario::Normalization(s) => s.run(),
            Scenario::EureToJson(s) => s.run(),
            Scenario::SchemaValidation(s) => s.run(),
            Scenario::SchemaErrorValidation(s) => s.run(),
            Scenario::EureSchemaToJsonSchema(s) => s.run(),
            Scenario::EureSchemaToJsonSchemaError(s) => s.run(),
        }
    }
}

pub struct PreprocessedCase {
    pub input_eure: Option<PreprocessedEure>,
    pub normalized: Option<PreprocessedEure>,
    pub output_json: Option<serde_json::Value>,
    pub schema: Option<PreprocessedEure>,
    pub schema_errors: Vec<String>,
    pub output_json_schema: Option<serde_json::Value>,
    pub json_schema_errors: Vec<String>,
}

pub enum PreprocessedEure {
    Ok {
        input: String,
        cst: Cst,
        doc: EureDocument,
        origins: eure::document::NodeOriginMap,
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
                Some(format_parse_error(error, input, "<test>"))
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

    pub fn origins(&self) -> eros::Result<&eure::document::NodeOriginMap> {
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

impl Case {
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
            Ok(cst) => match eure::document::cst_to_document_and_origins(&input, &cst) {
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
    pub fn preprocess(&self) -> PreprocessedCase {
        let input_eure = self.input_eure.as_ref().map(Self::preprocess_eure);
        let normalized = self.normalized.as_ref().map(Self::preprocess_eure);
        let output_json = self
            .output_json
            .as_ref()
            .map(|code| serde_json::from_str(code.as_str()).unwrap());
        let schema = self.schema.as_ref().map(Self::preprocess_eure);
        let schema_errors = self
            .schema_errors
            .iter()
            .map(|e| e.as_str().to_string())
            .collect();
        let output_json_schema = self
            .output_json_schema
            .as_ref()
            .map(|code| serde_json::from_str(code.as_str()).unwrap());
        let json_schema_errors = self
            .json_schema_errors
            .iter()
            .map(|e| e.as_str().to_string())
            .collect();

        PreprocessedCase {
            input_eure,
            normalized,
            output_json,
            schema,
            schema_errors,
            output_json_schema,
            json_schema_errors,
        }
    }
}

pub struct NormalizationScenario<'a> {
    input: &'a PreprocessedEure,
    normalized: &'a PreprocessedEure,
}

impl NormalizationScenario<'_> {
    pub fn run(&self) -> eros::Result<()> {
        let input_doc = self.input.doc()?;
        let normalized_doc = self.normalized.doc()?;
        assert_eq!(input_doc, normalized_doc);
        Ok(())
    }
}

pub struct EureToJsonScenario<'a> {
    input: &'a PreprocessedEure,
    output_json: &'a serde_json::Value,
    source: &'static str,
}

impl EureToJsonScenario<'_> {
    pub fn run(&self) -> eros::Result<()> {
        let input_doc = self.input.doc()?;
        let output_json = self.output_json;
        assert_eq!(
            eure_json::document_to_value(input_doc, &eure_json::Config::default()).unwrap(),
            *output_json
        );
        Ok(())
    }
}

pub struct SchemaValidationScenario<'a> {
    input: &'a PreprocessedEure,
    schema: &'a PreprocessedEure,
}

impl SchemaValidationScenario<'_> {
    pub fn run(&self) -> eros::Result<()> {
        let input_doc = self.input.doc()?;
        let input_cst = self.input.cst()?;
        let input_origins = self.input.origins()?;
        let schema_doc = self.schema.doc()?;
        let schema_cst = self.schema.cst()?;
        let schema_origins = self.schema.origins()?;

        // Convert schema document to SchemaDocument
        let (schema, schema_source_map) = eure_schema::convert::document_to_schema(schema_doc)
            .map_err(|e| eros::traced!("Schema conversion error: {:?}", e))?;

        // Validate document directly
        let result = eure_schema::validate::validate(input_doc, &schema);

        if !result.is_valid {
            // Format errors with source spans
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
            return Err(eros::traced!(
                "Schema validation failed:\n{}",
                formatted_errors.join("\n")
            ));
        }

        Ok(())
    }
}

pub struct SchemaErrorValidationScenario<'a> {
    input: &'a PreprocessedEure,
    schema: &'a PreprocessedEure,
    expected_errors: &'a [String],
}

impl SchemaErrorValidationScenario<'_> {
    pub fn run(&self) -> eros::Result<()> {
        let input_doc = self.input.doc()?;
        let schema_doc = self.schema.doc()?;

        // Convert schema document to SchemaDocument
        let (schema, _source_map) = eure_schema::convert::document_to_schema(schema_doc)
            .map_err(|e| eros::traced!("Schema conversion error: {:?}", e))?;

        // Validate document directly
        let result = eure_schema::validate::validate(input_doc, &schema);

        // Should have errors
        if result.is_valid {
            return Err(eros::traced!(
                "Expected validation to fail with errors {:?}, but validation passed",
                self.expected_errors
            ));
        }

        // Check that expected errors are present
        let actual_errors: Vec<String> = result.errors.iter().map(|e| e.to_string()).collect();

        for expected in self.expected_errors {
            let found = actual_errors.iter().any(|actual| actual.contains(expected));
            if !found {
                return Err(eros::traced!(
                    "Expected error containing '{}' not found. Actual errors: {:?}",
                    expected,
                    actual_errors
                ));
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
    pub fn run(&self) -> eros::Result<()> {
        let schema_doc = self.schema.doc()?;

        // Convert Eure document to SchemaDocument
        let (schema, _source_map) = eure_schema::convert::document_to_schema(schema_doc)
            .map_err(|e| eros::traced!("Schema conversion error: {:?}", e))?;

        // Convert SchemaDocument to JSON Schema
        let json_schema = eure_json_schema::eure_to_json_schema(&schema)
            .map_err(|e| eros::traced!("JSON Schema conversion error: {:?}", e))?;

        // Serialize to serde_json::Value for comparison
        let actual_json: serde_json::Value = serde_json::to_value(&json_schema)
            .map_err(|e| eros::traced!("JSON serialization error: {}", e))?;

        // Normalize both JSON values by sorting order-independent arrays
        let actual_normalized = normalize_json_schema(&actual_json);
        let expected_normalized = normalize_json_schema(self.output_json_schema);

        // Compare normalized outputs
        assert_eq!(
            actual_normalized,
            expected_normalized,
            "JSON Schema output mismatch.\nExpected: {}\nActual: {}",
            serde_json::to_string_pretty(&expected_normalized).unwrap(),
            serde_json::to_string_pretty(&actual_normalized).unwrap()
        );

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
    pub fn run(&self) -> eros::Result<()> {
        let schema_doc = self.schema.doc()?;

        // Convert Eure document to SchemaDocument
        let (schema, _source_map) = eure_schema::convert::document_to_schema(schema_doc)
            .map_err(|e| eros::traced!("Schema conversion error: {:?}", e))?;

        // Attempt to convert SchemaDocument to JSON Schema
        let result = eure_json_schema::eure_to_json_schema(&schema);

        // Should fail
        let error = match result {
            Ok(_) => {
                return Err(eros::traced!(
                    "Expected JSON Schema conversion to fail with errors {:?}, but conversion succeeded",
                    self.expected_errors
                ));
            }
            Err(e) => e.to_string(),
        };

        // Check that expected errors are present
        for expected in self.expected_errors {
            if !error.contains(expected) {
                return Err(eros::traced!(
                    "Expected error containing '{}' not found. Actual error: {}",
                    expected,
                    error
                ));
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

        // Schema validation scenarios
        if let (Some(input), Some(schema)) = (&self.input_eure, &self.schema) {
            if self.schema_errors.is_empty() {
                // No expected errors - validation should pass
                scenarios.push(Scenario::SchemaValidation(SchemaValidationScenario {
                    input,
                    schema,
                }));
            } else {
                // Expected errors - validation should fail with specific errors
                scenarios.push(Scenario::SchemaErrorValidation(
                    SchemaErrorValidationScenario {
                        input,
                        schema,
                        expected_errors: &self.schema_errors,
                    },
                ));
            }
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
                let result =
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scenario.run()))
                    {
                        Ok(Ok(())) => {
                            if config.trace {
                                eprintln!("✓ Scenario {} passed", i + 1);
                            }
                            ScenarioResult::Passed
                        }
                        Ok(Err(e)) => ScenarioResult::Failed {
                            error: format!("{:?}", e),
                        },
                        Err(panic) => {
                            let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = panic.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "Unknown panic".to_string()
                            };
                            ScenarioResult::Failed {
                                error: format!("panic: {}", msg),
                            }
                        }
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
            Ok(cst) => match eure::document::cst_to_document_and_origins(code, &cst) {
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
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
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
            output_json: None,
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].name(), "normalization");
    }

    #[test]
    fn scenarios_input_and_json_only() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            normalized: None,
            output_json: Some(serde_json::json!({"a": 1})),
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].name(), "eure_to_json(input_eure)");
    }

    #[test]
    fn scenarios_normalized_and_json_only() {
        let case = PreprocessedCase {
            input_eure: None,
            normalized: Some(preprocess("= { a => 1 }")),
            output_json: Some(serde_json::json!({"a": 1})),
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].name(), "eure_to_json(normalized)");
    }

    #[test]
    fn scenarios_input_only() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            normalized: None,
            output_json: None,
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenarios_normalized_only() {
        let case = PreprocessedCase {
            input_eure: None,
            normalized: Some(preprocess("= { a => 1 }")),
            output_json: None,
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenarios_json_only() {
        let case = PreprocessedCase {
            input_eure: None,
            normalized: None,
            output_json: Some(serde_json::json!({"a": 1})),
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenarios_empty() {
        let case = PreprocessedCase {
            input_eure: None,
            normalized: None,
            output_json: None,
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        let scenarios = case.scenarios();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn scenario_count_matches_scenarios_len() {
        let case = PreprocessedCase {
            input_eure: Some(preprocess("a = 1")),
            normalized: Some(preprocess("= { a => 1 }")),
            output_json: Some(serde_json::json!({"a": 1})),
            schema: None,
            schema_errors: vec![],
            output_json_schema: None,
            json_schema_errors: vec![],
        };

        assert_eq!(case.scenario_count(), case.scenarios().len());
        assert_eq!(case.scenario_count(), 3);
    }
}
