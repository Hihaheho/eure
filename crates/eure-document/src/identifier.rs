use alloc::borrow::Cow;
use alloc::string::String;
use alloc::string::ToString;
use core::fmt::{self, Display};
use regex::Regex;
use thiserror::Error;

#[cfg(feature = "std")]
static IDENTIFIER_PARSER: std::sync::LazyLock<IdentifierParser> =
    std::sync::LazyLock::new(IdentifierParser::init);

/// A parser and factory API for identifiers intended for no_std environments.
/// Prefer using `Identifier::from_str` and `.parse()` methods if you are using `std`.
pub struct IdentifierParser(Regex);

impl IdentifierParser {
    /// Initialize the parser. This internally compiles a regex, so don't call this in a hot path.
    /// Prefer using `FromStr` impl for `Identifier` if you are using `std`.
    pub fn init() -> Self {
        Self(Regex::new(r"^[\p{XID_Start}_][\p{XID_Continue}-]*").unwrap())
    }

    pub fn parse(&self, s: &str) -> Result<Identifier, IdentifierError> {
        // Check if starts with $ (would be parsed as extension)
        if s.starts_with('$') {
            return Err(IdentifierError::InvalidChar {
                at: 0,
                invalid_char: '$',
            });
        }

        let Some(matches) = self.0.find(s) else {
            if let Some(c) = s.chars().next() {
                return Err(IdentifierError::InvalidChar {
                    at: 0,
                    invalid_char: c,
                });
            } else {
                return Err(IdentifierError::Empty);
            }
        };
        if matches.len() == s.len() {
            Ok(Identifier(Cow::Owned(matches.as_str().to_string())))
        } else {
            // matches.end() is a byte index, but we need a character index for the error.
            // Count how many characters are in the matched portion.
            let char_index = matches.as_str().chars().count();
            // Get the invalid character from the remainder of the string.
            let invalid_char = s[matches.end()..].chars().next().unwrap();
            Err(IdentifierError::InvalidChar {
                at: char_index,
                invalid_char,
            })
        }
    }
}

