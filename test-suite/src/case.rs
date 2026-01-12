use std::path::PathBuf;

use eure::query::error::EureQueryError;
use eure::query::{
    OpenDocuments, OpenDocumentsList, TextFile, TextFileContent, UnionTagMode, Workspace,
    WorkspaceId, build_runtime,
};
use eure_document::Text;
use query_flow::{Db, DurabilityLevel, QueryRuntime};

use crate::parser::{CaseData, InputUnionTagMode};

// Convert InputUnionTagMode to UnionTagMode
impl From<InputUnionTagMode> for UnionTagMode {
    fn from(mode: InputUnionTagMode) -> Self {
        match mode {
            InputUnionTagMode::Eure => UnionTagMode::Eure,
            InputUnionTagMode::Repr => UnionTagMode::Repr,
        }
    }
}
use crate::scenarios::completions::CompletionsScenario;
use crate::scenarios::diagnostics::DiagnosticsScenario;
use crate::scenarios::eumd_error_validation::EumdErrorValidationScenario;
use crate::scenarios::eure_schema_to_json_schema::EureSchemaToJsonSchemaScenario;
use crate::scenarios::eure_schema_to_json_schema_error::EureSchemaToJsonSchemaErrorScenario;
use crate::scenarios::eure_to_json::EureToJsonScenario;
use crate::scenarios::formatting::FormattingScenario;
use crate::scenarios::json_to_eure::JsonToEureScenario;
use crate::scenarios::meta_schema::MetaSchemaScenario;
use crate::scenarios::normalization::NormalizationScenario;
use crate::scenarios::schema_conversion_error::SchemaConversionErrorScenario;
use crate::scenarios::schema_error_validation::SchemaErrorValidationScenario;
use crate::scenarios::schema_validation::SchemaValidationScenario;
use crate::scenarios::toml_to_eure_document::TomlToEureDocumentScenario;
use crate::scenarios::toml_to_eure_source::TomlToEureSourceScenario;
use crate::scenarios::toml_to_json::TomlToJsonScenario;
use crate::scenarios::{Scenario as ScenarioTrait, ScenarioError};

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

// ============================================================================
// Asset paths
// ============================================================================

const INPUT_EURE_PATH: &str = "input.eure";
const NORMALIZED_PATH: &str = "normalized.eure";
const SCHEMA_PATH: &str = "schema.eure";
const INPUT_TOML_PATH: &str = "input.toml";
const INPUT_JSON_PATH: &str = "input.json";
const OUTPUT_JSON_PATH: &str = "output.json";
const FORMATTED_INPUT_PATH: &str = "formatted_input.eure";
const FORMATTED_NORMALIZED_PATH: &str = "formatted_normalized.eure";
const OUTPUT_JSON_SCHEMA_PATH: &str = "output.json-schema.json";
const EDITOR_PATH: &str = "editor.eure";
const WORKSPACE_PATH: &str = "/test-workspace";
const META_SCHEMA_PATH: &str = "$eure/meta-schema.eure";

/// Bundled meta-schema content
const META_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/schemas/eure-schema.schema.eure"
));

// ============================================================================
// Scenario enum
// ============================================================================

/// A runnable scenario with its name
pub enum Scenario {
    Normalization(NormalizationScenario),
    Formatting(FormattingScenario),
    EureToJson(EureToJsonScenario),
    JsonToEure(JsonToEureScenario),
    TomlToEureDocument(TomlToEureDocumentScenario),
    TomlToJson(TomlToJsonScenario),
    TomlToEureSource(TomlToEureSourceScenario),
    SchemaValidation(SchemaValidationScenario),
    SchemaErrorValidation(SchemaErrorValidationScenario),
    SchemaConversionError(SchemaConversionErrorScenario),
    MetaSchema(MetaSchemaScenario),
    EureSchemaToJsonSchema(EureSchemaToJsonSchemaScenario),
    EureSchemaToJsonSchemaError(EureSchemaToJsonSchemaErrorScenario),
    EumdErrorValidation(EumdErrorValidationScenario),
    Completions(CompletionsScenario),
    Diagnostics(DiagnosticsScenario),
}

