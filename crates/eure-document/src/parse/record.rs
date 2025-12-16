//! RecordParser and ExtParser for parsing record types from Eure documents.

extern crate alloc;

use alloc::format;

use crate::prelude_internal::*;
use crate::{document::node::NodeMap, parse::DocumentParser};

use super::{
    AccessedSet, FlattenContext, ParseContext, ParseDocument, ParseError, ParseErrorKind,
    UnionTagMode,
};

/// Helper for parsing record (map with string keys) from Eure documents.
///
/// Tracks accessed fields for unknown field checking.
///
/// # Flatten Context
///
/// When `flatten_ctx` is `Some`, this parser is part of a flattened chain:
/// - Field accesses are recorded in the shared `FlattenContext`
/// - `deny_unknown_fields()` is a no-op (root parser validates)
///
/// When `flatten_ctx` is `None`, this is a root parser:
/// - Field accesses are recorded in local `accessed` set
/// - `deny_unknown_fields()` actually validates
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
#[must_use]
pub struct RecordParser<'doc> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    map: &'doc NodeMap,
    /// Accessed fields and extensions - shared via AccessedSet.
    /// Root creates new AccessedSet, children clone from FlattenContext.
    accessed: AccessedSet,
    /// Flatten context - None for root, Some for children.
    /// When Some, deny_unknown_fields is no-op.
    flatten_ctx: Option<FlattenContext>,
    /// Union tag mode inherited from context.
    union_tag_mode: UnionTagMode,
}

impl<'doc> RecordParser<'doc> {
    /// Create a new RecordParser for the given context.
    pub(crate) fn new(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        Self::from_doc_and_node_with_flatten_ctx(
            ctx.doc(),
            ctx.node_id(),
            ctx.flatten_ctx().cloned(),
            ctx.union_tag_mode(),
        )
    }

    /// Create a new RecordParser from document and node ID directly.
    pub(crate) fn from_doc_and_node(
        doc: &'doc EureDocument,
        node_id: NodeId,
    ) -> Result<Self, ParseError> {
        Self::from_doc_and_node_with_flatten_ctx(doc, node_id, None, UnionTagMode::default())
    }

