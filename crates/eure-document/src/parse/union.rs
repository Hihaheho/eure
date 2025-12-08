//! UnionParser for parsing union types from Eure documents.
//!
//! Implements oneOf semantics with priority-based ambiguity resolution.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::identifier::Identifier;

use super::variant_path::VariantPath;
use super::{ParseContext, ParseError, ParseErrorKind};

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
/// impl<'doc> ParseDocument<'doc> for Description {
///     fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
///         ctx.parse_union()
///             .variant("string", |ctx| {
///                 let text: String = ctx.parse()?;
///                 Ok(Description::String(text))
///             })
///             .variant("markdown", |ctx| {
///                 let text: String = ctx.parse()?;
///                 Ok(Description::Markdown(text))
///             })
///             .parse()
///     }
/// }
/// ```
pub struct UnionParser<'doc, 'ctx, T> {
    ctx: &'ctx ParseContext<'doc>,
    /// Variant path (from context or extracted from `$variant` extension)
    variant_path: Option<VariantPath>,
    /// Result when `$variant` is specified and matches
    variant_result: Option<Result<T, ParseError>>,
    /// First matching priority variant (short-circuit result)
    priority_result: Option<T>,
    /// Matching non-priority variants
    other_results: Vec<(String, T)>,
    /// Failed non-priority variants (for error reporting)
    other_failures: Vec<(String, ParseError)>,
}

impl<'doc, 'ctx, T> UnionParser<'doc, 'ctx, T> {
    /// Create a new UnionParser for the given context.
    pub(crate) fn new(ctx: &'ctx ParseContext<'doc>) -> Self {
        // Determine variant path:
        // - None (from context) = extract from node
        // - Some(non-empty) = use the path
        // - Some(empty) = variant consumed, use try-all logic (don't extract)
        let variant_path = match ctx.variant_path() {
            Some(vp) if !vp.is_empty() => Some(vp.clone()),
            Some(_) => None, // Empty path = no explicit variant, use try-all logic
            None => Self::extract_variant(ctx),
        };

        Self {
            ctx,
            variant_path,
            variant_result: None,
            priority_result: None,
            other_results: Vec::new(),
            other_failures: Vec::new(),
        }
    }

    /// Extract the `$variant` extension value from the node.
    fn extract_variant(ctx: &ParseContext<'doc>) -> Option<VariantPath> {
        let node = ctx.node();
        let variant_node_id = node.extensions.get(&VARIANT)?;
        let s: &str = ctx.doc().parse(*variant_node_id).ok()?;
        VariantPath::parse(s).ok()
    }

