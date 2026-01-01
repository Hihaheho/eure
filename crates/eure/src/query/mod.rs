//! Query-flow queries for Eure language processing.
//!
//! This module is the heart of Eure's language processing system.
//! All consumers (eure-ls, eure-cli, test-suite) use these queries
//! as the single source of truth.

pub mod assets;
pub mod config;
pub mod diagnostics;
pub mod schema;
pub mod semantic_token;

pub use assets::{TextFile, TextFileContent, Workspace, WorkspaceId};
pub use config::{
    LoadConfigError, ParseCst, ParseDocument, ParsedCst, ParsedDocument, load_config,
};
pub use diagnostics::{DiagnosticMessage, DiagnosticSeverity, GetDiagnostics};
pub use schema::{
    DocumentToSchemaQuery, ErrorSpan, GetSchemaExtension, GetSchemaExtensionDiagnostics,
    ResolveSchema, ValidateAgainstMetaSchema, ValidateAgainstSchema, ValidatedSchema,
};
pub use semantic_token::{
    GetSemanticTokens, SemanticToken, SemanticTokenModifier, SemanticTokenType, semantic_tokens,
};
