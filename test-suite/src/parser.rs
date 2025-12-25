use std::{collections::BTreeMap, path::PathBuf};

use eure::{
    ParseDocument,
    document::{
        DocumentConstructionError, NodeValue,
        parse::{ParseContext, ParseError as DocumentParseError, ParseErrorKind, RecordParser},
    },
    parol::EureParseError,
    tree::Cst,
    value::{PrimitiveValue, Text, ValueKind},
};

// ============================================================================
// Completions Scenario Types
// ============================================================================

/// A single completion item expected in completions scenario
#[derive(Debug, Clone, ParseDocument)]
pub struct CompletionItem {
    pub label: String,
    #[eure(default)]
    pub kind: Option<String>,
}

/// Completions test scenario
#[derive(Debug, Clone)]
pub struct CompletionsScenario {
    /// Editor content with cursor position marked as `|_|`
    pub editor: Text,
    /// Expected completions (exact match)
    pub completions: Vec<CompletionItem>,
    /// Trigger character (e.g., ".", "@", "=")
    pub trigger: Option<String>,
}

impl CompletionsScenario {
    pub fn run(&self) -> Result<(), crate::case::ScenarioError> {
        Err(crate::case::ScenarioError::Unimplemented {
            scenario_name: "completions".to_string(),
        })
    }
}

// ============================================================================
// Diagnostics Scenario Types
// ============================================================================

/// A single diagnostic item expected in diagnostics scenario
#[derive(Debug, Clone, ParseDocument)]
pub struct DiagnosticItem {
    #[eure(default)]
    pub severity: Option<String>,
    #[eure(default)]
    pub source: Option<String>,
    #[eure(default)]
    pub message: Option<String>,
    #[eure(default)]
    pub code: Option<String>,
}

/// Diagnostics test scenario
#[derive(Debug, Clone)]
pub struct DiagnosticsScenario {
    /// Editor content with cursor position marked as `|_|`
    pub editor: Text,
    /// Expected diagnostics (exact match, empty = no diagnostics expected)
    pub diagnostics: Vec<DiagnosticItem>,
}

impl DiagnosticsScenario {
    pub fn run(&self) -> Result<(), crate::case::ScenarioError> {
        Err(crate::case::ScenarioError::Unimplemented {
            scenario_name: "diagnostics".to_string(),
        })
    }
}

/// Union tag mode for validation tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ParseDocument)]
pub enum InputUnionTagMode {
    /// Use `$variant` extension or untagged matching (default for native Eure)
    #[default]
    Eure,
    /// Use only `VariantRepr` patterns (for JSON/YAML imports)
    Repr,
}

#[derive(Debug, Clone, ParseDocument)]
#[eure(crate = eure::document)]
pub enum JsonData {
    Both {
        json: Text,
    },
    Separate {
        #[eure(default)]
        input_json: Option<Text>,
        #[eure(default)]
        output_json: Option<Text>,
    },
}

impl JsonData {
    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            JsonData::Separate {
                input_json: None,
                output_json: None
            }
        )
    }

    pub fn input_json(&self) -> Option<&Text> {
        match self {
            JsonData::Both { json } => Some(json),
            JsonData::Separate {
                input_json: Some(input),
                ..
            } => Some(input),
            _ => None,
        }
    }

    pub fn output_json(&self) -> Option<&Text> {
        match self {
            JsonData::Both { json } => Some(json),
            JsonData::Separate {
                output_json: Some(output),
                ..
            } => Some(output),
            _ => None,
        }
    }
}

impl Default for JsonData {
    fn default() -> Self {
        JsonData::Separate {
            input_json: None,
            output_json: None,
        }
    }
}

/// JSON Schema data for test cases - supports bidirectional testing.
/// Similar to JsonData, but for JSON Schema conversion tests.
#[derive(Debug, Clone, ParseDocument)]
#[eure(crate = eure::document)]
pub enum JsonSchemaData {
    /// Same JSON Schema used for both input and output
    Both { json_schema: Text },
    /// Separate input and output JSON Schema (for testing round-trip or asymmetric conversion)
    Separate {
        #[eure(default)]
        input_json_schema: Option<Text>,
        #[eure(default)]
        output_json_schema: Option<Text>,
    },
}

