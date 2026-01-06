use std::{collections::BTreeMap, path::PathBuf};

use eure::{
    BuildSchema, ParseDocument,
    document::{DocumentConstructionError, parse::ParseError as DocumentParseError},
    parol::EureParseError,
    tree::Cst,
    value::Text,
};

// ============================================================================
// Completions Scenario Types
// ============================================================================

/// A single completion item expected in completions scenario
#[derive(Debug, Clone, ParseDocument, BuildSchema)]
pub struct CompletionItem {
    pub label: String,
    #[eure(default)]
    pub kind: Option<String>,
}

// ============================================================================
// Diagnostics Scenario Types
// ============================================================================

/// A single diagnostic item expected in diagnostics scenario
#[derive(Debug, Clone, ParseDocument, BuildSchema)]
pub struct DiagnosticItem {
    #[eure(default)]
    pub severity: Option<String>,
    #[eure(default)]
    pub source: Option<String>,
    #[eure(default)]
    pub message: Option<String>,
    #[eure(default)]
    pub code: Option<String>,
    /// Expected start byte offset (for span verification)
    #[eure(default)]
    pub start: Option<i64>,
    /// Expected end byte offset (for span verification)
    #[eure(default)]
    pub end: Option<i64>,
}

/// Union tag mode for validation tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ParseDocument, BuildSchema)]
pub enum InputUnionTagMode {
    /// Use `$variant` extension or untagged matching (default for native Eure)
    #[default]
    Eure,
    /// Use only `VariantRepr` patterns (for JSON/YAML imports)
    #[eure(rename = "repr")]
    Repr,
}

#[derive(Debug, Clone, ParseDocument, BuildSchema)]
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
#[derive(Debug, Clone, ParseDocument, BuildSchema)]
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
#[derive(Debug, Clone, Default, ParseDocument, BuildSchema)]
pub struct CaseData {
    #[eure(default)]
    pub input_eure: Option<Text>,
    #[eure(default)]
    pub input_toml: Option<Text>,
    #[eure(flatten)]
    pub json: JsonData,
    #[eure(default)]
    pub normalized: Option<Text>,
    #[eure(default)]
    pub schema: Option<Text>,
    #[eure(default)]
    pub schema_errors: Vec<Text>,
    #[eure(default)]
    pub schema_conversion_error: Option<Text>,
    #[eure(default)]
    pub meta_schema_errors: Vec<Text>,
    #[eure(flatten)]
    pub json_schema: JsonSchemaData,
    #[eure(default)]
    pub json_schema_errors: Vec<Text>,
    #[eure(default)]
    pub unimplemented: Option<UnimplementedReason>,
    /// Union tag mode for validation (default: eure)
    #[eure(default)]
    pub input_union_tag_mode: InputUnionTagMode,
    // Formatter testing fields
    #[eure(default)]
    pub formatted_input: Option<Text>,
    #[eure(default)]
    pub formatted_normalized: Option<Text>,
    // Eure-mark errors (for .eumd file validation)
    #[eure(default)]
    pub euremark_errors: Vec<Text>,
    // Editor scenario fields (for completions/diagnostics testing)
    #[eure(default)]
    pub editor: Option<Text>,
    #[eure(default)]
    pub completions: Vec<CompletionItem>,
    #[eure(default)]
    pub trigger: Option<String>,
    #[eure(default)]
    pub diagnostics: Vec<DiagnosticItem>,
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
            && self.euremark_errors.is_empty()
            && self.editor.is_none()
    }
}

#[derive(Debug, Clone, ParseDocument, BuildSchema)]
pub enum UnimplementedReason {
    Boolean(bool),
    Text(Text),
}

impl UnimplementedReason {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            UnimplementedReason::Boolean(true) => Some(""),
            UnimplementedReason::Boolean(false) => None,
            UnimplementedReason::Text(text) => Some(&text.content),
        }
    }
}

/// A file containing one or more test cases
#[derive(Debug, Clone, ParseDocument, BuildSchema)]
pub struct CaseFile {
    #[eure(flatten)]
    pub default_case: CaseData,
    #[eure(default)]
    pub cases: BTreeMap<String, CaseData>,
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

        let named_iter = self.cases.iter().map(|(k, v)| (k.as_str(), v));

        default_iter.into_iter().chain(named_iter)
    }

    /// Returns the number of cases in this file
    pub fn case_count(&self) -> usize {
        let default_count = if self.default_case.is_empty() { 0 } else { 1 };
        default_count + self.cases.len()
    }
}

pub struct ParseResult {
    pub path: PathBuf,
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

    let case_file: CaseFile =
        doc.parse(doc.get_root_id())
            .map_err(|e| ParseError::DocumentParseError {
                error: e,
                cst: cst.clone(),
            })?;

    Ok(ParseResult {
        path,
        case_file,
        cst,
        input: input.to_string(),
    })
}
