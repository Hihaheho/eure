//! RecordParser and ExtParser for parsing record types from Eure documents.

extern crate alloc;

use alloc::format;
use std::collections::HashSet;

use crate::document::node::NodeMap;
use crate::prelude_internal::*;

use super::{ParseContext, ParseDocument, ParseError, ParseErrorKind};

/// Helper for parsing record (map with string keys) from Eure documents.
///
/// Tracks accessed fields for unknown field checking.
///
/// # Example
///
/// ```ignore
/// impl<'doc> ParseDocument<'doc> for User {
///     fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
///         let mut rec = ctx.parse_record()?;
///         let name = rec.field::<String>("name")?;
///         let age = rec.field_optional::<u32>("age")?;
///         rec.deny_unknown_fields()?;
///         Ok(User { name, age })
///     }
/// }
/// ```
pub struct RecordParser<'doc> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    map: &'doc NodeMap,
    accessed: HashSet<String>,
}

impl<'doc> RecordParser<'doc> {
    /// Create a new RecordParser for the given context.
    pub(crate) fn new(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        Self::from_doc_and_node(ctx.doc(), ctx.node_id())
    }

    /// Create a new RecordParser from document and node ID directly.
    pub(crate) fn from_doc_and_node(
        doc: &'doc EureDocument,
        node_id: NodeId,
    ) -> Result<Self, ParseError> {
        let node = doc.node(node_id);
        match &node.content {
            NodeValue::Map(map) => Ok(Self {
                doc,
                node_id,
                map,
                accessed: HashSet::new(),
            }),
            NodeValue::Hole(_) => Err(ParseError {
                node_id,
                kind: ParseErrorKind::UnexpectedHole,
            }),
            value => Err(ParseError {
                node_id,
                kind: value
                    .value_kind()
                    .map(|actual| ParseErrorKind::TypeMismatch {
                        expected: crate::value::ValueKind::Map,
                        actual,
                    })
                    .unwrap_or(ParseErrorKind::UnexpectedHole),
            }),
        }
    }

