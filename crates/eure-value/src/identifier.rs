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
            Err(IdentifierError::InvalidChar {
                at: matches.end(),
                invalid_char: s.chars().nth(matches.end()).unwrap(),
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
    /// # Safety
    /// The caller must ensure that the string is a valid identifier according to Eure rules:
    /// - Must start with XID_Start character or underscore
    /// - Can contain XID_Continue characters or hyphens
    /// - Must not start with $
    pub const unsafe fn new_unchecked(s: &'static str) -> Self {
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
        const TEST_ID: Identifier = unsafe { Identifier::new_unchecked("test-const") };
        assert_eq!(TEST_ID.as_ref(), "test-const");

        // Verify it's using borrowed variant
        let id = unsafe { Identifier::new_unchecked("borrowed") };
        assert_eq!(id.as_ref(), "borrowed");
    }
}
