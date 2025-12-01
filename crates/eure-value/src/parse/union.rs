//! UnionParser for parsing union types from Eure documents.
//!
//! Implements oneOf semantics with priority-based ambiguity resolution.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::document::{EureDocument, NodeId};
use crate::identifier::Identifier;

use super::{DocumentParser, ParseError, ParseErrorKind};

/// The `$variant` extension identifier.
pub const VARIANT: Identifier = Identifier::new_unchecked("variant");

/// Helper for parsing union types from Eure documents.
///
/// Implements oneOf semantics:
/// - Exactly one variant must match
/// - Multiple matches resolved by registration order (priority)
/// - Short-circuits on first priority variant match
/// - When `$variant` extension is specified, matches by name directly
///
/// # Example
///
/// ```ignore
/// impl ParseDocument<'_> for Description {
///     fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
///         doc.parse_union(node_id)
///             .variant("string", |doc, id| {
///                 let text: String = doc.parse(id)?;
///                 Ok(Description::String(text))
///             })
///             .variant("markdown", |doc, id| {
///                 let text: String = doc.parse(id)?;
///                 Ok(Description::Markdown(text))
///             })
///             .parse()
///     }
/// }
/// ```
pub struct UnionParser<'doc, T> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    /// Variant name from `$variant` extension (if specified)
    variant_name: Option<String>,
    /// Result when `$variant` is specified and matches
    variant_result: Option<Result<T, ParseError>>,
    /// All registered variant names (for UnknownVariant check)
    all_variant_names: Vec<String>,
    /// First matching priority variant (short-circuit result)
    priority_result: Option<T>,
    /// Matching non-priority variants
    other_results: Vec<(String, T)>,
    /// Failed non-priority variants (for error reporting)
    other_failures: Vec<(String, ParseError)>,
}

impl<'doc, T> UnionParser<'doc, T> {
    /// Create a new UnionParser for the given node.
    pub(crate) fn new(doc: &'doc EureDocument, node_id: NodeId) -> Self {
        let variant_name = Self::extract_variant(doc, node_id);

        Self {
            doc,
            node_id,
            variant_name,
            variant_result: None,
            all_variant_names: Vec::new(),
            priority_result: None,
            other_results: Vec::new(),
            other_failures: Vec::new(),
        }
    }

    /// Extract the `$variant` extension value from the node.
    fn extract_variant(doc: &EureDocument, node_id: NodeId) -> Option<String> {
        let node = doc.node(node_id);
        let variant_node_id = node.extensions.get(&VARIANT)?;
        doc.parse::<&str>(*variant_node_id).ok().map(String::from)
    }

    /// Register a priority variant.
    ///
    /// Priority variants are tried in registration order.
    /// When a priority variant matches, parsing short-circuits and returns immediately.
    pub fn variant<P>(mut self, name: &str, parser: P) -> Self
    where
        P: DocumentParser<'doc, Output = T> + 'doc,
    {
        self.all_variant_names.push(name.to_string());

        if let Some(ref vn) = self.variant_name {
            // $variant specified: only parse if name matches and no result yet
            if vn == name && self.variant_result.is_none() {
                self.variant_result = Some(parser.parse(self.doc, self.node_id));
            }
        } else if self.priority_result.is_none()
            && let Ok(value) = parser.parse(self.doc, self.node_id)
        {
            // No $variant: short-circuit on first match
            self.priority_result = Some(value);
        }
        self
    }

    /// Register a non-priority variant.
    ///
    /// Non-priority variants are only tried if no priority variant matches.
    /// All non-priority variants are tried to detect ambiguity.
    pub fn other<P>(mut self, name: &str, parser: P) -> Self
    where
        P: DocumentParser<'doc, Output = T> + 'doc,
    {
        self.all_variant_names.push(name.to_string());

        if let Some(ref vn) = self.variant_name {
            // $variant specified: only parse if name matches and no result yet
            if vn == name && self.variant_result.is_none() {
                self.variant_result = Some(parser.parse(self.doc, self.node_id));
            }
        } else {
            // No $variant: try all for ambiguity detection (only if no priority match)
            if self.priority_result.is_none() {
                match parser.parse(self.doc, self.node_id) {
                    Ok(value) => self.other_results.push((name.to_string(), value)),
                    Err(e) => self.other_failures.push((name.to_string(), e)),
                }
            }
        }
        self
    }

    /// Execute the union parse with oneOf semantics.
    ///
    /// Returns:
    /// - `Ok(T)` if exactly one variant matches (or priority resolves ambiguity)
    /// - `Err(UnknownVariant)` if `$variant` specifies an unregistered name
    /// - `Err(NoMatchingVariant)` if no variants match
    /// - `Err(AmbiguousUnion)` if multiple non-priority variants match
    pub fn parse(self) -> Result<T, ParseError> {
        let node_id = self.node_id;

        // $variant specified
        if let Some(variant_name) = self.variant_name {
            // Check if variant name is registered
            if !self.all_variant_names.iter().any(|n| n == &variant_name) {
                return Err(ParseError {
                    node_id,
                    kind: ParseErrorKind::UnknownVariant(variant_name),
                });
            }
            // Return the parse result (success or failure)
            return self.variant_result.unwrap();
        }

        // No $variant: use priority/other logic
        if let Some(value) = self.priority_result {
            return Ok(value);
        }

        // Check non-priority variants
        match self.other_results.len() {
            0 => Err(Self::no_match_error(node_id, self.other_failures)),
            1 => Ok(self.other_results.into_iter().next().unwrap().1),
            _ => Err(ParseError {
                node_id,
                kind: ParseErrorKind::AmbiguousUnion(
                    self.other_results
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect(),
                ),
            }),
        }
    }