impl JsonSchemaData {
    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            JsonSchemaData::Separate {
                input_json_schema: None,
                output_json_schema: None
            }
        )
    }

    pub fn input_json_schema(&self) -> Option<&Text> {
        match self {
            JsonSchemaData::Both { json_schema } => Some(json_schema),
            JsonSchemaData::Separate {
                input_json_schema: Some(input),
                ..
            } => Some(input),
            _ => None,
        }
    }

    pub fn output_json_schema(&self) -> Option<&Text> {
        match self {
            JsonSchemaData::Both { json_schema } => Some(json_schema),
            JsonSchemaData::Separate {
                output_json_schema: Some(output),
                ..
            } => Some(output),
            _ => None,
        }
    }
}

impl Default for JsonSchemaData {
    fn default() -> Self {
        JsonSchemaData::Separate {
            input_json_schema: None,
            output_json_schema: None,
        }
    }
}

/// A single test case's data fields
#[derive(Debug, Clone, Default)]
pub struct CaseData {
    pub input_eure: Option<Text>,
    pub input_toml: Option<Text>,
    pub json: JsonData,
    pub normalized: Option<Text>,
    pub schema: Option<Text>,
    pub schema_errors: Vec<Text>,
    pub schema_conversion_error: Option<Text>,
    pub meta_schema_errors: Vec<Text>,
    pub json_schema: JsonSchemaData,
    pub json_schema_errors: Vec<Text>,
    pub unimplemented: Option<String>,
    /// Union tag mode for validation (default: eure)
    pub input_union_tag_mode: InputUnionTagMode,
    // Formatter testing fields
    pub formatted_input: Option<Text>,
    pub formatted_normalized: Option<Text>,
    // Editor scenarios
    pub completions_scenario: Option<CompletionsScenario>,
    pub diagnostics_scenario: Option<DiagnosticsScenario>,
    // Eure-mark errors (for .eumd file validation)
    pub euremark_errors: Vec<Text>,
}

impl CaseData {
    /// Check if this case has any meaningful content
    pub fn is_empty(&self) -> bool {
        self.input_eure.is_none()
            && self.input_toml.is_none()
            && self.json.is_empty()
            && self.normalized.is_none()
            && self.schema.is_none()
            && self.schema_errors.is_empty()
            && self.schema_conversion_error.is_none()
            && self.meta_schema_errors.is_empty()
            && self.json_schema.is_empty()
            && self.json_schema_errors.is_empty()
            && self.formatted_input.is_none()
            && self.formatted_normalized.is_none()
            && self.completions_scenario.is_none()
            && self.diagnostics_scenario.is_none()
            && self.euremark_errors.is_empty()
    }
}

impl ParseDocument<'_> for CaseData {
    type Error = DocumentParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let input_eure = rec.parse_field_optional::<Text>("input_eure")?;
        let input_toml = rec.parse_field_optional::<Text>("input_toml")?;
        let json = rec.flatten().parse::<JsonData>()?;
        let normalized = rec.parse_field_optional::<Text>("normalized")?;
        let schema = rec.parse_field_optional::<Text>("schema")?;
        let schema_errors = rec
            .parse_field_optional::<Vec<Text>>("schema_errors")?
            .unwrap_or_default();
        let schema_conversion_error =
            rec.parse_field_optional::<Text>("schema_conversion_error")?;
        let meta_schema_errors = rec
            .parse_field_optional::<Vec<Text>>("meta_schema_errors")?
            .unwrap_or_default();
        let json_schema = rec.flatten().parse::<JsonSchemaData>()?;
        let json_schema_errors = rec
            .parse_field_optional::<Vec<Text>>("json_schema_errors")?
            .unwrap_or_default();
        let unimplemented = parse_unimplemented_field(&mut rec)?;
        let input_union_tag_mode = rec
            .parse_field_optional::<InputUnionTagMode>("input_union_tag_mode")?
            .unwrap_or_default();

        // Parse formatter testing fields
        let formatted_input = rec.parse_field_optional::<Text>("formatted_input")?;
        let formatted_normalized = rec.parse_field_optional::<Text>("formatted_normalized")?;

        // Parse editor scenario fields
        let editor = rec.parse_field_optional::<Text>("editor")?;
        let completions = rec.parse_field_optional::<Vec<CompletionItem>>("completions")?;
        let trigger = rec.parse_field_optional::<String>("trigger")?;
        let diagnostics = rec.parse_field_optional::<Vec<DiagnosticItem>>("diagnostics")?;

        // Build scenarios based on which fields are present
        // If completions field exists (even if empty), create CompletionsScenario
        let completions_scenario = match (&editor, completions) {
            (Some(ed), Some(comps)) => Some(CompletionsScenario {
                editor: ed.clone(),
                completions: comps,
                trigger,
            }),
            _ => None,
        };

        // If diagnostics field exists (even if empty), create DiagnosticsScenario
        let diagnostics_scenario = match (&editor, diagnostics) {
            (Some(ed), Some(diags)) => Some(DiagnosticsScenario {
                editor: ed.clone(),
                diagnostics: diags,
            }),
            _ => None,
        };

        // Parse euremark errors
        let euremark_errors = rec
            .parse_field_optional::<Vec<Text>>("euremark_errors")?
            .unwrap_or_default();

        rec.deny_unknown_fields()?;

        Ok(CaseData {
            input_eure,
            input_toml,
            json,
            normalized,
            schema,
            schema_errors,
            schema_conversion_error,
            meta_schema_errors,
            json_schema,
            json_schema_errors,
            unimplemented,
            input_union_tag_mode,
            formatted_input,
            formatted_normalized,
            completions_scenario,
            diagnostics_scenario,
            euremark_errors,
        })
    }
}

