use eure_tree::{
    Cst,
    node_kind::TerminalKind,
    tree::{CstNodeData, CstNodeId, InputSpan, LineNumbers, TerminalData},
};
use lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokens, SemanticTokensLegend};

/// Define the token types we'll use for EURE syntax highlighting
const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,   // For keywords like true, false, null
    SemanticTokenType::STRING,    // For string literals
    SemanticTokenType::NUMBER,    // For numeric literals
    SemanticTokenType::PROPERTY,  // For property names/keys
    SemanticTokenType::NAMESPACE, // For extension namespaces (with @ prefix)
    SemanticTokenType::OPERATOR,  // For operators
    SemanticTokenType::COMMENT,   // For comments
    SemanticTokenType::VARIABLE,  // For variables/identifiers
];

/// Define the token modifiers we'll use
const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,   // For declarations
    SemanticTokenModifier::DOCUMENTATION, // For documentation comments
];

/// Get the semantic tokens legend for the LSP server
pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

/// Token data structure for collecting tokens before encoding
#[derive(Debug, Clone)]
struct TokenData {
    line: u32,
    start_char: u32,
    length: u32,
    token_type: u32,
    token_modifiers: u32,
}

/// Build semantic tokens for a EURE CST
///
/// This is the main entry point for semantic token generation
pub fn semantic_tokens(
    text: &str,
    cst: &Cst,
    legend: &SemanticTokensLegend,
) -> Option<SemanticTokens> {
    // Create a vector to hold our token data before encoding
    let mut token_data: Vec<TokenData> = Vec::new();

    // Pre-compute line numbers for the entire text
    let line_numbers = LineNumbers::new(text);
    // Traverse the CST to collect semantic tokens
    collect_tokens(
        cst,
        cst.root(),
        text,
        &line_numbers,
        &mut token_data,
        legend,
    );

    // Sort tokens by line and character
    token_data.sort_by(|a, b| {
        if a.line == b.line {
            a.start_char.cmp(&b.start_char)
        } else {
            a.line.cmp(&b.line)
        }
    });

    // Don't return empty tokens
    if token_data.is_empty() {
        return None;
    }

    // Convert token data to LSP format
    Some(SemanticTokens {
        result_id: None,
        data: encode_tokens(&token_data),
    })
}

/// Collect semantic tokens by traversing the CST
fn collect_tokens(
    cst: &Cst,
    node_id: CstNodeId,
    text: &str,
    line_numbers: &LineNumbers,
    tokens: &mut Vec<TokenData>,
    legend: &SemanticTokensLegend,
) {
    // Get node data
    if let Some(CstNodeData::Terminal {
        kind,
        data: TerminalData::Input(span),
    }) = cst.node_data(node_id)
    {
        // Process terminal nodes
        process_terminal(kind, span, text, line_numbers, tokens, legend);
    }

    // Process all children
    for child_id in cst.children(node_id) {
        collect_tokens(cst, child_id, text, line_numbers, tokens, legend);
    }
}

