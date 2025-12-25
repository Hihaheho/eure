//! Reference extraction from markdown content

use regex::Regex;
use std::sync::LazyLock;

use crate::error::ReferenceType;

/// A reference found in markdown content with position information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// Type of reference
    pub ref_type: ReferenceType,
    /// The key being referenced
    pub key: String,
    /// Byte offset of the reference start in the content
    pub offset: u32,
    /// Byte length of the entire reference string (e.g., "!cite[key]")
    pub len: u32,
}

/// Regex patterns for extracting references
static CITE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!cite\[([^\]]+)\]").expect("invalid cite regex"));

static FOOTNOTE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!footnote\[([^\]]+)\]").expect("invalid footnote regex"));

static REF_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!ref\[([^\]]+)\]").expect("invalid ref regex"));

/// Extract all references from markdown content with positions
pub fn extract_references(content: &str) -> Vec<Reference> {
    let mut refs = Vec::new();

    // Extract !cite[key] references (can have multiple keys separated by comma)
    for cap in CITE_PATTERN.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let keys = &cap[1];

        // For comma-separated keys, each key gets the same span (the whole !cite[...] match)
        for key in keys.split(',') {
            let trimmed = key.trim();
            refs.push(Reference {
                ref_type: ReferenceType::Cite,
                key: trimmed.to_string(),
                // For the whole !cite[...] match
                offset: full_match.start() as u32,
                len: full_match.len() as u32,
            });
        }
    }

    // Extract !footnote[key] references
    for cap in FOOTNOTE_PATTERN.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        refs.push(Reference {
            ref_type: ReferenceType::Footnote,
            key: cap[1].to_string(),
            offset: full_match.start() as u32,
            len: full_match.len() as u32,
        });
    }

    // Extract !ref[key] references
    for cap in REF_PATTERN.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        refs.push(Reference {
            ref_type: ReferenceType::Section,
            key: cap[1].to_string(),
            offset: full_match.start() as u32,
            len: full_match.len() as u32,
        });
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cite() {
        let refs = extract_references("See !cite[knuth1984] for details.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Cite);
        assert_eq!(refs[0].key, "knuth1984");
        assert_eq!(refs[0].offset, 4); // "See " = 4 chars
        assert_eq!(refs[0].len, 16); // "!cite[knuth1984]" = 16 chars
    }

    #[test]
    fn test_extract_multiple_cites() {
        let refs = extract_references("See !cite[knuth1984, lamport1994] for details.");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].key, "knuth1984");
        assert_eq!(refs[1].key, "lamport1994");
    }

    #[test]
    fn test_extract_footnote() {
        let refs = extract_references("This is important!footnote[note1].");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Footnote);
        assert_eq!(refs[0].key, "note1");
    }

    #[test]
    fn test_extract_ref() {
        let refs = extract_references("See !ref[intro] for more.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Section);
        assert_eq!(refs[0].key, "intro");
    }

    #[test]
    fn test_extract_mixed() {
        let content = "See !cite[knuth1984] and !footnote[note1]. Also !ref[intro].";
        let refs = extract_references(content);
        assert_eq!(refs.len(), 3);
    }
}
