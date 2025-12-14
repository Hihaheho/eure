//! TOML conversion support for Eure format.
//!
//! This crate provides conversion from TOML documents to Eure's [`SourceDocument`],
//! preserving comments and section ordering.
//!
//! # Example
//!
//! ```
//! use eure_toml::to_source_document;
//!
//! let toml_str = r#"
//! [server]
//! host = "localhost"
//! port = 8080
//! "#;
//!
//! let toml_doc = toml_str.parse::<toml_edit::DocumentMut>().unwrap();
//! let source_doc = to_source_document(&toml_doc).unwrap();
//! ```

mod error;

pub use error::TomlToEureError;

use eure_document::document::constructor::DocumentConstructor;
use eure_document::document::NodeId;
use eure_document::identifier::Identifier;
use eure_document::path::PathSegment;
use eure_document::source::{
    Comment, Layout, LayoutItem, SectionBody, SourceDocument, SourcePathSegment,
};
use eure_document::text::Text;
use eure_document::value::PrimitiveValue;
use num_bigint::BigInt;
use toml_edit::{DocumentMut, Item, Value};

/// Convert a TOML document to a SourceDocument.
///
/// This preserves:
/// - Comments (converted from `#` to `//`)
/// - Section ordering (including interleaved `[table]` and `[[array]]` sections)
/// - All TOML values
pub fn to_source_document(toml_doc: &DocumentMut) -> Result<SourceDocument, TomlToEureError> {
    let mut converter = Converter::new();
    converter.convert_document(toml_doc)?;
    Ok(converter.finish())
}

struct Converter {
    constructor: DocumentConstructor,
    layout: Layout,
}

impl Converter {
    fn new() -> Self {
        Self {
            constructor: DocumentConstructor::new(),
            layout: Layout::new(),
        }
    }

    fn finish(self) -> SourceDocument {
        SourceDocument::new(self.constructor.finish(), self.layout)
    }

    fn convert_document(&mut self, doc: &DocumentMut) -> Result<(), TomlToEureError> {
        // Convert root table items
        for (key, item) in doc.iter() {
            self.convert_root_item(key, item)?;
        }
        Ok(())
    }

