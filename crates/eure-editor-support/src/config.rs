use std::path::Path;
use std::sync::Arc;

use eure::document::{EureDocument, OriginMap, cst_to_document_and_origin_map};
use eure::parol;
use eure::report::{ErrorReports, FileId, report_document_error_simple, report_parse_error};
use eure_config::{EureConfig, report_config_error};
use eure_tree::prelude::Cst;
use query_flow::query;

use crate::assets::{TextFile, TextFileContent, WorkspaceId};

/// Step 1: Parse text content to CST.
#[query]
pub fn parse_cst(
    ctx: &mut query_flow::QueryContext,
    file: TextFile,
) -> Result<Option<Arc<Cst>>, query_flow::QueryError> {
    let content: Arc<TextFileContent> = ctx.asset(file)?.suspend()?;
    match content.as_ref() {
        TextFileContent::NotFound => Ok(None),
        TextFileContent::Content(text) => match parol::parse(text) {
            Ok(cst) => Ok(Some(Arc::new(cst))),
            Err(e) => {
                // TODO: Use proper FileId from a registry
                let file_id = FileId(0);
                Err(report_parse_error(&e, file_id))?
            }
        },
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
#[query]
pub fn parse_document(
    ctx: &mut query_flow::QueryContext,
    file: TextFile,
) -> Result<Option<ParsedDocument>, query_flow::QueryError> {
    // Get CST from previous step (UserError propagates automatically via ?)
    let cst = match &*ctx.query(ParseCst::new(file.clone()))? {
        None => return Ok(None),
        Some(cst) => cst.clone(),
    };

    // Get source text for document construction
    let content: Arc<TextFileContent> = ctx.asset(file)?.suspend()?;
    let source = match content.as_ref() {
        TextFileContent::NotFound => return Ok(None),
        TextFileContent::Content(text) => Arc::new(text.clone()),
    };

    // Build document
    match cst_to_document_and_origin_map(&source, &cst) {
        Ok((doc, origins)) => Ok(Some(ParsedDocument {
            doc: Arc::new(doc),
            cst,
            origins: Arc::new(origins),
            source,
        })),
        Err(e) => {
            // TODO: Use proper FileId from a registry
            let file_id = FileId(0);
            Err(ErrorReports::from(vec![report_document_error_simple(
                &e, file_id, &cst,
            )]))?
        }
    }
}

/// Step 3: Parse EureConfig from document.
#[query]
pub fn get_config(
    ctx: &mut query_flow::QueryContext,
    file: TextFile,
) -> Result<Option<EureConfig>, query_flow::QueryError> {
    let workspace_ids = ctx.list_asset_keys::<WorkspaceId>();
    if let Some(workspace_id) = workspace_ids.into_iter().next() {
        let workspace = ctx.asset(&workspace_id)?.suspend()?;
        if detect_workspace(&workspace.path, &file.path) {
            let config_file = TextFile::from_path(workspace.config_path.clone());
            // UserError propagates automatically via ?
            let parsed = match &*ctx.query(ParseDocument::new(config_file))? {
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
