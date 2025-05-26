/// A helper struct that stores the new line indexes of the input text.
/// This is used to get the line number of a given span.
pub struct LineNumbers<'a> {
    phantom: std::marker::PhantomData<&'a str>,
    indexes: Vec<u32>,
}

/// Information about a character position in text
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct CharInfo {
    /// The line number (0-indexed)
    pub line_number: u32,
    /// The column number (0-indexed)
    pub column_number: u32,
    /// The last newline position before this character (None if this is on the first line)
    pub last_newline: Option<u32>,
}

impl LineNumbers<'_> {
    pub fn new(input: &str) -> Self {
        let mut indexes = vec![];
        for (i, c) in input.chars().enumerate() {
            if c == '\n' {
                indexes.push(i as u32);
            }
        }
        Self {
            phantom: std::marker::PhantomData,
            indexes,
        }
    }

    /// Get the line number for a character index (0-indexed)
    /// This method is kept for backward compatibility but is now deprecated.
    /// Use get_char_info instead.
    #[deprecated(since = "0.1.0", note = "Use get_char_info instead")]
    pub fn get_line_number(&self, index: u32) -> u32 {
        self.get_char_info(index).line_number
    }

    /// Get detailed character position information for a given character index
    pub fn get_char_info(&self, index: u32) -> CharInfo {
        // Find the number of newline characters that come before this index
        let newlines_before = self.indexes.iter().take_while(|&&i| i < index).count();
        let line_number = newlines_before as u32;

        // Find the last newline before this position
        let last_newline = if line_number == 0 {
            None
        } else {
            Some(self.indexes[newlines_before - 1])
        };

        // Calculate column number based on last newline
        let column_number = match last_newline {
            Some(pos) => index - (pos + 1),
            None => index,
        };

        CharInfo {
            line_number,
            column_number,
            last_newline,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A span that is only valid within the context of the input text.
pub struct InputSpan {
    /// The start of the span.
    pub start: u32,
    /// The end of the span.
    pub end: u32,
}

impl InputSpan {
    pub const EMPTY: Self = Self {
        start: u32::MAX,
        end: 0,
    };

    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn merge_many(self, others: impl IntoIterator<Item = Self>) -> Self {
        others.into_iter().fold(self, |acc, other| acc.merge(other))
    }

    pub fn as_str<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start as usize..self.end as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        let line_numbers = LineNumbers::new("");
        assert!(line_numbers.indexes.is_empty());
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        );
        assert_eq!(
            line_numbers.get_char_info(1),
            CharInfo {
                line_number: 0,
                column_number: 1,
                last_newline: None
            }
        );
    }

    #[test]
    fn test_single_line() {
        let line_numbers = LineNumbers::new("hello world");
        assert!(line_numbers.indexes.is_empty());
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        );
        assert_eq!(
            line_numbers.get_char_info(5),
            CharInfo {
                line_number: 0,
                column_number: 5,
                last_newline: None
            }
        );
        assert_eq!(
            line_numbers.get_char_info(100),
            CharInfo {
                line_number: 0,
                column_number: 100,
                last_newline: None
            }
        );
    }

    #[test]
    fn test_multiple_lines() {
        let input = "line1\nline2\nline3";
        let line_numbers = LineNumbers::new(input);

        // Verify the newline indexes (after each newline character)
        assert_eq!(line_numbers.indexes, vec![5, 11]);

        // Test positions at the beginning of lines
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        ); // start of line 1
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(5)
            }
        ); // after \n, on line 2
        assert_eq!(
            line_numbers.get_char_info(12),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(11)
            }
        ); // after \n, on line 3

        // Test positions within lines
        assert_eq!(
            line_numbers.get_char_info(3),
            CharInfo {
                line_number: 0,
                column_number: 3,
                last_newline: None
            }
        ); // middle of line 1
        assert_eq!(
            line_numbers.get_char_info(8),
            CharInfo {
                line_number: 1,
                column_number: 2,
                last_newline: Some(5)
            }
        ); // middle of line 2
        assert_eq!(
            line_numbers.get_char_info(15),
            CharInfo {
                line_number: 2,
                column_number: 3,
                last_newline: Some(11)
            }
        ); // middle of line 3

        // Test positions at newlines
        assert_eq!(
            line_numbers.get_char_info(5),
            CharInfo {
                line_number: 0,
                column_number: 5,
                last_newline: None
            }
        ); // at the \n of line 1
        assert_eq!(
            line_numbers.get_char_info(11),
            CharInfo {
                line_number: 1,
                column_number: 5,
                last_newline: Some(5)
            }
        ); // at the \n of line 2
    }

    #[test]
    fn test_with_empty_lines() {
        let input = "line1\n\nline3";
        let line_numbers = LineNumbers::new(input);

        assert_eq!(line_numbers.indexes, vec![5, 6]);
        assert_eq!(
            line_numbers.get_char_info(4),
            CharInfo {
                line_number: 0,
                column_number: 4,
                last_newline: None
            }
        ); // end of line 1
        assert_eq!(
            line_numbers.get_char_info(5),
            CharInfo {
                line_number: 0,
                column_number: 5,
                last_newline: None
            }
        ); // at \n of line 1
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(5)
            }
        ); // at \n of empty line
        assert_eq!(
            line_numbers.get_char_info(7),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(6)
            }
        ); // on line 3
    }

    #[test]
    fn test_with_multiple_consecutive_newlines() {
        let input = "line1\n\n\nline4";
        let line_numbers = LineNumbers::new(input);

        assert_eq!(line_numbers.indexes, vec![5, 6, 7]);
        assert_eq!(
            line_numbers.get_char_info(5),
            CharInfo {
                line_number: 0,
                column_number: 5,
                last_newline: None
            }
        ); // at \n of line 1
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(5)
            }
        ); // at \n of first empty line
        assert_eq!(
            line_numbers.get_char_info(7),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(6)
            }
        ); // at \n of second empty line
        assert_eq!(
            line_numbers.get_char_info(8),
            CharInfo {
                line_number: 3,
                column_number: 0,
                last_newline: Some(7)
            }
        ); // on line 4
    }

    #[test]
    fn test_trailing_newline() {
        let input = "line1\nline2\n";
        let line_numbers = LineNumbers::new(input);

        assert_eq!(line_numbers.indexes, vec![5, 11]);
        assert_eq!(
            line_numbers.get_char_info(11),
            CharInfo {
                line_number: 1,
                column_number: 5,
                last_newline: Some(5)
            }
        ); // at the last \n (belongs to line 2)
        assert_eq!(
            line_numbers.get_char_info(12),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(11)
            }
        ); // after the last \n
        assert_eq!(
            line_numbers.get_char_info(100),
            CharInfo {
                line_number: 2,
                column_number: 88,
                last_newline: Some(11)
            }
        ); // after the last \n
    }

    #[test]
    fn test_unicode_characters() {
        let input = "こんにちは\n世界";
        let line_numbers = LineNumbers::new(input);

        assert_eq!(line_numbers.indexes, vec![5]);
        assert_eq!(
            line_numbers.get_char_info(3),
            CharInfo {
                line_number: 0,
                column_number: 3,
                last_newline: None
            }
        ); // middle of first line
        assert_eq!(
            line_numbers.get_char_info(4),
            CharInfo {
                line_number: 0,
                column_number: 4,
                last_newline: None
            }
        ); // end of first line
        assert_eq!(
            line_numbers.get_char_info(5),
            CharInfo {
                line_number: 0,
                column_number: 5,
                last_newline: None
            }
        ); // at \n
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(5)
            }
        ); // start of second line
    }

    #[test]
    fn test_large_index() {
        let input = "line1\nline2";
        let line_numbers = LineNumbers::new(input);

        // Even though the input is only 11 characters long, the method should work with larger indices
        assert_eq!(
            line_numbers.get_char_info(10000),
            CharInfo {
                line_number: 1,
                column_number: 9994,
                last_newline: Some(5)
            }
        );
    }

    #[test]
    fn test_line_starting_with_newline() {
        let input = "\nline2\nline3";
        let line_numbers = LineNumbers::new(input);

        assert_eq!(line_numbers.indexes, vec![0, 6]);
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        ); // The first character is a newline
        assert_eq!(
            line_numbers.get_char_info(1),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(0)
            }
        ); // First char of line 2
        assert_eq!(
            line_numbers.get_char_info(3),
            CharInfo {
                line_number: 1,
                column_number: 2,
                last_newline: Some(0)
            }
        ); // Middle of line 2
    }

    #[test]
    fn test_exact_boundary_indices() {
        let input = "line1\nline2\nline3";
        let line_numbers = LineNumbers::new(input);

        // Test boundary indices
        assert_eq!(line_numbers.indexes, vec![5, 11]);
        assert_eq!(
            line_numbers.get_char_info(4),
            CharInfo {
                line_number: 0,
                column_number: 4,
                last_newline: None
            }
        ); // Last character of line 1
        assert_eq!(
            line_numbers.get_char_info(5),
            CharInfo {
                line_number: 0,
                column_number: 5,
                last_newline: None
            }
        ); // Newline character of line 1
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(5)
            }
        ); // First character of line 2
        assert_eq!(
            line_numbers.get_char_info(10),
            CharInfo {
                line_number: 1,
                column_number: 4,
                last_newline: Some(5)
            }
        ); // Last character of line 2
        assert_eq!(
            line_numbers.get_char_info(11),
            CharInfo {
                line_number: 1,
                column_number: 5,
                last_newline: Some(5)
            }
        ); // Newline character of line 2
        assert_eq!(
            line_numbers.get_char_info(12),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(11)
            }
        ); // First character of line 3
    }

    #[test]
    fn test_only_newlines() {
        let input = "\n\n";
        let line_numbers = LineNumbers::new(input);
        // There are two newline characters at positions 0 and 1
        assert_eq!(line_numbers.indexes, vec![0, 1]);
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        ); // at first newline
        assert_eq!(
            line_numbers.get_char_info(1),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(0)
            }
        ); // at second newline
        assert_eq!(
            line_numbers.get_char_info(2),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(1)
            }
        ); // after second newline
        assert_eq!(
            line_numbers.get_char_info(3),
            CharInfo {
                line_number: 2,
                column_number: 1,
                last_newline: Some(1)
            }
        ); // beyond input
    }

    #[test]
    fn test_crlf_line_endings() {
        let input = "line1\r\nline2\r\nline3";
        let line_numbers = LineNumbers::new(input);
        // CRLF yields newline positions at the '\n' characters
        // 'line1\r\n' => newline at index 6, 'line2\r\n' => newline at index 13
        assert_eq!(line_numbers.indexes, vec![6, 13]);
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 0,
                column_number: 6,
                last_newline: None
            }
        ); // at first '\n'
        assert_eq!(
            line_numbers.get_char_info(7),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(6)
            }
        ); // after first '\n'
        assert_eq!(
            line_numbers.get_char_info(13),
            CharInfo {
                line_number: 1,
                column_number: 6,
                last_newline: Some(6)
            }
        ); // at second '\n'
        assert_eq!(
            line_numbers.get_char_info(14),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(13)
            }
        ); // after second '\n'
    }

    #[test]
    fn test_carriage_returns_only_not_supported() {
        let input = "line1\rline2\rline3";
        let line_numbers = LineNumbers::new(input);
        // No \n present, so no newlines recorded
        assert!(line_numbers.indexes.is_empty());
        // All indices should map to line 0
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        );
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 0,
                column_number: 6,
                last_newline: None
            }
        );
        assert_eq!(
            line_numbers.get_char_info(100),
            CharInfo {
                line_number: 0,
                column_number: 100,
                last_newline: None
            }
        );
    }

    #[test]
    fn test_emoji_characters() {
        let input = "😀\n😃\n😄";
        let line_numbers = LineNumbers::new(input);
        // Newline characters at char indices 1 and 3
        assert_eq!(line_numbers.indexes, vec![1, 3]);
        // Test mapping
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        ); // before first newline
        assert_eq!(
            line_numbers.get_char_info(1),
            CharInfo {
                line_number: 0,
                column_number: 1,
                last_newline: None
            }
        ); // at first newline
        assert_eq!(
            line_numbers.get_char_info(2),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(1)
            }
        ); // after first newline
        assert_eq!(
            line_numbers.get_char_info(3),
            CharInfo {
                line_number: 1,
                column_number: 1,
                last_newline: Some(1)
            }
        ); // at second newline
        assert_eq!(
            line_numbers.get_char_info(4),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(3)
            }
        ); // after second newline
    }

    #[test]
    fn test_crlf_trailing_newline() {
        let input = "line1\r\n";
        let line_numbers = LineNumbers::new(input);
        // Newline at '\n' index 6
        assert_eq!(line_numbers.indexes, vec![6]);
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 0,
                column_number: 6,
                last_newline: None
            }
        ); // at '\n'
        assert_eq!(
            line_numbers.get_char_info(7),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(6)
            }
        ); // after '\n'
        assert_eq!(
            line_numbers.get_char_info(100),
            CharInfo {
                line_number: 1,
                column_number: 93,
                last_newline: Some(6)
            }
        );
    }

    #[test]
    fn test_only_crlfs() {
        // Input composed solely of CRLF sequences
        let input = "\r\n\r\n";
        let line_numbers = LineNumbers::new(input);
        // CRLF at indices 1 and 3
        assert_eq!(line_numbers.indexes, vec![1, 3]);
        // Test mapping
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        ); // before first \n
        assert_eq!(
            line_numbers.get_char_info(1),
            CharInfo {
                line_number: 0,
                column_number: 1,
                last_newline: None
            }
        ); // at first \n
        assert_eq!(
            line_numbers.get_char_info(2),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(1)
            }
        ); // between \n and next CR
        assert_eq!(
            line_numbers.get_char_info(3),
            CharInfo {
                line_number: 1,
                column_number: 1,
                last_newline: Some(1)
            }
        ); // at second \n
        assert_eq!(
            line_numbers.get_char_info(4),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(3)
            }
        ); // after all newlines
    }

    #[test]
    fn test_line_starting_with_crlf() {
        // Input starting with a CRLF
        let input = "\r\nfirst\r\nsecond";
        let line_numbers = LineNumbers::new(input);
        // CRLF at indices of '\n' chars: 1 and 8
        assert_eq!(line_numbers.indexes, vec![1, 8]);
        // Test mapping
        assert_eq!(
            line_numbers.get_char_info(0),
            CharInfo {
                line_number: 0,
                column_number: 0,
                last_newline: None
            }
        ); // at CR
        assert_eq!(
            line_numbers.get_char_info(1),
            CharInfo {
                line_number: 0,
                column_number: 1,
                last_newline: None
            }
        ); // at LF
        assert_eq!(
            line_numbers.get_char_info(2),
            CharInfo {
                line_number: 1,
                column_number: 0,
                last_newline: Some(1)
            }
        ); // 'f'
        assert_eq!(
            line_numbers.get_char_info(6),
            CharInfo {
                line_number: 1,
                column_number: 4,
                last_newline: Some(1)
            }
        ); // 't' of 'first'
        assert_eq!(
            line_numbers.get_char_info(7),
            CharInfo {
                line_number: 1,
                column_number: 5,
                last_newline: Some(1)
            }
        ); // before second LF
        assert_eq!(
            line_numbers.get_char_info(8),
            CharInfo {
                line_number: 1,
                column_number: 6,
                last_newline: Some(1)
            }
        ); // at second LF
        assert_eq!(
            line_numbers.get_char_info(9),
            CharInfo {
                line_number: 2,
                column_number: 0,
                last_newline: Some(8)
            }
        ); // 's' of 'second'
    }
}
