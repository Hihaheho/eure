//! Query-flow queries for eure-toml.

use eure::query::{TextFile, read_text_file};
use eure_document::document::EureDocument;
use query_flow::{Db, QueryError, query};

use crate::{format_source_document, to_source_document};

/// Convert a TOML file to an Eure document.
///
/// This query reads the TOML file, parses it, and converts it to an EureDocument.
#[query]
pub fn toml_to_eure_document(
    db: &impl Db,
    toml_file: TextFile,
) -> Result<EureDocument, QueryError> {
    let content = read_text_file(db, toml_file)?;
    let source_doc = to_source_document(&content)?;
    Ok(source_doc.document)
}

/// Convert a TOML file to formatted Eure source code.
///
/// This query reads the TOML file, parses it, converts to SourceDocument,
/// and formats it as Eure source code.
#[query]
pub fn toml_to_eure_source(db: &impl Db, toml_file: TextFile) -> Result<String, QueryError> {
    let content = read_text_file(db, toml_file)?;
    let source_doc = to_source_document(&content)?;
    Ok(format_source_document(&source_doc))
}
