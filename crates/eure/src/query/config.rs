//! Configuration and parsing queries.

use std::path::Path;

use eure_config::EureConfig;
use query_flow::{Db, QueryError, query};

use crate::document::cst_to_document_and_origin_map;
use crate::report::report_config_error;

use super::assets::{TextFile, WorkspaceId};
use super::parse::{ParseCst, ParseDocument};

/// Step 3: Parse EureConfig from document.
#[query]
pub fn get_config(db: &impl Db, file: TextFile) -> Result<Option<EureConfig>, QueryError> {
    let workspace_ids = db.list_asset_keys::<WorkspaceId>();
    if let Some(workspace_id) = workspace_ids.into_iter().next() {
        let workspace = db.asset(workspace_id)?.suspend()?;
        if detect_workspace(&workspace.path, &file.path) {
            let config_file = TextFile::from_path(workspace.config_path.clone());
            // UserError propagates automatically via ?
            let parsed = db.query(ParseDocument::new(config_file.clone()))?;

            let root_id = parsed.doc.get_root_id();
            match parsed.doc.parse::<EureConfig>(root_id) {
                Ok(config) => return Ok(Some(config)),
                Err(e) => {
                    let cst = db.query(ParseCst::new(config_file.clone()))?;
                    Err(report_config_error(
                        &eure_config::ConfigError::from(e),
                        file,
                        &cst.cst,
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
