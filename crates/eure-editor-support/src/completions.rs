use eure_tree::Cst;
use lsp_types::{CompletionItem, Position};

use crate::completion_analyzer::CompletionAnalyzer;
use crate::schema_validation::SchemaManager;

pub fn get_completions(
    text: &str,
    cst: &Cst,
    position: Position,
    trigger_character: Option<String>,
    uri: &str,
    schema_manager: &SchemaManager,
) -> Vec<CompletionItem> {
    // First try error-based completion
    let parse_result = eure_parol::parse_tolerant(text);
    let analyzer = CompletionAnalyzer::new(
        text.to_string(),
        parse_result,
        position,
        schema_manager,
        uri,
    );

    let error_completions = analyzer.analyze();
    if !error_completions.is_empty() {
        return error_completions;
    }

    // Get the schema for this document
    let schema_uri = schema_manager.get_document_schema_uri(uri);
    let _schema = match schema_uri.and_then(|uri| schema_manager.get_schema(uri)) {
        Some(s) => s,
        None => return vec![], // No schema, no completions
    };

    // TODO: Re-implement context tracking with EureDocument
    // For now, return empty completions for non-error cases
    vec![]
}