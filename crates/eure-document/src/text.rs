//! Text type unifying strings and code in Eure.
//!
//! This module provides the [`Text`] type which represents all text values in Eure,
//! whether they originated from string syntax (`"..."`) or code syntax (`` `...` ``).

use alloc::{borrow::Cow, string::String, vec::Vec};
use core::iter::Peekable;
use thiserror::Error;

/// Language tag for text values.
///
/// # Variants
///
/// - [`Plaintext`](Language::Plaintext): Explicitly plain text, from `"..."` string syntax.
///   Use when the content is data/text, not code.
///
/// - [`Implicit`](Language::Implicit): No language specified, from `` `...` `` or
///   ```` ``` ```` without a language tag. The language can be inferred from schema context.
///
/// - [`Other`](Language::Other): Explicit language tag, from `` rust`...` `` or
///   ```` ```rust ```` syntax. Use when the language must be specified.
///
/// # Schema Validation
///
/// | Schema | `Plaintext` | `Implicit` | `Other("rust")` |
/// |--------|-------------|------------|-----------------|
/// | `.text` (any) | ✓ | ✓ | ✓ |
/// | `.text.plaintext` | ✓ | ✓ (coerce) | ✗ |
/// | `.text.rust` | ✗ | ✓ (coerce) | ✓ |
///
/// `Implicit` allows users to write `` `let a = 1;` `` when the schema
/// already specifies `.text.rust`, without redundantly repeating the language.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Language {
    /// Explicitly plain text (from `"..."` syntax).
    ///
    /// This variant is rejected by schemas expecting a specific language like `.text.rust`.
    /// Use this when the content is data/text, not code.
    #[default]
    Plaintext,
    /// No language specified (from `` `...` `` without language tag).
    ///
    /// Can be coerced to match the schema's expected language. This allows users
    /// to write `` `let a = 1;` `` when the schema already specifies `.text.rust`.
    Implicit,
    /// Explicit language tag (from `` lang`...` `` syntax).
    ///
    /// The string contains the language identifier (e.g., "rust", "sql", "email").
    Other(Cow<'static, str>),
}

impl Language {
    /// Create a Language from a string.
    ///
    /// - Empty string or "plaintext" → [`Plaintext`](Language::Plaintext)
    /// - Other strings → [`Other`](Language::Other)
    ///
    /// Note: This does NOT produce [`Implicit`](Language::Implicit). Use `Language::Implicit`
    /// directly when parsing code syntax without a language tag.
    pub fn new(s: impl Into<Cow<'static, str>>) -> Self {
        let s = s.into();
        if s == "plaintext" || s.is_empty() {
            Language::Plaintext
        } else {
            Language::Other(s)
        }
    }

    /// Returns the language as a string slice, or `None` for [`Implicit`](Language::Implicit).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Language::Plaintext => Some("plaintext"),
            Language::Implicit => None,
            Language::Other(s) => Some(s.as_ref()),
        }
    }

    /// Returns true if this is the [`Plaintext`](Language::Plaintext) variant.
    pub fn is_plaintext(&self) -> bool {
        matches!(self, Language::Plaintext)
    }

    /// Returns true if this is the [`Implicit`](Language::Implicit) variant.
    pub fn is_implicit(&self) -> bool {
        matches!(self, Language::Implicit)
    }

    /// Returns true if this language can be coerced to the expected language.
    ///
    /// # Coercion Rules
    ///
    /// - `Implicit` can be coerced to any language (it's "infer from schema")
    /// - Any language matches an `Implicit` expectation (schema says "any")
    /// - Otherwise, languages must match exactly
    pub fn is_compatible_with(&self, expected: &Language) -> bool {
        match (self, expected) {
            (_, Language::Implicit) => true, // Any matches implicit expectation
            (Language::Implicit, _) => true, // Implicit can be coerced to anything
            (a, b) => a == b,                // Otherwise must match exactly
        }
    }

    pub fn is_other(&self, arg: &str) -> bool {
        match self {
            Language::Other(s) => s == arg,
            _ => false,
        }
    }
}