    /// Create a new RecordParser with flatten context and mode.
    pub(crate) fn from_doc_and_node_with_flatten_ctx(
        doc: &'doc EureDocument,
        node_id: NodeId,
        flatten_ctx: Option<FlattenContext>,
        union_tag_mode: UnionTagMode,
    ) -> Result<Self, ParseError> {
        // Create or clone accessed set
        let accessed = match &flatten_ctx {
            Some(fc) => fc.accessed_set().clone(),
            None => AccessedSet::new(),
        };

        let node = doc.node(node_id);
        match &node.content {
            NodeValue::Map(map) => Ok(Self {
                doc,
                node_id,
                map,
                accessed,
                flatten_ctx,
                union_tag_mode,
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

    /// Mark a field as accessed.
    fn mark_accessed(&mut self, name: &str) {
        self.accessed.add_field(name);
    }

    /// Get the node ID being parsed.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get a required field.
    ///
    /// Returns `ParseErrorKind::MissingField` if the field is not present or is excluded.
    pub fn parse_field<T>(&mut self, name: &str) -> Result<T, T::Error>
    where
        T: ParseDocument<'doc>,
        T::Error: From<ParseError>,
    {
        self.parse_field_with(name, T::parse)
    }

    pub fn parse_field_with<T>(&mut self, name: &str, mut parser: T) -> Result<T::Output, T::Error>
    where
        T: DocumentParser<'doc>,
        T::Error: From<ParseError>,
    {
        self.mark_accessed(name);
        let field_node_id = self
            .map
            .get(&ObjectKey::String(name.to_string()))
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingField(name.to_string()),
            })?;
        let ctx = ParseContext::with_union_tag_mode(self.doc, field_node_id, self.union_tag_mode);
        parser.parse(&ctx)
    }

    pub fn parse_field_optional<T>(&mut self, name: &str) -> Result<Option<T>, T::Error>
    where
        T: ParseDocument<'doc>,
        T::Error: From<ParseError>,
    {
        self.parse_field_optional_with(name, T::parse)
    }

    /// Get an optional field.
    ///
    /// Returns `Ok(None)` if the field is not present.
    pub fn parse_field_optional_with<T>(
        &mut self,
        name: &str,
        mut parser: T,
    ) -> Result<Option<T::Output>, T::Error>
    where
        T: DocumentParser<'doc>,
        T::Error: From<ParseError>,
    {
        self.mark_accessed(name);
        match self.map.get(&ObjectKey::String(name.to_string())) {
            Some(field_node_id) => {
                let ctx =
                    ParseContext::with_union_tag_mode(self.doc, field_node_id, self.union_tag_mode);
                Ok(Some(parser.parse(&ctx)?))
            }
            None => Ok(None),
        }
    }

    /// Get the parse context for a field without parsing it.
    ///
    /// Use this when you need access to the field's NodeId or want to defer parsing.
    /// Returns `ParseErrorKind::MissingField` if the field is not present.
    pub fn field(&mut self, name: &str) -> Result<ParseContext<'doc>, ParseError> {
        self.mark_accessed(name);
        let field_node_id = self
            .map
            .get(&ObjectKey::String(name.to_string()))
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingField(name.to_string()),
            })?;
        Ok(ParseContext::with_union_tag_mode(
            self.doc,
            field_node_id,
            self.union_tag_mode,
        ))
    }

    /// Get the parse context for an optional field without parsing it.
    ///
    /// Use this when you need access to the field's NodeId or want to defer parsing.
    /// Returns `None` if the field is not present.
    pub fn field_optional(&mut self, name: &str) -> Option<ParseContext<'doc>> {
        self.mark_accessed(name);
        self.map
            .get(&ObjectKey::String(name.to_string()))
            .map(|node_id| {
                ParseContext::with_union_tag_mode(self.doc, node_id, self.union_tag_mode)
            })
    }

    /// Get a field as a nested record parser.
    ///
    /// Returns `ParseErrorKind::MissingField` if the field is not present.
    pub fn field_record(&mut self, name: &str) -> Result<RecordParser<'doc>, ParseError> {
        self.mark_accessed(name);
        let field_node_id = self
            .map
            .get(&ObjectKey::String(name.to_string()))
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingField(name.to_string()),
            })?;
        RecordParser::from_doc_and_node_with_flatten_ctx(
            self.doc,
            field_node_id,
            None,
            self.union_tag_mode,
        )
    }

    /// Get an optional field as a nested record parser.
    ///
    /// Returns `Ok(None)` if the field is not present.
    pub fn field_record_optional(
        &mut self,
        name: &str,
    ) -> Result<Option<RecordParser<'doc>>, ParseError> {
        self.mark_accessed(name);
        match self.map.get(&ObjectKey::String(name.to_string())) {
            Some(field_node_id) => Ok(Some(RecordParser::from_doc_and_node_with_flatten_ctx(
                self.doc,
                field_node_id,
                None,
                self.union_tag_mode,
            )?)),
            None => Ok(None),
        }
    }

    /// Finish parsing with Deny policy (error if unknown fields exist).
    ///
    /// This also errors if the map contains non-string keys, as records
    /// should only have string-keyed fields.
    ///
    /// **Flatten behavior**: If this parser has a flatten_ctx (i.e., is a child
    /// in a flatten chain), this is a no-op. Only root parsers validate.
    pub fn deny_unknown_fields(self) -> Result<(), ParseError> {
        // If child (has flatten_ctx), no-op - parent will validate
        if self.flatten_ctx.is_some() {
            return Ok(());
        }

        // Root parser - validate using accessed set
        for (key, _) in self.map.iter() {
            match key {
                ObjectKey::String(name) => {
                    if !self.accessed.has_field(name.as_str()) {
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
        let mode = self.union_tag_mode;
        // Clone the accessed set for filtering - we need the current state
        let accessed = self.accessed.clone();
        self.map.iter().filter_map(move |(key, &node_id)| {
            if let ObjectKey::String(name) = key
                && !accessed.has_field(name.as_str())
            {
                return Some((
                    name.as_str(),
                    ParseContext::with_union_tag_mode(doc, node_id, mode),
                ));
            }
            None
        })
    }

    /// Create a flatten context for child parsers.
    ///
    /// This creates a FlattenContext initialized with the current accessed fields,
    /// and returns a ParseContext that children can use. Children created from this
    /// context will:
    /// - Add their accessed fields to the shared FlattenContext
    /// - Have deny_unknown_fields() be a no-op
    ///
    /// The root parser should call deny_unknown_fields() after all children are done.
    pub fn flatten(&mut self) -> ParseContext<'doc> {
        // Create or reuse flatten context - DON'T mutate self
        let flatten_ctx = match &self.flatten_ctx {
            Some(fc) => fc.clone(),
            None => {
                // Root: create new FlattenContext wrapping our AccessedSet
                FlattenContext::from_accessed_set(self.accessed.clone())
            }
        };

        ParseContext::with_flatten_ctx(self.doc, self.node_id, flatten_ctx, self.union_tag_mode)
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
#[must_use]
pub struct ExtParser<'doc> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    extensions: &'doc Map<Identifier, NodeId>,
    /// Accessed fields and extensions - shared via AccessedSet.
    /// Root creates new AccessedSet, children clone from FlattenContext.
    accessed: AccessedSet,
    /// Flatten context - None for root, Some for children.
    /// - None: root parser, validates on deny_unknown_extensions
    /// - Some: child parser, no-op on deny_unknown_extensions (parent validates)
    flatten_ctx: Option<FlattenContext>,
    /// Union tag mode inherited from context.
    union_tag_mode: UnionTagMode,
}

impl<'doc> ExtParser<'doc> {
    /// Create a new ExtParser for the given node.
    pub(crate) fn new(
        doc: &'doc EureDocument,
        node_id: NodeId,
        extensions: &'doc Map<Identifier, NodeId>,
        flatten_ctx: Option<FlattenContext>,
        union_tag_mode: UnionTagMode,
    ) -> Self {
        // Create or clone accessed set
        let accessed = match &flatten_ctx {
            Some(fc) => fc.accessed_set().clone(),
            None => AccessedSet::new(),
        };
        Self {
            doc,
            node_id,
            extensions,
            accessed,
            flatten_ctx,
            union_tag_mode,
        }
    }

    /// Get the node ID being parsed.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Mark an extension as accessed.
    fn mark_accessed(&mut self, ident: Identifier) {
        self.accessed.add_ext(ident);
    }

    /// Get a required extension field.
    ///
    /// Returns `ParseErrorKind::MissingExtension` if the extension is not present.
    pub fn parse_ext<T>(&mut self, name: &str) -> Result<T, T::Error>
    where
        T: ParseDocument<'doc>,
        T::Error: From<ParseError>,
    {
        self.parse_ext_with(name, T::parse)
    }

    pub fn parse_ext_with<T>(&mut self, name: &str, mut parser: T) -> Result<T::Output, T::Error>
    where
        T: DocumentParser<'doc>,
        T::Error: From<ParseError>,
    {
        let ident: Identifier = name.parse().map_err(|e| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::InvalidIdentifier(e),
        })?;
        self.mark_accessed(ident.clone());
        let ext_node_id = self.extensions.get(&ident).ok_or_else(|| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::MissingExtension(name.to_string()),
        })?;
        let ctx = ParseContext::with_union_tag_mode(self.doc, *ext_node_id, self.union_tag_mode);
        parser.parse(&ctx)
    }

    /// Get an optional extension field.
    ///
    /// Returns `Ok(None)` if the extension is not present.
    pub fn parse_ext_optional<T>(&mut self, name: &str) -> Result<Option<T>, T::Error>
    where
        T: ParseDocument<'doc>,
        T::Error: From<ParseError>,
    {
        self.parse_ext_optional_with(name, T::parse)
    }

    /// Get an optional extension field with a custom parser.
    ///
    /// Returns `Ok(None)` if the extension is not present.
    pub fn parse_ext_optional_with<T>(
        &mut self,
        name: &str,
        mut parser: T,
    ) -> Result<Option<T::Output>, T::Error>
    where
        T: DocumentParser<'doc>,
        T::Error: From<ParseError>,
    {
        let ident: Identifier = name.parse().map_err(|e| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::InvalidIdentifier(e),
        })?;
        self.mark_accessed(ident.clone());
        match self.extensions.get(&ident) {
            Some(ext_node_id) => {
                let ctx =
                    ParseContext::with_union_tag_mode(self.doc, *ext_node_id, self.union_tag_mode);
                Ok(Some(parser.parse(&ctx)?))
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
        self.mark_accessed(ident.clone());
        let ext_node_id = self
            .extensions
            .get(&ident)
            .copied()
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingExtension(name.to_string()),
            })?;
        Ok(ParseContext::with_union_tag_mode(
            self.doc,
            ext_node_id,
            self.union_tag_mode,
        ))
    }

    /// Get the parse context for an optional extension field without parsing it.
    ///
    /// Use this when you need access to the extension's NodeId or want to defer parsing.
    /// Returns `None` if the extension is not present.
    pub fn ext_optional(&mut self, name: &str) -> Option<ParseContext<'doc>> {
        let ident: Identifier = name.parse().ok()?;
        self.mark_accessed(ident.clone());
        self.extensions.get(&ident).map(|&node_id| {
            ParseContext::with_union_tag_mode(self.doc, node_id, self.union_tag_mode)
        })
    }

    /// Finish parsing with Deny policy (error if unknown extensions exist).
    ///
    /// **Flatten behavior**: If this parser has a flatten_ctx (i.e., is a child
    /// in a flatten chain), this is a no-op. Only root parsers validate.
    pub fn deny_unknown_extensions(self) -> Result<(), ParseError> {
        // If child (has flatten_ctx), no-op - parent will validate
        if self.flatten_ctx.is_some() {
            return Ok(());
        }

        // Root parser - validate using accessed set
        for (ident, _) in self.extensions.iter() {
            if !self.accessed.has_ext(ident) {
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
        let mode = self.union_tag_mode;
        // Clone the accessed set for filtering - we need the current state
        let accessed = self.accessed.clone();
        self.extensions.iter().filter_map(move |(ident, &node_id)| {
            if !accessed.has_ext(ident) {
                Some((ident, ParseContext::with_union_tag_mode(doc, node_id, mode)))
            } else {
                None
            }
        })
    }

    /// Create a flatten context for child parsers.
    ///
    /// This creates a FlattenContext initialized with the current accessed extensions,
    /// and returns a ParseContext that children can use. Children created from this
    /// context will:
    /// - Add their accessed extensions to the shared FlattenContext
    /// - Have deny_unknown_extensions() be a no-op
    ///
    /// The root parser should call deny_unknown_extensions() after all children are done.
    pub fn flatten(&mut self) -> ParseContext<'doc> {
        // Create or reuse flatten context - DON'T mutate self
        let flatten_ctx = match &self.flatten_ctx {
            Some(fc) => fc.clone(),
            None => {
                // Root: create new FlattenContext wrapping our AccessedSet
                FlattenContext::from_accessed_set(self.accessed.clone())
            }
        };

        ParseContext::with_flatten_ctx(self.doc, self.node_id, flatten_ctx, self.union_tag_mode)
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

    /// Helper struct for testing three-level nested flatten pattern.
    /// Parses: { a, b, c, d, e } with three-level flatten.
    #[derive(Debug, PartialEq)]
    struct ThreeLevelFlatten {
        a: i32,
        b: i32,
        c: i32,
        d: i32,
        e: i32,
    }

    impl<'doc> ParseDocument<'doc> for ThreeLevelFlatten {
        type Error = ParseError;

        fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
            // Level 1
            let mut rec1 = ctx.parse_record()?;
            let a = rec1.parse_field("a")?;
            let ctx2 = rec1.flatten();

            // Level 2
            let mut rec2 = ctx2.parse_record()?;
            let b = rec2.parse_field("b")?;
            let c = rec2.parse_field("c")?;
            let ctx3 = rec2.flatten();

            // Level 3
            let mut rec3 = ctx3.parse_record()?;
            let d = rec3.parse_field("d")?;
            let e = rec3.parse_field("e")?;
            rec3.deny_unknown_fields()?;

            // Level 2 deny (no-op since child)
            rec2.deny_unknown_fields()?;

            // Level 1 deny (root - validates all)
            rec1.deny_unknown_fields()?;

            Ok(Self { a, b, c, d, e })
        }
    }

    #[test]
    fn test_nested_flatten_preserves_consumed_fields() {
        // Document: { a = 1, b = 2, c = 3, d = 4, e = 5 }
        //
        // Parsing structure:
        // Level 1: parse_record(), field(a), flatten() →
        //   Level 2: field(b), field(c), flatten() →
        //     Level 3: field(d), field(e), deny_unknown_fields()
        //   Level 2: deny_unknown_fields()
        // Level 1: deny_unknown_fields()
        //
        // Expected: All deny_unknown_fields() should succeed
        use crate::eure;

        let doc = eure!({ a = 1, b = 2, c = 3, d = 4, e = 5 });
        let result: ThreeLevelFlatten = doc.parse(doc.get_root_id()).unwrap();

        assert_eq!(
            result,
            ThreeLevelFlatten {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
                e: 5
            }
        );
    }

    #[test]
    fn test_nested_flatten_catches_unaccessed_field() {
        // Document: { a = 1, b = 2, c = 3, d = 4, e = 5, f = 6 }
        //
        // Parsing structure (NOT accessing f):
        // Level 1: field(a), flatten() →
        //   Level 2: field(b), field(c), flatten() →
        //     Level 3: field(d), field(e), deny_unknown_fields()
        //   Level 2: deny_unknown_fields()
        // Level 1: deny_unknown_fields() <- Should FAIL because f is not accessed
        //
        // Expected: Level 1's deny_unknown_fields() should fail with UnknownField("f")
        use crate::eure;

        let doc = eure!({ a = 1, b = 2, c = 3, d = 4, e = 5, f = 6 });
        let result: Result<ThreeLevelFlatten, _> = doc.parse(doc.get_root_id());

        assert_eq!(
            result.unwrap_err().kind,
            ParseErrorKind::UnknownField("f".to_string())
        );
    }

    #[test]
    fn test_flatten_union_reverts_accessed_fields_on_failure() {
        use crate::eure;

        let doc = eure!({
            a = 1
            b = 2
            c = 3
            d = 4
        });

        // Define enum with two variants
        #[derive(Debug, PartialEq)]
        enum TestOption {
            A { a: i32, c: i32, e: i32 },
            B { a: i32, b: i32 },
        }

        impl<'doc> ParseDocument<'doc> for TestOption {
            type Error = ParseError;

            fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
                ctx.parse_union(VariantRepr::default())?
                    .variant("A", |ctx: &ParseContext<'_>| {
                        let mut rec = ctx.parse_record()?;
                        let a = rec.parse_field("a")?;
                        let c = rec.parse_field("c")?;
                        let e = rec.parse_field("e")?; // Will fail - field doesn't exist
                        rec.deny_unknown_fields()?;
                        Ok(TestOption::A { a, c, e })
                    })
                    .variant("B", |ctx: &ParseContext<'_>| {
                        let mut rec = ctx.parse_record()?;
                        let a = rec.parse_field("a")?;
                        let b = rec.parse_field("b")?;
                        rec.deny_unknown_fields()?;
                        Ok(TestOption::B { a, b })
                    })
                    .parse()
            }
        }

        // Parse with flatten
        let root_id = doc.get_root_id();
        let root_ctx = ParseContext::new(&doc, root_id);
        let mut record = root_ctx.parse_record().unwrap();

        // Parse union - should succeed with VariantB
        let option = record.flatten().parse::<TestOption>().unwrap();
        assert_eq!(option, TestOption::B { a: 1, b: 2 });

        // Access field d
        let d: i32 = record.parse_field("d").unwrap();
        assert_eq!(d, 4);

        // BUG: This should FAIL because field 'c' was never accessed by VariantB
        // (the successful variant), but it SUCCEEDS because VariantA tried 'c'
        // before failing
        let result = record.deny_unknown_fields();

        assert_eq!(
            result.unwrap_err(),
            ParseError {
                node_id: root_id,
                kind: ParseErrorKind::UnknownField("c".to_string()),
            }
        );
    }
}
