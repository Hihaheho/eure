//! Server capabilities definition.

use lsp_types::{
    SemanticTokenModifier as LspModifier, SemanticTokenType as LspTokenType,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};

/// Build the server capabilities to advertise to the client.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                work_done_progress_options: Default::default(),
                legend: semantic_token_legend(),
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
        )),
        ..Default::default()
    }
}

/// Build the semantic token legend.
///
/// The legend defines the mapping from token type/modifier indices to names.
/// This must match the order defined in `SemanticTokenType` and `SemanticTokenModifier`.
fn semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            LspTokenType::KEYWORD,                // Keyword = 0
            LspTokenType::NUMBER,                 // Number = 1
            LspTokenType::STRING,                 // String = 2
            LspTokenType::COMMENT,                // Comment = 3
            LspTokenType::OPERATOR,               // Operator = 4
            LspTokenType::PROPERTY,               // Property = 5
            LspTokenType::new("punctuation"),     // Punctuation = 6
            LspTokenType::MACRO,                  // Macro = 7
            LspTokenType::DECORATOR,              // Decorator = 8
            LspTokenType::new("sectionMarker"),   // SectionMarker = 9
            LspTokenType::new("extensionMarker"), // ExtensionMarker = 10
            LspTokenType::new("extensionIdent"),  // ExtensionIdent = 11
        ],
        token_modifiers: vec![
            LspModifier::DECLARATION,          // Declaration = 0
            LspModifier::DEFINITION,           // Definition = 1
            LspModifier::new("sectionHeader"), // SectionHeader = 2
        ],
    }
}
