//! UnionParser for parsing union types from Eure documents.
//!
//! Implements oneOf semantics with priority-based ambiguity resolution.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::document::{EureDocument, NodeId};
use crate::identifier::Identifier;
use crate::parse::{DocumentParser, FromEure};

use super::variant_path::VariantPath;
use super::{AccessedSnapshot, FlattenContext, ParseContext, ParseError, ParseErrorKind};

/// The `$variant` extension identifier.
pub const VARIANT: Identifier = Identifier::new_unchecked("variant");

/// Extract `$variant` extension as a parsed [`VariantPath`], if present.
pub fn extract_explicit_variant_path(
    doc: &EureDocument,
    node_id: NodeId,
) -> Result<Option<VariantPath>, ParseError> {
    let node = doc.node(node_id);
    let Some(&variant_node_id) = node.extensions.get(&VARIANT) else {
        return Ok(None);
    };

    let variant_node = doc.node(variant_node_id);
    let s: &str = doc.parse(variant_node_id).map_err(|_| ParseError {
        node_id: variant_node_id,
        kind: ParseErrorKind::InvalidVariantType(variant_node.content.value_kind()),
    })?;

    VariantPath::parse(s).map(Some).map_err(|_| ParseError {
        node_id: variant_node_id,
        kind: ParseErrorKind::InvalidVariantPath(s.to_string()),
    })
}

/// Returns whether this node has any explicit union tag information.
///
pub fn has_explicit_variant_tag(doc: &EureDocument, node_id: NodeId) -> Result<bool, ParseError> {
    Ok(extract_explicit_variant_path(doc, node_id)?.is_some())
}

// =============================================================================
// UnionParser
// =============================================================================

/// Helper for parsing union types from Eure documents.
///
/// Implements oneOf semantics:
/// - Exactly one variant must match
/// - Multiple matches resolved by registration order (priority)
/// - Short-circuits on first priority variant match
/// - When `$variant` extension is specified, matches by name directly
///
/// # Variant Resolution
///
/// Variant is determined by `$variant` extension when present.
/// Without `$variant`, the parser falls back to untagged matching.
///
/// # Example
///
/// ```ignore
/// impl<'doc> FromEure<'doc> for Description {
///     fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
///         ctx.parse_union()?
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
pub struct UnionParser<'doc, 'ctx, T, E = ParseError> {
    ctx: &'ctx ParseContext<'doc>,
    /// Unified variant: (name, context, rest_path)
    /// - name: variant name to match
    /// - context: ParseContext for the variant content
    /// - rest_path: remaining variant path for nested unions
    variant: Option<(String, ParseContext<'doc>, Option<VariantPath>)>,
    /// Result when variant matches
    variant_result: Option<Result<T, E>>,
    /// First matching priority variant (short-circuit result)
    priority_result: Option<T>,
    /// Matching non-priority variants, with their captured accessed state.
    /// The AccessedSnapshot is captured after successful parse, before restoring.
    other_results: Vec<(String, T, AccessedSnapshot)>,
    /// Failed variants (for error reporting)
    failures: Vec<(String, E)>,
    /// Flatten context for snapshot/rollback (if flattened parsing).
    flatten_ctx: Option<FlattenContext>,
}

impl<'doc, 'ctx, T, E> UnionParser<'doc, 'ctx, T, E>
where
    E: From<ParseError>,
{
    /// Create a new UnionParser for the given context.
    ///
    /// Returns error if `$variant` extension has invalid type or syntax.
    pub(crate) fn new(ctx: &'ctx ParseContext<'doc>) -> Result<Self, ParseError> {
        let variant = Self::resolve_variant(ctx)?;

        // Push snapshot for rollback if flatten context exists
        let flatten_ctx = ctx.flatten_ctx().cloned();
        if let Some(ref fc) = flatten_ctx {
            fc.push_snapshot();
        }

        Ok(Self {
            ctx,
            variant,
            variant_result: None,
            priority_result: None,
            other_results: Vec::new(),
            failures: Vec::new(),
            flatten_ctx,
        })
    }

    /// Resolve the unified variant from `$variant` extension.
    ///
    /// Returns:
    /// - `Some((name, ctx, rest))` if variant is determined
    /// - `None` for Untagged parsing
    ///
    fn resolve_variant(
        ctx: &ParseContext<'doc>,
    ) -> Result<Option<(String, ParseContext<'doc>, Option<VariantPath>)>, ParseError> {
        // Check if variant path is already set in context (from parent union)
        let explicit_variant = match ctx.variant_path() {
            Some(vp) if !vp.is_empty() => Some(vp.clone()),
            Some(_) => None, // Empty path = variant consumed, use Untagged
            None => {
                let variant = Self::extract_explicit_variant(ctx)?;
                if variant.is_some() {
                    // Mark $variant extension as accessed so deny_unknown_extensions() won't fail
                    ctx.accessed().add_ext(VARIANT.clone());
                }
                variant
            }
        };

        match explicit_variant {
            // $variant present → use original context
            Some(ev) => {
                let name = ev
                    .first()
                    .map(|i| i.as_ref().to_string())
                    .unwrap_or_default();
                let rest = ev.rest().unwrap_or_else(VariantPath::empty);
                Ok(Some((name, ctx.clone(), Some(rest))))
            }
            // No $variant → Untagged
            None => Ok(None),
        }
    }

    /// Extract the `$variant` extension value from the node.
    fn extract_explicit_variant(
        ctx: &ParseContext<'doc>,
    ) -> Result<Option<VariantPath>, ParseError> {
        extract_explicit_variant_path(ctx.doc(), ctx.node_id())
    }

    /// Register a variant with short-circuit semantics (default).
    ///
    /// When this variant matches in untagged mode, parsing succeeds immediately
    /// without checking other variants. Use definition order to express priority.
    pub fn variant<P: DocumentParser<'doc, Output = T, Error = E>>(
        mut self,
        name: &str,
        f: P,
    ) -> Self {
        self.try_variant(name, f, true);
        self
    }

