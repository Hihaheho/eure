//! Eumd document data structures

use eure::FromEure;
use eure_document::map::Map;

/// Root document structure for `.eumd` files
#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct EumdDocument {
    /// Schema reference (extension field)
    #[eure(ext, default)]
    pub schema: Option<String>,

    /// Document title (inline markdown)
    pub title: String,

    /// Document authors
    #[eure(default)]
    pub authors: Vec<Author>,

    /// Publication/creation date
    #[eure(default)]
    pub date: Option<String>,

    /// Categorization tags
    #[eure(default)]
    pub tags: Vec<String>,

    /// Draft status
    #[eure(default)]
    pub draft: Option<bool>,

    /// Document description/abstract (block markdown)
    #[eure(default)]
    pub description: Option<String>,

    /// Bibliography in BibTeX format
    #[eure(default)]
    pub cites: Option<String>,

    /// Footnote definitions
    #[eure(default)]
    pub footnotes: Map<String, Footnote>,

    /// Introduction/preamble before first section
    #[eure(default)]
    pub intro: Option<String>,

    /// Document sections (map with ID as key)
    #[eure(default)]
    pub sections: Map<String, Section>,
}

/// Author information
#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub enum Author {
    /// Detailed author information (try first due to more specific shape)
    Detailed(DetailedAuthor),

    /// Simple author name
    Simple(String),
}

/// Detailed author information
#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct DetailedAuthor {
    pub name: String,
    #[eure(default)]
    pub affiliation: Option<String>,
    #[eure(default)]
    pub email: Option<String>,
    #[eure(default)]
    pub url: Option<String>,
}

/// Footnote definition
#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Footnote {
    /// Footnote content (inline markdown)
    pub content: String,
}

/// Section structure (recursive)
#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Section {
    /// Section header (inline markdown). If omitted, section key is used.
    #[eure(default)]
    pub header: Option<String>,

    /// Section body content (block markdown)
    #[eure(default)]
    pub body: Option<String>,

    /// Nested subsections
    #[eure(default)]
    pub sections: Map<String, Section>,
}

impl EumdDocument {
    /// Get the effective header for a section (uses key if header is None)
    pub fn get_section_header<'a>(key: &'a str, section: &'a Section) -> &'a str {
        section.header.as_deref().unwrap_or(key)
    }
}