/// Parse the special `unimplemented` field which can be:
/// - Not present → None (not unimplemented)
/// - `true` → Some("") (unimplemented, no reason)
/// - `false` → None (not unimplemented)
/// - A string → Some(string) (unimplemented with reason)
fn parse_unimplemented_field<'doc>(
    rec: &mut RecordParser<'doc>,
) -> Result<Option<String>, DocumentParseError> {
    let field_ctx = match rec.field_optional("unimplemented") {
        Some(ctx) => ctx,
        None => return Ok(None),
    };

    let node = field_ctx.node();
    let node_id = field_ctx.node_id();

    match &node.content {
        NodeValue::Primitive(PrimitiveValue::Bool(true)) => Ok(Some(String::new())),
        NodeValue::Primitive(PrimitiveValue::Bool(false)) => Ok(None),
        NodeValue::Primitive(PrimitiveValue::Text(text)) => Ok(Some(text.content.clone())),
        _ => Err(DocumentParseError {
            node_id,
            // unimplemented accepts bool or text, but TypeMismatch only supports one expected type
            kind: ParseErrorKind::TypeMismatch {
                expected: ValueKind::Text,
                actual: node.content.value_kind().unwrap_or(ValueKind::Null),
            },
        }),
    }
}

/// A file containing one or more test cases
#[derive(Debug, Clone)]
pub struct CaseFile {
    pub path: PathBuf,
    pub default_case: CaseData,
    pub named_cases: BTreeMap<String, CaseData>,
}

impl CaseFile {
    /// Returns an iterator over all cases (default + named)
    /// Each item is (name, case_data) where name is "" for default case
    pub fn all_cases(&self) -> impl Iterator<Item = (&str, &CaseData)> {
        let default_iter = if self.default_case.is_empty() {
            None
        } else {
            Some(("", &self.default_case))
        };

        let named_iter = self.named_cases.iter().map(|(k, v)| (k.as_str(), v));

        default_iter.into_iter().chain(named_iter)
    }

    /// Returns the number of cases in this file
    pub fn case_count(&self) -> usize {
        let default_count = if self.default_case.is_empty() { 0 } else { 1 };
        default_count + self.named_cases.len()
    }
}

