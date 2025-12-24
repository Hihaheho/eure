//! Reference checking for eumd documents

use std::collections::HashSet;

use eure_document::map::Map;
use regex::Regex;
use std::sync::LazyLock;

use crate::document::{EumdDocument, Section};
use crate::error::{ReferenceError, ReferenceType};
use crate::reference::extract_references;

/// Result of reference checking
#[derive(Debug, Default)]
pub struct CheckResult {
    /// List of reference errors
    pub errors: Vec<ReferenceError>,
}

impl CheckResult {
    /// Returns true if there are no errors
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Regex to extract BibTeX entry keys
static BIBTEX_ENTRY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@\w+\{([^,\s]+)").expect("invalid bibtex entry regex"));

/// Extract citation keys from BibTeX content
fn extract_bibtex_keys(bibtex: &str) -> HashSet<String> {
    BIBTEX_ENTRY_PATTERN
        .captures_iter(bibtex)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Collect all section keys recursively
fn collect_section_keys(sections: &Map<String, Section>, keys: &mut HashSet<String>) {
    for (key, section) in sections.iter() {
        keys.insert(key.clone());
        collect_section_keys(&section.sections, keys);
    }
}

/// Check all references in a document
pub fn check_references(doc: &EumdDocument) -> CheckResult {
    let mut result = CheckResult::default();

    // Collect available keys
    let cite_keys: HashSet<String> = doc
        .cites
        .as_ref()
        .map(|c| extract_bibtex_keys(c))
        .unwrap_or_default();

    let footnote_keys: HashSet<String> = doc.footnotes.iter().map(|(k, _)| k.clone()).collect();

    let mut section_keys = HashSet::new();
    collect_section_keys(&doc.sections, &mut section_keys);

    // Check references in all markdown content
    check_content(
        doc.description.as_deref(),
        "in description",
        &cite_keys,
        &footnote_keys,
        &section_keys,
        &mut result,
    );

    check_content(
        doc.intro.as_deref(),
        "in intro",
        &cite_keys,
        &footnote_keys,
        &section_keys,
        &mut result,
    );

    // Check sections recursively
    check_sections(
        &doc.sections,
        "",
        &cite_keys,
        &footnote_keys,
        &section_keys,
        &mut result,
    );

    // Check footnote content
    for (key, footnote) in doc.footnotes.iter() {
        check_content(
            Some(&footnote.content),
            &format!("in footnote '{key}'"),
            &cite_keys,
            &footnote_keys,
            &section_keys,
            &mut result,
        );
    }

    result
}

fn check_content(
    content: Option<&str>,
    location: &str,
    cite_keys: &HashSet<String>,
    footnote_keys: &HashSet<String>,
    section_keys: &HashSet<String>,
    result: &mut CheckResult,
) {
    let Some(content) = content else { return };

    for reference in extract_references(content) {
        let is_valid = match reference.ref_type {
            ReferenceType::Cite => cite_keys.contains(&reference.key),
            ReferenceType::Footnote => footnote_keys.contains(&reference.key),
            ReferenceType::Section => section_keys.contains(&reference.key),
        };

        if !is_valid {
            result.errors.push(ReferenceError {
                ref_type: reference.ref_type,
                key: reference.key,
                location: location.to_string(),
            });
        }
    }
}

fn check_sections(
    sections: &Map<String, Section>,
    path: &str,
    cite_keys: &HashSet<String>,
    footnote_keys: &HashSet<String>,
    section_keys: &HashSet<String>,
    result: &mut CheckResult,
) {
    for (key, section) in sections.iter() {
        let current_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };

        // Check header if present
        check_content(
            section.header.as_deref(),
            &format!("in section '{current_path}' header"),
            cite_keys,
            footnote_keys,
            section_keys,
            result,
        );

        // Check body
        check_content(
            section.body.as_deref(),
            &format!("in section '{current_path}'"),
            cite_keys,
            footnote_keys,
            section_keys,
            result,
        );

        // Recurse into nested sections
        check_sections(
            &section.sections,
            &current_path,
            cite_keys,
            footnote_keys,
            section_keys,
            result,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bibtex_keys() {
        let bibtex = r#"
@article{knuth1984,
  author = "Donald Knuth",
  title = "Literate Programming"
}

@book{lamport1994,
  author = "Leslie Lamport"
}
"#;
        let keys = extract_bibtex_keys(bibtex);
        assert!(keys.contains("knuth1984"));
        assert!(keys.contains("lamport1994"));
        assert_eq!(keys.len(), 2);
    }
}