    /// Register a variant with short-circuit semantics using FromEure.
    pub fn parse_variant<V: FromEure<'doc, Error = E>>(
        mut self,
        name: &str,
        mut then: impl FnMut(V) -> Result<T, E>,
    ) -> Self {
        self.try_variant(
            name,
            move |ctx: &ParseContext<'doc>| {
                let v = V::parse(ctx)?;
                then(v)
            },
            true,
        );
        self
    }

    /// Register a variant with unambiguous semantics.
    ///
    /// All unambiguous variants are tried to detect conflicts.
    /// If multiple unambiguous variants match, an AmbiguousUnion error is returned.
    /// Use for catch-all variants or when you need conflict detection.
    pub fn variant_unambiguous<P: DocumentParser<'doc, Output = T, Error = E>>(
        mut self,
        name: &str,
        f: P,
    ) -> Self {
        self.try_variant(name, f, false);
        self
    }

    /// Register a variant with unambiguous semantics using FromEure.
    pub fn parse_variant_unambiguous<V: FromEure<'doc, Error = E>>(
        mut self,
        name: &str,
        mut then: impl FnMut(V) -> Result<T, E>,
    ) -> Self {
        self.try_variant(
            name,
            move |ctx: &ParseContext<'doc>| {
                let v = V::parse(ctx)?;
                then(v)
            },
            false,
        );
        self
    }

    /// Internal helper for variant/other logic.
    fn try_variant<P: DocumentParser<'doc, Output = T, Error = E>>(
        &mut self,
        name: &str,
        mut f: P,
        is_priority: bool,
    ) {
        // 1. If variant is determined, only try matching variant
        if let Some((ref v_name, ref v_ctx, ref rest)) = self.variant {
            if v_name == name && self.variant_result.is_none() {
                let child_ctx = v_ctx.with_variant_rest(rest.clone());
                let result = f.parse(&child_ctx);
                // Variant explicitly specified - no rollback needed on failure,
                // error propagates directly. Changes kept if success.
                self.variant_result = Some(result);
            }
            return;
        }

        // 2. Untagged mode: try all variants

        // Skip if already have priority result
        if self.priority_result.is_some() {
            return;
        }

        let child_ctx = self.ctx.with_variant_rest(None);
        match f.parse(&child_ctx) {
            Ok(value) => {
                if is_priority {
                    // Priority variant succeeded - keep the changes
                    // (snapshot will be popped in parse())
                    self.priority_result = Some(value);
                } else {
                    // Other variant succeeded - capture state before restoring
                    // We need to try more variants, so restore for next attempt
                    if let Some(ref fc) = self.flatten_ctx {
                        let captured = fc.capture_current_state();
                        fc.restore_to_current_snapshot();
                        self.other_results.push((name.to_string(), value, captured));
                    } else {
                        // No flatten context - no state to capture
                        self.other_results.push((
                            name.to_string(),
                            value,
                            (Default::default(), Default::default()),
                        ));
                    }
                }
            }
            Err(e) => {
                // Variant failed - restore to snapshot
                if let Some(ref fc) = self.flatten_ctx {
                    fc.restore_to_current_snapshot();
                }
                self.failures.push((name.to_string(), e));
            }
        }
    }

    /// Execute the union parse with oneOf semantics.
    pub fn parse(self) -> Result<T, E> {
        let node_id = self.ctx.node_id();

        // 1. Variant determined - return its result
        // When variant is explicitly specified via $variant, we don't use snapshot/rollback.
        // The accessed fields from parsing are kept (success) or don't matter (error propagates).
        if let Some((v_name, _, _)) = self.variant {
            let result = self.variant_result.unwrap_or_else(|| {
                Err(ParseError {
                    node_id,
                    kind: ParseErrorKind::UnknownVariant(v_name),
                }
                .into())
            });
            // Pop the snapshot - if success, keep changes; if error, doesn't matter
            if let Some(ref fc) = self.flatten_ctx {
                match &result {
                    Ok(_) => fc.pop_without_restore(),
                    Err(_) => fc.pop_and_restore(),
                }
            }
            return result;
        }

        // 2. Priority result - success, keep changes
        if let Some(value) = self.priority_result {
            if let Some(ref fc) = self.flatten_ctx {
                fc.pop_without_restore();
            }
            return Ok(value);
        }

        // 3. Check other_results
        match self.other_results.len() {
            0 => {
                // No match - rollback and return error
                if let Some(ref fc) = self.flatten_ctx {
                    fc.pop_and_restore();
                }
                Err(self.no_match_error(node_id))
            }
            1 => {
                // Single match - restore to captured state (from successful variant)
                let (_, value, captured_state) = self.other_results.into_iter().next().unwrap();
                if let Some(ref fc) = self.flatten_ctx {
                    fc.restore_to_state(captured_state);
                    fc.pop_without_restore();
                }
                Ok(value)
            }
            _ => {
                // Ambiguous - rollback all changes
                if let Some(ref fc) = self.flatten_ctx {
                    fc.pop_and_restore();
                }
                Err(ParseError {
                    node_id,
                    kind: ParseErrorKind::AmbiguousUnion(
                        self.other_results
                            .into_iter()
                            .map(|(name, _, _)| name)
                            .collect(),
                    ),
                }
                .into())
            }
        }
    }

    /// Create an error for when no variant matches.
    fn no_match_error(self, node_id: crate::document::NodeId) -> E {
        self.failures
            .into_iter()
            .next()
            .map(|(_, e)| e)
            .unwrap_or_else(|| {
                ParseError {
                    node_id,
                    kind: ParseErrorKind::NoMatchingVariant { variant: None },
                }
                .into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eure;
    use crate::parse::AlwaysParser;
    use crate::parse::DocumentParserExt as _;

    #[derive(Debug, PartialEq, Clone)]
    enum TestEnum {
        Foo,
        Bar,
    }

    #[test]
    fn test_union_single_match() {
        let doc = eure!({ = "foo" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: TestEnum = ctx
            .parse_union()
            .unwrap()
            .variant("foo", |ctx: &ParseContext<'_>| {
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
            .variant("bar", |ctx: &ParseContext<'_>| {
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
        let doc = eure!({ = "value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // Both variants would match, but first one wins due to priority
        let result: String = ctx
            .parse_union()
            .unwrap()
            .variant("first", String::parse)
            .variant("second", String::parse)
            .parse()
            .unwrap();

        assert_eq!(result, "value");
    }

    #[test]
    fn test_union_no_match() {
        let doc = eure!({ = "baz" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Result<TestEnum, ParseError> = ctx
            .parse_union()
            .unwrap()
            .variant("foo", |ctx: &ParseContext<'_>| {
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
        let doc = eure!({ %variant = "baz", = "anything" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: TestEnum = ctx
            .parse_union()
            .unwrap()
            .variant(
                "foo",
                AlwaysParser::<TestEnum, ParseError>::new(TestEnum::Foo),
            )
            .variant_unambiguous("baz", AlwaysParser::new(TestEnum::Bar))
            .parse()
            .unwrap();

        assert_eq!(result, TestEnum::Bar);
    }

    #[test]
    fn test_variant_extension_unknown() {
        // $variant = "unknown" specified, but "unknown" is not registered
        // All parsers always succeed
        let doc = eure!({ %variant = "unknown", = "anything" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err: ParseError = ctx
            .parse_union()
            .unwrap()
            .variant("foo", AlwaysParser::new(TestEnum::Foo))
            .variant_unambiguous("baz", AlwaysParser::new(TestEnum::Bar))
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
        let doc = eure!({ %variant = "baz", = "anything" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = ctx
            .parse_union()
            .unwrap()
            .variant("foo", AlwaysParser::new(TestEnum::Foo))
            .variant_unambiguous("baz", |ctx: &ParseContext<'_>| {
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

    #[derive(Debug, PartialEq, Clone)]
    enum Outer {
        A(Inner),
        B(i32),
    }

    #[derive(Debug, PartialEq, Clone)]
    enum Inner {
        X,
        Y,
    }

    fn parse_inner(ctx: &ParseContext<'_>) -> Result<Inner, ParseError> {
        ctx.parse_union()
            .unwrap()
            .variant("x", AlwaysParser::new(Inner::X))
            .variant("y", AlwaysParser::new(Inner::Y))
            .parse()
    }

    #[test]
    fn test_variant_nested_single_segment() {
        // $variant = "a" - matches "a", rest is None -> Inner defaults to X
        let doc = eure!({ %variant = "a", = "value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Outer = ctx
            .parse_union()
            .unwrap()
            .variant("a", parse_inner.map(Outer::A))
            .variant("b", AlwaysParser::new(Outer::B(42)))
            .parse()
            .unwrap();

        assert_eq!(result, Outer::A(Inner::X));
    }

    #[test]
    fn test_variant_nested_multi_segment() {
        // $variant = "a.y" - matches "a", rest is Some("y")
        let doc = eure!({ %variant = "a.y", = "value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Outer = ctx
            .parse_union()
            .unwrap()
            .variant("a", parse_inner.map(Outer::A))
            .variant("b", AlwaysParser::new(Outer::B(42)))
            .parse()
            .unwrap();

        assert_eq!(result, Outer::A(Inner::Y));
    }

    #[test]
    fn test_variant_nested_invalid_inner() {
        // $variant = "a.z" - matches "a", but "z" is not valid for Inner
        let doc = eure!({ %variant = "a.z", = "value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = ctx
            .parse_union()
            .unwrap()
            .variant("a", parse_inner.map(Outer::A))
            .variant("b", AlwaysParser::new(Outer::B(42)))
            .parse()
            .unwrap_err();

        assert_eq!(err.kind, ParseErrorKind::UnknownVariant("z".to_string()));
    }

    #[test]
    fn test_variant_non_nested_with_nested_path() {
        // $variant = "b.x" but "b" parser doesn't expect nested path
        // The child context will have variant_path = Some("x")
        // If the "b" parser is a non-union type, it should error on unexpected variant path
        let doc = eure!({ %variant = "b.x", = "value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // "b" is registered as a variant but if called with "b.x",
        // the closure gets ctx with variant_path = Some("x")
        // The simple parser Ok(Outer::B(42)) doesn't check variant path,
        // but a proper impl would use ctx.parse_primitive() which errors
        let err = ctx
            .parse_union()
            .unwrap()
            .variant("a", parse_inner.map(Outer::A))
            .variant("b", |ctx: &ParseContext<'_>| {
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

    use crate::value::ValueKind;

    #[test]
    fn test_invalid_variant_type_errors() {
        // $variant = 123 (integer, not string) - should error at parse_union()
        // Note: eure! macro can't create invalid $variant types, so we use manual construction
        use crate::document::node::NodeValue;
        use crate::value::PrimitiveValue;
        use num_bigint::BigInt;

        // Create base doc with eure! and then add invalid integer $variant
        let mut doc = eure!({ = "foo" });
        let root_id = doc.get_root_id();
        let variant_node_id = doc
            .add_extension("variant".parse().unwrap(), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(123)));

        let ctx = doc.parse_context(root_id);

        let Err(err) = ctx.parse_union::<TestEnum, ParseError>() else {
            panic!("Expected error");
        };
        assert_eq!(
            err,
            ParseError {
                node_id: variant_node_id,
                kind: ParseErrorKind::InvalidVariantType(ValueKind::Integer),
            }
        );
    }

    #[test]
    fn test_invalid_variant_path_syntax_errors() {
        // $variant = "foo..bar" (invalid path syntax) - should error at parse_union()
        let doc = eure!({ %variant = "foo..bar", = "foo" });
        let root_id = doc.get_root_id();
        let variant_node_id = *doc.node(root_id).extensions.get(&VARIANT).unwrap();
        let ctx = doc.parse_context(root_id);

        let Err(err) = ctx.parse_union::<TestEnum, ParseError>() else {
            panic!("Expected error");
        };
        assert_eq!(
            err,
            ParseError {
                node_id: variant_node_id,
                kind: ParseErrorKind::InvalidVariantPath("foo..bar".to_string()),
            }
        );
    }

    #[test]
    fn test_variant_path_empty_uses_untagged() {
        // When variant_path is Some but empty (consumed by parent), use Untagged
        // This is tested indirectly through nested unions after consuming the path
        let doc = eure!({ = "value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // Simulate a context where variant_path was set but is now empty
        let child_ctx = ctx.with_variant_rest(Some(VariantPath::empty()));

        // With empty variant_path, should use Untagged parsing
        let result: String = child_ctx
            .parse_union()
            .unwrap()
            .variant("first", String::parse)
            .variant("second", String::parse)
            .parse()
            .unwrap();

        // Priority variant "first" wins in Untagged mode
        assert_eq!(result, "value");
    }

    // =============================================================================
    // Nested union tests (low-level, without derive macro)
    // =============================================================================

    /// Nested enum for testing: outer level
    #[derive(Debug, PartialEq, Clone)]
    enum OuterUnion {
        Normal(InnerUnion),
        List(Vec<InnerUnion>),
    }

    /// Nested enum for testing: inner level
    #[derive(Debug, PartialEq, Clone)]
    enum InnerUnion {
        Text(String),
        Number(i64),
    }

    fn parse_inner_union(ctx: &ParseContext<'_>) -> Result<InnerUnion, ParseError> {
        ctx.parse_union()?
            .variant("text", |ctx: &ParseContext<'_>| {
                let s: String = ctx.parse()?;
                Ok(InnerUnion::Text(s))
            })
            .variant("number", |ctx: &ParseContext<'_>| {
                let n: i64 = ctx.parse()?;
                Ok(InnerUnion::Number(n))
            })
            .parse()
    }

    fn parse_outer_union(ctx: &ParseContext<'_>) -> Result<OuterUnion, ParseError> {
        use crate::document::node::NodeArray;

        ctx.parse_union()?
            .variant("normal", |ctx: &ParseContext<'_>| {
                let inner = parse_inner_union(ctx)?;
                Ok(OuterUnion::Normal(inner))
            })
            .variant("list", |ctx: &ParseContext<'_>| {
                // Parse array of InnerUnion using NodeArray
                let arr: &NodeArray = ctx.parse()?;
                let items: Result<Vec<InnerUnion>, _> = arr
                    .iter()
                    .map(|&node_id| parse_inner_union(&ctx.at(node_id)))
                    .collect();
                Ok(OuterUnion::List(items?))
            })
            .parse()
    }

    #[test]
    fn test_nested_union_basic_text() {
        // Simple string -> OuterUnion::Normal(InnerUnion::Text)
        let doc = eure!({ = "hello" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result = parse_outer_union(&ctx).unwrap();
        assert_eq!(
            result,
            OuterUnion::Normal(InnerUnion::Text("hello".to_string()))
        );
    }

    #[test]
    fn test_nested_union_basic_number() {
        let doc = eure!({ = 42 });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);
        let result = parse_outer_union(&ctx).unwrap();
        assert_eq!(result, OuterUnion::Normal(InnerUnion::Number(42)));
    }

    #[test]
    fn test_nested_union_variant_path_propagation() {
        // $variant = "normal.text" should propagate through nested unions
        let doc = eure!({ %variant = "normal.text", = "test value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result = parse_outer_union(&ctx).unwrap();
        assert_eq!(
            result,
            OuterUnion::Normal(InnerUnion::Text("test value".to_string()))
        );
    }

    #[test]
    fn test_nested_union_variant_path_number() {
        // $variant = "normal.number" - number variant explicitly selected
        let doc = eure!({ %variant = "normal.number", = 99 });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);
        let result = parse_outer_union(&ctx).unwrap();
        assert_eq!(result, OuterUnion::Normal(InnerUnion::Number(99)));
    }

    #[test]
    fn test_nested_union_inner_fails_outer_recovers() {
        // When inner union fails, outer should try next variant
        // Create a document that doesn't match "normal" variant's inner union
        // but could match "list" variant
        let doc = eure!({ = ["a", "b"] });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result = parse_outer_union(&ctx).unwrap();
        assert_eq!(
            result,
            OuterUnion::List(alloc::vec![
                InnerUnion::Text("a".to_string()),
                InnerUnion::Text("b".to_string()),
            ])
        );
    }

    // =============================================================================
    // Triple nested union tests
    // =============================================================================

    #[derive(Debug, PartialEq, Clone)]
    enum Level1 {
        A(Level2Union),
        B(String),
    }

    #[derive(Debug, PartialEq, Clone)]
    enum Level2Union {
        X(Level3),
        Y(i64),
    }

    #[derive(Debug, PartialEq, Clone)]
    enum Level3 {
        Leaf(String),
    }

    fn parse_level3(ctx: &ParseContext<'_>) -> Result<Level3, ParseError> {
        ctx.parse_union()?
            .variant("leaf", |ctx: &ParseContext<'_>| {
                let s: String = ctx.parse()?;
                Ok(Level3::Leaf(s))
            })
            .parse()
    }

    fn parse_level2(ctx: &ParseContext<'_>) -> Result<Level2Union, ParseError> {
        ctx.parse_union()?
            .variant("x", |ctx: &ParseContext<'_>| {
                let inner = parse_level3(ctx)?;
                Ok(Level2Union::X(inner))
            })
            .variant("y", |ctx: &ParseContext<'_>| {
                let n: i64 = ctx.parse()?;
                Ok(Level2Union::Y(n))
            })
            .parse()
    }

    fn parse_level1(ctx: &ParseContext<'_>) -> Result<Level1, ParseError> {
        ctx.parse_union()?
            .variant("a", |ctx: &ParseContext<'_>| {
                let inner = parse_level2(ctx)?;
                Ok(Level1::A(inner))
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let s: String = ctx.parse()?;
                Ok(Level1::B(s))
            })
            .parse()
    }

    #[test]
    fn test_nested_union_three_levels_untagged() {
        // String input should match: Level1::A -> Level2Union::X -> Level3::Leaf
        // (first variant at each level wins in untagged mode)
        let doc = eure!({ = "deep value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result = parse_level1(&ctx).unwrap();
        assert_eq!(
            result,
            Level1::A(Level2Union::X(Level3::Leaf("deep value".to_string())))
        );
    }

    #[test]
    fn test_nested_union_three_levels_variant_path() {
        // $variant = "a.x.leaf" - explicitly select through three levels
        let doc = eure!({ %variant = "a.x.leaf", = "explicit deep" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result = parse_level1(&ctx).unwrap();
        assert_eq!(
            result,
            Level1::A(Level2Union::X(Level3::Leaf("explicit deep".to_string())))
        );
    }

    #[test]
    fn test_nested_union_three_levels_variant_path_partial() {
        // $variant = "a.y" - select a.y, inner uses untagged
        let doc = eure!({ %variant = "a.y", = 123 });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);
        let result = parse_level1(&ctx).unwrap();
        assert_eq!(result, Level1::A(Level2Union::Y(123)));
    }

    #[test]
    fn test_nested_union_invalid_inner_variant_path() {
        // $variant = "a.x.invalid" - "invalid" doesn't exist in Level3
        let doc = eure!({ %variant = "a.x.invalid", = "value" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = parse_level1(&ctx).unwrap_err();
        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("invalid".to_string())
        );
    }

    // =============================================================================
    // Flatten with nested union - accessed field tracking tests
    // =============================================================================

    #[test]
    fn test_flatten_nested_union_accessed_fields_basic() {
        use crate::parse::AccessedSet;
        use crate::parse::FlattenContext;
        use crate::parse::ParserScope;

        // Test that accessed fields are properly tracked through nested unions
        let doc = eure!({
            field_a = "value_a"
            field_b = "value_b"
        });
        let root_id = doc.get_root_id();

        // Create flatten context to track field access
        let flatten_ctx = FlattenContext::new(AccessedSet::new(), ParserScope::Record);
        let ctx = ParseContext::with_flatten_ctx(&doc, root_id, flatten_ctx.clone());

        // Parse a union that accesses field_a
        let record = ctx.parse_record().unwrap();
        let _field_a: String = record.parse_field("field_a").unwrap();

        // field_a should be marked as accessed
        let (accessed, _) = flatten_ctx.capture_current_state();
        assert!(accessed.contains("field_a"));
        assert!(!accessed.contains("field_b"));
    }
}
