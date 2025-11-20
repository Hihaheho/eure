use thiserror::Error;

use crate::prelude_internal::*;

#[derive(Debug, Clone)]
pub struct Code {
    /// Hint on whether the code is a block or inline code on rendering.
    pub is_block_hint: Option<bool>,
    pub language: Option<String>,
    pub content: String,
}

// Manually implement PartialEq to ignore the is_block field
impl PartialEq for Code {
    fn eq(&self, other: &Self) -> bool {
        self.language == other.language && self.content == other.content
    }
}

#[derive(Debug, Error, PartialEq)]
#[error(
    "Invalid indent on code block at line {line}: actual {actual_indent} to be indented more than {expected_indent}"
)]
pub struct IndentError {
    pub line: usize,
    pub actual_indent: usize,
    pub expected_indent: usize,
}

impl Code {
    pub fn new(language: Option<String>, content: String) -> Self {
        Self {
            is_block_hint: None,
            language,
            content,
        }
    }

    /// Create a new code block with the trailing newline if it's not already present.
    pub fn new_block(language: Option<String>, mut content: String) -> Self {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        Self {
            is_block_hint: Some(true),
            language,
            content,
        }
    }

    /// Create a new code block without adding the trailing newline.
    pub fn new_block_without_trailing_newline(language: Option<String>, content: String) -> Self {
        Self {
            is_block_hint: Some(true),
            language,
            content,
        }
    }

    pub fn new_inline(content: String) -> Self {
        Self {
            is_block_hint: Some(false),
            language: None,
            content,
        }
    }

    /// Create a new indented code block from a string. Returns the line number if the indent is invalid.
    pub fn parse_indented_block(
        language: Option<String>,
        content: String,
        base_indent: usize,
    ) -> Result<Self, IndentError> {
        let total_lines = content.lines().count();
        let expected_whitespace_removals = base_indent * total_lines;
        let mut result = String::with_capacity(content.len() - expected_whitespace_removals);
        for (line_number, line) in content.lines().enumerate() {
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
                return Err(IndentError {
                    line: line_number + 1,
                    actual_indent,
                    expected_indent: base_indent,
                });
            }
            // Remove the base indent from the line
            result.push_str(&line[base_indent..]);
            result.push('\n');
        }