    /// Get the node ID being parsed.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get a required field.
    ///
    /// Returns `ParseErrorKind::MissingField` if the field is not present.
    pub fn parse_field<T: ParseDocument<'doc>>(&mut self, name: &str) -> Result<T, ParseError> {
        self.accessed.insert(name.to_string());
        let field_node_id = self
            .map
            .get(&ObjectKey::String(name.to_string()))
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingField(name.to_string()),
            })?;
        let ctx = ParseContext::new(self.doc, field_node_id);
        T::parse(&ctx)
    }

    /// Get an optional field.
    ///
    /// Returns `Ok(None)` if the field is not present.
    pub fn parse_field_optional<T: ParseDocument<'doc>>(
        &mut self,
        name: &str,
    ) -> Result<Option<T>, ParseError> {
        self.accessed.insert(name.to_string());
        match self.map.get(&ObjectKey::String(name.to_string())) {
            Some(field_node_id) => {
                let ctx = ParseContext::new(self.doc, field_node_id);
                Ok(Some(T::parse(&ctx)?))
            }
            None => Ok(None),
        }
    }

    /// Get the parse context for a field without parsing it.
    ///
    /// Use this when you need access to the field's NodeId or want to defer parsing.
    /// Returns `ParseErrorKind::MissingField` if the field is not present.
    pub fn field(&mut self, name: &str) -> Result<ParseContext<'doc>, ParseError> {
        self.accessed.insert(name.to_string());
        let field_node_id = self
            .map
            .get(&ObjectKey::String(name.to_string()))
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingField(name.to_string()),
            })?;
        Ok(ParseContext::new(self.doc, field_node_id))
    }

    /// Get the parse context for an optional field without parsing it.
    ///
    /// Use this when you need access to the field's NodeId or want to defer parsing.
    /// Returns `None` if the field is not present.
    pub fn field_optional(&mut self, name: &str) -> Option<ParseContext<'doc>> {
        self.accessed.insert(name.to_string());
        self.map
            .get(&ObjectKey::String(name.to_string()))
            .map(|node_id| ParseContext::new(self.doc, node_id))
    }

    /// Get a field as a nested record parser.
    ///
    /// Returns `ParseErrorKind::MissingField` if the field is not present.
    pub fn field_record(&mut self, name: &str) -> Result<RecordParser<'doc>, ParseError> {
        self.accessed.insert(name.to_string());
        let field_node_id = self
            .map
            .get(&ObjectKey::String(name.to_string()))
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingField(name.to_string()),
            })?;
        RecordParser::from_doc_and_node(self.doc, field_node_id)
    }

    /// Get an optional field as a nested record parser.
    ///
    /// Returns `Ok(None)` if the field is not present.
    pub fn field_record_optional(
        &mut self,
        name: &str,
    ) -> Result<Option<RecordParser<'doc>>, ParseError> {
        self.accessed.insert(name.to_string());
        match self.map.get(&ObjectKey::String(name.to_string())) {
            Some(field_node_id) => Ok(Some(RecordParser::from_doc_and_node(
                self.doc,
                field_node_id,
            )?)),
            None => Ok(None),
        }
    }

    /// Finish parsing with Deny policy (error if unknown fields exist).
    ///
    /// This also errors if the map contains non-string keys, as records
    /// should only have string-keyed fields.
    pub fn deny_unknown_fields(self) -> Result<(), ParseError> {
        for (key, _) in self.map.iter() {
            match key {
                ObjectKey::String(name) => {
                    if !self.accessed.contains(name.as_str()) {
                        return Err(ParseError {
                            node_id: self.node_id,
                            kind: ParseErrorKind::UnknownField(name.clone()),
                        });
                    }
                }
                // Non-string keys are invalid in records
                other => {
                    return Err(ParseError {
                        node_id: self.node_id,
                        kind: ParseErrorKind::InvalidKeyType(other.clone()),
                    });
                }
            }
        }
        Ok(())
    }

    /// Finish parsing with Allow policy (allow unknown string fields).
    ///
    /// This still errors if the map contains non-string keys, as records
    /// should only have string-keyed fields.
    pub fn allow_unknown_fields(self) -> Result<(), ParseError> {
        // Check for non-string keys (invalid in records)
        for (key, _) in self.map.iter() {
            if !matches!(key, ObjectKey::String(_)) {
                return Err(ParseError {
                    node_id: self.node_id,
                    kind: ParseErrorKind::InvalidKeyType(key.clone()),
                });
            }
        }
        Ok(())
    }

    /// Get an iterator over unknown fields (for Schema policy or custom handling).
    ///
    /// Returns (field_name, context) pairs for fields that haven't been accessed.
    pub fn unknown_fields(&self) -> impl Iterator<Item = (&'doc str, ParseContext<'doc>)> + '_ {
        let doc = self.doc;
        self.map.iter().filter_map(move |(key, &node_id)| {
            if let ObjectKey::String(name) = key
                && !self.accessed.contains(name.as_str())
            {
                return Some((name.as_str(), ParseContext::new(doc, node_id)));
            }
            None
        })
    }
}

/// Helper for parsing extension types ($ext-type) from Eure documents.
///
/// Similar API to RecordParser but for extension type fields.
///
/// # Example
///
/// ```ignore
/// let mut ext = ctx.parse_extension();
/// let optional = ext.ext_optional::<bool>("optional")?;
/// let binding_style = ext.ext_optional::<BindingStyle>("binding-style")?;
/// ext.allow_unknown_extensions();
/// ```
pub struct ExtParser<'doc> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    extensions: &'doc Map<Identifier, NodeId>,
    accessed: HashSet<Identifier>,
}

impl<'doc> ExtParser<'doc> {
    /// Create a new ExtParser for the given node.
    pub(crate) fn new(
        doc: &'doc EureDocument,
        node_id: NodeId,
        extensions: &'doc Map<Identifier, NodeId>,
    ) -> Self {
        Self {
            doc,
            node_id,
            extensions,
            accessed: HashSet::new(),
        }
    }