impl core::str::FromStr for Identifier {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[cfg(feature = "std")]
        {
            IDENTIFIER_PARSER.parse(s)
        }
        #[cfg(not(feature = "std"))]
        {
            IdentifierParser::init().parse(s)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(Cow<'static, str>);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentifierError {
    #[error("Empty identifier")]
    Empty,
    #[error("Invalid character for identifier: {invalid_char} at {at}")]
    InvalidChar {
        /// the problem index of the identifier in the string
        at: usize,
        /// the invalid character
        invalid_char: char,
    },
}

impl Identifier {
    /// Creates a new Identifier without validation.
    ///
    /// This function is intended for creating compile-time constants where the
    /// identifier string is known to be valid. The caller should ensure that the
    /// string is a valid identifier according to Eure rules:
    /// - Must start with XID_Start character or underscore
    /// - Can contain XID_Continue characters or hyphens
    /// - Must not start with $
    ///
    /// Note: This function is not marked `unsafe` because passing an invalid string
    /// does not cause memory unsafety - it only results in a logically invalid Identifier.
    pub const fn new_unchecked(s: &'static str) -> Self {
        Identifier(Cow::Borrowed(s))
    }

    pub fn into_string(self) -> String {
        self.0.into()
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;

    #[test]
    fn test_identifier() {
        assert_eq!(
            Identifier::from_str("hello"),
            Ok(Identifier(Cow::Owned("hello".to_string())))
        );
    }
    #[test]
    fn test_identifier_with_hyphen() {
        assert_eq!(
            Identifier::from_str("hello-world"),
            Ok(Identifier(Cow::Owned("hello-world".to_string())))
        );
    }

    #[test]
    fn test_identifier_おーい() {
        assert_eq!(
            Identifier::from_str("おーい"),
            Ok(Identifier(Cow::Owned("おーい".to_string())))
        );
    }

    #[test]
    fn test_identifier_error() {
        assert_eq!(
            Identifier::from_str("invalid identifier"),
            Err(IdentifierError::InvalidChar {
                at: 7,
                invalid_char: ' ',
            })
        );
    }

    #[test]
    fn test_identifier_invalid_first_char() {
        assert_eq!(
            Identifier::from_str("1hello"),
            Err(IdentifierError::InvalidChar {
                at: 0,
                invalid_char: '1',
            })
        );
    }

    #[test]
    fn test_identifier_error_empty() {
        assert_eq!(Identifier::from_str(""), Err(IdentifierError::Empty));
    }

    #[test]
    fn test_identifier_accept_literals() {
        assert_eq!(
            Identifier::from_str("true"),
            Ok(Identifier(Cow::Owned("true".to_string())))
        );
        assert_eq!(
            Identifier::from_str("false"),
            Ok(Identifier(Cow::Owned("false".to_string())))
        );
        assert_eq!(
            Identifier::from_str("null"),
            Ok(Identifier(Cow::Owned("null".to_string())))
        );
    }

    #[test]
    fn test_identifier_reject_dollar_prefix() {
        assert_eq!(
            Identifier::from_str("$id"),
            Err(IdentifierError::InvalidChar {
                at: 0,
                invalid_char: '$'
            })
        );
    }

    #[test]
    fn test_identifier_new_unchecked() {
        // This test verifies that const construction works
        const TEST_ID: Identifier = Identifier::new_unchecked("test-const");
        assert_eq!(TEST_ID.as_ref(), "test-const");

        // Verify it's using borrowed variant
        let id = Identifier::new_unchecked("borrowed");
        assert_eq!(id.as_ref(), "borrowed");
    }

    #[test]
    fn test_empty_string_returns_empty_error() {
        let result = Identifier::from_str("");
        assert_eq!(result, Err(IdentifierError::Empty));
    }
}

#[cfg(test)]
mod proptests {
    extern crate std;

    use super::*;
    use core::str::FromStr;
    use proptest::prelude::*;
    use std::format;
    use std::string::String;
    use std::vec;

    /// Characters valid as the first character of an identifier (XID_Start or underscore).
    /// We use a representative subset of XID_Start for efficiency.
    fn xid_start_char() -> impl Strategy<Value = char> {
        prop_oneof![
            // ASCII letters
            prop::char::range('a', 'z'),
            prop::char::range('A', 'Z'),
            // Underscore (explicitly allowed)
            Just('_'),
            // Some Unicode XID_Start characters
            Just('α'), // Greek
            Just('β'),
            Just('お'), // Japanese hiragana
            Just('日'), // CJK
            Just('é'),  // Latin extended
            Just('ñ'),
        ]
    }

    /// Characters valid in the continuation of an identifier (XID_Continue or hyphen).
    fn xid_continue_char() -> impl Strategy<Value = char> {
        prop_oneof![
            // ASCII letters and digits
            prop::char::range('a', 'z'),
            prop::char::range('A', 'Z'),
            prop::char::range('0', '9'),
            // Underscore and hyphen
            Just('_'),
            Just('-'),
            // Some Unicode XID_Continue characters
            Just('α'),
            Just('β'),
            Just('ー'), // Japanese prolonged sound mark (XID_Continue)
            Just('日'),
            Just('é'),
        ]
    }

    /// Strategy to generate valid identifiers.
    fn valid_identifier() -> impl Strategy<Value = String> {
        (
            xid_start_char(),
            proptest::collection::vec(xid_continue_char(), 0..20),
        )
            .prop_map(|(first, rest)| {
                let mut s = String::with_capacity(1 + rest.len());
                s.push(first);
                s.extend(rest);
                s
            })
    }

    /// Characters that are invalid as the first character of an identifier.
    fn invalid_first_char() -> impl Strategy<Value = char> {
        prop_oneof![
            // Digits
            prop::char::range('0', '9'),
            // Dollar sign (reserved for extensions)
            Just('$'),
            // Common invalid punctuation
            Just(' '),
            Just('\t'),
            Just('\n'),
            Just('.'),
            Just(','),
            Just('!'),
            Just('@'),
            Just('#'),
            Just('%'),
            Just('^'),
            Just('&'),
            Just('*'),
            Just('('),
            Just(')'),
            Just('='),
            Just('+'),
            Just('['),
            Just(']'),
            Just('{'),
            Just('}'),
            Just('|'),
            Just('\\'),
            Just('/'),
            Just('<'),
            Just('>'),
            Just('?'),
            Just(':'),
            Just(';'),
            Just('"'),
            Just('\''),
        ]
    }

    /// Characters that are invalid in the continuation of an identifier.
    fn invalid_continue_char() -> impl Strategy<Value = char> {
        prop_oneof![
            // Common invalid characters
            Just(' '),
            Just('\t'),
            Just('\n'),
            Just('.'),
            Just(','),
            Just('!'),
            Just('@'),
            Just('#'),
            Just('$'),
            Just('%'),
            Just('^'),
            Just('&'),
            Just('*'),
            Just('('),
            Just(')'),
            Just('='),
            Just('+'),
            Just('['),
            Just(']'),
            Just('{'),
            Just('}'),
            Just('|'),
            Just('\\'),
            Just('/'),
            Just('<'),
            Just('>'),
            Just('?'),
            Just(':'),
            Just(';'),
            Just('"'),
            Just('\''),
        ]
    }

    proptest! {
        /// Valid identifiers should always parse successfully.
        #[test]
        fn valid_identifiers_parse_successfully(s in valid_identifier()) {
            let result = Identifier::from_str(&s);
            prop_assert!(result.is_ok(), "Failed to parse valid identifier: {:?}", s);
        }

        /// Parsed identifiers should round-trip correctly (parse -> to_string -> parse).
        #[test]
        fn round_trip_stability(s in valid_identifier()) {
            let id1 = Identifier::from_str(&s).expect("should parse");
            let string_repr = id1.to_string();
            let id2 = Identifier::from_str(&string_repr).expect("should re-parse");
            prop_assert_eq!(id1.as_ref(), id2.as_ref(), "Round-trip failed for: {:?}", s);
        }

        /// Identifiers starting with invalid characters should be rejected with error at position 0.
        #[test]
        fn invalid_first_char_rejected(
            first in invalid_first_char(),
            rest in proptest::collection::vec(xid_continue_char(), 0..10)
        ) {
            let mut s = String::with_capacity(1 + rest.len());
            s.push(first);
            s.extend(rest);

            let result = Identifier::from_str(&s);
            prop_assert!(result.is_err(), "Should reject invalid first char: {:?}", s);

            if let Err(IdentifierError::InvalidChar { at, invalid_char }) = result {
                prop_assert_eq!(at, 0, "Error position should be 0 for invalid first char");
                prop_assert_eq!(invalid_char, first, "Reported char should match first char");
            } else {
                prop_assert!(false, "Expected InvalidChar error, got {:?}", result);
            }
        }

        /// Identifiers with invalid characters in the middle should be rejected at the correct position.
        #[test]
        fn invalid_middle_char_rejected(
            prefix_len in 1usize..10,
            invalid in invalid_continue_char()
        ) {
            // Build a valid prefix
            let prefix: String = (0..prefix_len)
                .map(|i| if i == 0 { 'a' } else { 'b' })
                .collect();

            let mut s = prefix.clone();
            s.push(invalid);
            s.push_str("suffix");

            let result = Identifier::from_str(&s);
            prop_assert!(result.is_err(), "Should reject invalid middle char: {:?}", s);

            if let Err(IdentifierError::InvalidChar { at, invalid_char }) = result {
                prop_assert_eq!(at, prefix_len, "Error position should be at the invalid char position");
                prop_assert_eq!(invalid_char, invalid, "Reported char should match invalid char");
            } else {
                prop_assert!(false, "Expected InvalidChar error, got {:?}", result);
            }
        }

        /// Dollar prefix should always be rejected with InvalidChar at position 0.
        #[test]
        fn dollar_prefix_always_rejected(rest in "[a-zA-Z0-9_-]*") {
            let s = format!("${}", rest);
            let result = Identifier::from_str(&s);

            match result {
                Err(IdentifierError::InvalidChar { at: 0, invalid_char: '$' }) => {
                    // Expected
                }
                _ => {
                    prop_assert!(false, "Dollar prefix should return InvalidChar at 0, got {:?}", result);
                }
            }
        }

        /// For InvalidChar errors, the position should always be within character bounds.
        /// Note: `at` is a character index (not byte index), so we compare against chars().count().
        #[test]
        fn error_position_within_bounds(s in ".+") {
            if let Err(IdentifierError::InvalidChar { at, invalid_char }) = Identifier::from_str(&s) {
                let char_count = s.chars().count();
                prop_assert!(at < char_count, "Error position {} out of bounds for string with {} chars", at, char_count);
                // Verify the character at that position matches
                let actual_char = s.chars().nth(at);
                prop_assert_eq!(actual_char, Some(invalid_char), "Char at position {} should match reported char", at);
            }
        }

        /// AsRef<str> should return the same string that was parsed.
        #[test]
        fn as_ref_returns_original_string(s in valid_identifier()) {
            let id = Identifier::from_str(&s).expect("should parse");
            prop_assert_eq!(id.as_ref(), s.as_str());
        }

        /// Display implementation should match AsRef<str>.
        #[test]
        fn display_matches_as_ref(s in valid_identifier()) {
            let id = Identifier::from_str(&s).expect("should parse");
            prop_assert_eq!(id.to_string(), id.as_ref());
        }
    }
}
