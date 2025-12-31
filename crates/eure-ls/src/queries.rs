//! LSP-specific queries that convert to LSP types.

use eure_editor_support::assets::TextFile;
use eure_editor_support::diagnostics::{DiagnosticMessage, DiagnosticSeverity, GetDiagnostics};
use eure_editor_support::semantic_token::{GetSemanticTokens, SemanticToken};
use lsp_types::{
    Diagnostic, DiagnosticSeverity as LspSeverity, Position, Range,
    SemanticToken as LspSemanticToken, SemanticTokens,
};
use query_flow::query;

/// LSP-formatted semantic tokens query.
///
/// Wraps `GetSemanticTokens` and converts to LSP `SemanticTokens` format.
#[query]
pub fn lsp_semantic_tokens(
    ctx: &mut query_flow::QueryContext,
    file: TextFile,
    source: String,
) -> Result<Option<SemanticTokens>, query_flow::QueryError> {
    let tokens = ctx.query(GetSemanticTokens::new(file.clone()))?;
    match tokens.as_ref() {
        Some(tokens) => Ok(Some(convert_tokens(tokens, source))),
        None => Ok(None),
    }
}

/// LSP-formatted diagnostics query.
///
/// Wraps `GetDiagnostics` and converts to LSP `Diagnostic` format.
#[query]
pub fn lsp_diagnostics(
    ctx: &mut query_flow::QueryContext,
    file: TextFile,
    source: String,
) -> Result<Vec<Diagnostic>, query_flow::QueryError> {
    let diagnostics = ctx.query(GetDiagnostics::new(file.clone()))?;
    let line_offsets = compute_line_offsets(source);
    Ok(diagnostics
        .iter()
        .map(|d| convert_diagnostic(d, source, &line_offsets))
        .collect())
}

/// Convert internal semantic tokens to LSP format.
///
/// LSP semantic tokens use a delta encoding:
/// - Each token is encoded as (deltaLine, deltaStartChar, length, tokenType, tokenModifiers)
/// - deltaLine is relative to the previous token's line
/// - deltaStartChar is relative to the previous token's start (or line start if on new line)
/// - All character positions and lengths are in UTF-16 code units
fn convert_tokens(tokens: &[SemanticToken], source: &str) -> SemanticTokens {
    let line_offsets = compute_line_offsets(source);

    let mut data = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for token in tokens {
        let start = token.start as usize;
        let end = start + token.length as usize;
        let (line, char) = offset_to_position(start, source, &line_offsets);
        let length = byte_len_to_utf16_len(source, start, end);

        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            char - prev_start
        } else {
            char
        };

        data.push(LspSemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: token.token_type as u32,
            token_modifiers_bitset: token.modifiers,
        });

        prev_line = line;
        prev_start = char;
    }

    SemanticTokens {
        result_id: None,
        data,
    }
}