/// Hint for serialization: which syntax was used to parse this text.
///
/// This hint allows round-tripping to preserve the original syntax when possible.
/// The generic variants (`Inline`, `Block`) let the serializer pick the best syntax
/// when the exact form doesn't matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxHint {
    /// String syntax: `"..."`
    Quoted,
    /// Generic inline code (serializer picks between Inline1/Inline2)
    Inline,
    /// Single backtick inline: `` `...` ``
    Inline1,
    /// Double backtick inline: ``` ``...`` ```
    Inline2,
    /// Generic block code (serializer picks backtick count)
    Block,
    /// Triple backtick block: ```` ```...``` ````
    Block3,
    /// Quadruple backtick block: ````` ````...```` `````
    Block4,
    /// Quintuple backtick block
    Block5,
    /// Sextuple backtick block
    Block6,
}

impl SyntaxHint {
    /// Returns true if this is a quoted string syntax.
    pub fn is_quoted(&self) -> bool {
        matches!(self, SyntaxHint::Quoted)
    }

    /// Returns true if this is any inline code syntax.
    pub fn is_inline(&self) -> bool {
        matches!(
            self,
            SyntaxHint::Inline | SyntaxHint::Inline1 | SyntaxHint::Inline2
        )
    }

    /// Returns true if this is any block code syntax.
    pub fn is_block(&self) -> bool {
        matches!(
            self,
            SyntaxHint::Block
                | SyntaxHint::Block3
                | SyntaxHint::Block4
                | SyntaxHint::Block5
                | SyntaxHint::Block6
        )
    }
}

/// A text value in Eure, unifying strings and code.
///
/// # Overview
///
/// `Text` represents all text values in Eure, regardless of whether they were
/// written using string syntax (`"..."`) or code syntax (`` `...` ``). This
/// unification simplifies the data model while preserving the semantic distinction
/// through the [`language`](Text::language) field.
///
/// # Syntax Mapping
///
/// | Syntax | Language | SyntaxHint |
/// |--------|----------|------------|
/// | `"hello"` | `Plaintext` | `Quoted` |
/// | `` `hello` `` | `Implicit` | `Inline1` |
/// | ``` ``hello`` ``` | `Implicit` | `Inline2` |
/// | `` sql`SELECT` `` | `Other("sql")` | `Inline1` |
/// | ```` ``` ```` (no lang) | `Implicit` | `Block3` |
/// | ```` ```rust ```` | `Other("rust")` | `Block3` |
///
/// # Key Distinction
///
/// - `"..."` → `Plaintext` (explicit: "this is text, not code")
/// - `` `...` `` without lang → `Implicit` (code, language inferred from schema)
/// - `` lang`...` `` → `Other(lang)` (code with explicit language)
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// The text content.
    pub content: String,
    /// The language tag for this text.
    pub language: Language,
    /// Hint for serialization about the original syntax.
    pub syntax_hint: Option<SyntaxHint>,
}

impl Text {
    /// Create a new text value.
    pub fn new(content: impl Into<String>, language: Language) -> Self {
        Self {
            content: content.into(),
            language,
            syntax_hint: None,
        }
    }

    /// Create a new text value with a syntax hint.
    ///
    /// For block syntax hints, automatically ensures trailing newline.
    pub fn with_syntax_hint(
        content: impl Into<String>,
        language: Language,
        syntax_hint: SyntaxHint,
    ) -> Self {
        let mut content = content.into();
        if syntax_hint.is_block() && !content.ends_with('\n') {
            content.push('\n');
        }
        Self {
            content,
            language,
            syntax_hint: Some(syntax_hint),
        }
    }