impl ParseDocument<'_> for CaseFile {
    type Error = DocumentParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        // Parse root-level fields as default case
        let input_eure = rec.parse_field_optional::<Text>("input_eure")?;
        let input_toml = rec.parse_field_optional::<Text>("input_toml")?;
        let json = rec.flatten().parse::<JsonData>()?;
        let normalized = rec.parse_field_optional::<Text>("normalized")?;
        let schema = rec.parse_field_optional::<Text>("schema")?;
        let schema_errors = rec
            .parse_field_optional::<Vec<Text>>("schema_errors")?
            .unwrap_or_default();
        let schema_conversion_error =
            rec.parse_field_optional::<Text>("schema_conversion_error")?;
        let meta_schema_errors = rec
            .parse_field_optional::<Vec<Text>>("meta_schema_errors")?
            .unwrap_or_default();
        let json_schema = rec.flatten().parse::<JsonSchemaData>()?;
        let json_schema_errors = rec
            .parse_field_optional::<Vec<Text>>("json_schema_errors")?
            .unwrap_or_default();
        let unimplemented = parse_unimplemented_field(&mut rec)?;
        let input_union_tag_mode = rec
            .parse_field_optional::<InputUnionTagMode>("input_union_tag_mode")?
            .unwrap_or_default();

        // Parse formatter testing fields
        let formatted_input = rec.parse_field_optional::<Text>("formatted_input")?;
        let formatted_normalized = rec.parse_field_optional::<Text>("formatted_normalized")?;

        // Parse editor scenario fields
        let editor = rec.parse_field_optional::<Text>("editor")?;
        let completions = rec.parse_field_optional::<Vec<CompletionItem>>("completions")?;
        let trigger = rec.parse_field_optional::<String>("trigger")?;
        let diagnostics = rec.parse_field_optional::<Vec<DiagnosticItem>>("diagnostics")?;

        // Build scenarios based on which fields are present
        // If completions field exists (even if empty), create CompletionsScenario
        let completions_scenario = match (&editor, completions) {
            (Some(ed), Some(comps)) => Some(CompletionsScenario {
                editor: ed.clone(),
                completions: comps,
                trigger,
            }),
            _ => None,
        };

        // If diagnostics field exists (even if empty), create DiagnosticsScenario
        let diagnostics_scenario = match (&editor, diagnostics) {
            (Some(ed), Some(diags)) => Some(DiagnosticsScenario {
                editor: ed.clone(),
                diagnostics: diags,
            }),
            _ => None,
        };

        // Parse euremark errors
        let euremark_errors = rec
            .parse_field_optional::<Vec<Text>>("euremark_errors")?
            .unwrap_or_default();

        // Parse named cases from "cases" section
        // Note: Some legacy test files use "cases[]" (array) instead of "cases.<name>" (map).
        // We only support the map format for named cases; arrays are ignored.
        let named_cases = match rec.field_optional("cases") {
            Some(cases_ctx) => {
                // Try to parse as Map<String, CaseData>. If it fails (e.g., it's an array),
                // just return an empty map and let the tests handle unknown fields.
                cases_ctx
                    .parse::<BTreeMap<String, CaseData>>()
                    .unwrap_or_default()
            }
            None => BTreeMap::default(),
        };

        rec.deny_unknown_fields()?;

        Ok(CaseFile {
            path: PathBuf::new(), // Set by caller
            default_case: CaseData {
                input_eure,
                input_toml,
                json,
                normalized,
                schema,
                schema_errors,
                schema_conversion_error,
                meta_schema_errors,
                json_schema,
                json_schema_errors,
                unimplemented,
                input_union_tag_mode,
                formatted_input,
                formatted_normalized,
                completions_scenario,
                diagnostics_scenario,
                euremark_errors,
            },
            named_cases,
        })
    }
}

pub struct ParseResult {
    pub case_file: CaseFile,
    pub cst: Cst,
    pub input: String,
}

#[derive(Debug)]
pub enum ParseError {
    ParolError(EureParseError),
    DocumentConstructionError {
        error: DocumentConstructionError,
        cst: Cst,
    },
    DocumentParseError {
        error: DocumentParseError,
        cst: Cst,
    },
}

#[allow(clippy::result_large_err)]
pub fn parse_case_file(input: &str, path: PathBuf) -> Result<ParseResult, ParseError> {
    let cst = eure::parol::parse(input).map_err(ParseError::ParolError)?;
    let doc = eure::document::cst_to_document(input, &cst).map_err(|e| {
        ParseError::DocumentConstructionError {
            error: e,
            cst: cst.clone(),
        }
    })?;

    let mut case_file: CaseFile =
        doc.parse(doc.get_root_id())
            .map_err(|e| ParseError::DocumentParseError {
                error: e,
                cst: cst.clone(),
            })?;
    case_file.path = path;

    Ok(ParseResult {
        case_file,
        cst,
        input: input.to_string(),
    })
}
