//! Server capabilities definition.

use lsp_types::{
    CompletionOptions, HoverProviderCapability, SemanticTokenModifier as LspModifier,
    SemanticTokenType as LspTokenType, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions, SemanticTokensServerCapabilities, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};

/// Characters that start a new completable position in Eure syntax:
/// section marker, key separator, extension marker, and the two binding
/// operators.
const COMPLETION_TRIGGER_CHARACTERS: [&str; 5] = ["@", ".", "$", "=", ":"];

/// Build the server capabilities to advertise to the client.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(
                COMPLETION_TRIGGER_CHARACTERS
                    .iter()
                    .map(|c| c.to_string())
                    .collect(),
            ),
            ..Default::default()
        }),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
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
