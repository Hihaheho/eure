use alloc::{borrow::Cow, string::String};
use core::iter::Peekable;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EureString(Cow<'static, str>);

impl PartialEq<str> for EureString {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for EureString {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for EureString {
    fn eq(&self, other: &String) -> bool {
        self.0 == other.as_str()
    }
}

impl PartialEq<EureString> for str {
    fn eq(&self, other: &EureString) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<EureString> for &str {
    fn eq(&self, other: &EureString) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<EureString> for String {
    fn eq(&self, other: &EureString) -> bool {
        self == other.as_str()
    }
}

impl core::fmt::Display for EureString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Clone, Error)]
pub enum EureStringError {
    #[error("Invalid escape sequence: {0}")]
    InvalidEscapeSequence(char),
    #[error("Invalid end of string after escape")]
    InvalidEndOfStringAfterEscape,
    #[error("Invalid unicode code point: {0}")]
    InvalidUnicodeCodePoint(u32),
    #[error("Newline in text binding")]
    NewlineInTextBinding,
}

impl EureString {
    /// Creates a new EureString from a string.
    pub fn new(s: impl Into<Cow<'static, str>>) -> Self {
        EureString(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse a content after the colon of a text binding like `: hello world\n` into an EureString.
    pub fn parse_text_binding(s: &str) -> Result<Self, EureStringError> {
        let stripped = s.strip_suffix('\n').unwrap_or(s);
        let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
        if stripped.contains(['\r', '\n']) {
            return Err(EureStringError::NewlineInTextBinding);
        }
        Self::parse_quoted_string(stripped.trim()) // Safe to trim because we ensured there is no newline or carriage return.
    }

    /// Parse a content of a quoted string like `"hello world"` into an EureString.
    pub fn parse_quoted_string(s: &str) -> Result<Self, EureStringError> {
        let mut result = String::with_capacity(s.len()); // Most parsed string have equal or more characters than the original string.
        let mut chars = s.chars().peekable();

        fn parse_unicode_escape(
            chars: &mut Peekable<impl Iterator<Item = char>>,
        ) -> Result<char, EureStringError> {
            match chars.next() {
                Some('{') => {}
                Some(ch) => return Err(EureStringError::InvalidEscapeSequence(ch)),
                None => return Err(EureStringError::InvalidEndOfStringAfterEscape),
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
                return Err(EureStringError::InvalidUnicodeCodePoint(code_point));
            };

            match chars.next() {
                Some('}') => {}
                Some(ch) => return Err(EureStringError::InvalidEscapeSequence(ch)),
                None => return Err(EureStringError::InvalidEndOfStringAfterEscape),
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
                    Some(ch) => return Err(EureStringError::InvalidEscapeSequence(ch)),
                    None => return Err(EureStringError::InvalidEndOfStringAfterEscape),
                },
                _ => result.push(ch),
            }
        }

        Ok(EureString::new(result))
    }
}

#[cfg(test)]
mod parse_quoted_string_tests {
    use super::*;

    // 1. Basic String Parsing (No Escapes)
    #[test]
    fn test_empty_string() {
        let result = EureString::parse_quoted_string("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_plain_text() {
        let result = EureString::parse_quoted_string("hello").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_plain_text_with_spaces() {
        let result = EureString::parse_quoted_string("hello world").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_plain_text_with_special_chars() {
        let result = EureString::parse_quoted_string("hello@world#123").unwrap();
        assert_eq!(result, "hello@world#123");
    }

    // 2. Standard Escape Sequences
    #[test]
    fn test_escape_backslash() {
        let result = EureString::parse_quoted_string("\\\\").unwrap();
        assert_eq!(result, "\\");
    }

    #[test]
    fn test_escape_double_quote() {
        let result = EureString::parse_quoted_string("\\\"").unwrap();
        assert_eq!(result, "\"");
    }

    #[test]
    fn test_escape_single_quote() {
        let result = EureString::parse_quoted_string("\\'").unwrap();
        assert_eq!(result, "'");
    }

    #[test]
    fn test_escape_newline() {
        let result = EureString::parse_quoted_string("\\n").unwrap();
        assert_eq!(result, "\n");
    }

    #[test]
    fn test_escape_carriage_return() {
        let result = EureString::parse_quoted_string("\\r").unwrap();
        assert_eq!(result, "\r");
    }

    #[test]
    fn test_escape_tab() {
        let result = EureString::parse_quoted_string("\\t").unwrap();
        assert_eq!(result, "\t");
    }

    #[test]
    fn test_escape_null() {
        let result = EureString::parse_quoted_string("\\0").unwrap();
        assert_eq!(result, "\0");
    }

    // 3. Unicode Escape Sequences
    #[test]
    fn test_unicode_basic() {
        let result = EureString::parse_quoted_string("\\u{41}").unwrap();
        assert_eq!(result, "A");
    }

    #[test]
    fn test_unicode_with_underscore_separator() {
        let result = EureString::parse_quoted_string("\\u{1F_600}").unwrap();
        assert_eq!(result, "😀");
    }

    #[test]
    fn test_unicode_with_dash_separator() {
        let result = EureString::parse_quoted_string("\\u{1F-600}").unwrap();
        assert_eq!(result, "😀");
    }

    #[test]
    fn test_unicode_minimum_digits() {
        let result = EureString::parse_quoted_string("\\u{0}").unwrap();
        assert_eq!(result, "\0");
    }

    #[test]
    fn test_unicode_many_zeros() {
        let result = EureString::parse_quoted_string("\\u{0000}").unwrap();
        assert_eq!(result, "\0");
    }

    #[test]
    fn test_unicode_maximum_valid() {
        let result = EureString::parse_quoted_string("\\u{10FFFF}").unwrap();
        assert_eq!(result, "\u{10FFFF}");
    }

    #[test]
    fn test_unicode_japanese_hiragana() {
        let result = EureString::parse_quoted_string("\\u{3042}").unwrap();
        assert_eq!(result, "あ");
    }

    #[test]
    fn test_unicode_mixed_case() {
        let result = EureString::parse_quoted_string("\\u{AaBb}").unwrap();
        assert_eq!(result, "\u{AaBb}");
    }

    #[test]
    fn test_unicode_multiple_separators() {
        let result = EureString::parse_quoted_string("\\u{1F_60-0}").unwrap();
        assert_eq!(result, "😀");
    }

    // 4. Error Cases
    #[test]
    fn test_error_invalid_escape_sequence() {
        let result = EureString::parse_quoted_string("\\x");
        assert_eq!(result, Err(EureStringError::InvalidEscapeSequence('x')));
    }

    #[test]
    fn test_error_end_of_string_after_escape() {
        let result = EureString::parse_quoted_string("\\");
        assert_eq!(result, Err(EureStringError::InvalidEndOfStringAfterEscape));
    }

    #[test]
    fn test_error_unicode_without_brace() {
        let result = EureString::parse_quoted_string("\\u");
        assert_eq!(result, Err(EureStringError::InvalidEndOfStringAfterEscape));
    }

    #[test]
    fn test_error_unicode_invalid_character() {
        let result = EureString::parse_quoted_string("\\u{X}");
        assert_eq!(result, Err(EureStringError::InvalidEscapeSequence('X')));
    }

    #[test]
    fn test_error_unicode_missing_closing_brace() {
        let result = EureString::parse_quoted_string("\\u{41");
        assert_eq!(result, Err(EureStringError::InvalidEndOfStringAfterEscape));
    }

    #[test]
    fn test_error_unicode_out_of_range() {
        let result = EureString::parse_quoted_string("\\u{110000}");
        assert_eq!(
            result,
            Err(EureStringError::InvalidUnicodeCodePoint(0x110000))
        );
    }

    #[test]
    fn test_error_unicode_surrogate() {
        let result = EureString::parse_quoted_string("\\u{D800}");
        assert_eq!(
            result,
            Err(EureStringError::InvalidUnicodeCodePoint(0xD800))
        );
    }

    #[test]
    fn test_error_unicode_invalid_start_char() {
        let result = EureString::parse_quoted_string("\\ua");
        assert_eq!(result, Err(EureStringError::InvalidEscapeSequence('a')));
    }

    #[test]
    fn test_error_unicode_invalid_char_before_brace() {
        let result = EureString::parse_quoted_string("\\ux{41}");
        assert_eq!(result, Err(EureStringError::InvalidEscapeSequence('x')));
    }

    #[test]
    fn test_error_unicode_invalid_char_after_brace() {
        let result = EureString::parse_quoted_string("\\u{41x}");
        assert_eq!(result, Err(EureStringError::InvalidEscapeSequence('x')));
    }

    // 5. Combined Cases
    #[test]
    fn test_multiple_escapes() {
        let result = EureString::parse_quoted_string("\\n\\t\\r").unwrap();
        assert_eq!(result, "\n\t\r");
    }

    #[test]
    fn test_mixed_escapes_and_text() {
        let result = EureString::parse_quoted_string("hello\\nworld\\t!").unwrap();
        assert_eq!(result, "hello\nworld\t!");
    }

    #[test]
    fn test_consecutive_escapes() {
        let result = EureString::parse_quoted_string("\\\\\\n\\t").unwrap();
        assert_eq!(result, "\\\n\t");
    }

    #[test]
    fn test_unicode_with_text() {
        let result = EureString::parse_quoted_string("A\\u{3042}B").unwrap();
        assert_eq!(result, "AあB");
    }

    #[test]
    fn test_multiple_unicode_escapes() {
        let result = EureString::parse_quoted_string("\\u{41}\\u{42}\\u{43}").unwrap();
        assert_eq!(result, "ABC");
    }

    #[test]
    fn test_mixed_all_escape_types() {
        let result = EureString::parse_quoted_string("\\n\\t\\u{3042}\\r\\0").unwrap();
        assert_eq!(result, "\n\tあ\r\0");
    }

    #[test]
    fn test_escape_in_middle_of_text() {
        let result = EureString::parse_quoted_string("start\\nend").unwrap();
        assert_eq!(result, "start\nend");
    }

    #[test]
    fn test_quotes_with_escapes() {
        let result = EureString::parse_quoted_string("\\\"hello\\'world\\\"").unwrap();
        assert_eq!(result, "\"hello'world\"");
    }

    #[test]
    fn test_unicode_emoji_with_text() {
        let result = EureString::parse_quoted_string("Hello \\u{1F600} World").unwrap();
        assert_eq!(result, "Hello 😀 World");
    }

    #[test]
    fn test_complex_mixed_content() {
        let result = EureString::parse_quoted_string("Line1\\nLine2\\tTabbed\\u{3042}").unwrap();
        assert_eq!(result, "Line1\nLine2\tTabbedあ");
    }
}

#[cfg(test)]
mod parse_text_binding_tests {
    use super::*;

    // 1. Basic Text Binding (No Trailing Newlines)
    #[test]
    fn test_plain_text_without_trailing_newline() {
        let result = EureString::parse_text_binding("hello world 世界").unwrap();
        assert_eq!(result, "hello world 世界");
    }

    #[test]
    fn test_text_with_mixed_whitespace_and_ideographic_space() {
        let result = EureString::parse_text_binding(" \u{3000}\t hello world \t\u{3000} ").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_empty_string() {
        let result = EureString::parse_text_binding("").unwrap();
        assert_eq!(result, "");
    }

    // 2. Trailing Newline Handling
    #[test]
    fn test_text_ending_with_newline() {
        let result = EureString::parse_text_binding("hello world\n").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_text_ending_with_crlf() {
        let result = EureString::parse_text_binding("hello world\r\n").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_text_ending_with_cr() {
        let result = EureString::parse_text_binding("hello world\r").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_text_with_ideographic_space_before_trailing_newline() {
        let result = EureString::parse_text_binding("\u{3000}\t hello world \t\u{3000}\n").unwrap();
        assert_eq!(result, "hello world");
    }

    // 3. Error Cases - Embedded Newlines
    #[test]
    fn test_error_newlines_in_middle() {
        let result = EureString::parse_text_binding("hello\nworld");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
        let result = EureString::parse_text_binding("hello\rworld");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
        let result = EureString::parse_text_binding("hello\r\nworld");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
        let result = EureString::parse_text_binding("hello\nworld\ntest");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
    }

    #[test]
    fn test_error_newline_at_start() {
        let result = EureString::parse_text_binding("\nhello world");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
    }

    // 4. Error Cases - Multiple Trailing Newlines
    #[test]
    fn test_error_multiple_trailing_newlines() {
        let result = EureString::parse_text_binding("hello world\n\n");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
    }

    #[test]
    fn test_error_multiple_trailing_crlf() {
        let result = EureString::parse_text_binding("hello world\r\n\r\n");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
    }

    #[test]
    fn test_error_mixed_trailing_newline_patterns() {
        let result = EureString::parse_text_binding("hello world\n\r\n");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
    }

    #[test]
    fn test_error_trailing_newline_followed_by_cr() {
        let result = EureString::parse_text_binding("hello world\n\r");
        assert_eq!(result, Err(EureStringError::NewlineInTextBinding));
    }
}
