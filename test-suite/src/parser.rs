use std::path::PathBuf;

use eure::{
    document::{DocumentConstructionError, EureDocument},
    parol::EureParseError,
    tree::Cst,
    value::{IdentifierError, ObjectKey, Text},
};

use crate::case::Case;

pub struct ParseResult {
    pub case: Case,
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
    IdentifierError {
        error: IdentifierError,
        cst: Cst,
    },
}

#[allow(clippy::result_large_err)]
pub fn parse_case(input: &str, path: PathBuf) -> Result<ParseResult, ParseError> {
    let cst = eure::parol::parse(input).map_err(ParseError::ParolError)?;
    let doc = eure::document::cst_to_document(input, &cst).map_err(|e| {
        ParseError::DocumentConstructionError {
            error: e,
            cst: cst.clone(),
        }
    })?;

    let case = Case {
        path: path.clone(),
        input_eure: get_text(&doc, "input_eure").map_err(|e| ParseError::IdentifierError {
            error: e,
            cst: cst.clone(),
        })?,
        normalized: get_text(&doc, "normalized").map_err(|e| ParseError::IdentifierError {
            error: e,
            cst: cst.clone(),
        })?,
        output_json: get_text(&doc, "output_json").map_err(|e| ParseError::IdentifierError {
            error: e,
            cst: cst.clone(),
        })?,
        schema: get_text(&doc, "schema").map_err(|e| ParseError::IdentifierError {
            error: e,
            cst: cst.clone(),
        })?,
        schema_errors: get_text_array(&doc, "schema_errors").map_err(|e| {
            ParseError::IdentifierError {
                error: e,
                cst: cst.clone(),
            }
        })?,
        output_json_schema: get_text(&doc, "output_json_schema").map_err(|e| {
            ParseError::IdentifierError {
                error: e,
                cst: cst.clone(),
            }
        })?,
        json_schema_errors: get_text_array(&doc, "json_schema_errors").map_err(|e| {
            ParseError::IdentifierError {
                error: e,
                cst: cst.clone(),
            }
        })?,
        unimplemented: get_unimplemented(&doc, "unimplemented").map_err(|e| {
            ParseError::IdentifierError {
                error: e,
                cst: cst.clone(),
            }
        })?,
    };

    Ok(ParseResult {
        case,
        cst,
        input: input.to_string(),
    })
}

fn get_text(doc: &EureDocument, key: &str) -> Result<Option<Text>, IdentifierError> {
    Ok(doc
        .root()
        .as_map()
        .unwrap()
        .get(&ObjectKey::String(key.into()))
        .map(move |node| {
            doc.node(node)
                .as_primitive()
                .expect("Expected a primitive value")
                .as_text()
                .expect("Expected a text value")
                .clone()
        }))
}

fn get_text_array(doc: &EureDocument, key: &str) -> Result<Vec<Text>, IdentifierError> {
    Ok(doc
        .root()
        .as_map()
        .unwrap()
        .get(&ObjectKey::String(key.into()))
        .map(move |node| {
            doc.node(node)
                .as_array()
                .expect("Expected an array value")
                .iter()
                .map(|item| {
                    doc.node(*item)
                        .as_primitive()
                        .expect("Expected a primitive value")
                        .as_text()
                        .expect("Expected a text value")
                        .clone()
                })
                .collect()
        })
        .unwrap_or_default())
}

fn get_unimplemented(doc: &EureDocument, key: &str) -> Result<Option<String>, IdentifierError> {
    use eure::value::PrimitiveValue;

    // Try to get the field from document
    let value = doc
        .root()
        .as_map()
        .unwrap()
        .get(&ObjectKey::String(key.into()));

    match value {
        None => Ok(None), // Field not present
        Some(node) => {
            let primitive = doc
                .node(node)
                .as_primitive()
                .expect("Expected primitive value");

            match primitive {
                // Check if it's true - return empty string (no reason)
                PrimitiveValue::Bool(true) => Ok(Some(String::new())),
                // Check if it's false - treat as not unimplemented
                PrimitiveValue::Bool(false) => Ok(None),
                // Check if it's a string
                PrimitiveValue::Text(text) => Ok(Some(text.content.clone())),
                // Any other type is invalid
                _ => panic!("unimplemented field must be boolean or string"),
            }
        }
    }
}
