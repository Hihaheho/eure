//! Configuration and parsing queries.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eure_env::EureConfig;
use query_flow::{Db, QueryError, query};

use crate::document::cst_to_document_and_origin_map;
use crate::report::report_config_error;

use super::assets::{TextFile, WorkspaceId};
use super::parse::{ParseCst, ParseDocument};

/// Resolved configuration with its directory.
#[derive(Clone, PartialEq)]
pub struct ResolvedConfig {
    pub config: Arc<EureConfig>,
    pub config_dir: PathBuf,
    pub workspace_path: PathBuf,
}

/// Parse EureConfig from a config file.
#[query]
pub fn parse_config(db: &impl Db, config_file: TextFile) -> Result<EureConfig, QueryError> {
    let parsed = db.query(ParseDocument::new(config_file.clone()))?;
    let root_id = parsed.doc.get_root_id();

    match parsed.doc.parse::<EureConfig>(root_id) {
        Ok(config) => Ok(config),
        Err(e) => {
            let cst = db.query(ParseCst::new(config_file.clone()))?;
            Err(report_config_error(
                &eure_env::ConfigError::from(e),
                config_file,
                &cst.cst,
                &parsed.origins,
            ))?
        }
    }
}

/// Resolve the EureConfig that applies to a file.
///
/// Iterates through all registered workspaces and returns the config
/// for the workspace that contains the file.
///
/// Returns `None` if the file is not in any workspace.
#[query]
pub fn resolve_config(db: &impl Db, file: TextFile) -> Result<Option<ResolvedConfig>, QueryError> {
    let Some(file_path) = file.as_local_path() else {
        return Ok(None);
    };

    for workspace_id in db.list_asset_keys::<WorkspaceId>() {
        let workspace = db.asset(workspace_id)?;

        if file_path.starts_with(&workspace.path) {
            let config_file = TextFile::from_path(workspace.config_path.clone());
            let config = db.query(ParseConfig::new(config_file))?;

            return Ok(Some(ResolvedConfig {
                config,
                config_dir: workspace.path.clone(),
                workspace_path: workspace.path.clone(),
            }));
        }
    }

    Ok(None)
}

#[query]
pub fn workspace_config(
    db: &impl Db,
    workspace_id: WorkspaceId,
) -> Result<ResolvedConfig, QueryError> {
    let workspace = db.asset(workspace_id)?;
    let config_file = TextFile::from_path(workspace.config_path.clone());
    let config = db.query(ParseConfig::new(config_file))?;
    Ok(ResolvedConfig {
        config,
        config_dir: workspace.path.clone(),
        workspace_path: workspace.path.clone(),
    })
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
