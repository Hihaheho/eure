//! VariantPath type for nested union variant paths.

extern crate alloc;

use alloc::fmt;
use alloc::vec::Vec;
use core::str::FromStr;

use crate::identifier::{Identifier, IdentifierError};

/// A path through nested union variants (e.g., "ok.some.left" -> [ok, some, left]).
///
/// Used for parsing and validating nested unions with `$variant` extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantPath(Vec<Identifier>);

impl VariantPath {
    /// Create a new VariantPath from a vector of identifiers.
    pub fn new(segments: Vec<Identifier>) -> Self {
        Self(segments)
    }

    /// Create an empty variant path.
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Parse a variant path from a dotted string (e.g., "ok.some.left").
    pub fn parse(s: &str) -> Result<Self, IdentifierError> {
        let segments: Result<Vec<_>, _> =
            s.split('.').map(|seg| seg.parse::<Identifier>()).collect();
        segments.map(Self)
    }

    /// Get the first segment.
    pub fn first(&self) -> Option<&Identifier> {
        self.0.first()
    }

    /// Get the remaining path after the first segment.
    pub fn rest(&self) -> Option<Self> {
        if self.0.len() > 1 {
            Some(Self(self.0[1..].to_vec()))
        } else {
            None
        }
    }

    /// Check if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check if this is a single-segment path.
    pub fn is_single(&self) -> bool {
        self.0.len() == 1
    }

    /// Get the number of segments in the path.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get the segments as a slice.
    pub fn segments(&self) -> &[Identifier] {
        &self.0
    }
}

impl FromStr for VariantPath {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for VariantPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for seg in &self.0 {
            if !first {
                write!(f, ".")?;
            }
            write!(f, "{}", seg)?;
            first = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_parse_single() {
        let path: VariantPath = "ok".parse().unwrap();
        assert!(path.is_single());
        assert_eq!(path.first().unwrap().as_ref(), "ok");
        assert!(path.rest().is_none());
    }

    #[test]
    fn test_parse_nested() {
        let path: VariantPath = "ok.some.left".parse().unwrap();
        assert!(!path.is_single());
        assert_eq!(path.len(), 3);
        assert_eq!(path.first().unwrap().as_ref(), "ok");

        let rest = path.rest().unwrap();
        assert_eq!(rest.len(), 2);
        assert_eq!(rest.first().unwrap().as_ref(), "some");

        let rest2 = rest.rest().unwrap();
        assert!(rest2.is_single());
        assert_eq!(rest2.first().unwrap().as_ref(), "left");
        assert!(rest2.rest().is_none());
    }

    #[test]
    fn test_display() {
        let path: VariantPath = "ok.some.left".parse().unwrap();
        assert_eq!(path.to_string(), "ok.some.left");
    }

    #[test]
    fn test_invalid_identifier() {
        let result = VariantPath::parse("ok.123invalid");
        assert!(result.is_err());
    }
}