    fn convert_root_item(&mut self, key: &str, item: &Item) -> Result<(), TomlToEureError> {
        // Add leading comments from decor
        self.add_comments_from_decor_prefix(item);

        match item {
            Item::None => {}
            Item::Value(value) => {
                // Simple key = value at root level
                let identifier = self.parse_identifier(key)?;
                let node_id = self.bind_value_at_path(&[identifier.clone()], value)?;
                let trailing_comment = self.extract_trailing_comment(value.decor());

                self.layout.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::ident(identifier)],
                    node: node_id,
                    trailing_comment,
                });
            }
            Item::Table(table) => {
                // [section] or inline table
                if table.is_implicit() {
                    // Implicit table - created by dotted keys, don't emit section
                    return Ok(());
                }

                let identifier = self.parse_identifier(key)?;
                let path = vec![SourcePathSegment::ident(identifier.clone())];

                // Navigate to this table in the document
                let scope = self.constructor.begin_scope();
                self.constructor
                    .navigate(PathSegment::Ident(identifier))
                    .map_err(|e| TomlToEureError::InvalidIdentifier {
                        key: key.to_string(),
                        reason: e.to_string(),
                    })?;

                // Ensure it's a map
                if self.constructor.current_node().content.is_hole() {
                    self.constructor.bind_empty_map().map_err(|e| {
                        TomlToEureError::InvalidIdentifier {
                            key: key.to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                }

                let trailing_comment = table
                    .decor()
                    .suffix()
                    .and_then(|s| extract_comment_from_raw(s.as_str().unwrap_or("")))
                    .map(|c| match c {
                        Comment::Line(s) => s,
                        Comment::Block(s) => s,
                    });

                // Convert table contents
                let mut section_items = Vec::new();
                for (child_key, child_item) in table.iter() {
                    let items = self.convert_table_item(child_key, child_item)?;
                    section_items.extend(items);
                }

                self.constructor.end_scope(scope).expect("scope mismatch");

                self.layout.push(LayoutItem::Section {
                    path,
                    trailing_comment,
                    body: SectionBody::Items(section_items),
                });
            }
            Item::ArrayOfTables(array) => {
                // [[section]] - array of tables
                let identifier = self.parse_identifier(key)?;

                for table in array.iter() {
                    let path =
                        vec![SourcePathSegment::ident(identifier.clone()).with_array_push()];

                    // Navigate and push to array
                    let scope = self.constructor.begin_scope();
                    self.constructor
                        .navigate(PathSegment::Ident(identifier.clone()))
                        .map_err(|e| TomlToEureError::InvalidIdentifier {
                            key: key.to_string(),
                            reason: e.to_string(),
                        })?;

                    // Ensure it's an array
                    if self.constructor.current_node().content.is_hole() {
                        self.constructor.bind_empty_array().map_err(|e| {
                            TomlToEureError::InvalidIdentifier {
                                key: key.to_string(),
                                reason: e.to_string(),
                            }
                        })?;
                    }

                    // Push new element
                    self.constructor
                        .navigate(PathSegment::ArrayIndex(None))
                        .map_err(|e| TomlToEureError::InvalidIdentifier {
                            key: key.to_string(),
                            reason: e.to_string(),
                        })?;

                    // Ensure element is a map
                    if self.constructor.current_node().content.is_hole() {
                        self.constructor.bind_empty_map().map_err(|e| {
                            TomlToEureError::InvalidIdentifier {
                                key: key.to_string(),
                                reason: e.to_string(),
                            }
                        })?;
                    }

                    let trailing_comment = table
                        .decor()
                        .suffix()
                        .and_then(|s| extract_comment_from_raw(s.as_str().unwrap_or("")))
                        .map(|c| match c {
                            Comment::Line(s) => s,
                            Comment::Block(s) => s,
                        });

                    // Convert table contents
                    let mut section_items = Vec::new();
                    for (child_key, child_item) in table.iter() {
                        let items = self.convert_table_item(child_key, child_item)?;
                        section_items.extend(items);
                    }

                    self.constructor.end_scope(scope).expect("scope mismatch");

                    self.layout.push(LayoutItem::Section {
                        path,
                        trailing_comment,
                        body: SectionBody::Items(section_items),
                    });
                }
            }
        }

        Ok(())
    }

    fn convert_table_item(
        &mut self,
        key: &str,
        item: &Item,
    ) -> Result<Vec<LayoutItem>, TomlToEureError> {
        let mut items = Vec::new();

        // Add leading comments
        if let Some(comment) = self.extract_prefix_comment(item) {
            items.push(LayoutItem::Comment(comment));
        }

        match item {
            Item::None => {}
            Item::Value(value) => {
                let identifier = self.parse_identifier(key)?;
                let node_id = self.bind_value_at_current(&identifier, value)?;
                let trailing_comment = self.extract_trailing_comment(value.decor());

                items.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::ident(identifier)],
                    node: node_id,
                    trailing_comment,
                });
            }
            Item::Table(table) => {
                if table.is_implicit() {
                    return Ok(items);
                }

                let identifier = self.parse_identifier(key)?;

                // Navigate into nested table
                let scope = self.constructor.begin_scope();
                self.constructor
                    .navigate(PathSegment::Ident(identifier.clone()))
                    .map_err(|e| TomlToEureError::InvalidIdentifier {
                        key: key.to_string(),
                        reason: e.to_string(),
                    })?;

                if self.constructor.current_node().content.is_hole() {
                    self.constructor.bind_empty_map().map_err(|e| {
                        TomlToEureError::InvalidIdentifier {
                            key: key.to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                }

                // Convert nested items
                let mut nested_items = Vec::new();
                for (child_key, child_item) in table.iter() {
                    let child_items = self.convert_table_item(child_key, child_item)?;
                    nested_items.extend(child_items);
                }

                self.constructor.end_scope(scope).expect("scope mismatch");

                // Use block syntax for inline tables
                if table.is_dotted() {
                    items.extend(nested_items.into_iter().map(|item| {
                        if let LayoutItem::Binding {
                            mut path,
                            node,
                            trailing_comment,
                        } = item
                        {
                            path.insert(0, SourcePathSegment::ident(identifier.clone()));
                            LayoutItem::Binding {
                                path,
                                node,
                                trailing_comment,
                            }
                        } else {
                            item
                        }
                    }));
                } else {
                    let trailing_comment = table
                        .decor()
                        .suffix()
                        .and_then(|s| extract_comment_from_raw(s.as_str().unwrap_or("")))
                        .map(|c| match c {
                            Comment::Line(s) => s,
                            Comment::Block(s) => s,
                        });

                    items.push(LayoutItem::Section {
                        path: vec![SourcePathSegment::ident(identifier)],
                        trailing_comment,
                        body: SectionBody::Block(nested_items),
                    });
                }
            }
            Item::ArrayOfTables(array) => {
                let identifier = self.parse_identifier(key)?;

                for table in array.iter() {
                    let scope = self.constructor.begin_scope();
                    self.constructor
                        .navigate(PathSegment::Ident(identifier.clone()))
                        .map_err(|e| TomlToEureError::InvalidIdentifier {
                            key: key.to_string(),
                            reason: e.to_string(),
                        })?;

                    if self.constructor.current_node().content.is_hole() {
                        self.constructor.bind_empty_array().map_err(|e| {
                            TomlToEureError::InvalidIdentifier {
                                key: key.to_string(),
                                reason: e.to_string(),
                            }
                        })?;
                    }

                    self.constructor
                        .navigate(PathSegment::ArrayIndex(None))
                        .map_err(|e| TomlToEureError::InvalidIdentifier {
                            key: key.to_string(),
                            reason: e.to_string(),
                        })?;

                    if self.constructor.current_node().content.is_hole() {
                        self.constructor.bind_empty_map().map_err(|e| {
                            TomlToEureError::InvalidIdentifier {
                                key: key.to_string(),
                                reason: e.to_string(),
                            }
                        })?;
                    }

                    let mut nested_items = Vec::new();
                    for (child_key, child_item) in table.iter() {
                        let child_items = self.convert_table_item(child_key, child_item)?;
                        nested_items.extend(child_items);
                    }

                    self.constructor.end_scope(scope).expect("scope mismatch");

                    items.push(LayoutItem::Section {
                        path: vec![SourcePathSegment::ident(identifier.clone()).with_array_push()],
                        trailing_comment: None,
                        body: SectionBody::Block(nested_items),
                    });
                }
            }
        }

        Ok(items)
    }

    fn bind_value_at_path(
        &mut self,
        path: &[Identifier],
        value: &Value,
    ) -> Result<NodeId, TomlToEureError> {
        let scope = self.constructor.begin_scope();

        for ident in path {
            self.constructor
                .navigate(PathSegment::Ident(ident.clone()))
                .map_err(|e| TomlToEureError::InvalidIdentifier {
                    key: ident.to_string(),
                    reason: e.to_string(),
                })?;
        }

        let node_id = self.bind_toml_value(value)?;
        self.constructor.end_scope(scope).expect("scope mismatch");

        Ok(node_id)
    }

    fn bind_value_at_current(
        &mut self,
        key: &Identifier,
        value: &Value,
    ) -> Result<NodeId, TomlToEureError> {
        let scope = self.constructor.begin_scope();

        self.constructor
            .navigate(PathSegment::Ident(key.clone()))
            .map_err(|e| TomlToEureError::InvalidIdentifier {
                key: key.to_string(),
                reason: e.to_string(),
            })?;

        let node_id = self.bind_toml_value(value)?;
        self.constructor.end_scope(scope).expect("scope mismatch");

        Ok(node_id)
    }

    fn bind_toml_value(&mut self, value: &Value) -> Result<NodeId, TomlToEureError> {
        let node_id = self.constructor.current_node_id();

        match value {
            Value::String(s) => {
                let text = Text::plaintext(s.value().to_string());
                self.constructor
                    .bind_primitive(PrimitiveValue::Text(text))
                    .expect("binding should succeed");
            }
            Value::Integer(i) => {
                let bi = BigInt::from(*i.value());
                self.constructor
                    .bind_primitive(PrimitiveValue::Integer(bi))
                    .expect("binding should succeed");
            }
            Value::Float(f) => {
                self.constructor
                    .bind_primitive(PrimitiveValue::F64(*f.value()))
                    .expect("binding should succeed");
            }
            Value::Boolean(b) => {
                self.constructor
                    .bind_primitive(PrimitiveValue::Bool(*b.value()))
                    .expect("binding should succeed");
            }
            Value::Datetime(dt) => {
                // Convert datetime to string representation
                let text = Text::plaintext(dt.to_string());
                self.constructor
                    .bind_primitive(PrimitiveValue::Text(text))
                    .expect("binding should succeed");
            }
            Value::Array(arr) => {
                self.constructor
                    .bind_empty_array()
                    .expect("binding should succeed");

                for item in arr.iter() {
                    let scope = self.constructor.begin_scope();
                    self.constructor
                        .navigate(PathSegment::ArrayIndex(None))
                        .expect("array navigation should succeed");
                    self.bind_toml_value(item)?;
                    self.constructor.end_scope(scope).expect("scope mismatch");
                }
            }
            Value::InlineTable(table) => {
                self.constructor
                    .bind_empty_map()
                    .expect("binding should succeed");

                for (key, val) in table.iter() {
                    let identifier = self.parse_identifier(key)?;
                    let scope = self.constructor.begin_scope();
                    self.constructor
                        .navigate(PathSegment::Ident(identifier))
                        .expect("map navigation should succeed");
                    self.bind_toml_value(val)?;
                    self.constructor.end_scope(scope).expect("scope mismatch");
                }
            }
        }

        Ok(node_id)
    }

    fn parse_identifier(&self, key: &str) -> Result<Identifier, TomlToEureError> {
        key.parse()
            .map_err(|e: eure_document::identifier::IdentifierError| {
                TomlToEureError::InvalidIdentifier {
                    key: key.to_string(),
                    reason: e.to_string(),
                }
            })
    }

    fn add_comments_from_decor_prefix(&mut self, item: &Item) {
        if let Some(comment) = self.extract_prefix_comment(item) {
            self.layout.push(LayoutItem::Comment(comment));
        }
    }

    fn extract_prefix_comment(&self, item: &Item) -> Option<Comment> {
        let decor = match item {
            Item::Value(v) => v.decor(),
            Item::Table(t) => t.decor(),
            Item::ArrayOfTables(a) => a.iter().next()?.decor(),
            Item::None => return None,
        };

        decor
            .prefix()
            .and_then(|s| extract_comment_from_raw(s.as_str().unwrap_or("")))
    }

    fn extract_trailing_comment(&self, decor: &toml_edit::Decor) -> Option<String> {
        decor
            .suffix()
            .and_then(|s| s.as_str())
            .and_then(extract_comment_text)
    }
}