    /// Create a plaintext value (from `"..."` syntax).
    pub fn plaintext(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            language: Language::Plaintext,
            syntax_hint: Some(SyntaxHint::Quoted),
        }
    }

    /// Create an inline code value with implicit language (from `` `...` `` syntax).
    pub fn inline_implicit(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            language: Language::Implicit,
            syntax_hint: Some(SyntaxHint::Inline1),
        }
    }

    /// Create an inline code value with explicit language (from `` lang`...` `` syntax).
    pub fn inline(content: impl Into<String>, language: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: content.into(),
            language: Language::new(language),
            syntax_hint: Some(SyntaxHint::Inline1),
        }
    }

    /// Create a block code value with implicit language (from ```` ``` ```` syntax without lang).
    pub fn block_implicit(content: impl Into<String>) -> Self {
        let mut content = content.into();
        if !content.ends_with('\n') {
            content.push('\n');
        }
        Self {
            content,
            language: Language::Implicit,
            syntax_hint: Some(SyntaxHint::Block3),
        }
    }

    /// Create a block code value with explicit language.
    pub fn block(content: impl Into<String>, language: impl Into<Cow<'static, str>>) -> Self {
        let mut content = content.into();
        if !content.ends_with('\n') {
            content.push('\n');
        }
        Self {
            content,
            language: Language::new(language),
            syntax_hint: Some(SyntaxHint::Block3),
        }
    }

    /// Create a block code value without adding a trailing newline. This must be used only when performing convertion to eure from another data format.
    pub fn block_without_trailing_newline(
        content: impl Into<String>,
        language: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            content: content.into(),
            language: Language::new(language),
            syntax_hint: Some(SyntaxHint::Block3),
        }
    }

    /// Returns the content as a string slice.
    pub fn as_str(&self) -> &str {
        &self.content
    }
}

/// Errors that can occur when parsing text.
#[derive(Debug, PartialEq, Eq, Clone, Error)]
pub enum TextParseError {
    /// Invalid escape sequence encountered.
    #[error("Invalid escape sequence: {0}")]
    InvalidEscapeSequence(char),
    /// Unexpected end of string after escape character.
    #[error("Invalid end of string after escape")]
    InvalidEndOfStringAfterEscape,
    /// Invalid Unicode code point in escape sequence.
    #[error("Invalid unicode code point: {0}")]
    InvalidUnicodeCodePoint(u32),
    /// Newline found in text binding (only single line allowed).
    #[error("Newline in text binding")]
    NewlineInTextBinding,
    /// Invalid indent in code block.
    #[error(
        "Invalid indent on code block at line {line}: actual {actual_indent} to be indented more than {expected_indent}"
    )]
    IndentError {
        line: usize,
        actual_indent: usize,
        expected_indent: usize,
    },
}

impl Text {
    /// Parse a quoted string like `"hello world"` into a Text value.
    ///
    /// Handles escape sequences: `\\`, `\"`, `\'`, `\n`, `\r`, `\t`, `\0`, `\u{...}`.
    pub fn parse_quoted_string(s: &str) -> Result<Self, TextParseError> {
        let content = parse_escape_sequences(s)?;
        Ok(Text::plaintext(content))
    }

    /// Parse a text binding content (after the colon) like `: hello world\n`.
    ///
    /// Strips trailing newline and trims whitespace.
    pub fn parse_text_binding(s: &str) -> Result<Self, TextParseError> {
        let stripped = s.strip_suffix('\n').unwrap_or(s);
        let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
        if stripped.contains(['\r', '\n']) {
            return Err(TextParseError::NewlineInTextBinding);
        }
        let content = parse_escape_sequences(stripped.trim())?;
        Ok(Text::plaintext(content))
    }