    /// Register a priority variant.
    ///
    /// Priority variants are tried in registration order.
    /// When a priority variant matches, parsing short-circuits and returns immediately.
    ///
    /// The closure receives a child context with the remaining variant path (if any).
    /// This handles both single-segment and multi-segment paths uniformly.
    pub fn variant<F>(mut self, name: &str, f: F) -> Self
    where
        F: FnOnce(&ParseContext<'doc>) -> Result<T, ParseError>,
    {
        let name_ident: Identifier = match name.parse() {
            Ok(i) => i,
            Err(_) => return self,
        };

        if let Some(ref vp) = self.variant_path {
            // $variant specified: match if path starts with this segment
            if vp.first() == Some(&name_ident) && self.variant_result.is_none() {
                // Use Some(empty) when path is consumed to signal "don't extract from node"
                let rest = Some(vp.rest().unwrap_or_else(VariantPath::empty));
                let child_ctx = self.ctx.with_variant_rest(rest);
                self.variant_result = Some(f(&child_ctx));
            }
        } else if self.priority_result.is_none() {
            // No $variant: try parsing with empty variant path
            let child_ctx = self.ctx.with_variant_rest(None);
            if let Ok(value) = f(&child_ctx) {
                self.priority_result = Some(value);
            }
        }
        self
    }

    /// Register a non-priority variant.
    ///
    /// Non-priority variants are only tried if no priority variant matches.
    /// All non-priority variants are tried to detect ambiguity.
    ///
    /// The closure receives a child context with the remaining variant path (if any).
    pub fn other<F>(mut self, name: &str, f: F) -> Self
    where
        F: FnOnce(&ParseContext<'doc>) -> Result<T, ParseError>,
    {
        let name_ident: Identifier = match name.parse() {
            Ok(i) => i,
            Err(_) => return self,
        };

        if let Some(ref vp) = self.variant_path {
            // $variant specified: match if path starts with this segment
            if vp.first() == Some(&name_ident) && self.variant_result.is_none() {
                // Use Some(empty) when path is consumed to signal "don't extract from node"
                let rest = Some(vp.rest().unwrap_or_else(VariantPath::empty));
                let child_ctx = self.ctx.with_variant_rest(rest);
                self.variant_result = Some(f(&child_ctx));
            }
        } else {
            // No $variant: try all for ambiguity detection (only if no priority match)
            if self.priority_result.is_none() {
                let child_ctx = self.ctx.with_variant_rest(None);
                match f(&child_ctx) {
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
        let node_id = self.ctx.node_id();

        // $variant specified
        if let Some(variant_path) = self.variant_path {
            return self.variant_result.unwrap_or_else(|| {
                Err(ParseError {
                    node_id,
                    kind: ParseErrorKind::UnknownVariant(variant_path.to_string()),
                })
            });
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
    fn no_match_error(
        node_id: crate::document::NodeId,
        failures: Vec<(String, ParseError)>,
    ) -> ParseError {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::EureDocument;
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
        let ctx = doc.parse_context(root_id);

        let result: TestEnum = ctx
            .parse_union()
            .variant("foo", |ctx| {
                let s: &str = ctx.parse()?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: ctx.node_id(),
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .variant("bar", |ctx| {
                let s: &str = ctx.parse()?;
                if s == "bar" {
                    Ok(TestEnum::Bar)
                } else {
                    Err(ParseError {
                        node_id: ctx.node_id(),
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
        let ctx = doc.parse_context(root_id);

        // Both variants would match, but first one wins due to priority
        let result: String = ctx
            .parse_union()
            .variant("first", |ctx| ctx.parse::<String>())
            .variant("second", |ctx| ctx.parse::<String>())
            .parse()
            .unwrap();

        assert_eq!(result, "value");
    }

    #[test]
    fn test_union_no_match() {
        let doc = create_text_doc("baz");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Result<TestEnum, _> = ctx
            .parse_union()
            .variant("foo", |ctx| {
                let s: &str = ctx.parse()?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: ctx.node_id(),
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .parse();

        assert!(result.is_err());
    }

    // --- $variant extension tests ---

    #[test]
    fn test_variant_extension_match_success() {
        // $variant = "baz" specified, matches other("baz")
        // All parsers always succeed
        let doc = create_doc_with_variant("anything", "baz");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: TestEnum = ctx
            .parse_union()
            .variant("foo", |_| Ok(TestEnum::Foo))
            .other("baz", |_| Ok(TestEnum::Bar))
            .parse()
            .unwrap();

        assert_eq!(result, TestEnum::Bar);
    }

    #[test]
    fn test_variant_extension_unknown() {
        // $variant = "unknown" specified, but "unknown" is not registered
        // All parsers always succeed
        let doc = create_doc_with_variant("anything", "unknown");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = ctx
            .parse_union::<TestEnum>()
            .variant("foo", |_| Ok(TestEnum::Foo))
            .other("baz", |_| Ok(TestEnum::Bar))
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
        // $variant = "baz" specified, "baz" parser fails
        let doc = create_doc_with_variant("anything", "baz");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = ctx
            .parse_union::<TestEnum>()
            .variant("foo", |_| Ok(TestEnum::Foo))
            .other("baz", |ctx| {
                Err(ParseError {
                    node_id: ctx.node_id(),
                    kind: ParseErrorKind::MissingField("test".to_string()),
                })
            })
            .parse()
            .unwrap_err();

        // Parser's error is returned directly
        assert_eq!(err.node_id, root_id);
        assert_eq!(err.kind, ParseErrorKind::MissingField("test".to_string()));
    }

    // --- nested variant tests ---

    #[derive(Debug, PartialEq)]
    enum Outer {
        A(Inner),
        B(i32),
    }

    #[derive(Debug, PartialEq)]
    enum Inner {
        X,
        Y,
    }

    fn parse_inner(ctx: &ParseContext<'_>) -> Result<Inner, ParseError> {
        ctx.parse_union()
            .variant("x", |_| Ok(Inner::X))
            .variant("y", |_| Ok(Inner::Y))
            .parse()
    }

    #[test]
    fn test_variant_nested_single_segment() {
        // $variant = "a" - matches "a", rest is None -> Inner defaults to X
        let doc = create_doc_with_variant("value", "a");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Outer = ctx
            .parse_union()
            .variant("a", |ctx| parse_inner(ctx).map(Outer::A))
            .variant("b", |_| Ok(Outer::B(42)))
            .parse()
            .unwrap();

        assert_eq!(result, Outer::A(Inner::X));
    }

    #[test]
    fn test_variant_nested_multi_segment() {
        // $variant = "a.y" - matches "a", rest is Some("y")
        let doc = create_doc_with_variant("value", "a.y");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Outer = ctx
            .parse_union()
            .variant("a", |ctx| parse_inner(ctx).map(Outer::A))
            .variant("b", |_| Ok(Outer::B(42)))
            .parse()
            .unwrap();

        assert_eq!(result, Outer::A(Inner::Y));
    }

    #[test]
    fn test_variant_nested_invalid_inner() {
        // $variant = "a.z" - matches "a", but "z" is not valid for Inner
        let doc = create_doc_with_variant("value", "a.z");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = ctx
            .parse_union::<Outer>()
            .variant("a", |ctx| parse_inner(ctx).map(Outer::A))
            .variant("b", |_| Ok(Outer::B(42)))
            .parse()
            .unwrap_err();

        assert_eq!(err.kind, ParseErrorKind::UnknownVariant("z".to_string()));
    }

    #[test]
    fn test_variant_non_nested_with_nested_path() {
        // $variant = "b.x" but "b" parser doesn't expect nested path
        // The child context will have variant_path = Some("x")
        // If the "b" parser is a non-union type, it should error on unexpected variant path
        let doc = create_doc_with_variant("value", "b.x");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // "b" is registered as a variant but if called with "b.x",
        // the closure gets ctx with variant_path = Some("x")
        // The simple parser Ok(Outer::B(42)) doesn't check variant path,
        // but a proper impl would use ctx.parse_primitive() which errors
        let err = ctx
            .parse_union::<Outer>()
            .variant("a", |ctx| parse_inner(ctx).map(Outer::A))
            .variant("b", |ctx| {
                // Simulate parsing a primitive that checks variant path
                ctx.parse_primitive()?;
                Ok(Outer::B(42))
            })
            .parse()
            .unwrap_err();

        // parse_primitive should error because variant path "x" remains
        assert!(matches!(err.kind, ParseErrorKind::UnexpectedVariantPath(_)));
    }

    // --- invalid $variant tests ---

    /// Create a document with $variant set to an integer (invalid type)
    fn create_doc_with_integer_variant(content: &str, variant_value: i64) -> EureDocument {
        use num_bigint::BigInt;

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Set content
        doc.node_mut(root_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(content.to_string())));

        // Add $variant extension with integer value (invalid!)
        let variant_node_id = doc
            .add_extension(identifier("variant"), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(variant_value)));

        doc
    }

    #[test]
    #[ignore = "BUG: invalid $variant type silently falls back to try-all"]
    fn test_invalid_variant_type_should_error() {
        // $variant = 123 (integer, not string) - should error, not silently fall back
        let doc = create_doc_with_integer_variant("foo", 123);
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // Current behavior: silently ignores invalid $variant and falls back to try-all
        // Expected behavior: should return an error about invalid $variant type
        let result = ctx
            .parse_union::<TestEnum>()
            .variant("foo", |ctx| {
                let s: &str = ctx.parse()?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: ctx.node_id(),
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .variant("bar", |_| Ok(TestEnum::Bar))
            .parse();

        // BUG: This currently succeeds with Foo (falls back to try-all)
        // It SHOULD fail with an error about invalid $variant type
        assert!(result.is_err(), "Should error on invalid $variant type");
    }

    #[test]
    #[ignore = "BUG: invalid $variant path syntax silently falls back to try-all"]
    fn test_invalid_variant_path_syntax_should_error() {
        // $variant = "foo..bar" (invalid path syntax) - should error
        let doc = create_doc_with_variant("foo", "foo..bar");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result = ctx
            .parse_union::<TestEnum>()
            .variant("foo", |ctx| {
                let s: &str = ctx.parse()?;
                if s == "foo" {
                    Ok(TestEnum::Foo)
                } else {
                    Err(ParseError {
                        node_id: ctx.node_id(),
                        kind: ParseErrorKind::UnknownVariant(s.to_string()),
                    })
                }
            })
            .variant("bar", |_| Ok(TestEnum::Bar))
            .parse();

        // BUG: This currently succeeds with Foo (falls back to try-all)
        // It SHOULD fail with an error about invalid $variant path syntax
        assert!(result.is_err(), "Should error on invalid $variant path syntax");
    }
}
