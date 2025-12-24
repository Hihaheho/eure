//! Reference checking for eumd documents

use std::collections::HashSet;

use eure_document::document::{EureDocument, NodeId};
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

/// Check all references in a document (basic version without spans)
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
    check_content_simple(
        doc.description.as_deref(),
        "in description",
        &cite_keys,
        &footnote_keys,
        &section_keys,
        &mut result,
    );

    check_content_simple(
        doc.intro.as_deref(),
        "in intro",
        &cite_keys,
        &footnote_keys,
        &section_keys,
        &mut result,
    );

    // Check sections recursively
    check_sections_simple(
        &doc.sections,
        "",
        &cite_keys,
        &footnote_keys,
        &section_keys,
        &mut result,
    );

    // Check footnote content
    for (key, footnote) in doc.footnotes.iter() {
        check_content_simple(
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

fn check_content_simple(
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
            result.errors.push(ReferenceError::new(
                reference.ref_type,
                reference.key,
                location.to_string(),
            ));
        }
    }
}

fn check_sections_simple(
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
        check_content_simple(
            section.header.as_deref(),
            &format!("in section '{current_path}' header"),
            cite_keys,
            footnote_keys,
            section_keys,
            result,
        );

        // Check body
        check_content_simple(
            section.body.as_deref(),
            &format!("in section '{current_path}'"),
            cite_keys,
            footnote_keys,
            section_keys,
            result,
        );

        // Recurse into nested sections
        check_sections_simple(
            &section.sections,
            &current_path,
            cite_keys,
            footnote_keys,
            section_keys,
            result,
        );
    }
}

// ============================================================================
// Advanced checking with span information
// ============================================================================

/// Context for checking with span information
struct CheckContext<'a> {
    raw_doc: &'a EureDocument,
    cite_keys: HashSet<String>,
    footnote_keys: HashSet<String>,
    section_keys: HashSet<String>,
    result: CheckResult,
}

impl<'a> CheckContext<'a> {
    fn new(eumd_doc: &EumdDocument, raw_doc: &'a EureDocument) -> Self {
        let cite_keys: HashSet<String> = eumd_doc
            .cites
            .as_ref()
            .map(|c| extract_bibtex_keys(c))
            .unwrap_or_default();

        let footnote_keys: HashSet<String> =
            eumd_doc.footnotes.iter().map(|(k, _)| k.clone()).collect();

        let mut section_keys = HashSet::new();
        collect_section_keys(&eumd_doc.sections, &mut section_keys);

        CheckContext {
            raw_doc,
            cite_keys,
            footnote_keys,
            section_keys,
            result: CheckResult::default(),
        }
    }

    fn check_content(&mut self, content: &str, location: &str, node_id: NodeId) {
        // Get the actual text content offset within the code block
        let content_offset = get_code_block_content_offset(self.raw_doc, node_id);

        for reference in extract_references(content) {
            let is_valid = match reference.ref_type {
                ReferenceType::Cite => self.cite_keys.contains(&reference.key),
                ReferenceType::Footnote => self.footnote_keys.contains(&reference.key),
                ReferenceType::Section => self.section_keys.contains(&reference.key),
            };

            if !is_valid {
                self.result.errors.push(ReferenceError::with_span(
                    reference.ref_type,
                    reference.key,
                    location.to_string(),
                    node_id,
                    content_offset + reference.offset,
                    reference.len,
                ));
            }
        }
    }

    fn check_sections(
        &mut self,
        sections: &Map<String, Section>,
        path: &str,
        sections_node_id: NodeId,
    ) {
        let sections_node = self.raw_doc.node(sections_node_id);
        let Some(sections_map) = sections_node.as_map() else {
            return;
        };

        for (key, section) in sections.iter() {
            let current_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };

            let Some(section_node_id) = sections_map.get_node_id(&key.clone().into()) else {
                continue;
            };

            let section_node = self.raw_doc.node(section_node_id);
            let Some(section_map) = section_node.as_map() else {
                continue;
            };

            // Check header if present
            if let Some(ref header) = section.header
                && let Some(header_node_id) = section_map.get_node_id(&"header".into())
            {
                self.check_content(
                    header,
                    &format!("in section '{current_path}' header"),
                    header_node_id,
                );
            }

            // Check body
            if let Some(ref body) = section.body
                && let Some(body_node_id) = section_map.get_node_id(&"body".into())
            {
                self.check_content(body, &format!("in section '{current_path}'"), body_node_id);
            }

            // Recurse into nested sections
            if let Some(nested_sections_id) = section_map.get_node_id(&"sections".into()) {
                self.check_sections(&section.sections, &current_path, nested_sections_id);
            }
        }
    }
}

/// Get the byte offset of the code block content start within the node
fn get_code_block_content_offset(_raw_doc: &EureDocument, _node_id: NodeId) -> u32 {
    // For code blocks, we need to account for the opening ``` and language tag
    // However, since we're using the node's span which points to the content,
    // we can return 0 here. The actual offset calculation happens in report.rs
    // when we compute the final span using OriginMap.
    0
}

/// Check references with span information for better error reporting
pub fn check_references_with_spans(eumd_doc: &EumdDocument, raw_doc: &EureDocument) -> CheckResult {
    let mut ctx = CheckContext::new(eumd_doc, raw_doc);

    let root_id = raw_doc.get_root_id();
    let root = raw_doc.node(root_id);

    let Some(map) = root.as_map() else {
        return ctx.result;
    };

    // Check description
    if let Some(ref content) = eumd_doc.description
        && let Some(node_id) = map.get_node_id(&"description".into())
    {
        ctx.check_content(content, "in description", node_id);
    }

    // Check intro
    if let Some(ref content) = eumd_doc.intro
        && let Some(node_id) = map.get_node_id(&"intro".into())
    {
        ctx.check_content(content, "in intro", node_id);
    }

    // Check sections recursively
    if let Some(sections_node_id) = map.get_node_id(&"sections".into()) {
        ctx.check_sections(&eumd_doc.sections, "", sections_node_id);
    }

    // Check footnotes
    if let Some(footnotes_node_id) = map.get_node_id(&"footnotes".into()) {
        let footnotes_node = raw_doc.node(footnotes_node_id);
        if let Some(footnotes_map) = footnotes_node.as_map() {
            for (key, footnote) in eumd_doc.footnotes.iter() {
                if let Some(footnote_node_id) = footnotes_map.get_node_id(&key.clone().into())
                    && let Some(content_node_id) = raw_doc
                        .node(footnote_node_id)
                        .as_map()
                        .and_then(|m| m.get_node_id(&"content".into()))
                {
                    ctx.check_content(
                        &footnote.content,
                        &format!("in footnote '{key}'"),
                        content_node_id,
                    );
                }
            }
        }
    }

    ctx.result
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