impl Scenario {
    pub fn name(&self) -> String {
        match self {
            Scenario::Normalization(_) => "normalization".to_string(),
            Scenario::Formatting(_) => "formatting".to_string(),
            Scenario::EureToJson(s) => format!("eure_to_json({})", s.source_name),
            Scenario::JsonToEure(s) => format!("json_to_eure({})", s.source_name),
            Scenario::TomlToEureDocument(_) => "toml_to_eure_document".to_string(),
            Scenario::TomlToJson(_) => "toml_to_json".to_string(),
            Scenario::TomlToEureSource(_) => "toml_to_eure_source".to_string(),
            Scenario::SchemaValidation(_) => "schema_validation".to_string(),
            Scenario::SchemaErrorValidation(_) => "schema_error_validation".to_string(),
            Scenario::SchemaConversionError(_) => "schema_conversion_error".to_string(),
            Scenario::MetaSchema(_) => "meta_schema".to_string(),
            Scenario::EureSchemaToJsonSchema(_) => "eure_schema_to_json_schema".to_string(),
            Scenario::EureSchemaToJsonSchemaError(_) => {
                "eure_schema_to_json_schema_error".to_string()
            }
            Scenario::EumdErrorValidation(_) => "eumd_error_validation".to_string(),
            Scenario::Completions(_) => "completions".to_string(),
            Scenario::Diagnostics(_) => "diagnostics".to_string(),
        }
    }

    pub fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        match self {
            Scenario::Normalization(s) => s.run(db),
            Scenario::Formatting(s) => s.run(db),
            Scenario::EureToJson(s) => s.run(db),
            Scenario::JsonToEure(s) => s.run(db),
            Scenario::TomlToEureDocument(s) => s.run(db),
            Scenario::TomlToJson(s) => s.run(db),
            Scenario::TomlToEureSource(s) => s.run(db),
            Scenario::SchemaValidation(s) => s.run(db),
            Scenario::SchemaErrorValidation(s) => s.run(db),
            Scenario::SchemaConversionError(s) => s.run(db),
            Scenario::MetaSchema(s) => s.run(db),
            Scenario::EureSchemaToJsonSchema(s) => s.run(db),
            Scenario::EureSchemaToJsonSchemaError(s) => s.run(db),
            Scenario::EumdErrorValidation(s) => s.run(db),
            Scenario::Completions(s) => s.run(db),
            Scenario::Diagnostics(s) => s.run(db),
        }
    }
}

