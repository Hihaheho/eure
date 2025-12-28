//! Error aggregation and formatting utilities.

use eure::error::format_parse_error_plain;
use eure_parol::EureParseError;
use query_flow::query;

use crate::assets::TextFile;
use crate::config::ParseCst;
use crate::schema::{
    DocumentToSchemaQuery, ErrorSpan, ValidateAgainstMetaSchema, ValidateAgainstSchema,
};

/// Aggregated errors from all sources.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct AllErrors {
    pub doc_parser_errors: Vec<ErrorSpan>,
    pub schema_parser_errors: Vec<ErrorSpan>,
    pub schema_conversion_errors: Vec<ErrorSpan>,
    pub schema_validation_errors: Vec<ErrorSpan>,
    pub validation_errors: Vec<ErrorSpan>,
}

impl AllErrors {
    pub fn total_count(&self) -> usize {
        self.doc_parser_errors.len()
            + self.schema_parser_errors.len()
            + self.schema_conversion_errors.len()
            + self.schema_validation_errors.len()
            + self.validation_errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }
}

/// Get all errors from document and schema parsing/validation.
#[query]
pub fn get_all_errors(
    ctx: &mut query_flow::QueryContext,
    doc_file: TextFile,
    schema_file: TextFile,
    meta_schema_file: TextFile,
) -> Result<AllErrors, query_flow::QueryError> {
    // Get parser errors
    let doc_cst_result = ctx.query(ParseCst::new(doc_file.clone()))?;
    let schema_cst_result = ctx.query(ParseCst::new(schema_file.clone()))?;

    let doc_parser_errors = match &*doc_cst_result {
        Some(parsed) if parsed.error.is_some() => format_parser_errors(
            parsed.error.as_ref().unwrap(),
            &parsed.source,
            "document.eure",
        ),
        _ => vec![],
    };

    let schema_parser_errors = match &*schema_cst_result {
        Some(parsed) if parsed.error.is_some() => format_parser_errors(
            parsed.error.as_ref().unwrap(),
            &parsed.source,
            "schema.eure",
        ),
        _ => vec![],
    };

    // Get schema conversion errors (via UserError)
    let schema_conversion_errors = match ctx.query(DocumentToSchemaQuery::new(schema_file.clone()))
    {
        Ok(_) => vec![],
        Err(e) => vec![ErrorSpan {
            start: 0,
            end: 1,
            message: e.to_string(),
        }],
    };

    // Get schema validation against meta-schema
    let schema_validation_errors =
        if schema_parser_errors.is_empty() && schema_conversion_errors.is_empty() {
            (*ctx.query(ValidateAgainstMetaSchema::new(
                schema_file.clone(),
                meta_schema_file.clone(),
            ))?)
            .clone()
        } else {
            vec![]
        };

    // Get document validation against schema
    let validation_errors = if doc_parser_errors.is_empty()
        && schema_conversion_errors.is_empty()
        && schema_validation_errors.is_empty()
    {
        (*ctx.query(ValidateAgainstSchema::new(
            doc_file.clone(),
            schema_file.clone(),
        ))?)
        .clone()
    } else {
        vec![]
    };

    Ok(AllErrors {
        doc_parser_errors,
        schema_parser_errors,
        schema_conversion_errors,
        schema_validation_errors,
        validation_errors,
    })
}

/// Format parser errors to ErrorSpan list.
fn format_parser_errors(error: &EureParseError, source: &str, filename: &str) -> Vec<ErrorSpan> {
    let message = format_parse_error_plain(error, source, filename);
    error
        .entries
        .iter()
        .filter_map(|entry| {
            entry.span.map(|s| ErrorSpan {
                start: s.start as usize,
                end: s.end as usize,
                message: message.clone(),
            })
        })
        .collect()
}
