//! Configuration and parsing queries.

use std::path::Path;
use std::sync::Arc;

use eure_config::EureConfig;
use eure_parol::{EureParseError, ParseResult, parse_tolerant};
use eure_tree::prelude::Cst;
use query_flow::query;

use crate::document::{EureDocument, OriginMap, cst_to_document_and_origin_map};
use crate::report::{ErrorReports, FileId, report_config_error, report_document_error_simple};

use super::assets::{TextFile, TextFileContent, WorkspaceId};

/// Result of tolerant parsing - always returns CST, optionally with error.
#[derive(Clone, PartialEq)]
pub struct ParsedCst {
    pub cst: Arc<Cst>,
    pub source: Arc<String>,
    pub error: Option<EureParseError>,
}

/// Step 1: Parse text content to CST (tolerant).
///
/// Always succeeds and returns a (possibly partial) CST.
/// Parse errors are included in the result for downstream processing.
#[query]
pub fn parse_cst(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<ParsedCst>, query_flow::QueryError> {
    let content: Arc<TextFileContent> = db.asset(file.clone())?.suspend()?;
    match content.as_ref() {
        TextFileContent::NotFound => Ok(None),
        TextFileContent::Content(text) => {
            let source = Arc::new(text.clone());
            let parsed = match parse_tolerant(text) {
                ParseResult::Ok(cst) => ParsedCst {
                    cst: Arc::new(cst),
                    source,
                    error: None,
                },
                ParseResult::ErrWithCst { cst, error } => ParsedCst {
                    cst: Arc::new(cst),
                    source,
                    error: Some(error),
                },
            };
            Ok(Some(parsed))
        }
    }
}

/// Parsed document with CST and OriginMap for error reporting.
#[derive(Clone, PartialEq)]
pub struct ParsedDocument {
    pub doc: Arc<EureDocument>,
    pub cst: Arc<Cst>,
    pub origins: Arc<OriginMap>,
    pub source: Arc<String>,
}

/// Step 2: Build EureDocument from CST.
///
/// Returns `None` if file not found or if there was a parse error.
/// Returns `UserError` if document construction fails on a valid CST.
#[query]
pub fn parse_document(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<ParsedDocument>, query_flow::QueryError> {
    // Get CST from previous step
    let parsed_cst = match &*db.query(ParseCst::new(file.clone()))? {
        None => return Ok(None),
        Some(parsed) => parsed.clone(),
    };

    // Only build document if no parse error
    if parsed_cst.error.is_some() {
        return Ok(None);
    }

    // Build document
    match cst_to_document_and_origin_map(&parsed_cst.source, &parsed_cst.cst) {
        Ok((doc, origins)) => Ok(Some(ParsedDocument {
            doc: Arc::new(doc),
            cst: parsed_cst.cst,
            origins: Arc::new(origins),
            source: parsed_cst.source,
        })),
        Err(e) => {
            // TODO: Use proper FileId from a registry
            let file_id = FileId(0);
            Err(ErrorReports::from(vec![report_document_error_simple(
                &e,
                file_id,
                &parsed_cst.cst,
            )]))?
        }
    }
}

/// Step 3: Parse EureConfig from document.
#[query]
pub fn get_config(
    db: &impl Db,
    file: TextFile,
) -> Result<Option<EureConfig>, query_flow::QueryError> {
    let workspace_ids = db.list_asset_keys::<WorkspaceId>();
    if let Some(workspace_id) = workspace_ids.into_iter().next() {
        let workspace = db.asset(workspace_id)?.suspend()?;
        if detect_workspace(&workspace.path, &file.path) {
            let config_file = TextFile::from_path(workspace.config_path.clone());
            // UserError propagates automatically via ?
            let parsed = match &*db.query(ParseDocument::new(config_file))? {
                None => return Ok(None),
                Some(parsed) => parsed.clone(),
            };

            let root_id = parsed.doc.get_root_id();
            match parsed.doc.parse::<EureConfig>(root_id) {
                Ok(config) => return Ok(Some(config)),
                Err(e) => {
                    // TODO: Use proper FileId from a registry
                    let file_id = FileId(0);
                    return Err(report_config_error(
                        &eure_config::ConfigError::from(e),
                        file_id,
                        &parsed.cst,
                        &parsed.origins,
                    ))?;
                }
            }
        }
    }
    Ok(None)
}

fn detect_workspace(workspace_path: &Path, file_path: &Path) -> bool {
    file_path.starts_with(workspace_path)
}

// ============================================================================
// Synchronous API for CLI usage
// ============================================================================

/// Error type for synchronous config loading.
#[derive(Debug, thiserror::Error)]
pub enum LoadConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Document error: {0}")]
    Document(String),

    #[error("Config error: {0}")]
    Config(String),
}

/// Load EureConfig from a file path synchronously.
///
/// This is a convenience function for CLI tools that don't need
/// the query-flow incremental computation infrastructure.
pub fn load_config(path: &Path) -> Result<EureConfig, LoadConfigError> {
    // Read file
    let source = std::fs::read_to_string(path)?;

    // Parse CST (tolerant mode to get error messages)
    let parse_result = eure_parol::parse_tolerant(&source);
    if let Some(error) = parse_result.error() {
        return Err(LoadConfigError::Parse(error.to_string()));
    }

    // Build document
    let cst = parse_result.cst();
    let (doc, _origins) = cst_to_document_and_origin_map(&source, &cst)
        .map_err(|e| LoadConfigError::Document(e.to_string()))?;

    // Parse config
    let root_id = doc.get_root_id();
    doc.parse::<EureConfig>(root_id)
        .map_err(|e| LoadConfigError::Config(e.to_string()))
}
