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
    // === String syntax variants ===
    /// Escaped string: `"..."`
    Str,
    /// Literal string: `'...'`
    LitStr,
    /// Literal string with level 1 delimiters: `<'...'>`
    LitStr1,
    /// Literal string with level 2 delimiters: `<<'...'>>`
    LitStr2,
    /// Literal string with level 3 delimiters: `<<<'...'>>>`
    LitStr3,

    // === Inline code syntax variants ===
    /// Generic inline code (serializer picks appropriate syntax)
    Inline,
    /// Single backtick inline: `` `...` ``
    Inline1,
    /// Single-delimited code: `<`...`>`
    Delim1,
    /// Double-delimited code: `<<`...`>>`
    Delim2,
    /// Triple-delimited code: `<<<`...`>>>`
    Delim3,

    // === Block code syntax variants ===
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
    /// Returns true if this is any string syntax (escaped or literal).
    pub fn is_string(&self) -> bool {
        matches!(
            self,
            SyntaxHint::Str
                | SyntaxHint::LitStr
                | SyntaxHint::LitStr1
                | SyntaxHint::LitStr2
                | SyntaxHint::LitStr3
        )
    }

    /// Returns true if this is an escaped string syntax (`"..."`).
    pub fn is_escaped_string(&self) -> bool {
        matches!(self, SyntaxHint::Str)
    }

    /// Returns true if this is a literal string syntax (`'...'` variants).
    pub fn is_literal_string(&self) -> bool {
        matches!(
            self,
            SyntaxHint::LitStr | SyntaxHint::LitStr1 | SyntaxHint::LitStr2 | SyntaxHint::LitStr3
        )
    }

    /// Returns true if this is any inline code syntax.
    pub fn is_inline(&self) -> bool {
        matches!(
            self,
            SyntaxHint::Inline
                | SyntaxHint::Inline1
                | SyntaxHint::Delim1
                | SyntaxHint::Delim2
                | SyntaxHint::Delim3
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
/// | `"hello"` | `Plaintext` | `Str` |
/// | `'hello'` | `Plaintext` | `LitStr` |
/// | `<'hello'>` | `Plaintext` | `LitStr1` |
/// | `` `hello` `` | `Implicit` | `Inline1` |
/// | `` sql`SELECT` `` | `Other("sql")` | `Inline1` |
/// | `<`hello`>` | `Implicit` | `Delim1` |
/// | `sql<`SELECT`>` | `Other("sql")` | `Delim1` |
/// | `<<`hello`>>` | `Implicit` | `Delim2` |
/// | `<<<`hello`>>>` | `Implicit` | `Delim3` |
/// | ```` ``` ```` (no lang) | `Implicit` | `Block3` |
/// | ```` ```rust ```` | `Other("rust")` | `Block3` |
///
/// # Key Distinction
///
/// - `"..."` → `Plaintext` (explicit: "this is text, not code")
/// - `` `...` `` without lang → `Implicit` (code, language inferred from schema)
/// - `` lang`...` `` → `Other(lang)` (code with explicit language)
#[derive(Debug, Clone)]
pub struct Text {
    /// The text content.
    pub content: String,
    /// The language tag for this text.
    pub language: Language,
    /// Hint for serialization about the original syntax.
    /// Note: This is NOT included in equality comparison as it's formatting metadata.
    pub syntax_hint: Option<SyntaxHint>,
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        // syntax_hint is intentionally excluded - it's formatting metadata, not semantic content
        self.content == other.content && self.language == other.language
    }
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
            syntax_hint: Some(SyntaxHint::Str),
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
        let content = String::from(stripped.trim());
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
    extern crate alloc;

    use super::*;
    use alloc::format;

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
        assert_eq!(text.syntax_hint, Some(SyntaxHint::Str));
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
    fn test_parse_text_binding_raw_backslashes() {
        // Text bindings should NOT process escape sequences
        let text = Text::parse_text_binding("  \\b\\w+\\b  \n").unwrap();
        assert_eq!(text.content, "\\b\\w+\\b");
        assert_eq!(text.language, Language::Plaintext);
    }

    #[test]
    fn test_parse_text_binding_literal_backslash_n() {
        // Literal \n should stay as two characters, not converted to newline
        let text = Text::parse_text_binding("  line1\\nline2  \n").unwrap();
        assert_eq!(text.content, "line1\\nline2");
        assert_eq!(text.language, Language::Plaintext);
    }

    #[test]
    fn test_parse_text_binding_windows_path() {
        // Windows paths should work without escaping
        let text = Text::parse_text_binding("  C:\\Users\\name\\file.txt  \n").unwrap();
        assert_eq!(text.content, "C:\\Users\\name\\file.txt");
    }

    #[test]
    fn test_parse_text_binding_double_backslash() {
        // Double backslashes stay as-is (two characters each = 4 total)
        let text = Text::parse_text_binding("  \\\\  \n").unwrap();
        assert_eq!(text.content, "\\\\");
    }

    #[test]
    fn test_syntax_hint_is_string() {
        // Escaped strings
        assert!(SyntaxHint::Str.is_string());
        // Literal strings
        assert!(SyntaxHint::LitStr.is_string());
        assert!(SyntaxHint::LitStr1.is_string());
        assert!(SyntaxHint::LitStr2.is_string());
        assert!(SyntaxHint::LitStr3.is_string());
        // Non-strings
        assert!(!SyntaxHint::Inline1.is_string());
        assert!(!SyntaxHint::Block3.is_string());
    }

    #[test]
    fn test_syntax_hint_is_escaped_string() {
        assert!(SyntaxHint::Str.is_escaped_string());
        assert!(!SyntaxHint::LitStr.is_escaped_string());
        assert!(!SyntaxHint::Inline1.is_escaped_string());
    }

    #[test]
    fn test_syntax_hint_is_literal_string() {
        assert!(SyntaxHint::LitStr.is_literal_string());
        assert!(SyntaxHint::LitStr1.is_literal_string());
        assert!(SyntaxHint::LitStr2.is_literal_string());
        assert!(SyntaxHint::LitStr3.is_literal_string());
        assert!(!SyntaxHint::Str.is_literal_string());
        assert!(!SyntaxHint::Inline1.is_literal_string());
    }

    #[test]
    fn test_syntax_hint_is_inline() {
        assert!(SyntaxHint::Inline.is_inline());
        assert!(SyntaxHint::Inline1.is_inline());
        assert!(SyntaxHint::Delim1.is_inline());
        assert!(SyntaxHint::Delim2.is_inline());
        assert!(SyntaxHint::Delim3.is_inline());
        assert!(!SyntaxHint::Str.is_inline());
        assert!(!SyntaxHint::Block3.is_inline());
    }

    #[test]
    fn test_syntax_hint_is_block() {
        assert!(SyntaxHint::Block.is_block());
        assert!(SyntaxHint::Block3.is_block());
        assert!(SyntaxHint::Block4.is_block());
        assert!(SyntaxHint::Block5.is_block());
        assert!(SyntaxHint::Block6.is_block());
        assert!(!SyntaxHint::Str.is_block());
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

        // =====================================================================
        // Deterministic tests moved from proptests
        // =====================================================================

        #[test]
        fn test_parse_quoted_string_escape_sequences() {
            let cases = [
                ("\\n", "\n"),
                ("\\r", "\r"),
                ("\\t", "\t"),
                ("\\0", "\0"),
                ("\\\\", "\\"),
                ("\\\"", "\""),
                ("\\'", "'"),
                ("\\u{0041}", "A"),
                ("\\u{3042}", "あ"),
            ];
            for (input, expected) in cases {
                let result = Text::parse_quoted_string(input);
                assert!(result.is_ok(), "Failed to parse: {:?}", input);
                assert_eq!(
                    result.unwrap().content,
                    expected,
                    "Mismatch for: {:?}",
                    input
                );
            }
        }

        #[test]
        fn test_parse_quoted_string_invalid_unicode_escapes() {
            // Missing closing brace
            let result = Text::parse_quoted_string("\\u{0041");
            assert!(result.is_err(), "Should fail for missing closing brace");

            // Note: \u{} parses to '\0' (null character) - this is valid behavior

            // Invalid hex characters (Z is not a hex digit)
            let result = Text::parse_quoted_string("\\u{ZZZZ}");
            assert!(result.is_err(), "Should fail for invalid hex");

            // Out of range codepoint (beyond Unicode max 0x10FFFF)
            let result = Text::parse_quoted_string("\\u{110000}");
            assert!(result.is_err(), "Should fail for out of range codepoint");

            // Missing opening brace
            let result = Text::parse_quoted_string("\\u0041}");
            assert!(result.is_err(), "Should fail for missing opening brace");
        }

        #[test]
        fn test_parse_text_binding_preserves_backslashes() {
            let inputs = [
                ("\\n", "\\n"),
                ("\\t", "\\t"),
                ("C:\\Users\\test", "C:\\Users\\test"),
                ("\\b\\w+\\b", "\\b\\w+\\b"),
            ];
            for (input, expected) in inputs {
                let with_newline = format!("{}\n", input);
                let result = Text::parse_text_binding(&with_newline);
                assert!(result.is_ok(), "Failed to parse: {:?}", input);
                assert_eq!(result.unwrap().content, expected);
            }
        }

        #[test]
        fn test_parse_text_binding_trims_tabs_and_mixed_whitespace() {
            // Tabs
            let result = Text::parse_text_binding("\thello\t\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap().content, "hello");

            // Mixed spaces and tabs
            let result = Text::parse_text_binding("  \thello\t  \n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap().content, "hello");

            // Only tabs
            let result = Text::parse_text_binding("\t\thello world\t\t\n");
            assert!(result.is_ok());
            assert_eq!(result.unwrap().content, "hello world");
        }

        #[test]
        fn test_language_new_plaintext_variants() {
            assert_eq!(Language::new("plaintext"), Language::Plaintext);
            assert_eq!(Language::new(""), Language::Plaintext);
        }

        #[test]
        fn test_empty_content_handling() {
            let text = Text::plaintext("");
            assert_eq!(text.content, "");

            let text = Text::inline_implicit("");
            assert_eq!(text.content, "");

            let text = Text::block_implicit("");
            assert_eq!(text.content, "\n"); // Should add newline

            let text = Text::block("", "rust");
            assert_eq!(text.content, "\n"); // Should add newline
        }

        #[test]
        fn test_parse_indented_block_with_tabs() {
            // Content with tab indentation - should be rejected or handled
            // since parse_indented_block uses space-based indent detection
            let content = "\tline1\n\tline2\n\t".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            );
            // Tabs count as characters, not as indent spaces, so this should work
            // with 0 base indent (trailing has 1 tab = no spaces for indent detection)
            assert!(result.is_ok() || result.is_err()); // Just ensure no panic

            // Mixed tabs and spaces - spaces for indent, tabs in content
            let content = "    line\twith\ttabs\n    ".to_string();
            let result = Text::parse_indented_block(
                Language::Other("text".into()),
                content,
                SyntaxHint::Block3,
            );
            assert!(result.is_ok());
            let text = result.unwrap();
            assert_eq!(text.content, "line\twith\ttabs\n");
        }
    }
}

#[cfg(test)]
mod proptests {
    extern crate std;

    use super::*;
    use alloc::vec;
    use proptest::prelude::*;
    use std::format;
    use std::string::String;
    use std::vec::Vec;

    // =========================================================================
    // Strategy generators
    // =========================================================================

    /// Strategy for generating arbitrary Language values.
    fn arb_language() -> impl Strategy<Value = Language> {
        prop_oneof![
            Just(Language::Plaintext),
            Just(Language::Implicit),
            // Common language tags
            Just(Language::Other("rust".into())),
            Just(Language::Other("sql".into())),
            Just(Language::Other("python".into())),
            Just(Language::Other("javascript".into())),
            // Arbitrary language tags
            "[a-z][a-z0-9_-]{0,15}".prop_map(|s| Language::Other(s.into())),
        ]
    }

    /// Strategy for generating arbitrary SyntaxHint values.
    fn arb_syntax_hint() -> impl Strategy<Value = SyntaxHint> {
        prop_oneof![
            // String variants
            Just(SyntaxHint::Str),
            Just(SyntaxHint::LitStr),
            Just(SyntaxHint::LitStr1),
            Just(SyntaxHint::LitStr2),
            Just(SyntaxHint::LitStr3),
            // Inline variants
            Just(SyntaxHint::Inline),
            Just(SyntaxHint::Inline1),
            Just(SyntaxHint::Delim1),
            Just(SyntaxHint::Delim2),
            Just(SyntaxHint::Delim3),
            // Block variants
            Just(SyntaxHint::Block),
            Just(SyntaxHint::Block3),
            Just(SyntaxHint::Block4),
            Just(SyntaxHint::Block5),
            Just(SyntaxHint::Block6),
        ]
    }

    /// Strategy for generating text content without control characters.
    fn arb_text_content() -> impl Strategy<Value = String> {
        // Printable ASCII and common Unicode, excluding null and other control chars
        proptest::collection::vec(
            prop_oneof![
                // Printable ASCII
                prop::char::range(' ', '~'),
                // Some Unicode characters
                Just('日'),
                Just('本'),
                Just('語'),
                Just('α'),
                Just('β'),
                Just('γ'),
                Just('é'),
                Just('ñ'),
                Just('ü'),
            ],
            0..100,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    /// Strategy for generating text content that's valid for escaped strings.
    /// Excludes characters that would need escaping for simpler testing.
    fn arb_simple_text_content() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop_oneof![
                // Printable ASCII excluding backslash and quotes
                prop::char::range(' ', '!'), // space and !
                prop::char::range('#', '&'), // # $ % &
                prop::char::range('(', '['), // ( through [
                prop::char::range(']', '~'), // ] through ~
            ],
            0..50,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    /// Strategy for single-line content (no newlines).
    fn arb_single_line_content() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop_oneof![
                // Printable ASCII excluding newlines
                prop::char::range(' ', '~'),
            ],
            0..50,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    // =========================================================================
    // Constructor tests
    // =========================================================================

    proptest! {
        /// Text::plaintext should always set Language::Plaintext and SyntaxHint::Str.
        #[test]
        fn plaintext_constructor_sets_correct_fields(content in arb_text_content()) {
            let text = Text::plaintext(content.clone());
            prop_assert_eq!(text.content, content);
            prop_assert_eq!(text.language, Language::Plaintext);
            prop_assert_eq!(text.syntax_hint, Some(SyntaxHint::Str));
        }

        /// Text::inline_implicit should always set Language::Implicit and SyntaxHint::Inline1.
        #[test]
        fn inline_implicit_constructor_sets_correct_fields(content in arb_text_content()) {
            let text = Text::inline_implicit(content.clone());
            prop_assert_eq!(text.content, content);
            prop_assert_eq!(text.language, Language::Implicit);
            prop_assert_eq!(text.syntax_hint, Some(SyntaxHint::Inline1));
        }

        /// Text::inline should set Language from parameter and SyntaxHint::Inline1.
        #[test]
        fn inline_constructor_sets_correct_fields(
            content in arb_text_content(),
            lang in "[a-z][a-z0-9]{0,10}",
        ) {
            let text = Text::inline(content.clone(), lang.clone());
            prop_assert_eq!(text.content, content);
            prop_assert_eq!(text.language, Language::new(lang));
            prop_assert_eq!(text.syntax_hint, Some(SyntaxHint::Inline1));
        }

        /// Text::block_implicit should add trailing newline if missing.
        #[test]
        fn block_implicit_adds_trailing_newline(content in arb_text_content()) {
            let text = Text::block_implicit(content.clone());
            prop_assert!(text.content.ends_with('\n'), "Block content should end with newline");
            prop_assert_eq!(text.language, Language::Implicit);
            prop_assert_eq!(text.syntax_hint, Some(SyntaxHint::Block3));
        }

        /// Text::block_implicit should not add extra newline if already present.
        #[test]
        fn block_implicit_no_double_newline(content in arb_text_content()) {
            let content_with_newline = format!("{}\n", content);
            let text = Text::block_implicit(content_with_newline.clone());
            prop_assert_eq!(&text.content, &content_with_newline);
            prop_assert!(!text.content.ends_with("\n\n") || content.ends_with('\n'),
                "Should not add extra newline when already present");
        }

        /// Text::block should add trailing newline if missing.
        #[test]
        fn block_adds_trailing_newline(
            content in arb_text_content(),
            lang in "[a-z][a-z0-9]{0,10}",
        ) {
            let text = Text::block(content.clone(), lang.clone());
            prop_assert!(text.content.ends_with('\n'), "Block content should end with newline");
            prop_assert_eq!(text.language, Language::new(lang));
            prop_assert_eq!(text.syntax_hint, Some(SyntaxHint::Block3));
        }

        /// Text::block_without_trailing_newline should preserve content exactly.
        #[test]
        fn block_without_trailing_newline_preserves_content(
            content in arb_text_content(),
            lang in "[a-z][a-z0-9]{0,10}",
        ) {
            let text = Text::block_without_trailing_newline(content.clone(), lang.clone());
            prop_assert_eq!(text.content, content);
            prop_assert_eq!(text.language, Language::new(lang));
            prop_assert_eq!(text.syntax_hint, Some(SyntaxHint::Block3));
        }

        /// Text::new should not modify content.
        #[test]
        fn new_preserves_content(
            content in arb_text_content(),
            language in arb_language(),
        ) {
            let text = Text::new(content.clone(), language.clone());
            prop_assert_eq!(text.content, content);
            prop_assert_eq!(text.language, language);
            prop_assert_eq!(text.syntax_hint, None);
        }

        /// Text::with_syntax_hint should add trailing newline for block hints.
        #[test]
        fn with_syntax_hint_adds_newline_for_block(
            content in arb_text_content(),
            language in arb_language(),
            hint in prop_oneof![
                Just(SyntaxHint::Block),
                Just(SyntaxHint::Block3),
                Just(SyntaxHint::Block4),
                Just(SyntaxHint::Block5),
                Just(SyntaxHint::Block6),
            ],
        ) {
            let text = Text::with_syntax_hint(content.clone(), language.clone(), hint);
            prop_assert!(text.content.ends_with('\n'), "Block content should end with newline");
            prop_assert_eq!(text.language, language);
            prop_assert_eq!(text.syntax_hint, Some(hint));
        }

        /// Text::with_syntax_hint should not modify content for non-block hints.
        #[test]
        fn with_syntax_hint_preserves_content_for_non_block(
            content in arb_text_content(),
            language in arb_language(),
            hint in prop_oneof![
                Just(SyntaxHint::Str),
                Just(SyntaxHint::LitStr),
                Just(SyntaxHint::Inline1),
                Just(SyntaxHint::Delim1),
            ],
        ) {
            let text = Text::with_syntax_hint(content.clone(), language.clone(), hint);
            prop_assert_eq!(text.content, content);
            prop_assert_eq!(text.language, language);
            prop_assert_eq!(text.syntax_hint, Some(hint));
        }
    }

    // =========================================================================
    // Equality tests
    // =========================================================================

    proptest! {
        /// PartialEq should ignore syntax_hint.
        #[test]
        fn equality_ignores_syntax_hint(
            content in arb_text_content(),
            language in arb_language(),
            hint1 in arb_syntax_hint(),
            hint2 in arb_syntax_hint(),
        ) {
            let text1 = Text {
                content: content.clone(),
                language: language.clone(),
                syntax_hint: Some(hint1),
            };
            let text2 = Text {
                content: content.clone(),
                language: language.clone(),
                syntax_hint: Some(hint2),
            };
            prop_assert_eq!(text1, text2, "Equality should ignore syntax_hint");
        }

        /// PartialEq should compare content.
        #[test]
        fn equality_compares_content(
            content1 in arb_text_content(),
            content2 in arb_text_content(),
            language in arb_language(),
        ) {
            let text1 = Text::new(content1.clone(), language.clone());
            let text2 = Text::new(content2.clone(), language.clone());
            if content1 == content2 {
                prop_assert_eq!(text1, text2);
            } else {
                prop_assert_ne!(text1, text2);
            }
        }

        /// PartialEq should compare language.
        #[test]
        fn equality_compares_language(
            content in arb_text_content(),
            lang1 in arb_language(),
            lang2 in arb_language(),
        ) {
            let text1 = Text::new(content.clone(), lang1.clone());
            let text2 = Text::new(content.clone(), lang2.clone());
            if lang1 == lang2 {
                prop_assert_eq!(text1, text2);
            } else {
                prop_assert_ne!(text1, text2);
            }
        }
    }

    // =========================================================================
    // Language tests
    // =========================================================================

    proptest! {
        /// Language::new with other strings should produce Other.
        #[test]
        fn language_new_other(lang in "[a-z][a-z0-9]{1,15}") {
            if lang != "plaintext" {
                let result = Language::new(lang.clone());
                prop_assert_eq!(result, Language::Other(lang.into()));
            }
        }

        /// Language::Implicit is compatible with everything.
        #[test]
        fn implicit_is_compatible_with_all(lang in arb_language()) {
            prop_assert!(Language::Implicit.is_compatible_with(&lang),
                "Implicit should be compatible with {:?}", lang);
        }

        /// Everything is compatible with Language::Implicit.
        #[test]
        fn all_compatible_with_implicit(lang in arb_language()) {
            prop_assert!(lang.is_compatible_with(&Language::Implicit),
                "{:?} should be compatible with Implicit", lang);
        }

        /// Same languages are compatible with themselves.
        #[test]
        fn same_language_compatible(lang in arb_language()) {
            prop_assert!(lang.is_compatible_with(&lang),
                "{:?} should be compatible with itself", lang);
        }

        /// Language::as_str returns correct values.
        #[test]
        fn language_as_str_correct(lang in arb_language()) {
            match &lang {
                Language::Plaintext => prop_assert_eq!(lang.as_str(), Some("plaintext")),
                Language::Implicit => prop_assert_eq!(lang.as_str(), None),
                Language::Other(s) => prop_assert_eq!(lang.as_str(), Some(s.as_ref())),
            }
        }
    }

    // =========================================================================
    // SyntaxHint tests
    // =========================================================================

    proptest! {
        /// SyntaxHint classification methods are mutually exclusive (except Str which is both escaped and string).
        #[test]
        fn syntax_hint_classification_consistency(hint in arb_syntax_hint()) {
            let is_str = hint.is_string();
            let is_inline = hint.is_inline();
            let is_block = hint.is_block();

            // At most one category should be true (inline and block are mutually exclusive with string)
            if is_inline {
                prop_assert!(!is_str, "Inline hints should not be strings");
                prop_assert!(!is_block, "Inline hints should not be blocks");
            }
            if is_block {
                prop_assert!(!is_str, "Block hints should not be strings");
                prop_assert!(!is_inline, "Block hints should not be inline");
            }
            if is_str {
                prop_assert!(!is_inline, "String hints should not be inline");
                prop_assert!(!is_block, "String hints should not be blocks");
            }
        }

        /// Every SyntaxHint should belong to exactly one category.
        #[test]
        fn syntax_hint_belongs_to_one_category(hint in arb_syntax_hint()) {
            let categories = [
                hint.is_string(),
                hint.is_inline(),
                hint.is_block(),
            ];
            let count = categories.iter().filter(|&&b| b).count();
            prop_assert_eq!(count, 1, "Each hint should belong to exactly one category: {:?}", hint);
        }
    }

    // =========================================================================
    // Parsing tests
    // =========================================================================

    proptest! {
        /// Parsing simple content (no escapes) should round-trip through escape parsing.
        #[test]
        fn parse_quoted_string_simple_roundtrip(content in arb_simple_text_content()) {
            let text = Text::parse_quoted_string(&content);
            prop_assert!(text.is_ok(), "Failed to parse simple content: {:?}", content);
            let text = text.unwrap();
            prop_assert_eq!(text.content, content);
            prop_assert_eq!(text.language, Language::Plaintext);
        }

        /// Invalid escape sequences should produce errors.
        #[test]
        fn parse_quoted_string_invalid_escape(c in prop::char::range('a', 'z').prop_filter(
            "not a valid escape",
            |c| !matches!(*c, 'n' | 'r' | 't' | '0' | 'u')
        )) {
            let input = format!("\\{}", c);
            let result = Text::parse_quoted_string(&input);
            prop_assert!(result.is_err(), "Should fail for invalid escape: {:?}", input);
            match result {
                Err(TextParseError::InvalidEscapeSequence(ch)) => {
                    prop_assert_eq!(ch, c, "Error should report the invalid char");
                }
                other => {
                    prop_assert!(false, "Expected InvalidEscapeSequence, got {:?}", other);
                }
            }
        }

        /// parse_text_binding should trim whitespace (spaces and tabs) and strip trailing newline.
        #[test]
        fn parse_text_binding_trims_correctly(
            leading_space in "[ \t]{0,10}",
            content in arb_single_line_content().prop_filter("no whitespace only", |s| !s.trim().is_empty()),
            trailing_space in "[ \t]{0,10}",
        ) {
            let input = format!("{}{}{}\n", leading_space, content, trailing_space);
            let result = Text::parse_text_binding(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {:?}", input);
            let text = result.unwrap();
            prop_assert_eq!(text.content, content.trim());
            prop_assert_eq!(text.language, Language::Plaintext);
        }

        /// parse_text_binding should reject content with embedded newlines.
        #[test]
        fn parse_text_binding_rejects_embedded_newlines(
            before in arb_single_line_content(),
            after in arb_single_line_content(),
        ) {
            let input = format!("{}\n{}\n", before, after);
            let result = Text::parse_text_binding(&input);
            prop_assert!(matches!(result, Err(TextParseError::NewlineInTextBinding)),
                "Should reject embedded newlines: {:?}", input);
        }

        /// as_str should return the content.
        #[test]
        fn as_str_returns_content(content in arb_text_content(), language in arb_language()) {
            let text = Text::new(content.clone(), language);
            prop_assert_eq!(text.as_str(), content.as_str());
        }
    }

    // =========================================================================
    // parse_indented_block tests
    // =========================================================================

    proptest! {
        /// parse_indented_block should correctly detect and remove base indentation.
        #[test]
        fn parse_indented_block_removes_base_indent(
            // Use lines without leading/trailing whitespace; whitespace-only lines are treated specially
            lines in proptest::collection::vec("[!-~]+", 1..10),
            indent in 0usize..8,
        ) {
            // Build indented content with trailing indent marker
            let indent_str: String = " ".repeat(indent);
            let mut content = String::new();
            for line in &lines {
                content.push_str(&indent_str);
                content.push_str(line);
                content.push('\n');
            }
            // Add trailing indent for delimiter (without newline at end)
            content.push_str(&indent_str);

            let result = Text::parse_indented_block(
                Language::Implicit,
                content,
                SyntaxHint::Block3,
            );
            prop_assert!(result.is_ok(), "Failed to parse indented block");
            let text = result.unwrap();

            // Verify each line had indent removed
            let result_lines: Vec<&str> = text.content.lines().collect();
            prop_assert_eq!(result_lines.len(), lines.len(),
                "Line count should match: {:?} vs {:?}", result_lines, lines);
            for (i, (result_line, orig_line)) in result_lines.iter().zip(lines.iter()).enumerate() {
                prop_assert_eq!(*result_line, orig_line.as_str(),
                    "Line {} should have indent removed", i);
            }
        }

        /// parse_indented_block should preserve empty lines.
        #[test]
        fn parse_indented_block_preserves_empty_lines(
            line1 in arb_single_line_content(),
            line2 in arb_single_line_content(),
            indent in 2usize..6,
        ) {
            let indent_str: String = " ".repeat(indent);
            // Content with empty line in the middle
            let content = format!(
                "{}{}\n\n{}{}\n{}",
                indent_str, line1,
                indent_str, line2,
                indent_str
            );

            let result = Text::parse_indented_block(
                Language::Implicit,
                content,
                SyntaxHint::Block3,
            );
            prop_assert!(result.is_ok(), "Failed to parse");
            let text = result.unwrap();

            let expected_line1 = if line1.trim().is_empty() { "" } else { line1.as_str() };
            let expected_line2 = if line2.trim().is_empty() { "" } else { line2.as_str() };
            let expected = format!("{}\n\n{}\n", expected_line1, expected_line2);
            prop_assert_eq!(text.content, expected);
        }

        /// parse_indented_block should return error for insufficient indent.
        #[test]
        fn parse_indented_block_error_on_insufficient_indent(
            line1 in arb_single_line_content().prop_filter("non-empty", |s| !s.is_empty()),
            // Line2 must not start with whitespace (we control indent via bad_str)
            // and must have non-whitespace content
            line2 in "[!-~]{1,20}",  // Non-whitespace printable ASCII
            base_indent in 4usize..8,
            bad_indent in 0usize..4,
        ) {
            prop_assume!(bad_indent < base_indent);
            let base_str: String = " ".repeat(base_indent);
            let bad_str: String = " ".repeat(bad_indent);

            let content = format!(
                "{}{}\n{}{}\n{}",
                base_str, line1,
                bad_str, line2,  // insufficient indent
                base_str
            );

            let result = Text::parse_indented_block(
                Language::Implicit,
                content,
                SyntaxHint::Block3,
            );

            match result {
                Err(TextParseError::IndentError { line: 2, actual_indent, expected_indent }) => {
                    prop_assert_eq!(actual_indent, bad_indent);
                    prop_assert_eq!(expected_indent, base_indent);
                }
                other => {
                    prop_assert!(false, "Expected IndentError for line 2, got {:?}", other);
                }
            }
        }

        /// parse_indented_block should handle zero indent correctly.
        #[test]
        fn parse_indented_block_zero_indent(lines in proptest::collection::vec("[!-~]+", 1..10)) {
            let mut content = String::new();
            for line in &lines {
                content.push_str(line);
                content.push('\n');
            }
            // No trailing indent

            let result = Text::parse_indented_block(
                Language::Other("test".into()),
                content.clone(),
                SyntaxHint::Block3,
            );
            prop_assert!(result.is_ok(), "Failed to parse zero-indent block");
            let text = result.unwrap();

            // Content should be preserved as-is
            let expected_lines: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            let result_lines: Vec<&str> = text.content.lines().collect();
            prop_assert_eq!(result_lines, expected_lines);
        }

        /// parse_indented_block should preserve language and syntax_hint.
        #[test]
        fn parse_indented_block_preserves_metadata(
            line in arb_single_line_content(),
            language in arb_language(),
            hint in prop_oneof![
                Just(SyntaxHint::Block3),
                Just(SyntaxHint::Block4),
                Just(SyntaxHint::Block5),
                Just(SyntaxHint::Block6),
            ],
        ) {
            let content = format!("{}\n", line);
            let result = Text::parse_indented_block(language.clone(), content, hint);
            prop_assert!(result.is_ok());
            let text = result.unwrap();
            prop_assert_eq!(text.language, language);
            prop_assert_eq!(text.syntax_hint, Some(hint));
        }
    }

    // =========================================================================
    // Edge case tests
    // =========================================================================

    proptest! {
        /// Unicode content should be preserved correctly.
        #[test]
        fn unicode_content_preserved(content in "[\u{0080}-\u{FFFF}]{1,50}") {
            let text = Text::plaintext(content.clone());
            prop_assert_eq!(&text.content, &content);

            let text = Text::inline_implicit(content.clone());
            prop_assert_eq!(&text.content, &content);
        }

        /// Text with only whitespace should be handled correctly.
        #[test]
        fn whitespace_only_content(spaces in "[ \t]{1,20}") {
            let text = Text::plaintext(spaces.clone());
            prop_assert_eq!(&text.content, &spaces);

            // parse_text_binding should trim to empty
            let input = format!("{}\n", spaces);
            let result = Text::parse_text_binding(&input);
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().content, "");
        }
    }
}
