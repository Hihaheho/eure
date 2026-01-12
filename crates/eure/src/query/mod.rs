//! Query-flow queries for Eure language processing.
//!
//! This module is the heart of Eure's language processing system.
//! All consumers (eure-ls, eure-cli, test-suite) use these queries
//! as the single source of truth.

pub mod asset_locator;
pub mod assets;
pub mod config;
pub mod diagnostics;
pub mod error;
#[cfg(feature = "http")]
pub mod http;
pub mod parse;
pub mod schema;
pub mod semantic_token;
pub mod validation;

pub use asset_locator::TextFileLocator;
pub use assets::{
    DecorStyle, DecorStyleKey, Glob, GlobResult, OpenDocuments, OpenDocumentsList, TextFile,
    TextFileContent, Workspace, WorkspaceId,
};

use query_flow::{QueryRuntime, QueryRuntimeBuilder};

use crate::report::error_reports_comparator;

/// Build a query runtime with the standard Eure configuration.
///
/// This includes:
/// - TextFileLocator for URL host validation against security.allowed-hosts
/// - error_reports_comparator for proper ErrorReports comparison
///
/// All consumers should use this function to ensure consistent behavior.
pub fn build_runtime() -> QueryRuntime {
    let runtime = QueryRuntimeBuilder::new()
        .error_comparator(error_reports_comparator)
        .build();
    runtime.register_asset_locator(TextFileLocator);
    runtime
}
pub use config::{LoadConfigError, ParseConfig, ResolveConfig, ResolvedConfig, load_config};
pub use diagnostics::{
    CollectDiagnosticTargets, CollectSchemaFiles, DiagnosticMessage, DiagnosticSeverity,
    GetAllDiagnostics, GetDiagnostics, GetFileDiagnostics, GetParseDiagnostics,
    GetSchemaConversionDiagnostics, GetValidationDiagnostics,
};
#[cfg(feature = "http")]
pub use http::fetch_url;
#[cfg(feature = "native")]
pub use http::{CacheOptions, base_cache_dir, fetch_url_cached, https_cache_dir, parse_duration};
pub use parse::{ParseCst, ParseDocument, ParsedCst, ParsedDocument, ValidCst};
#[cfg(feature = "http")]
pub use reqwest;
pub use schema::{
    DocumentToSchemaQuery, GetSchemaConversionErrorFormatted, GetSchemaExtension,
    GetSchemaExtensionDiagnostics, GetValidationErrorsFormatted,
    GetValidationErrorsFormattedExplicit, GetValidationErrorsFormattedExplicitWithMode,
    GetValidationErrorsFormattedWithMode, ResolveSchema, UnionTagMode,
    ValidateAgainstExplicitSchema, ValidateAgainstExplicitSchemaWithMode, ValidateAgainstSchema,
    ValidateAgainstSchemaWithMode, ValidatedSchema,
};
pub use semantic_token::{
    GetSemanticTokens, SemanticToken, SemanticTokenModifier, SemanticTokenType, semantic_tokens,
};
pub use validation::{
    TargetValidationResult, TargetsValidationResult, ValidateDocument, ValidateTarget,
    ValidateTargetResult, ValidateTargets, ValidateTargetsResult,
};
