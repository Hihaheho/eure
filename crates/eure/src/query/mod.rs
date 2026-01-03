//! Query-flow queries for Eure language processing.
//!
//! This module is the heart of Eure's language processing system.
//! All consumers (eure-ls, eure-cli, test-suite) use these queries
//! as the single source of truth.

pub mod assets;
pub mod config;
pub mod diagnostics;
pub mod error;
pub mod parse;
pub mod schema;
pub mod semantic_token;
pub mod validation;

pub use assets::{TextFile, TextFileContent, Workspace, WorkspaceId};
pub use config::{GetConfig, LoadConfigError, load_config};
pub use diagnostics::{DiagnosticMessage, DiagnosticSeverity, GetDiagnostics};
pub use parse::{ParseCst, ParseDocument, ParsedCst, ParsedDocument, ValidCst, read_text_file};
pub use schema::{
    DocumentToSchemaQuery, GetSchemaConversionErrorFormatted, GetSchemaExtension,
    GetSchemaExtensionDiagnostics, GetValidationErrorsFormatted,
    GetValidationErrorsFormattedWithMode, ResolveSchema, UnionTagMode, ValidateAgainstSchema,
    ValidateAgainstSchemaWithMode, ValidatedSchema,
};
pub use semantic_token::{
    GetSemanticTokens, SemanticToken, SemanticTokenModifier, SemanticTokenType, semantic_tokens,
};
pub use validation::{
    TargetValidationResult, TargetsValidationResult, ValidateDocument, ValidateTarget,
    ValidateTargetResult, ValidateTargets, ValidateTargetsResult,
};
