use std::path::PathBuf;

use eure::{
    Map,
    document::{
        DocumentConstructionError, NodeValue,
        parse::{
            ParseContext, ParseDocument, ParseError as DocumentParseError, ParseErrorKind,
            RecordParser,
        },
    },
    parol::EureParseError,
    tree::Cst,
    value::{PrimitiveValue, Text, ValueKind},
};

/// Fields that are recognized but not yet implemented in the test runner.
/// These are consumed during parsing to avoid "unknown field" errors.
const IGNORED_FIELDS: &[&str] = &[
    // Editor-specific fields
    "editor",
    "expect_no_diagnostics",
    "document_uri",
    "completions",
    "completions_should_not_contain",
    "trigger",
    "reference_tests",
    "expected_path",
    "expected_schema_path",
    "expected_schema_ref",
    "valid_expect_no_schema_errors",
    "expect_non_empty_completions",
    "expect_empty_completions",
    "diagnostics",
    "diagnostics_should_not_contain_code",
    "document_path",
    "error",
    "invalid_editor",
    "valid_editor",
    "schema_uri",
    "cached_document",
    "cursor_offset",
    // Legacy field names
    "expected_json",
    "expected_json_schema",
];

/// A single test case's data fields
#[derive(Debug, Clone, Default)]
pub struct CaseData {
    pub input_eure: Option<Text>,
    pub input_json: Option<Text>,
    pub normalized: Option<Text>,
    pub output_json: Option<Text>,
    pub schema: Option<Text>,
    pub schema_errors: Vec<Text>,
    pub output_json_schema: Option<Text>,
    pub json_schema_errors: Vec<Text>,
    pub unimplemented: Option<String>,
}

impl CaseData {
    /// Check if this case has any meaningful content
    pub fn is_empty(&self) -> bool {
        self.input_eure.is_none()
            && self.input_json.is_none()
            && self.normalized.is_none()
            && self.output_json.is_none()
            && self.schema.is_none()
            && self.schema_errors.is_empty()
            && self.output_json_schema.is_none()
            && self.json_schema_errors.is_empty()
    }
}

impl ParseDocument<'_> for CaseData {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, DocumentParseError> {
        let mut rec = ctx.parse_record()?;

        let input_eure = rec.field_optional::<Text>("input_eure")?;
        let input_json = rec.field_optional::<Text>("input_json")?;
        let normalized = rec.field_optional::<Text>("normalized")?;
        let output_json = rec.field_optional::<Text>("output_json")?;
        let schema = rec.field_optional::<Text>("schema")?;
        let schema_errors = rec
            .field_optional::<Vec<Text>>("schema_errors")?
            .unwrap_or_default();
        let output_json_schema = rec.field_optional::<Text>("output_json_schema")?;
        let json_schema_errors = rec
            .field_optional::<Vec<Text>>("json_schema_errors")?
            .unwrap_or_default();
        let unimplemented = parse_unimplemented_field(&mut rec)?;

        // Editor-specific fields (consume but don't parse - not yet implemented)
        for field in IGNORED_FIELDS {
            rec.field_ctx_optional(field);
        }

        rec.deny_unknown_fields()?;

        Ok(CaseData {
            input_eure,
            input_json,
            normalized,
            output_json,
            schema,
            schema_errors,
            output_json_schema,
            json_schema_errors,
            unimplemented,
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
    let field_ctx = match rec.field_ctx_optional("unimplemented") {
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
    pub named_cases: Map<String, CaseData>,
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
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, DocumentParseError> {
        let mut rec = ctx.parse_record()?;

        // Parse root-level fields as default case
        let input_eure = rec.field_optional::<Text>("input_eure")?;
        let input_json = rec.field_optional::<Text>("input_json")?;
        let normalized = rec.field_optional::<Text>("normalized")?;
        let output_json = rec.field_optional::<Text>("output_json")?;
        let schema = rec.field_optional::<Text>("schema")?;
        let schema_errors = rec
            .field_optional::<Vec<Text>>("schema_errors")?
            .unwrap_or_default();
        let output_json_schema = rec.field_optional::<Text>("output_json_schema")?;
        let json_schema_errors = rec
            .field_optional::<Vec<Text>>("json_schema_errors")?
            .unwrap_or_default();
        let unimplemented = parse_unimplemented_field(&mut rec)?;

        // Editor-specific fields (consume but don't parse - not yet implemented)
        for field in IGNORED_FIELDS {
            rec.field_ctx_optional(field);
        }

        // Parse named cases from "cases" section
        // Note: Some legacy test files use "cases[]" (array) instead of "cases.<name>" (map).
        // We only support the map format for named cases; arrays are ignored.
        let named_cases = match rec.field_ctx_optional("cases") {
            Some(cases_ctx) => {
                // Try to parse as Map<String, CaseData>. If it fails (e.g., it's an array),
                // just return an empty map and let the tests handle unknown fields.
                cases_ctx
                    .parse::<Map<String, CaseData>>()
                    .unwrap_or_default()
            }
            None => Map::default(),
        };

        rec.deny_unknown_fields()?;

        Ok(CaseFile {
            path: PathBuf::new(), // Set by caller
            default_case: CaseData {
                input_eure,
                input_json,
                normalized,
                output_json,
                schema,
                schema_errors,
                output_json_schema,
                json_schema_errors,
                unimplemented,
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