// ============================================================================
// Case implementation
// ============================================================================

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
        self.data.unimplemented.as_ref().and_then(|r| r.as_str())
    }

    /// Check if implicit schema is used (editor + schema provided, but no $schema in editor)
    fn uses_implicit_schema(&self) -> bool {
        self.data.editor.is_some()
            && self.data.schema.is_some()
            && !self
                .data
                .editor
                .as_ref()
                .unwrap()
                .as_str()
                .contains("$schema")
    }

    /// Get the editor file path, considering implicit schema workspace setup
    fn editor_file_path(&self) -> String {
        if self.uses_implicit_schema() {
            format!("{}/{}", WORKSPACE_PATH, EDITOR_PATH)
        } else {
            EDITOR_PATH.to_string()
        }
    }

    /// Get the schema file path, considering implicit schema workspace setup
    fn schema_file_path(&self) -> String {
        if self.uses_implicit_schema() {
            format!("{}/{}", WORKSPACE_PATH, SCHEMA_PATH)
        } else {
            SCHEMA_PATH.to_string()
        }
    }

    pub fn resolve_path(text: &Text, default_path: &str) -> TextFile {
        if text.language.is_other("path") {
            TextFile::from_path(PathBuf::from(text.as_str()))
        } else {
            TextFile::from_path(PathBuf::from(default_path))
        }
    }

    pub fn resolve_asset(
        runtime: &QueryRuntime,
        default_path: &str,
        text: &Text,
    ) -> Result<(), ScenarioError> {
        let file = Self::resolve_path(text, default_path);
        if text.language.is_other("path") {
            let content = std::fs::read_to_string(text.as_str()).map_err(|e| {
                ScenarioError::FileReadError {
                    file: file.clone(),
                    error: format!("{e}"),
                }
            })?;
            runtime.resolve_asset(file, TextFileContent(content), DurabilityLevel::Static);
        } else {
            runtime.resolve_asset(
                file,
                TextFileContent(text.as_str().to_string()),
                DurabilityLevel::Static,
            );
        }
        Ok(())
    }

    /// Resolve all assets for this case into the runtime
    pub fn resolve_assets(&self, runtime: &query_flow::QueryRuntime) -> Result<(), ScenarioError> {
        // input_eure → "input.eure"
        if let Some(input_eure) = &self.data.input_eure {
            Self::resolve_asset(runtime, INPUT_EURE_PATH, input_eure)?;
        }

        // normalized → "normalized.eure"
        if let Some(normalized) = &self.data.normalized {
            Self::resolve_asset(runtime, NORMALIZED_PATH, normalized)?;
        }

        // schema → "schema.eure" or "/test-workspace/schema.eure" for implicit schema
        if let Some(schema) = &self.data.schema {
            Self::resolve_asset(runtime, &self.schema_file_path(), schema)?;
        }

        // input_toml → "input.toml"
        if let Some(input_toml) = &self.data.input_toml {
            Self::resolve_asset(runtime, INPUT_TOML_PATH, input_toml)?;
        }

        // input_json → "input.json"
        if let Some(input_json) = self.data.json.input_json() {
            Self::resolve_asset(runtime, INPUT_JSON_PATH, input_json)?;
        }

        // output_json → "output.json"
        if let Some(output_json) = self.data.json.output_json() {
            Self::resolve_asset(runtime, OUTPUT_JSON_PATH, output_json)?;
        }

        // formatted_input → "formatted_input.eure"
        if let Some(formatted_input) = &self.data.formatted_input {
            Self::resolve_asset(runtime, FORMATTED_INPUT_PATH, formatted_input)?;
        }

        // formatted_normalized → "formatted_normalized.eure"
        if let Some(formatted_normalized) = &self.data.formatted_normalized {
            Self::resolve_asset(runtime, FORMATTED_NORMALIZED_PATH, formatted_normalized)?;
        }

        // output_json_schema → "output.json-schema.json"
        if let Some(output_json_schema) = self.data.json_schema.output_json_schema() {
            Self::resolve_asset(runtime, OUTPUT_JSON_SCHEMA_PATH, output_json_schema)?;
        }

        // editor → "editor.eure" or "/test-workspace/editor.eure" for implicit schema
        if let Some(editor) = &self.data.editor {
            Self::resolve_asset(runtime, &self.editor_file_path(), editor)?;
        }

        // meta-schema → "$eure/meta-schema.eure" (always available)
        runtime.resolve_asset(
            TextFile::from_path(PathBuf::from(META_SCHEMA_PATH)),
            TextFileContent(META_SCHEMA.to_string()),
            DurabilityLevel::Static,
        );

        // OpenDocuments → list of open documents for diagnostics collection
        // When editor is present, set it as the open document for CollectDiagnosticTargets
        if let Some(editor) = &self.data.editor {
            let editor_path = self.editor_file_path();
            runtime.resolve_asset(
                OpenDocuments,
                OpenDocumentsList(vec![Self::resolve_path(editor, &editor_path)]),
                DurabilityLevel::Volatile,
            );

            // Implicit schema via workspace config:
            // When both editor and schema are provided, but editor doesn't have $schema,
            // set up workspace config to associate editor with schema implicitly.
            if self.uses_implicit_schema() {
                let workspace_path = PathBuf::from(WORKSPACE_PATH);
                let config_path = workspace_path.join("Eure.eure");

                // Register workspace
                runtime.resolve_asset(
                    WorkspaceId("test".to_string()),
                    Workspace {
                        path: workspace_path.clone(),
                        config_path: config_path.clone(),
                    },
                    DurabilityLevel::Static,
                );

                // Register workspace config that maps editor.eure to schema.eure
                let config_content = format!(
                    r#"targets.default {{
    globs = ["{editor_path}"]
    schema = "{schema_path}"
}}"#,
                    editor_path = self.editor_file_path(),
                    schema_path = self.schema_file_path(),
                );
                runtime.resolve_asset(
                    TextFile::from_path(config_path),
                    TextFileContent(config_content),
                    DurabilityLevel::Static,
                );
            }
        }

        Ok(())
    }

    /// Returns all scenarios that this case will run.
    /// This is the single source of truth for scenario collection.
    pub fn scenarios(&self) -> Vec<Scenario> {
        let mut scenarios = Vec::new();

        // Normalization scenario
        if let (Some(input_eure), Some(normalized)) = (&self.data.input_eure, &self.data.normalized)
        {
            scenarios.push(Scenario::Normalization(NormalizationScenario {
                input: Self::resolve_path(input_eure, INPUT_EURE_PATH),
                normalized: Self::resolve_path(normalized, NORMALIZED_PATH),
            }));
        }

        // Eure-to-JSON scenarios
        if let (Some(input_eure), Some(output_json)) =
            (&self.data.input_eure, &self.data.json.output_json())
        {
            scenarios.push(Scenario::EureToJson(EureToJsonScenario {
                input: Self::resolve_path(input_eure, INPUT_EURE_PATH),
                output_json: Self::resolve_path(output_json, OUTPUT_JSON_PATH),
                source_name: "input_eure",
            }));
        }
        if let (Some(normalized), Some(output_json)) =
            (&self.data.normalized, &self.data.json.output_json())
        {
            scenarios.push(Scenario::EureToJson(EureToJsonScenario {
                input: Self::resolve_path(normalized, NORMALIZED_PATH),
                output_json: Self::resolve_path(output_json, OUTPUT_JSON_PATH),
                source_name: "normalized",
            }));
        }

        // JSON-to-Eure scenarios
        if let (Some(input_json), Some(input_eure)) =
            (&self.data.json.input_json(), &self.data.input_eure)
        {
            scenarios.push(Scenario::JsonToEure(JsonToEureScenario {
                input_json: Self::resolve_path(input_json, INPUT_JSON_PATH),
                expected: Self::resolve_path(input_eure, INPUT_EURE_PATH),
                source_name: "input_eure",
            }));
        }
        if let (Some(input_json), Some(normalized)) =
            (&self.data.json.input_json(), &self.data.normalized)
        {
            scenarios.push(Scenario::JsonToEure(JsonToEureScenario {
                input_json: Self::resolve_path(input_json, INPUT_JSON_PATH),
                expected: Self::resolve_path(normalized, NORMALIZED_PATH),
                source_name: "normalized",
            }));
        }

        // TOML-to-Eure scenarios
        if let (Some(input_toml), Some(input_eure)) = (&self.data.input_toml, &self.data.input_eure)
        {
            let input_toml = Self::resolve_path(input_toml, INPUT_TOML_PATH);
            let input_eure = Self::resolve_path(input_eure, INPUT_EURE_PATH);
            scenarios.push(Scenario::TomlToEureDocument(TomlToEureDocumentScenario {
                input_toml: input_toml.clone(),
                input_eure: input_eure.clone(),
            }));
            scenarios.push(Scenario::TomlToJson(TomlToJsonScenario {
                input_toml: input_toml.clone(),
                input_eure: input_eure.clone(),
            }));
            scenarios.push(Scenario::TomlToEureSource(TomlToEureSourceScenario {
                input_toml,
                input_eure,
            }));
        }

        // Schema validation scenarios
        if let (Some(input_eure), Some(schema)) = (&self.data.input_eure, &self.data.schema) {
            let input = Self::resolve_path(input_eure, INPUT_EURE_PATH);
            let schema = Self::resolve_path(schema, SCHEMA_PATH);
            if self.data.schema_errors.is_empty() {
                scenarios.push(Scenario::SchemaValidation(SchemaValidationScenario {
                    input,
                    schema,
                    union_tag_mode: self.data.input_union_tag_mode,
                }));
            } else {
                let expected_errors: Vec<String> = self
                    .data
                    .schema_errors
                    .iter()
                    .map(|e| e.as_str().to_string())
                    .collect();
                scenarios.push(Scenario::SchemaErrorValidation(
                    SchemaErrorValidationScenario {
                        input,
                        schema,
                        expected_errors,
                        union_tag_mode: self.data.input_union_tag_mode,
                    },
                ));
            }
        }

        // Schema conversion scenario
        if let Some(schema) = &self.data.schema {
            scenarios.push(Scenario::SchemaConversionError(
                SchemaConversionErrorScenario {
                    schema: Self::resolve_path(schema, &self.schema_file_path()),
                    expected_error: self
                        .data
                        .schema_conversion_error
                        .as_ref()
                        .map(|e| e.as_str().to_string()),
                },
            ));
        }

        // Meta schema validation scenario
        if let Some(schema) = &self.data.schema {
            let expected_errors: Vec<String> = self
                .data
                .meta_schema_errors
                .iter()
                .map(|e| e.as_str().to_string())
                .collect();
            scenarios.push(Scenario::MetaSchema(MetaSchemaScenario {
                schema: Self::resolve_path(schema, &self.schema_file_path()),
                meta_schema: TextFile::from_path(PathBuf::from(META_SCHEMA_PATH)),
                expected_errors,
            }));
        }

        // Eure Schema to JSON Schema conversion scenarios
        if let Some(schema) = &self.data.schema {
            if let Some(output_json_schema) = &self.data.json_schema.output_json_schema() {
                scenarios.push(Scenario::EureSchemaToJsonSchema(
                    EureSchemaToJsonSchemaScenario {
                        schema: Self::resolve_path(schema, SCHEMA_PATH),
                        output_json_schema: Self::resolve_path(
                            output_json_schema,
                            OUTPUT_JSON_SCHEMA_PATH,
                        ),
                    },
                ));
            } else if !self.data.json_schema_errors.is_empty() {
                let expected_errors: Vec<String> = self
                    .data
                    .json_schema_errors
                    .iter()
                    .map(|e| e.as_str().to_string())
                    .collect();
                scenarios.push(Scenario::EureSchemaToJsonSchemaError(
                    EureSchemaToJsonSchemaErrorScenario {
                        schema: Self::resolve_path(schema, SCHEMA_PATH),
                        expected_errors,
                    },
                ));
            }
        }

        // Formatting scenarios
        if let (Some(input_eure), Some(formatted_input)) =
            (&self.data.input_eure, &self.data.formatted_input)
        {
            scenarios.push(Scenario::Formatting(FormattingScenario {
                input: Self::resolve_path(input_eure, INPUT_EURE_PATH),
                expected: Self::resolve_path(formatted_input, FORMATTED_INPUT_PATH),
            }));
        }
        if let (Some(normalized), Some(formatted_normalized)) =
            (&self.data.normalized, &self.data.formatted_normalized)
        {
            scenarios.push(Scenario::Formatting(FormattingScenario {
                input: Self::resolve_path(normalized, NORMALIZED_PATH),
                expected: Self::resolve_path(formatted_normalized, FORMATTED_NORMALIZED_PATH),
            }));
        }

        // Eure-mark error validation scenario
        if let Some(input_eure) = &self.data.input_eure
            && !self.data.euremark_errors.is_empty()
        {
            let expected_errors: Vec<String> = self
                .data
                .euremark_errors
                .iter()
                .map(|e| e.as_str().to_string())
                .collect();
            scenarios.push(Scenario::EumdErrorValidation(EumdErrorValidationScenario {
                input: Self::resolve_path(input_eure, INPUT_EURE_PATH),
                expected_errors,
            }));
        }

        // Editor scenarios (completions, diagnostics)
        // When 'editor' is present, we create:
        // - Diagnostics scenario: always (empty diagnostics = expect zero diagnostics)
        // - Completions scenario: when trigger is specified
        if let Some(editor) = &self.data.editor {
            // Diagnostics scenario - always run when editor is present
            // Include schema if present for validation tests
            let schema = self
                .data
                .schema
                .as_ref()
                .map(|s| Self::resolve_path(s, &self.schema_file_path()));
            scenarios.push(Scenario::Diagnostics(DiagnosticsScenario {
                editor: Self::resolve_path(editor, &self.editor_file_path()),
                schema,
                diagnostics: self.data.diagnostics.clone(),
            }));

            // Completions scenario - run when trigger is specified
            if self.data.trigger.is_some() {
                scenarios.push(Scenario::Completions(CompletionsScenario {
                    editor: Self::resolve_path(editor, &self.editor_file_path()),
                    completions: self.data.completions.clone(),
                    trigger: self.data.trigger.clone(),
                }));
            }
        }

        scenarios
    }

    /// Run all scenarios and return structured results.
    /// This does not panic on assertion failures - it captures them as failed scenarios.
    pub fn run_all(&self, config: &RunConfig) -> CaseResult {
        // Create a new QueryRuntime for this case
        let runtime = build_runtime();
        self.resolve_assets(&runtime)
            .expect("Failed to resolve assets");

        if config.trace {
            self.print_trace_header();
        }

        // First pass: run scenarios and collect pending assets
        let scenarios = self.scenarios();
        if config.trace {
            eprintln!("\n--- Running {} scenarios (pass 1) ---", scenarios.len());
        }

        for scenario in scenarios {
            if config.trace {
                eprintln!("Running scenario: {}", scenario.name());
            }
            // Run scenario (ignore result, we'll re-run after resolving pending assets)
            let _ = scenario.run(&runtime);
        }

        // Resolve any pending assets with ContentNotFound
        // This handles cases like $schema pointing to non-existing files
        let pending = runtime.pending_assets();
        if !pending.is_empty() {
            if config.trace {
                eprintln!("\n--- Resolving {} pending assets ---", pending.len());
            }
            for pending_asset in pending {
                if let Some(file) = pending_asset.key::<TextFile>() {
                    if config.trace {
                        eprintln!("Resolving missing file: {}", file);
                    }
                    runtime.resolve_asset_error::<TextFile>(
                        file.clone(),
                        EureQueryError::ContentNotFound(file.clone()),
                        DurabilityLevel::Static,
                    );
                }
            }
        }

        // Second pass: run scenarios with all assets resolved
        let scenarios = self.scenarios();
        if config.trace {
            eprintln!("\n--- Running {} scenarios (pass 2) ---", scenarios.len());
        }

        let results = scenarios
            .into_iter()
            .enumerate()
            .map(|(i, scenario)| {
                let name = scenario.name();
                if config.trace {
                    eprintln!("Running scenario {}: {}", i + 1, name);
                }
                let result = match scenario.run(&runtime) {
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
        eprintln!("\n=== Case Debug Trace ===");
        eprintln!("Path: {:?}", self.path);
        eprintln!("Name: {}", self.name);
        eprintln!(
            "input_eure: {}",
            if self.data.input_eure.is_some() {
                "Some"
            } else {
                "None"
            }
        );
        eprintln!(
            "normalized: {}",
            if self.data.normalized.is_some() {
                "Some"
            } else {
                "None"
            }
        );
        eprintln!(
            "schema: {}",
            if self.data.schema.is_some() {
                "Some"
            } else {
                "None"
            }
        );
        eprintln!(
            "output_json: {}",
            if self.data.json.output_json().is_some() {
                "Some"
            } else {
                "None"
            }
        );
    }

    /// Returns the number of scenarios this case will run
    pub fn scenario_count(&self) -> usize {
        self.scenarios().len()
    }
}
