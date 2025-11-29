use std::path::PathBuf;

use eure::{
    document::{DocumentConstructionError, EureDocument},
    parol::parol_runtime::ParolError,
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
    ParolError(ParolError),
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