    /// Create an error for when no variant matches.
    fn no_match_error(node_id: NodeId, failures: Vec<(String, ParseError)>) -> ParseError {
        // For now, return the first failure or a generic error
        // TODO: Implement "closest error" selection based on error depth
        failures
            .into_iter()
            .next()
            .map(|(_, e)| e)
            .unwrap_or(ParseError {
                node_id,
                kind: ParseErrorKind::NoMatchingVariant,
            })
    }
}

impl EureDocument {
    /// Get a UnionParser for parsing a union type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// doc.parse_union(node_id)
    ///     .variant("foo", parser_for_foo)
    ///     .variant("bar", parser_for_bar)
    ///     .parse()
    /// ```
    pub fn parse_union<T>(&self, node_id: NodeId) -> UnionParser<'_, T> {
        UnionParser::new(self, node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::node::NodeValue;
    use crate::text::Text;
    use crate::value::PrimitiveValue;

    fn identifier(s: &str) -> Identifier {
        s.parse().unwrap()
    }

    #[derive(Debug, PartialEq)]
    enum TestEnum {
        Foo,
        Bar,
    }

    fn create_text_doc(text: &str) -> EureDocument {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(text.to_string())));
        doc
    }

    /// Create a document with $variant extension
    fn create_doc_with_variant(content: &str, variant: &str) -> EureDocument {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Set content
        doc.node_mut(root_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(content.to_string())));

        // Add $variant extension
        let variant_node_id = doc
            .add_extension(identifier("variant"), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(variant.to_string())));

        doc
    }

    #[test]
    fn test_union_single_match() {
        let doc = create_text_doc("foo");
        let root_id = doc.get_root_id();

        let result: TestEnum = doc
            .parse_union(root_id)
            .variant("foo", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .variant("bar", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "bar" {
                    Ok(TestEnum::Bar)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .parse()
            .unwrap();

        assert_eq!(result, TestEnum::Foo);
    }

    #[test]
    fn test_union_priority_short_circuit() {
        let doc = create_text_doc("value");
        let root_id = doc.get_root_id();

        // Both variants would match, but first one wins due to priority
        let result: String = doc
            .parse_union(root_id)
            .variant("first", |doc: &EureDocument, id| doc.parse::<String>(id))
            .variant("second", |doc: &EureDocument, id| doc.parse::<String>(id))
            .parse()
            .unwrap();

        assert_eq!(result, "value");
    }

    #[test]
    fn test_union_no_match() {
        let doc = create_text_doc("baz");
        let root_id = doc.get_root_id();

        let result: Result<TestEnum, _> = doc
            .parse_union(root_id)
            .variant("foo", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_union_with_borrowed_str_fn_pointer() {
        // 関数ポインタで &str を返す
        fn parse_str(doc: &EureDocument, id: NodeId) -> Result<&str, ParseError> {
            doc.parse(id)
        }

        let doc = create_text_doc("hello");
        let root_id = doc.get_root_id();

        let result: &str = doc
            .parse_union(root_id)
            .variant("str", parse_str)
            .parse()
            .unwrap();

        assert_eq!(result, "hello");
    }

    #[test]
    fn test_union_with_borrowed_str_closure() {
        // クロージャで &str を返す (型注釈で関数ポインタに coerce)
        let doc = create_text_doc("world");
        let root_id = doc.get_root_id();

        let result: &str = doc
            .parse_union(root_id)
            .variant(
                "str",
                (|doc, id| doc.parse(id)) as fn(&EureDocument, NodeId) -> Result<&str, ParseError>,
            )
            .parse()
            .unwrap();

        assert_eq!(result, "world");
    }

    // --- $variant extension tests ---

    #[test]
    fn test_variant_extension_match_success() {
        // $variant = "bar" specified, "bar" parser succeeds
        let doc = create_doc_with_variant("bar", "bar");
        let root_id = doc.get_root_id();

        let result: TestEnum = doc
            .parse_union(root_id)
            .variant("foo", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .variant("bar", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "bar" {
                    Ok(TestEnum::Bar)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .parse()
            .unwrap();

        assert_eq!(result, TestEnum::Bar);
    }

    #[test]
    fn test_variant_extension_unknown() {
        // $variant = "unknown" specified, but "unknown" is not registered
        let doc = create_doc_with_variant("hello", "unknown");
        let root_id = doc.get_root_id();

        let err = doc
            .parse_union::<TestEnum>(root_id)
            .variant("foo", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .variant("bar", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "bar" {
                    Ok(TestEnum::Bar)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .parse()
            .unwrap_err();

        assert_eq!(err.node_id, root_id);
        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("unknown".to_string())
        );
    }

    #[test]
    fn test_variant_extension_match_parse_failure() {
        // $variant = "bar" specified, "bar" parser fails (content is "wrong")
        let doc = create_doc_with_variant("wrong", "bar");
        let root_id = doc.get_root_id();

        let err = doc
            .parse_union::<TestEnum>(root_id)
            .variant("foo", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .variant("bar", |doc: &EureDocument, id| {
                let s: &str = doc.parse(id)?;
                if s == "bar" {
                    Ok(TestEnum::Bar)
                } else {
                    Err(ParseError {
                        node_id: id,
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .parse()
            .unwrap_err();

        // Parser's error is returned directly (not UnknownVariant for "bar")
        assert_eq!(err.node_id, root_id);
        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("wrong".to_string())
        );
    }
}
