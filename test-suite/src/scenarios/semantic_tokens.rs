use eure::query::TextFile;
use eure::query::semantic_token::{GetSemanticTokens, SemanticTokenModifier, SemanticTokenType};
use query_flow::Db;

use crate::parser::SemanticTokenItem;
use crate::scenarios::{Scenario, ScenarioError};

/// Semantic tokens test scenario
#[derive(Debug, Clone)]
pub struct SemanticTokensScenario {
    /// Editor content file
    pub editor: TextFile,
    /// Expected semantic tokens in source order
    pub semantic_tokens: Vec<SemanticTokenItem>,
}

/// Map SemanticTokenType to LSP legend name (must match capabilities.rs)
fn token_type_name(tt: SemanticTokenType) -> &'static str {
    match tt {
        SemanticTokenType::Keyword => "keyword",
        SemanticTokenType::Number => "number",
        SemanticTokenType::String => "string",
        SemanticTokenType::Comment => "comment",
        SemanticTokenType::Operator => "operator",
        SemanticTokenType::Property => "property",
        SemanticTokenType::Punctuation => "punctuation",
        SemanticTokenType::Macro => "macro",
        SemanticTokenType::Decorator => "decorator",
        SemanticTokenType::SectionMarker => "sectionMarker",
        SemanticTokenType::ExtensionMarker => "extensionMarker",
        SemanticTokenType::ExtensionIdent => "extensionIdent",
    }
}

/// Decode modifier bitmask into sorted list of modifier names
fn modifier_names(modifiers: u32) -> Vec<&'static str> {
    let mut names = Vec::new();
    for modifier in SemanticTokenModifier::all() {
        if modifiers & modifier.bitmask() != 0 {
            names.push(match modifier {
                SemanticTokenModifier::Declaration => "declaration",
                SemanticTokenModifier::Definition => "definition",
                SemanticTokenModifier::SectionHeader => "sectionHeader",
            });
        }
    }
    names
}

/// Format a token for comparison: ("span", "type") or ("span", "type", ["mod1", "mod2"])
fn format_token(span: &str, token_type: &str, modifiers: &[impl AsRef<str>]) -> String {
    if modifiers.is_empty() {
        format!("(\"{}\", \"{}\")", span, token_type)
    } else {
        let mods: Vec<&str> = modifiers.iter().map(|m| m.as_ref()).collect();
        format!(
            "(\"{}\", \"{}\", [{}])",
            span,
            token_type,
            mods.iter()
                .map(|m| format!("\"{}\"", m))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl Scenario for SemanticTokensScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual_tokens = db.query(GetSemanticTokens::new(self.editor.clone()))?;
        let source: std::sync::Arc<eure::query::TextFileContent> = db.asset(self.editor.clone())?;

        // Format actual tokens
        let actual_strs: Vec<String> = actual_tokens
            .iter()
            .map(|t| {
                let span = &source.get()[t.start as usize..(t.start + t.length) as usize];
                let tt = token_type_name(t.token_type);
                let mods = modifier_names(t.modifiers);
                format_token(span, tt, &mods)
            })
            .collect();

        // Format expected tokens
        let expected_strs: Vec<String> = self
            .semantic_tokens
            .iter()
            .map(|item| format_token(item.span(), item.token_type(), item.modifiers()))
            .collect();

        if expected_strs != actual_strs {
            return Err(ScenarioError::SemanticTokensMismatch {
                expected: expected_strs,
                actual: actual_strs,
            });
        }

        Ok(())
    }
}