/// Convert internal diagnostic to LSP format.
fn convert_diagnostic(msg: &DiagnosticMessage, source: &str, line_offsets: &[usize]) -> Diagnostic {
    let start = offset_to_lsp_position(msg.start, source, line_offsets);
    let end = offset_to_lsp_position(msg.end, source, line_offsets);

    Diagnostic {
        range: Range { start, end },
        severity: Some(convert_severity(msg.severity)),
        code: None,
        code_description: None,
        source: Some("eure".to_string()),
        message: msg.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Convert internal severity to LSP severity.
fn convert_severity(severity: DiagnosticSeverity) -> LspSeverity {
    match severity {
        DiagnosticSeverity::Error => LspSeverity::ERROR,
        DiagnosticSeverity::Warning => LspSeverity::WARNING,
        DiagnosticSeverity::Info => LspSeverity::INFORMATION,
        DiagnosticSeverity::Hint => LspSeverity::HINT,
    }
}

/// Compute line offsets for a source string.
///
/// Returns a vector where `line_offsets[i]` is the byte offset of line `i`.
fn compute_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, c) in source.char_indices() {
        if c == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Convert a byte offset to (line, character) position.
///
/// Line is 0-indexed. Character is in UTF-16 code units (as required by LSP).
fn offset_to_position(offset: usize, source: &str, line_offsets: &[usize]) -> (u32, u32) {
    let line = line_offsets.iter().rposition(|&o| o <= offset).unwrap_or(0);
    let line_start = line_offsets[line];
    // Count UTF-16 code units from line start to offset
    let end = offset.min(source.len());
    let line_content = &source[line_start..end];
    let utf16_offset: usize = line_content.chars().map(|c| c.len_utf16()).sum();
    (line as u32, utf16_offset as u32)
}

/// Convert a byte offset to LSP Position with UTF-16 character position.
fn offset_to_lsp_position(offset: usize, source: &str, line_offsets: &[usize]) -> Position {
    let (line, character) = offset_to_position(offset, source, line_offsets);
    Position { line, character }
}

/// Convert a byte length to UTF-16 code unit length.
fn byte_len_to_utf16_len(source: &str, start: usize, end: usize) -> u32 {
    let end = end.min(source.len());
    let start = start.min(end);
    source[start..end]
        .chars()
        .map(|c| c.len_utf16())
        .sum::<usize>() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_line_offsets() {
        let source = "hello\nworld\n";
        let offsets = compute_line_offsets(source);
        assert_eq!(offsets, vec![0, 6, 12]);
    }

    #[test]
    fn test_offset_to_position_ascii() {
        let source = "hello\nworld\n";
        let offsets = compute_line_offsets(source);
        assert_eq!(offset_to_position(0, source, &offsets), (0, 0));
        assert_eq!(offset_to_position(5, source, &offsets), (0, 5));
        assert_eq!(offset_to_position(6, source, &offsets), (1, 0));
        assert_eq!(offset_to_position(11, source, &offsets), (1, 5));
    }

    #[test]
    fn test_offset_to_position_utf8() {
        // "日本語" is 9 bytes (3 chars × 3 bytes each), but 3 UTF-16 code units
        let source = "日本語\ntest";
        let offsets = compute_line_offsets(source);
        // Byte offset 0 -> (line 0, char 0)
        assert_eq!(offset_to_position(0, source, &offsets), (0, 0));
        // Byte offset 3 (after 日) -> (line 0, char 1)
        assert_eq!(offset_to_position(3, source, &offsets), (0, 1));
        // Byte offset 6 (after 日本) -> (line 0, char 2)
        assert_eq!(offset_to_position(6, source, &offsets), (0, 2));
        // Byte offset 9 (after 日本語) -> (line 0, char 3)
        assert_eq!(offset_to_position(9, source, &offsets), (0, 3));
        // Byte offset 10 (after \n) -> (line 1, char 0)
        assert_eq!(offset_to_position(10, source, &offsets), (1, 0));
    }

    #[test]
    fn test_offset_to_position_emoji() {
        // "😀" is 4 bytes in UTF-8, but 2 UTF-16 code units (surrogate pair)
        let source = "😀a";
        let offsets = compute_line_offsets(source);
        // Byte offset 0 -> (line 0, char 0)
        assert_eq!(offset_to_position(0, source, &offsets), (0, 0));
        // Byte offset 4 (after 😀) -> (line 0, char 2) because emoji is 2 UTF-16 units
        assert_eq!(offset_to_position(4, source, &offsets), (0, 2));
        // Byte offset 5 (after 😀a) -> (line 0, char 3)
        assert_eq!(offset_to_position(5, source, &offsets), (0, 3));
    }

    #[test]
    fn test_byte_len_to_utf16_len() {
        // ASCII: 1 byte = 1 UTF-16 unit
        assert_eq!(byte_len_to_utf16_len("hello", 0, 5), 5);
        // Japanese: 3 bytes per char, 1 UTF-16 unit per char
        assert_eq!(byte_len_to_utf16_len("日本語", 0, 9), 3);
        // Emoji: 4 bytes, 2 UTF-16 units
        assert_eq!(byte_len_to_utf16_len("😀", 0, 4), 2);
    }
}