        Ok(Self::new_block(language, result))
    }

    pub fn as_str(&self) -> &str {
        &self.content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. new() method tests
    #[test]
    fn test_new_with_language() {
        let code = Code::new(Some("rust".to_string()), "fn main() {}".to_string());
        assert_eq!(code.is_block_hint, None);
        assert_eq!(code.language, Some("rust".to_string()));
        assert_eq!(code.content, "fn main() {}");
    }

    #[test]
    fn test_new_without_language() {
        let code = Code::new(None, "hello world".to_string());
        assert_eq!(code.is_block_hint, None);
        assert_eq!(code.language, None);
        assert_eq!(code.content, "hello world");
    }

    // 2. new_block() method tests
    #[test]
    fn test_new_block_empty_string() {
        let code = Code::new_block(Some("rust".to_string()), String::new());
        assert_eq!(code.is_block_hint, Some(true));
        assert_eq!(code.language, Some("rust".to_string()));
        assert_eq!(code.content, "\n");
    }

    #[test]
    fn test_new_block_multiline_content() {
        let content = "line1\nline2\nline3".to_string();
        let code = Code::new_block(Some("text".to_string()), content.clone());
        assert_eq!(code.is_block_hint, Some(true));
        assert_eq!(code.language, Some("text".to_string()));
        assert_eq!(code.content, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_new_block_multiline_with_trailing_newline() {
        let content = "line1\nline2\nline3\n".to_string();
        let code = Code::new_block(Some("text".to_string()), content.clone());
        assert_eq!(code.is_block_hint, Some(true));
        assert_eq!(code.language, Some("text".to_string()));
        assert_eq!(code.content, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_new_block_without_language() {
        let code = Code::new_block(None, "hello".to_string());
        assert_eq!(code.is_block_hint, Some(true));
        assert_eq!(code.language, None);
        assert_eq!(code.content, "hello\n");
    }

    // 3. new_block_without_trailing_newline() method tests
    #[test]
    fn test_new_block_without_trailing_newline_preserves_content() {
        let code = Code::new_block_without_trailing_newline(
            Some("rust".to_string()),
            "fn main() {}".to_string(),
        );
        assert_eq!(code.is_block_hint, Some(true));
        assert_eq!(code.language, Some("rust".to_string()));
        assert_eq!(code.content, "fn main() {}");
    }

    #[test]
    fn test_new_block_without_trailing_newline_with_newline() {
        let code = Code::new_block_without_trailing_newline(
            Some("rust".to_string()),
            "fn main() {}\n".to_string(),
        );
        assert_eq!(code.is_block_hint, Some(true));
        assert_eq!(code.language, Some("rust".to_string()));
        assert_eq!(code.content, "fn main() {}\n");
    }

    #[test]
    fn test_new_block_without_trailing_newline_without_language() {
        let code = Code::new_block_without_trailing_newline(None, "hello".to_string());
        assert_eq!(code.is_block_hint, Some(true));
        assert_eq!(code.language, None);
        assert_eq!(code.content, "hello");
    }

    // 4. new_inline() method tests
    #[test]
    fn test_new_inline_sets_is_block_hint_false() {
        let code = Code::new_inline("hello world".to_string());
        assert_eq!(code.is_block_hint, Some(false));
        assert_eq!(code.language, None);
        assert_eq!(code.content, "hello world");
    }

    // 5. parse_indented_block() method tests
    mod parse_indented_block_tests {
        use super::*;

        // 5.1. Success cases
        #[test]
        fn test_parse_indented_block_single_line() {
            let content = "    hello".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // After removing indent, should be "hello\n"
            assert_eq!(result.content, "hello\n");
        }

        #[test]
        fn test_parse_indented_block_multiple_lines() {
            let content = "    line1\n    line2\n    line3".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // After removing indent, should be "line1\nline2\nline3\n"
            assert_eq!(result.content, "line1\nline2\nline3\n");
        }

        #[test]
        fn test_parse_indented_block_with_empty_lines() {
            let content = "    line1\n    \n    line2".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // After removing indent, empty line should remain empty
            assert_eq!(result.content, "line1\n\nline2\n");
        }

        #[test]
        fn test_parse_indented_block_whitespace_only_line() {
            let content = "    line1\n        \n    line2".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // Whitespace-only lines are treated as empty lines
            assert_eq!(result.content, "line1\n\nline2\n");
        }

        #[test]
        fn test_parse_indented_block_empty_content() {
            let content = "    ".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // After removing indent, should be empty line "\n"
            assert_eq!(result.content, "\n");
        }

        #[test]
        fn test_parse_indented_block_without_language() {
            let content = "    hello".to_string();
            let result = Code::parse_indented_block(None, content, 4).unwrap();
            assert_eq!(result.language, None);
            // After removing indent, should be "hello\n"
            assert_eq!(result.content, "hello\n");
        }

        // 5.2. Error cases
        #[test]
        fn test_parse_indented_block_insufficient_indent() {
            let content = "    line1\n  line2".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4);
            assert_eq!(
                result,
                Err(IndentError {
                    line: 2,
                    actual_indent: 2,
                    expected_indent: 4,
                })
            );
        }

        #[test]
        fn test_parse_indented_block_no_indent() {
            let content = "line1\n    line2".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4);
            assert_eq!(
                result,
                Err(IndentError {
                    line: 1,
                    actual_indent: 0,
                    expected_indent: 4,
                })
            );
        }

        #[test]
        fn test_parse_indented_block_empty_string() {
            let content = String::new();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4);
            assert!(result.is_ok());
        }

        #[test]
        fn test_parse_indented_block_zero_indent() {
            let content = "line1\nline2".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 0).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // With zero indent, content should remain the same
            assert_eq!(result.content, "line1\nline2\n");
        }

        #[test]
        fn test_parse_indented_block_empty_line_only() {
            let content = "    \n    ".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // After removing indent, should be two empty lines "\n\n"
            assert_eq!(result.content, "\n\n");
        }

        #[test]
        fn test_parse_indented_block_whitespace_only_line_insufficient_indent() {
            let content = "    line1\n  \n    line2".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // Whitespace-only lines are treated as empty lines and don't need to match indent
            assert_eq!(result.content, "line1\n\nline2\n");
        }

        #[test]
        fn test_parse_indented_block_whitespace_only_line_no_indent() {
            let content = "    line1\n\n    line2".to_string();
            let result = Code::parse_indented_block(Some("text".to_string()), content, 4).unwrap();
            assert_eq!(result.language, Some("text".to_string()));
            // Empty line (no whitespace) should be preserved
            assert_eq!(result.content, "line1\n\nline2\n");
        }
    }

    // 6. PartialEq implementation tests
    #[test]
    fn test_partial_eq_ignores_is_block_hint() {
        let code1 = Code {
            is_block_hint: Some(true),
            language: Some("rust".to_string()),
            content: "hello".to_string(),
        };
        let code2 = Code {
            is_block_hint: Some(false),
            language: Some("rust".to_string()),
            content: "hello".to_string(),
        };
        assert_eq!(code1, code2);
    }

    #[test]
    fn test_partial_eq_same_language_and_content() {
        let code1 = Code::new(Some("rust".to_string()), "hello".to_string());
        let code2 = Code::new(Some("rust".to_string()), "hello".to_string());
        assert_eq!(code1, code2);
    }

    #[test]
    fn test_partial_eq_different_language() {
        let code1 = Code::new(Some("rust".to_string()), "hello".to_string());
        let code2 = Code::new(Some("python".to_string()), "hello".to_string());
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_partial_eq_different_content() {
        let code1 = Code::new(Some("rust".to_string()), "hello".to_string());
        let code2 = Code::new(Some("rust".to_string()), "world".to_string());
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_partial_eq_none_language() {
        let code1 = Code::new(None, "hello".to_string());
        let code2 = Code::new(None, "hello".to_string());
        assert_eq!(code1, code2);
    }

    #[test]
    fn test_partial_eq_one_none_language() {
        let code1 = Code::new(Some("rust".to_string()), "hello".to_string());
        let code2 = Code::new(None, "hello".to_string());
        assert_ne!(code1, code2);
    }
}