/// Process a terminal node to extract token information
fn process_terminal(
    kind: TerminalKind,
    span: InputSpan,
    text: &str,
    line_numbers: &LineNumbers,
    tokens: &mut Vec<TokenData>,
    legend: &SemanticTokensLegend,
) {
    let Some((line, start_char, length)) = span_to_line_char(span, line_numbers, text) else {
        return;
    };
    // Map terminal kind to semantic token type
    let token_type = match kind {
        TerminalKind::True | TerminalKind::False | TerminalKind::Null => SemanticTokenType::KEYWORD,
        TerminalKind::Integer => SemanticTokenType::NUMBER,
        TerminalKind::Str | TerminalKind::Text => SemanticTokenType::STRING,
        TerminalKind::NamedCode => {
            let code_pos = text[span.start as usize..span.end as usize]
                .find('`')
                .expect("This token should contain `") as u32;
            push_multiple_tokens_oneline(
                legend,
                tokens,
                line,
                start_char,
                length,
                [
                    (code_pos, SemanticTokenType::NAMESPACE),
                    (1, SemanticTokenType::OPERATOR),
                    (length - code_pos - 2, SemanticTokenType::STRING),
                    (1, SemanticTokenType::OPERATOR),
                ],
            );
            return;
        }
        TerminalKind::Code => SemanticTokenType::STRING,
        TerminalKind::LineComment | TerminalKind::BlockComment => SemanticTokenType::COMMENT,
        TerminalKind::At => SemanticTokenType::NAMESPACE,
        TerminalKind::Dot
        | TerminalKind::LBrace
        | TerminalKind::RBrace
        | TerminalKind::LBracket
        | TerminalKind::RBracket
        | TerminalKind::Bind
        | TerminalKind::Hole
        | TerminalKind::Comma
        | TerminalKind::TextStart
        | TerminalKind::CodeBlock => {
            push_multiple_tokens_oneline(
                legend,
                tokens,
                line,
                start_char,
                length,
                [
                    (3, SemanticTokenType::OPERATOR),
                    (length - 3, SemanticTokenType::NAMESPACE),
                ],
            );
            return;
        }
        TerminalKind::Ident => SemanticTokenType::PROPERTY,
        TerminalKind::Dollar => SemanticTokenType::OPERATOR,
        TerminalKind::NewLine
        | TerminalKind::Whitespace
        | TerminalKind::Newline
        | TerminalKind::Ws
        | TerminalKind::Esc => return,
    };
    let token_type_idx = get_token_type_index(token_type, legend);

    // Skip tokens with zero length
    if span.start == span.end {
        return;
    }

    // Calculate line and character for the span
    tokens.push(TokenData {
        line,
        start_char,
        length,
        token_type: token_type_idx,
        token_modifiers: 0, // No modifiers for now
    });
}

fn push_multiple_tokens_oneline(
    legend: &SemanticTokensLegend,
    tokens: &mut Vec<TokenData>,
    line: u32,
    start_char: u32,
    length: u32,
    iter: impl IntoIterator<Item = (u32, SemanticTokenType)>,
) {
    let mut start_char = start_char;
    let mut rest_length = length;
    for (length, token_type) in iter {
        tokens.push(TokenData {
            line,
            start_char,
            length,
            token_type: get_token_type_index(token_type, legend),
            token_modifiers: 0,
        });
        start_char += length;
        rest_length -= length;
    }
    assert_eq!(rest_length, 0);
}

/// Convert a span to line and character position
fn span_to_line_char(
    span: InputSpan,
    line_numbers: &LineNumbers,
    text: &str,
) -> Option<(u32, u32, u32)> {
    // Get character info for start and end positions
    let start_info = line_numbers.get_char_info(span.start);
    let end_info = line_numbers.get_char_info(span.end);

    // Start character is already calculated in CharInfo
    let start_character = start_info.column_number;

    // Calculate length (in characters)
    let length = if start_info.line_number == end_info.line_number {
        // Simple case: span is on a single line
        span.end - span.start
    } else {
        // Multi-line case: just count to end of first line
        let first_line_end = text[span.start as usize..]
            .find('\n')
            .map(|pos| span.start + pos as u32)
            .unwrap_or(span.end);

        first_line_end - span.start
    };

    Some((start_info.line_number, start_character, length))
}

/// Get the index of a token type in the legend
fn get_token_type_index(token_type: SemanticTokenType, legend: &SemanticTokensLegend) -> u32 {
    legend
        .token_types
        .iter()
        .position(|t| t == &token_type)
        .unwrap_or(0) as u32
}

/// Encode tokens in the LSP delta format
fn encode_tokens(tokens: &[TokenData]) -> Vec<lsp_types::SemanticToken> {
    if tokens.is_empty() {
        return vec![];
    }

    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0;
    let mut prev_start = 0;

    for token in tokens {
        // Delta encoding for line
        let delta_line = if result.is_empty() {
            token.line
        } else if token.line == prev_line {
            0
        } else {
            token.line - prev_line
        };

        // Delta encoding for character
        let delta_start = if delta_line == 0 {
            token.start_char - prev_start
        } else {
            token.start_char
        };

        // Create a SemanticToken with the 5 values for this token
        result.push(lsp_types::SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: token.token_modifiers,
        });

        prev_line = token.line;
        prev_start = token.start_char;
    }

    result
}
