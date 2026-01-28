//! Tests for struct variants with #[eure(flatten)] and #[eure(ext)] together
//!
//! Use case: A union that can be either a Text value with optional mark options,
//! or a nested structure.
//!
//! Example syntax:
//! ```eure
//! draft-note = ```markdown
//! This is a draft.
//! ```
//! draft-note.$mark.alert: NOTE
//! ```
//!
//! Expected: `draft-note` is parsed as Text variant with:
//! - text: the markdown code block
//! - mark: MarkOptions { alert: Some(AlertType::Note) }

use eure::ParseDocument;
use eure::value::Text;

/// Alert type enum
#[derive(Debug, Clone, Copy, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub enum AlertType {
    #[eure(rename = "NOTE")]
    Note,
    #[eure(rename = "TIP")]
    Tip,
    #[eure(rename = "WARNING")]
    Warning,
}

/// Mark options parsed from extensions
#[derive(Debug, Clone, PartialEq, ParseDocument, Default)]
#[eure(crate = ::eure::document)]
pub struct MarkOptions {
    #[eure(default)]
    pub alert: Option<AlertType>,
}

/// Union that can be Text with extensions, or a nested struct
#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub enum TextOrNested {
    /// Text variant with flattened text and extension-parsed mark options
    Text {
        #[eure(flatten)]
        text: Text,
        #[eure(ext, default)]
        mark: MarkOptions,
    },
    /// Nested variant with a struct
    Nested(NestedContent),
}

#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub struct NestedContent {
    pub header: String,
    pub body: String,
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn test_text_variant_without_mark() {
    use eure::eure;

    // Simple text without any extensions
    let doc = eure!({ value = "Hello World" });
    let root = doc.parse_record(doc.get_root_id()).unwrap();
    let result: TextOrNested = root.parse_field("value").unwrap();

    assert_eq!(
        result,
        TextOrNested::Text {
            text: Text::plaintext("Hello World".to_string()),
            mark: MarkOptions::default(),
        }
    );
}

#[test]
fn test_text_variant_with_mark_extension() {
    use eure::eure;

    // Text with $mark.alert extension
    // This is the main reproduction case
    let doc = eure!({
        value = "Hello World"
        value.%mark.alert = "NOTE"
    });
    let root = doc.parse_record(doc.get_root_id()).unwrap();
    let result: TextOrNested = root.parse_field("value").unwrap();

    assert_eq!(
        result,
        TextOrNested::Text {
            text: Text::plaintext("Hello World".to_string()),
            mark: MarkOptions {
                alert: Some(AlertType::Note),
            },
        }
    );
}

#[test]
fn test_nested_variant() {
    use eure::eure;

    // Nested struct variant
    let doc = eure!({
        value {
            header = "Title"
            body = "Content"
        }
    });
    let root = doc.parse_record(doc.get_root_id()).unwrap();
    let result: TextOrNested = root.parse_field("value").unwrap();

    assert_eq!(
        result,
        TextOrNested::Nested(NestedContent {
            header: "Title".to_string(),
            body: "Content".to_string(),
        })
    );
}

// =============================================================================
// Simpler reproduction case: struct variant with flatten + ext
// =============================================================================

/// Simpler enum to isolate the issue
#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub enum SimpleTextOrMap {
    /// Text with optional extension
    Text {
        #[eure(flatten)]
        text: Text,
        #[eure(ext, default)]
        optional: Option<bool>,
    },
    /// Map variant
    Map { key: String },
}

#[test]
fn test_simple_text_variant_without_ext() {
    use eure::eure;

    let doc = eure!({ value = "Hello" });
    let root = doc.parse_record(doc.get_root_id()).unwrap();
    let result: SimpleTextOrMap = root.parse_field("value").unwrap();

    assert_eq!(
        result,
        SimpleTextOrMap::Text {
            text: Text::plaintext("Hello".to_string()),
            optional: None,
        }
    );
}

#[test]
fn test_simple_text_variant_with_ext() {
    use eure::eure;

    let doc = eure!({
        value = "Hello"
        value.%optional = true
    });
    let root = doc.parse_record(doc.get_root_id()).unwrap();
    let result: SimpleTextOrMap = root.parse_field("value").unwrap();

    assert_eq!(
        result,
        SimpleTextOrMap::Text {
            text: Text::plaintext("Hello".to_string()),
            optional: Some(true),
        }
    );
}

#[test]
fn test_simple_map_variant() {
    use eure::eure;

    let doc = eure!({
        value {
            key = "test"
        }
    });
    let root = doc.parse_record(doc.get_root_id()).unwrap();
    let result: SimpleTextOrMap = root.parse_field("value").unwrap();

    assert_eq!(
        result,
        SimpleTextOrMap::Map {
            key: "test".to_string(),
        }
    );
}