    /// Get the node ID being parsed.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get a required extension field.
    ///
    /// Returns `ParseErrorKind::MissingExtension` if the extension is not present.
    pub fn parse_ext<T: ParseDocument<'doc>>(&mut self, name: &str) -> Result<T, ParseError> {
        let ident: Identifier = name.parse().map_err(|e| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::InvalidIdentifier(e),
        })?;
        self.accessed.insert(ident.clone());
        let ext_node_id = self.extensions.get(&ident).ok_or_else(|| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::MissingExtension(name.to_string()),
        })?;
        let ctx = ParseContext::new(self.doc, *ext_node_id);
        T::parse(&ctx)
    }

    /// Get an optional extension field.
    ///
    /// Returns `Ok(None)` if the extension is not present.
    pub fn parse_ext_optional<T: ParseDocument<'doc>>(
        &mut self,
        name: &str,
    ) -> Result<Option<T>, ParseError> {
        let ident: Identifier = name.parse().map_err(|e| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::InvalidIdentifier(e),
        })?;
        self.accessed.insert(ident.clone());
        match self.extensions.get(&ident) {
            Some(ext_node_id) => {
                let ctx = ParseContext::new(self.doc, *ext_node_id);
                Ok(Some(T::parse(&ctx)?))
            }
            None => Ok(None),
        }
    }

    /// Get the parse context for an extension field without parsing it.
    ///
    /// Use this when you need access to the extension's NodeId or want to defer parsing.
    /// Returns `ParseErrorKind::MissingExtension` if the extension is not present.
    pub fn ext(&mut self, name: &str) -> Result<ParseContext<'doc>, ParseError> {
        let ident: Identifier = name.parse().map_err(|e| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::InvalidIdentifier(e),
        })?;
        self.accessed.insert(ident.clone());
        let ext_node_id = self
            .extensions
            .get(&ident)
            .copied()
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingExtension(name.to_string()),
            })?;
        Ok(ParseContext::new(self.doc, ext_node_id))
    }

    /// Get the parse context for an optional extension field without parsing it.
    ///
    /// Use this when you need access to the extension's NodeId or want to defer parsing.
    /// Returns `None` if the extension is not present.
    pub fn ext_optional(&mut self, name: &str) -> Option<ParseContext<'doc>> {
        let ident: Identifier = name.parse().ok()?;
        self.accessed.insert(ident.clone());
        self.extensions
            .get(&ident)
            .map(|&node_id| ParseContext::new(self.doc, node_id))
    }

    /// Finish parsing with Deny policy (error if unknown extensions exist).
    pub fn deny_unknown_extensions(self) -> Result<(), ParseError> {
        for (ident, _) in self.extensions.iter() {
            if !self.accessed.contains(ident) {
                return Err(ParseError {
                    node_id: self.node_id,
                    kind: ParseErrorKind::UnknownField(format!("$ext-type.{}", ident)),
                });
            }
        }
        Ok(())
    }

    /// Finish parsing with Allow policy (ignore unknown extensions).
    pub fn allow_unknown_extensions(self) {
        // Nothing to do - just consume self
    }

    /// Get an iterator over unknown extensions (for custom handling).
    ///
    /// Returns (identifier, context) pairs for extensions that haven't been accessed.
    pub fn unknown_extensions(
        &self,
    ) -> impl Iterator<Item = (&'doc Identifier, ParseContext<'doc>)> + '_ {
        let doc = self.doc;
        self.extensions.iter().filter_map(move |(ident, &node_id)| {
            if !self.accessed.contains(ident) {
                Some((ident, ParseContext::new(doc, node_id)))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::PrimitiveValue;

    fn create_test_doc() -> EureDocument {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Add fields: name = "Alice", age = 30
        let name_id = doc
            .add_map_child(ObjectKey::String("name".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(name_id).content = NodeValue::Primitive(PrimitiveValue::Text(
            crate::text::Text::plaintext("Alice".to_string()),
        ));

        let age_id = doc
            .add_map_child(ObjectKey::String("age".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(age_id).content = NodeValue::Primitive(PrimitiveValue::Integer(30.into()));

        doc
    }

    #[test]
    fn test_record_field() {
        let doc = create_test_doc();
        let mut rec = doc.parse_record(doc.get_root_id()).unwrap();

        let name: String = rec.parse_field("name").unwrap();
        assert_eq!(name, "Alice");
    }

    #[test]
    fn test_record_field_missing() {
        let doc = create_test_doc();
        let mut rec = doc.parse_record(doc.get_root_id()).unwrap();

        let result: Result<String, _> = rec.parse_field("nonexistent");
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::MissingField(_)
        ));
    }

    #[test]
    fn test_record_field_optional() {
        let doc = create_test_doc();
        let mut rec = doc.parse_record(doc.get_root_id()).unwrap();

        let name: Option<String> = rec.parse_field_optional("name").unwrap();
        assert_eq!(name, Some("Alice".to_string()));

        let missing: Option<String> = rec.parse_field_optional("nonexistent").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_record_deny_unknown_fields() {
        let doc = create_test_doc();
        let mut rec = doc.parse_record(doc.get_root_id()).unwrap();

        let _name: String = rec.parse_field("name").unwrap();
        // Didn't access "age", so deny should fail
        let result = rec.deny_unknown_fields();
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::UnknownField(_)
        ));
    }

    #[test]
    fn test_record_deny_unknown_fields_all_accessed() {
        let doc = create_test_doc();
        let mut rec = doc.parse_record(doc.get_root_id()).unwrap();

        let _name: String = rec.parse_field("name").unwrap();
        let _age: num_bigint::BigInt = rec.parse_field("age").unwrap();
        // Accessed all fields, should succeed
        rec.deny_unknown_fields().unwrap();
    }

    #[test]
    fn test_record_allow_unknown_fields() {
        let doc = create_test_doc();
        let mut rec = doc.parse_record(doc.get_root_id()).unwrap();

        let _name: String = rec.parse_field("name").unwrap();
        // Didn't access "age", but allow should succeed
        rec.allow_unknown_fields().unwrap();
    }

    #[test]
    fn test_record_unknown_fields_iterator() {
        let doc = create_test_doc();
        let mut rec = doc.parse_record(doc.get_root_id()).unwrap();

        let _name: String = rec.parse_field("name").unwrap();
        // "age" should be in unknown fields
        let unknown: Vec<_> = rec.unknown_fields().collect();
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].0, "age");
    }

    #[test]
    fn test_record_with_non_string_keys_deny_should_error() {
        // BUG: deny_unknown_fields() silently skips non-string keys
        // Expected: Should error when a map has numeric keys
        // Actual: Silently ignores them
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Add a field with numeric key: { 0 => "value" }
        use num_bigint::BigInt;
        let value_id = doc
            .add_map_child(ObjectKey::Number(BigInt::from(0)), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(value_id).content = NodeValue::Primitive(PrimitiveValue::Text(
            crate::text::Text::plaintext("value".to_string()),
        ));

        let rec = doc.parse_record(doc.get_root_id()).unwrap();

        // BUG: This should error because there's an unaccessed non-string key
        // but currently it succeeds
        let result = rec.deny_unknown_fields();
        assert!(
            result.is_err(),
            "BUG: deny_unknown_fields() should error on non-string keys, but it succeeds"
        );
    }

    #[test]
    fn test_record_with_non_string_keys_unknown_fields_iterator() {
        // unknown_fields() intentionally only returns string keys (signature: (&str, NodeId))
        // Non-string keys are caught by deny_unknown_fields() instead
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Add a field with numeric key: { 0 => "value" }
        use num_bigint::BigInt;
        let value_id = doc
            .add_map_child(ObjectKey::Number(BigInt::from(0)), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(value_id).content = NodeValue::Primitive(PrimitiveValue::Text(
            crate::text::Text::plaintext("value".to_string()),
        ));

        let rec = doc.parse_record(doc.get_root_id()).unwrap();

        // unknown_fields() returns empty because it only returns string keys
        // (the numeric key is not included in the iterator by design)
        let unknown: Vec<_> = rec.unknown_fields().collect();
        assert_eq!(
            unknown.len(),
            0,
            "unknown_fields() should only return string keys, numeric keys are excluded"
        );
    }

    #[test]
    fn test_ext_parser() {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Add extension: $ext-type.optional = true
        let ext_id = doc
            .add_extension("optional".parse().unwrap(), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(ext_id).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        let mut ext = doc.parse_extension(root_id);
        let optional: bool = ext.parse_ext("optional").unwrap();
        assert!(optional);
    }

    #[test]
    fn test_ext_parser_optional_missing() {
        let doc = EureDocument::new();
        let root_id = doc.get_root_id();

        let mut ext = doc.parse_extension(root_id);
        let optional: Option<bool> = ext.parse_ext_optional("optional").unwrap();
        assert_eq!(optional, None);
    }
}
