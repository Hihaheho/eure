use alloc::string::String;
use alloc::string::ToString;
use core::{
    fmt::{self, Display},
    str::FromStr,
};
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
            Ok(Identifier(matches.as_str().to_string()))
        } else {
            Err(IdentifierError::InvalidChar {
                at: matches.end(),
                invalid_char: s.chars().nth(matches.end()).unwrap(),
            })
        }
    }
}

#[cfg(feature = "std")]
impl FromStr for Identifier {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IDENTIFIER_PARSER.parse(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(String);

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

impl Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "std"))]
    impl FromStr for Identifier {
        type Err = IdentifierError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let parser = IdentifierParser::init();
            parser.parse(s)
        }
    }

    #[test]
    fn test_identifier() {
        assert_eq!(
            Identifier::from_str("hello"),
            Ok(Identifier("hello".to_string()))
        );
    }
    #[test]
    fn test_identifier_with_hyphen() {
        assert_eq!(
            Identifier::from_str("hello-world"),
            Ok(Identifier("hello-world".to_string()))
        );
    }

    #[test]
    fn test_identifier_おーい() {
        assert_eq!(
            Identifier::from_str("おーい"),
            Ok(Identifier("おーい".to_string()))
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
}