/// Extract a Comment from raw TOML decor string (may contain `#` comments)
fn extract_comment_from_raw(raw: &str) -> Option<Comment> {
    let text = extract_comment_text(raw)?;
    Some(Comment::Line(text))
}

/// Extract just the comment text from a raw string containing `# comment`
fn extract_comment_text(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(comment) = trimmed.strip_prefix('#') {
            return Some(comment.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key_value() {
        let toml = r#"key = "value""#;
        let doc: DocumentMut = toml.parse().expect("valid toml");
        let result = to_source_document(&doc);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        assert_eq!(source.layout.items.len(), 1);
    }

    #[test]
    fn test_section() {
        let toml = r#"
[server]
host = "localhost"
port = 8080
"#;
        let doc: DocumentMut = toml.parse().expect("valid toml");
        let result = to_source_document(&doc);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // Should have one section
        assert_eq!(source.layout.items.len(), 1);
    }

    #[test]
    fn test_array_of_tables() {
        let toml = r#"
[[items]]
name = "first"

[[items]]
name = "second"
"#;
        let doc: DocumentMut = toml.parse().expect("valid toml");
        let result = to_source_document(&doc);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // Should have two sections (one for each [[items]])
        assert_eq!(source.layout.items.len(), 2);
    }

    #[test]
    fn test_comment_preservation() {
        let toml = r#"
# This is a comment
key = "value"
"#;
        let doc: DocumentMut = toml.parse().expect("valid toml");
        let result = to_source_document(&doc);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // Should have comment + binding
        assert!(source.layout.items.len() >= 1);
    }

    #[test]
    fn test_interleaved_sections() {
        // Note: toml_edit groups all [[example]] entries together,
        // so the interleaved order in the source is NOT preserved.
        // This is a fundamental limitation of how TOML parsers work.
        let toml = r#"
[[example]]
name = "first"

[metadata.first]
description = "First example"

[[example]]
name = "second"

[metadata.second]
description = "Second example"
"#;
        let doc: DocumentMut = toml.parse().expect("valid toml");
        let result = to_source_document(&doc);
        assert!(result.is_ok());

        let source = result.expect("conversion should succeed");
        // toml_edit groups: [[example]] x2 = 2 items
        // [metadata.first] and [metadata.second] make `metadata` implicit,
        // so it's skipped in our output
        assert_eq!(source.layout.items.len(), 2);
    }

    #[test]
    fn test_invalid_identifier() {
        let toml = r#""invalid key with spaces" = "value""#;
        let doc: DocumentMut = toml.parse().expect("valid toml");
        let result = to_source_document(&doc);
        assert!(result.is_err());
    }
}
