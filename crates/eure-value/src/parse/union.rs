//! UnionParser for parsing union types from Eure documents.
//!
//! Implements oneOf semantics with priority-based ambiguity resolution.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::document::{EureDocument, NodeId};

use super::{DocumentParser, ParseError, ParseErrorKind};

/// Helper for parsing union types from Eure documents.
///
/// Implements oneOf semantics:
/// - Exactly one variant must match
/// - Multiple matches resolved by registration order (priority)
/// - Short-circuits on first priority variant match
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
    /// Priority variants - tried in order, first match wins (short-circuit)
    priority_variants: Vec<(String, Box<dyn ErasedParser<'doc, T> + 'doc>)>,
    /// Non-priority variants - always tried for ambiguity detection
    other_variants: Vec<(String, Box<dyn ErasedParser<'doc, T> + 'doc>)>,
}

/// Type-erased parser trait for storing heterogeneous parsers.
trait ErasedParser<'doc, T> {
    fn parse_erased(
        self: Box<Self>,
        doc: &'doc EureDocument,
        node_id: NodeId,
    ) -> Result<T, ParseError>;
}

impl<'doc, T, P> ErasedParser<'doc, T> for P
where
    P: DocumentParser<'doc, Output = T>,
{
    fn parse_erased(
        self: Box<Self>,
        doc: &'doc EureDocument,
        node_id: NodeId,
    ) -> Result<T, ParseError> {
        (*self).parse(doc, node_id)
    }
}

impl<'doc, T> UnionParser<'doc, T> {
    /// Create a new UnionParser for the given node.
    pub(crate) fn new(doc: &'doc EureDocument, node_id: NodeId) -> Self {
        Self {
            doc,
            node_id,
            priority_variants: Vec::new(),
            other_variants: Vec::new(),
        }
    }

    /// Register a priority variant.
    ///
    /// Priority variants are tried in registration order.
    /// When a priority variant matches, parsing short-circuits and returns immediately.
    pub fn variant<P>(mut self, name: &str, parser: P) -> Self
    where
        P: DocumentParser<'doc, Output = T> + 'doc,
    {
        self.priority_variants
            .push((name.to_string(), Box::new(parser)));
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
        self.other_variants
            .push((name.to_string(), Box::new(parser)));
        self
    }

    /// Execute the union parse with oneOf semantics.
    ///
    /// Returns:
    /// - `Ok(T)` if exactly one variant matches (or priority resolves ambiguity)
    /// - `Err(NoMatchingVariant)` if no variants match
    /// - `Err(AmbiguousUnion)` if multiple non-priority variants match
    pub fn parse(self) -> Result<T, ParseError> {
        let doc = self.doc;
        let node_id = self.node_id;

        // 1. Try priority variants in order (short-circuit on first match)
        for (_name, parser) in self.priority_variants {
            match parser.parse_erased(doc, node_id) {
                Ok(value) => return Ok(value),
                Err(_) => continue,
            }
        }

        // 2. Try all non-priority variants
        let mut matching: Vec<(String, T)> = Vec::new();
        let mut failures: Vec<(String, ParseError)> = Vec::new();

        for (name, parser) in self.other_variants {
            match parser.parse_erased(doc, node_id) {
                Ok(value) => matching.push((name, value)),
                Err(e) => failures.push((name, e)),
            }
        }

        // 3. Determine result
        match matching.len() {
            0 => Err(Self::no_match_error(node_id, failures)),
            1 => Ok(matching.into_iter().next().unwrap().1),
            _ => Err(ParseError {
                node_id,
                kind: ParseErrorKind::AmbiguousUnion(
                    matching.into_iter().map(|(name, _)| name).collect(),
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
}