    /// Parse an indented code block, removing base indentation.
    ///
    /// The base indentation is auto-detected from trailing whitespace in the content.
    /// If the content ends with `\n` followed by spaces, those spaces represent
    /// the closing delimiter's indentation and determine how much to strip.
    pub fn parse_indented_block(
        language: Language,
        content: String,
        syntax_hint: SyntaxHint,
    ) -> Result<Self, TextParseError> {
        // Detect base_indent from trailing whitespace after last newline
        let base_indent = if let Some(last_newline_pos) = content.rfind('\n') {
            let trailing = &content[last_newline_pos + 1..];
            if trailing.chars().all(|c| c == ' ') {
                trailing.len()
            } else {
                0
            }
        } else {
            0
        };

        // Collect lines, excluding the trailing whitespace line (delimiter indent)
        let lines: Vec<&str> = content.lines().collect();
        let line_count = if base_indent > 0 && !content.ends_with('\n') && lines.len() > 1 {
            lines.len() - 1
        } else {
            lines.len()
        };

        let expected_whitespace_removals = base_indent * line_count;
        let mut result = String::with_capacity(content.len() - expected_whitespace_removals);

        for (line_number, line) in lines.iter().take(line_count).enumerate() {
            // Empty lines (including whitespace-only lines) are allowed and don't need to match the indent
            if line.trim_start().is_empty() {
                result.push('\n');
                continue;
            }

            let actual_indent = line
                .chars()
                .take_while(|c| *c == ' ')
                .take(base_indent)
                .count();
            if actual_indent < base_indent {
                return Err(TextParseError::IndentError {
                    line: line_number + 1,
                    actual_indent,
                    expected_indent: base_indent,
                });
            }
            // Remove the base indent from the line
            result.push_str(&line[base_indent..]);
            result.push('\n');
        }

        Ok(Self {
            content: result,
            language,
            syntax_hint: Some(syntax_hint),
        })
    }
}

/// Parse escape sequences in a string.
fn parse_escape_sequences(s: &str) -> Result<String, TextParseError> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    fn parse_unicode_escape(
        chars: &mut Peekable<impl Iterator<Item = char>>,
    ) -> Result<char, TextParseError> {
        match chars.next() {
            Some('{') => {}
            Some(ch) => return Err(TextParseError::InvalidEscapeSequence(ch)),
            None => return Err(TextParseError::InvalidEndOfStringAfterEscape),
        }

        let mut count = 0;
        let mut code_point = 0;
        while let Some(ch) = chars.peek()
            && count < 6
        // max 6 hex digits
        {
            if let Some(digit) = match ch {
                '0'..='9' => Some(*ch as u32 - '0' as u32),
                'a'..='f' => Some(*ch as u32 - 'a' as u32 + 10),
                'A'..='F' => Some(*ch as u32 - 'A' as u32 + 10),
                '_' | '-' => None,
                _ => break,
            } {
                code_point = code_point * 16 + digit;
                count += 1;
            }
            chars.next();
        }

        let Some(result) = core::char::from_u32(code_point) else {
            return Err(TextParseError::InvalidUnicodeCodePoint(code_point));
        };

        match chars.next() {
            Some('}') => {}
            Some(ch) => return Err(TextParseError::InvalidEscapeSequence(ch)),
            None => return Err(TextParseError::InvalidEndOfStringAfterEscape),
        }

        Ok(result)
    }

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('0') => result.push('\0'),
                Some('u') => result.push(parse_unicode_escape(&mut chars)?),
                Some(ch) => return Err(TextParseError::InvalidEscapeSequence(ch)),
                None => return Err(TextParseError::InvalidEndOfStringAfterEscape),
            },
            _ => result.push(ch),
        }
    }

    Ok(result)
}

// Re-export for backwards compatibility during transition
pub use TextParseError as EureStringError;

