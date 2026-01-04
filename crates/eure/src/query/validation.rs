//! Validation queries - Single Source of Truth for document validation.
//!
//! These queries are used by all consumers:
//! - eure-cli (check command)
//! - eure-dev (web editor)
//! - eure-ls (language server)

use std::path::PathBuf;
use std::sync::Arc;

use eure_env::Target;
use query_flow::{Db, QueryError, QueryResultExt, query};
use thisisplural::Plural;

use crate::report::ErrorReports;

use super::assets::{Glob, TextFile};
use super::parse::ParseDocument;
use super::schema::ValidateAgainstSchema;

// =============================================================================
// Document Validation
// =============================================================================

/// Validate a document, optionally against a specific schema.
///
/// If schema_file is Some, validates against that schema.
/// If schema_file is None, tries to resolve schema via ResolveSchema.
/// If no schema can be determined, returns SyntaxOnly (syntax check passed).
///
/// This is the SSoT for document validation - used by CLI, web editor, and LSP.
#[query]
pub fn validate_document(
    db: &impl Db,
    doc_file: TextFile,
    schema_file: Option<TextFile>,
) -> Result<ErrorReports, QueryError> {
    // 3. If no schema, syntax check passed
    if let Some(sf) = schema_file {
        db.query(ValidateAgainstSchema::new(doc_file, sf))
            .map(|reports| reports.as_ref().clone())
    } else if let Err(reports) = db
        .query(ParseDocument::new(doc_file.clone()))
        .downcast_err::<ErrorReports>()?
    {
        Ok(reports.get().clone())
    } else {
        Ok(ErrorReports::new())
    }
}

// =============================================================================
// Target Validation
// =============================================================================

/// Result of validating a single target.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct TargetValidationResult {
    /// Number of files checked.
    pub files_checked: usize,
    /// Files with errors: (file, errors).
    pub file_errors: Vec<(TextFile, ErrorReports)>,
}

impl TargetValidationResult {
    /// Returns true if all files passed validation.
    pub fn is_ok(&self) -> bool {
        self.file_errors.is_empty()
    }

    /// Returns the number of files with errors.
    pub fn error_count(&self) -> usize {
        self.file_errors.len()
    }
}

#[derive(Clone, PartialEq, Debug, Default, Plural)]
pub struct ValidateTargetResult(Vec<(TextFile, ErrorReports)>);

/// Validate all files matching a target's globs against its schema.
///
/// Expands glob patterns, resolves schema path, and validates each file.
#[query]
pub fn validate_target(
    db: &impl Db,
    target: Target,
    config_dir: PathBuf,
) -> Result<ValidateTargetResult, QueryError> {
    // Resolve schema file if specified
    let schema_file = target
        .schema
        .as_ref()
        .map(|schema_path| TextFile::resolve(schema_path, &config_dir));

    // Expand glob patterns via asset (platform-specific implementation)
    let files: Vec<TextFile> = target
        .globs
        .iter()
        .map(|glob_pattern| {
            let glob_key = Glob::new(config_dir.clone(), glob_pattern.clone());
            db.asset(glob_key)?.suspend()
        })
        // Register all Glob as pending assets before suspending
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<Result<Vec<_>, QueryError>>()?
        .into_iter()
        .flat_map(|result| result.0.clone())
        .collect();

    // Validate each file
    files
        .into_iter()
        .map(|file| {
            db.query(ValidateDocument::new(file.clone(), schema_file.clone()))
                .map(|reports| (file, reports.as_ref().clone()))
        })
        .collect()
}

// =============================================================================
// Multiple Targets Validation
// =============================================================================

/// Result of validating multiple targets.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct TargetsValidationResult {
    /// Results per target: (name, result).
    pub target_results: Vec<(String, TargetValidationResult)>,
}

impl TargetsValidationResult {
    /// Returns true if all targets passed validation.
    pub fn is_ok(&self) -> bool {
        self.target_results.iter().all(|(_, r)| r.is_ok())
    }

    /// Returns total number of files checked across all targets.
    pub fn total_files_checked(&self) -> usize {
        self.target_results
            .iter()
            .map(|(_, r)| r.files_checked)
            .sum()
    }

    /// Returns total number of files with errors across all targets.
    pub fn total_error_count(&self) -> usize {
        self.target_results
            .iter()
            .map(|(_, r)| r.error_count())
            .sum()
    }
}

#[derive(Clone, PartialEq, Debug, Default, Plural)]
pub struct ValidateTargetsResult(Vec<(String, ValidateTargetResult)>);

/// Validate multiple targets.
///
/// Validates each target and aggregates results.
#[query]
pub fn validate_targets(
    db: &impl Db,
    targets: Arc<Vec<(String, Target)>>,
    config_dir: PathBuf,
) -> Result<ValidateTargetsResult, QueryError> {
    targets
        .iter()
        .map(|(name, target)| {
            db.query(ValidateTarget::new(target.clone(), config_dir.clone()))
                .map(|result| (name.clone(), result.as_ref().clone()))
        })
        .collect()
}