/// Backwards-compatible type alias for EureString.
///
/// **Deprecated**: Use [`Text`] instead.
pub type EureString = Cow<'static, str>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_new_plaintext() {
        assert_eq!(Language::new("plaintext"), Language::Plaintext);
        assert_eq!(Language::new(""), Language::Plaintext);
    }

    #[test]
    fn test_language_new_other() {
        assert_eq!(Language::new("rust"), Language::Other("rust".into()));
        assert_eq!(Language::new("sql"), Language::Other("sql".into()));
    }

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Plaintext.as_str(), Some("plaintext"));
        assert_eq!(Language::Implicit.as_str(), None);
        assert_eq!(Language::Other("rust".into()).as_str(), Some("rust"));
    }

    #[test]
    fn test_language_compatibility() {
        // Implicit is compatible with everything
        assert!(Language::Implicit.is_compatible_with(&Language::Plaintext));
        assert!(Language::Implicit.is_compatible_with(&Language::Other("rust".into())));

        // Everything is compatible with Implicit expectation
        assert!(Language::Plaintext.is_compatible_with(&Language::Implicit));
        assert!(Language::Other("rust".into()).is_compatible_with(&Language::Implicit));

        // Same languages are compatible
        assert!(Language::Plaintext.is_compatible_with(&Language::Plaintext));
        assert!(Language::Other("rust".into()).is_compatible_with(&Language::Other("rust".into())));

        // Different explicit languages are not compatible
        assert!(!Language::Plaintext.is_compatible_with(&Language::Other("rust".into())));
        assert!(!Language::Other("rust".into()).is_compatible_with(&Language::Plaintext));
        assert!(!Language::Other("rust".into()).is_compatible_with(&Language::Other("sql".into())));
    }

    #[test]
    fn test_text_plaintext() {
        let text = Text::plaintext("hello");
        assert_eq!(text.content, "hello");
        assert_eq!(text.language, Language::Plaintext);
        assert_eq!(text.syntax_hint, Some(SyntaxHint::Quoted));
    }

    #[test]
    fn test_text_inline_implicit() {
        let text = Text::inline_implicit("let a = 1");
        assert_eq!(text.content, "let a = 1");
        assert_eq!(text.language, Language::Implicit);
        assert_eq!(text.syntax_hint, Some(SyntaxHint::Inline1));
    }

    #[test]
    fn test_text_inline_with_language() {
        let text = Text::inline("SELECT *", "sql");
        assert_eq!(text.content, "SELECT *");
        assert_eq!(text.language, Language::Other("sql".into()));
        assert_eq!(text.syntax_hint, Some(SyntaxHint::Inline1));
    }

    #[test]
    fn test_text_block_implicit() {
        let text = Text::block_implicit("fn main() {}");
        assert_eq!(text.content, "fn main() {}\n");
        assert_eq!(text.language, Language::Implicit);
        assert_eq!(text.syntax_hint, Some(SyntaxHint::Block3));
    }

    #[test]
    fn test_text_block_with_language() {
        let text = Text::block("fn main() {}", "rust");
        assert_eq!(text.content, "fn main() {}\n");
        assert_eq!(text.language, Language::Other("rust".into()));
        assert_eq!(text.syntax_hint, Some(SyntaxHint::Block3));
    }

    #[test]
    fn test_parse_quoted_string() {
        let text = Text::parse_quoted_string("hello\\nworld").unwrap();
        assert_eq!(text.content, "hello\nworld");
        assert_eq!(text.language, Language::Plaintext);
    }

    #[test]
    fn test_parse_text_binding() {
        let text = Text::parse_text_binding("  hello world  \n").unwrap();
        assert_eq!(text.content, "hello world");
        assert_eq!(text.language, Language::Plaintext);
    }

    #[test]
    fn test_syntax_hint_is_quoted() {
        assert!(SyntaxHint::Quoted.is_quoted());
        assert!(!SyntaxHint::Inline1.is_quoted());
        assert!(!SyntaxHint::Block3.is_quoted());
    }

    #[test]
    fn test_syntax_hint_is_inline() {
        assert!(SyntaxHint::Inline.is_inline());
        assert!(SyntaxHint::Inline1.is_inline());
        assert!(SyntaxHint::Inline2.is_inline());
        assert!(!SyntaxHint::Quoted.is_inline());
        assert!(!SyntaxHint::Block3.is_inline());
    }

    #[test]
    fn test_syntax_hint_is_block() {
        assert!(SyntaxHint::Block.is_block());
        assert!(SyntaxHint::Block3.is_block());
        assert!(SyntaxHint::Block4.is_block());
        assert!(SyntaxHint::Block5.is_block());
        assert!(SyntaxHint::Block6.is_block());
        assert!(!SyntaxHint::Quoted.is_block());
        assert!(!SyntaxHint::Inline1.is_block());
    }

    mod parse_indented_block_tests {
        use super::*;
        use alloc::string::ToString;

        #[test]
        fn test_parse_indented_block_single_line() {
            // 4 spaces trailing = base_indent of 4
            let content = "    hello\n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            assert_eq!(result.language, Language::Other("text".into()));
            assert_eq!(result.content, "hello\n");
        }

        #[test]
        fn test_parse_indented_block_multiple_lines() {
            // 4 spaces trailing = base_indent of 4
            let content = "    line1\n    line2\n    line3\n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            assert_eq!(result.content, "line1\nline2\nline3\n");
        }

        #[test]
        fn test_parse_indented_block_with_empty_lines() {
            // 4 spaces trailing = base_indent of 4
            let content = "    line1\n\n    line2\n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            assert_eq!(result.content, "line1\n\nline2\n");
        }

        #[test]
        fn test_parse_indented_block_whitespace_only_line() {
            // 3 spaces trailing = base_indent of 3
            let content = "    line1\n        \n    line2\n   ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            assert_eq!(result.content, " line1\n\n line2\n");
        }

        #[test]
        fn test_parse_indented_block_empty_content() {
            // Just trailing whitespace, no actual content lines
            let content = "    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            // No newline in content, so it's treated as single empty line
            assert_eq!(result.content, "\n");
        }

        #[test]
        fn test_parse_indented_block_implicit_language() {
            let content = "    hello\n    ".to_string();
            let result =
                Text::parse_indented_block(Language::Implicit, content, SyntaxHint::Block3)
                    .unwrap();
            assert_eq!(result.language, Language::Implicit);
            assert_eq!(result.content, "hello\n");
        }

        #[test]
        fn test_parse_indented_block_insufficient_indent() {
            // 4 spaces trailing = base_indent of 4, but line2 only has 2
            let content = "    line1\n  line2\n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            );
            assert_eq!(
                result,
                Err(TextParseError::IndentError {
                    line: 2,
                    actual_indent: 2,
                    expected_indent: 4,
                })
            );
        }

        #[test]
        fn test_parse_indented_block_no_indent() {
            // 4 spaces trailing = base_indent of 4, but line1 has 0
            let content = "line1\n    line2\n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            );
            assert_eq!(
                result,
                Err(TextParseError::IndentError {
                    line: 1,
                    actual_indent: 0,
                    expected_indent: 4,
                })
            );
        }

        #[test]
        fn test_parse_indented_block_empty_string() {
            let content = String::new();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_parse_indented_block_zero_indent() {
            // No trailing whitespace = base_indent of 0
            let content = "line1\nline2\n".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            assert_eq!(result.content, "line1\nline2\n");
        }

        #[test]
        fn test_parse_indented_block_empty_line_only() {
            // 4 spaces trailing = base_indent of 4
            let content = "    \n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            // First line is whitespace-only, treated as empty
            assert_eq!(result.content, "\n");
        }

        #[test]
        fn test_parse_indented_block_whitespace_only_line_insufficient_indent() {
            // 4 spaces trailing = base_indent of 4
            let content = "    line1\n  \n    line2\n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            // Whitespace-only lines are treated as empty and don't need to match indent
            assert_eq!(result.content, "line1\n\nline2\n");
        }

        #[test]
        fn test_parse_indented_block_whitespace_only_line_no_indent() {
            // 3 spaces trailing = base_indent of 3
            let content = "    line1\n\n    line2\n   ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            )
            .unwrap();
            // Empty line (no whitespace) should be preserved
            assert_eq!(result.content, " line1\n\n line2\n");
        }
    }
}
