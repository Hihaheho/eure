//! UnionParser for parsing union types from Eure documents.
//!
//! Implements oneOf semantics with priority-based ambiguity resolution.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::data_model::VariantRepr;
use crate::document::node::NodeValue;
use crate::document::{EureDocument, NodeId};
use crate::identifier::Identifier;
use crate::parse::{DocumentParser, ParseDocument};
use crate::value::ObjectKey;

use super::variant_path::VariantPath;
use super::{
    AccessedSnapshot, FlattenContext, ParseContext, ParseError, ParseErrorKind, UnionTagMode,
};

/// The `$variant` extension identifier.
pub const VARIANT: Identifier = Identifier::new_unchecked("variant");

// =============================================================================
// Shared variant extraction helpers (used by both parsing and validation)
// =============================================================================

/// Extract variant name and content node from repr pattern.
///
/// Returns:
/// - `Ok(Some((name, content_node_id)))` - pattern matched
/// - `Ok(None)` - pattern did not match (not a map, wrong structure, etc.)
/// - `Err(...)` - tag field exists but has invalid type
pub fn extract_repr_variant(
    doc: &EureDocument,
    node_id: NodeId,
    repr: &VariantRepr,
) -> Result<Option<(String, NodeId)>, ParseError> {
    match repr {
        VariantRepr::Untagged => Ok(None),
        VariantRepr::External => Ok(try_extract_external(doc, node_id)),
        VariantRepr::Internal { tag } => try_extract_internal(doc, node_id, tag),
        VariantRepr::Adjacent { tag, content } => try_extract_adjacent(doc, node_id, tag, content),
    }
}

/// Try to extract External repr: `{ variant_name = content }`
fn try_extract_external(doc: &EureDocument, node_id: NodeId) -> Option<(String, NodeId)> {
    let node = doc.node(node_id);
    let NodeValue::Map(map) = &node.content else {
        return None;
    };

    if map.len() != 1 {
        return None;
    }

    let (key, &content_node_id) = map.iter().next()?;
    let ObjectKey::String(variant_name) = key else {
        return None;
    };
    Some((variant_name.clone(), content_node_id))
}

/// Try to extract Internal repr: `{ type = "variant_name", ...fields... }`
///
/// Returns the same node_id as content - the tag field should be excluded during record parsing/validation.
fn try_extract_internal(
    doc: &EureDocument,
    node_id: NodeId,
    tag: &str,
) -> Result<Option<(String, NodeId)>, ParseError> {
    let node = doc.node(node_id);
    let NodeValue::Map(map) = &node.content else {
        return Ok(None);
    };

    let tag_key = ObjectKey::String(tag.to_string());
    let Some(tag_node_id) = map.get(&tag_key) else {
        return Ok(None);
    };

    let variant_name: &str = doc.parse(tag_node_id)?;
    Ok(Some((variant_name.to_string(), node_id)))
}

/// Try to extract Adjacent repr: `{ type = "variant_name", content = {...} }`
fn try_extract_adjacent(
    doc: &EureDocument,
    node_id: NodeId,
    tag: &str,
    content: &str,
) -> Result<Option<(String, NodeId)>, ParseError> {
    let node = doc.node(node_id);
    let NodeValue::Map(map) = &node.content else {
        return Ok(None);
    };

    let tag_key = ObjectKey::String(tag.to_string());
    let Some(tag_node_id) = map.get(&tag_key) else {
        return Ok(None);
    };

    let variant_name: &str = doc.parse(tag_node_id)?;

    let content_key = ObjectKey::String(content.to_string());
    let Some(content_node_id) = map.get(&content_key) else {
        return Ok(None);
    };

    Ok(Some((variant_name.to_string(), content_node_id)))
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
/// - When `$variant` extension or repr is specified, matches by name directly
///
/// # Variant Resolution
///
/// Variant is determined by combining `$variant` extension and `VariantRepr`:
/// - Both agree on same name → use repr's context (with tag excluded for Internal)
/// - `$variant` only (repr didn't extract) → use original context
/// - Repr only → use repr's context
/// - Conflict (different names) → `ConflictingVariantTags` error
/// - Neither → Untagged parsing (try all variants)
///
/// # Example
///
/// ```ignore
/// impl<'doc> ParseDocument<'doc> for Description {
///     fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
///         ctx.parse_union(VariantRepr::default())?
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
    /// Create a new UnionParser for the given context and repr.
    ///
    /// Returns error if:
    /// - `$variant` extension has invalid type or syntax
    /// - `$variant` and repr extract conflicting variant names
    pub(crate) fn new(
        ctx: &'ctx ParseContext<'doc>,
        repr: VariantRepr,
    ) -> Result<Self, ParseError> {
        let variant = Self::resolve_variant(ctx, &repr)?;

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

    /// Resolve the unified variant from `$variant` extension and repr.
    ///
    /// Returns:
    /// - `Some((name, ctx, rest))` if variant is determined
    /// - `None` for Untagged parsing
    ///
    /// The behavior depends on `UnionTagMode`:
    /// - `Eure`: Use `$variant` extension or untagged matching (ignore repr)
    /// - `Repr`: Use only repr patterns (ignore `$variant`, no untagged fallback)
    fn resolve_variant(
        ctx: &ParseContext<'doc>,
        repr: &VariantRepr,
    ) -> Result<Option<(String, ParseContext<'doc>, Option<VariantPath>)>, ParseError> {
        match ctx.union_tag_mode() {
            UnionTagMode::Eure => Self::resolve_variant_eure_mode(ctx),
            UnionTagMode::Repr => Self::resolve_variant_repr_mode(ctx, repr),
        }
    }

    /// Resolve variant in Eure mode: `$variant` extension or untagged matching.
    ///
    /// In this mode:
    /// - If `$variant` extension is present, use it to determine the variant
    /// - Otherwise, use untagged matching (try all variants)
    /// - `VariantRepr` is ignored
    fn resolve_variant_eure_mode(
        ctx: &ParseContext<'doc>,
    ) -> Result<Option<(String, ParseContext<'doc>, Option<VariantPath>)>, ParseError> {
        // Check if variant path is already set in context (from parent union)
        let explicit_variant = match ctx.variant_path() {
            Some(vp) if !vp.is_empty() => Some(vp.clone()),
            Some(_) => None, // Empty path = variant consumed, use Untagged
            None => Self::extract_explicit_variant(ctx)?,
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

    /// Resolve variant in Repr mode: use only `VariantRepr` patterns.
    ///
    /// In this mode:
    /// - Extract variant tag using `VariantRepr` (External, Internal, Adjacent)
    /// - `$variant` extension is ignored
    /// - If repr doesn't extract a tag, return `None` (will result in NoMatchingVariant error)
    fn resolve_variant_repr_mode(
        ctx: &ParseContext<'doc>,
        repr: &VariantRepr,
    ) -> Result<Option<(String, ParseContext<'doc>, Option<VariantPath>)>, ParseError> {
        // Extract repr_variant using shared helper
        let repr_variant = extract_repr_variant(ctx.doc(), ctx.node_id(), repr)?;

        match repr_variant {
            // Repr extracted a tag → use repr's context
            Some((name, content_node_id)) => {
                let content_ctx = Self::make_content_context(ctx, repr, content_node_id);
                Ok(Some((name, content_ctx, Some(VariantPath::empty()))))
            }
            // Repr didn't extract → no tag (will be handled as untagged, but in repr mode
            // this should result in an error for non-Untagged reprs)
            None => {
                // For non-Untagged reprs, the structure doesn't match the expected pattern
                // Return None to trigger untagged parsing, which will fail if no variant matches
                Ok(None)
            }
        }
    }

    /// Create ParseContext for variant content based on repr type.
    fn make_content_context(
        ctx: &ParseContext<'doc>,
        repr: &VariantRepr,
        content_node_id: NodeId,
    ) -> ParseContext<'doc> {
        match repr {
            // Internal repr: mark tag field as accessed in shared context
            // This way deny_unknown_fields won't complain about the tag
            VariantRepr::Internal { tag } => {
                // Get or create flatten context, add tag to accessed fields
                let flatten_ctx = match ctx.flatten_ctx() {
                    Some(fc) => {
                        fc.add_field(tag);
                        fc.clone()
                    }
                    None => {
                        let fc = super::FlattenContext::new();
                        fc.add_field(tag);
                        fc
                    }
                };
                ParseContext::with_flatten_ctx(
                    ctx.doc(),
                    content_node_id,
                    flatten_ctx,
                    ctx.union_tag_mode(),
                )
            }
            // Other reprs: just use the content node
            _ => ctx.at(content_node_id),
        }
    }

    /// Extract the `$variant` extension value from the node.
    fn extract_explicit_variant(
        ctx: &ParseContext<'doc>,
    ) -> Result<Option<VariantPath>, ParseError> {
        let node = ctx.node();
        let Some(&variant_node_id) = node.extensions.get(&VARIANT) else {
            return Ok(None);
        };

        let variant_node = ctx.doc().node(variant_node_id);
        let s: &str = ctx.doc().parse(variant_node_id).map_err(|_| ParseError {
            node_id: variant_node_id,
            kind: ParseErrorKind::InvalidVariantType(
                variant_node
                    .content
                    .value_kind()
                    .unwrap_or(crate::value::ValueKind::Null),
            ),
        })?;

        VariantPath::parse(s).map(Some).map_err(|_| ParseError {
            node_id: variant_node_id,
            kind: ParseErrorKind::InvalidVariantPath(s.to_string()),
        })
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

    /// Register a variant with short-circuit semantics using ParseDocument.
    pub fn parse_variant<V: ParseDocument<'doc, Error = E>>(
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

    /// Register a variant with unambiguous semantics using ParseDocument.
    pub fn parse_variant_unambiguous<V: ParseDocument<'doc, Error = E>>(
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
    use crate::document::EureDocument;
    use crate::document::node::NodeValue;
    use crate::parse::AlwaysParser;
    use crate::parse::DocumentParserExt as _;
    use crate::text::Text;
    use crate::value::PrimitiveValue;

    fn identifier(s: &str) -> Identifier {
        s.parse().unwrap()
    }

    #[derive(Debug, PartialEq, Clone)]
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
            .parse_union(VariantRepr::default())
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
        let doc = create_text_doc("value");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // Both variants would match, but first one wins due to priority
        let result: String = ctx
            .parse_union(VariantRepr::default())
            .unwrap()
            .variant("first", String::parse)
            .variant("second", String::parse)
            .parse()
            .unwrap();

        assert_eq!(result, "value");
    }

    #[test]
    fn test_union_no_match() {
        let doc = create_text_doc("baz");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Result<TestEnum, ParseError> = ctx
            .parse_union(VariantRepr::default())
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
        let doc = create_doc_with_variant("anything", "baz");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: TestEnum = ctx
            .parse_union(VariantRepr::default())
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
        let doc = create_doc_with_variant("anything", "unknown");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err: ParseError = ctx
            .parse_union(VariantRepr::default())
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
        let doc = create_doc_with_variant("anything", "baz");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = ctx
            .parse_union(VariantRepr::default())
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
        ctx.parse_union(VariantRepr::default())
            .unwrap()
            .variant("x", AlwaysParser::new(Inner::X))
            .variant("y", AlwaysParser::new(Inner::Y))
            .parse()
    }

    #[test]
    fn test_variant_nested_single_segment() {
        // $variant = "a" - matches "a", rest is None -> Inner defaults to X
        let doc = create_doc_with_variant("value", "a");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Outer = ctx
            .parse_union(VariantRepr::default())
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
        let doc = create_doc_with_variant("value", "a.y");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let result: Outer = ctx
            .parse_union(VariantRepr::default())
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
        let doc = create_doc_with_variant("value", "a.z");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let err = ctx
            .parse_union(VariantRepr::default())
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
        let doc = create_doc_with_variant("value", "b.x");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // "b" is registered as a variant but if called with "b.x",
        // the closure gets ctx with variant_path = Some("x")
        // The simple parser Ok(Outer::B(42)) doesn't check variant path,
        // but a proper impl would use ctx.parse_primitive() which errors
        let err = ctx
            .parse_union(VariantRepr::default())
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

    /// Create a document with $variant set to an integer (invalid type).
    /// Returns (doc, variant_node_id) for error assertion.
    fn create_doc_with_integer_variant(
        content: &str,
        variant_value: i64,
    ) -> (EureDocument, crate::document::NodeId) {
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

        (doc, variant_node_id)
    }

    /// Create a document with $variant extension.
    /// Returns (doc, variant_node_id) for error assertion.
    fn create_doc_with_variant_ext(
        content: &str,
        variant: &str,
    ) -> (EureDocument, crate::document::NodeId) {
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

        (doc, variant_node_id)
    }

    #[test]
    fn test_invalid_variant_type_errors() {
        // $variant = 123 (integer, not string) - should error at parse_union()
        let (doc, variant_node_id) = create_doc_with_integer_variant("foo", 123);
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let Err(err) = ctx.parse_union::<TestEnum, ParseError>(VariantRepr::default()) else {
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
        let (doc, variant_node_id) = create_doc_with_variant_ext("foo", "foo..bar");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        let Err(err) = ctx.parse_union::<TestEnum, ParseError>(VariantRepr::default()) else {
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

    // --- VariantRepr tests ---

    use crate::eure;
    use crate::value::ObjectKey;

    #[derive(Debug, PartialEq)]
    enum ReprTestEnum {
        A { value: i64 },
        B { name: String },
    }

    fn parse_repr_test_enum(
        ctx: &ParseContext<'_>,
        repr: VariantRepr,
    ) -> Result<ReprTestEnum, ParseError> {
        ctx.parse_union(repr)?
            .variant("a", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value: i64 = rec.parse_field("value")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::A { value })
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let name: String = rec.parse_field("name")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::B { name })
            })
            .parse()
    }

    /// Create a document with Internal repr: { type = "a", value = 42 }
    fn create_internal_repr_doc(type_val: &str, value: i64) -> EureDocument {
        use num_bigint::BigInt;

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Map(Default::default());

        // Add "type" field
        let type_node_id = doc
            .add_map_child(ObjectKey::String("type".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(type_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(type_val.to_string())));

        // Add "value" field
        let value_node_id = doc
            .add_map_child(ObjectKey::String("value".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(value_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(value)));

        doc
    }

    /// Create a document with External repr: { a = { value = 42 } }
    fn create_external_repr_doc(variant_name: &str, value: i64) -> EureDocument {
        use num_bigint::BigInt;

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Map(Default::default());

        // Add variant container
        let variant_node_id = doc
            .add_map_child(ObjectKey::String(variant_name.to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content = NodeValue::Map(Default::default());

        // Add "value" field inside variant
        let value_node_id = doc
            .add_map_child(ObjectKey::String("value".to_string()), variant_node_id)
            .unwrap()
            .node_id;
        doc.node_mut(value_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(value)));

        doc
    }

    /// Create a document with Adjacent repr: { type = "a", content = { value = 42 } }
    fn create_adjacent_repr_doc(type_val: &str, value: i64) -> EureDocument {
        use num_bigint::BigInt;

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Map(Default::default());

        // Add "type" field
        let type_node_id = doc
            .add_map_child(ObjectKey::String("type".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(type_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(type_val.to_string())));

        // Add "content" container
        let content_node_id = doc
            .add_map_child(ObjectKey::String("content".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(content_node_id).content = NodeValue::Map(Default::default());

        // Add "value" field inside content
        let value_node_id = doc
            .add_map_child(ObjectKey::String("value".to_string()), content_node_id)
            .unwrap()
            .node_id;
        doc.node_mut(value_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(value)));

        doc
    }

    #[test]
    fn test_internal_repr_success() {
        // { type = "a", value = 42 } with Internal { tag: "type" }
        // Using Repr mode to enable repr-based variant resolution
        let doc = create_internal_repr_doc("a", 42);
        let root_id = doc.get_root_id();
        let ctx = ParseContext::with_union_tag_mode(&doc, root_id, UnionTagMode::Repr);

        let result = parse_repr_test_enum(
            &ctx,
            VariantRepr::Internal {
                tag: "type".to_string(),
            },
        );
        assert_eq!(result.unwrap(), ReprTestEnum::A { value: 42 });
    }

    #[test]
    fn test_external_repr_success() {
        // { a = { value = 42 } } with External
        // Using Repr mode to enable repr-based variant resolution
        let doc = create_external_repr_doc("a", 42);
        let root_id = doc.get_root_id();
        let ctx = ParseContext::with_union_tag_mode(&doc, root_id, UnionTagMode::Repr);

        let result = parse_repr_test_enum(&ctx, VariantRepr::External);
        assert_eq!(result.unwrap(), ReprTestEnum::A { value: 42 });
    }

    #[test]
    fn test_adjacent_repr_success() {
        // { type = "a", content = { value = 42 } } with Adjacent { tag: "type", content: "content" }
        // Using Repr mode to enable repr-based variant resolution
        let doc = create_adjacent_repr_doc("a", 42);
        let root_id = doc.get_root_id();
        let ctx = ParseContext::with_union_tag_mode(&doc, root_id, UnionTagMode::Repr);

        let result = parse_repr_test_enum(
            &ctx,
            VariantRepr::Adjacent {
                tag: "type".to_string(),
                content: "content".to_string(),
            },
        );
        assert_eq!(result.unwrap(), ReprTestEnum::A { value: 42 });
    }

    #[test]
    fn test_repr_mode_ignores_variant_extension() {
        // In Repr mode, $variant extension is ignored - only repr pattern is used
        let mut doc = create_internal_repr_doc("a", 42);
        let root_id = doc.get_root_id();

        // Add $variant = "b" extension (would conflict in old behavior)
        let variant_node_id = doc
            .add_extension(identifier("variant"), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("b".to_string())));

        // In Repr mode, $variant is ignored, so repr extracts "a" and variant "a" is matched
        let ctx = ParseContext::with_union_tag_mode(&doc, root_id, UnionTagMode::Repr);

        let result = parse_repr_test_enum(
            &ctx,
            VariantRepr::Internal {
                tag: "type".to_string(),
            },
        );
        assert_eq!(result.unwrap(), ReprTestEnum::A { value: 42 });
    }

    #[test]
    fn test_eure_mode_ignores_repr() {
        // In Eure mode (default), repr is ignored - only $variant or untagged matching is used
        let doc = create_internal_repr_doc("a", 42);
        let root_id = doc.get_root_id();

        // Default mode is Eure, which ignores repr
        let ctx = doc.parse_context(root_id);

        // Since there's no $variant and repr is ignored, this becomes untagged matching
        // Both variants will be tried, and "a" has a "value" field so it should match
        let result = ctx
            .parse_union::<_, ParseError>(VariantRepr::Internal {
                tag: "type".to_string(),
            })
            .unwrap()
            .variant("a", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value: i64 = rec.parse_field("value")?;
                rec.allow_unknown_fields()?;
                Ok(ReprTestEnum::A { value })
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let name: String = rec.parse_field("name")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::B { name })
            })
            .parse();

        assert_eq!(result.unwrap(), ReprTestEnum::A { value: 42 });
    }

    #[test]
    fn test_internal_repr_unknown_variant_name() {
        // { type = "unknown", value = 42 } - "unknown" is not a registered variant
        // Using Repr mode to enable repr-based variant resolution
        let doc = create_internal_repr_doc("unknown", 42);
        let root_id = doc.get_root_id();
        let ctx = ParseContext::with_union_tag_mode(&doc, root_id, UnionTagMode::Repr);

        let result = parse_repr_test_enum(
            &ctx,
            VariantRepr::Internal {
                tag: "type".to_string(),
            },
        );

        // Should get UnknownVariant error since repr extracts "unknown"
        let err = result.unwrap_err();
        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("unknown".to_string())
        );
    }

    #[test]
    fn test_repr_not_extracted_falls_back_to_untagged() {
        // Document has 2 keys, so External repr (requires exactly 1 key) won't match
        // Falls back to Untagged parsing
        let doc = eure!({ value = 100, extra = "ignored" });
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // External repr won't match (2 keys), so Untagged will try each variant
        let result = ctx
            .parse_union::<_, ParseError>(VariantRepr::External)
            .unwrap()
            .variant("a", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value: i64 = rec.parse_field("value")?;
                // Don't deny_unknown_fields - we have "extra"
                Ok(ReprTestEnum::A { value })
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let name: String = rec.parse_field("name")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::B { name })
            })
            .parse();

        // Untagged parsing should succeed with variant "a"
        assert_eq!(result.unwrap(), ReprTestEnum::A { value: 100 });
    }

    #[test]
    fn test_external_repr_single_key_extracts_variant() {
        // Document has exactly 1 key, so External repr extracts it as variant name
        // Using Repr mode to enable repr-based variant resolution
        let doc = eure!({ value = 100 });
        let root_id = doc.get_root_id();
        let ctx = ParseContext::with_union_tag_mode(&doc, root_id, UnionTagMode::Repr);

        // External repr extracts "value" as variant name
        // Since "value" is not a registered variant, we get UnknownVariant
        let err: ParseError = ctx
            .parse_union(VariantRepr::External)
            .unwrap()
            .variant("a", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value: i64 = rec.parse_field("value")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::A { value })
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let name: String = rec.parse_field("name")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::B { name })
            })
            .parse()
            .unwrap_err();

        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("value".to_string())
        );
    }

    // --- Corner case tests for resolve_variant ---

    #[test]
    fn test_internal_repr_tag_is_integer_errors() {
        // { type = 123, value = 42 } - tag field is integer, not string
        // Using Repr mode to enable repr-based variant resolution
        use num_bigint::BigInt;

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Map(Default::default());

        // Add "type" field with integer value (invalid!)
        let type_node_id = doc
            .add_map_child(ObjectKey::String("type".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(type_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(123)));

        // Add "value" field
        let value_node_id = doc
            .add_map_child(ObjectKey::String("value".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(value_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(42)));

        let ctx = ParseContext::with_union_tag_mode(&doc, root_id, UnionTagMode::Repr);

        // Internal repr should error because tag field is not a string
        let Err(err) = ctx.parse_union::<ReprTestEnum, ParseError>(VariantRepr::Internal {
            tag: "type".to_string(),
        }) else {
            panic!("Expected error");
        };

        // Error should point to the tag node
        assert_eq!(err.node_id, type_node_id);
    }

    #[test]
    fn test_adjacent_repr_missing_content_falls_back_to_untagged() {
        // { type = "a", value = 42 } - has tag but no "content" field
        // Adjacent repr should not match, falls back to Untagged
        let doc = create_internal_repr_doc("a", 42); // This has "type" and "value", not "content"
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // Adjacent repr won't match (no "content" key), so Untagged parsing
        let result = ctx
            .parse_union::<_, ParseError>(VariantRepr::Adjacent {
                tag: "type".to_string(),
                content: "content".to_string(),
            })
            .unwrap()
            .variant("a", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value: i64 = rec.parse_field("value")?;
                // Don't deny_unknown_fields - we have "type"
                Ok(ReprTestEnum::A { value })
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let name: String = rec.parse_field("name")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::B { name })
            })
            .parse();

        // Untagged parsing should succeed with variant "a"
        assert_eq!(result.unwrap(), ReprTestEnum::A { value: 42 });
    }

    #[test]
    fn test_external_repr_non_string_key_falls_back_to_untagged() {
        // { 123 => { value = 42 } } - key is integer, not string
        use num_bigint::BigInt;

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Map(Default::default());

        // Add integer key
        let variant_node_id = doc
            .add_map_child(ObjectKey::Number(BigInt::from(123)), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content = NodeValue::Map(Default::default());

        // Add "value" field inside
        let value_node_id = doc
            .add_map_child(ObjectKey::String("value".to_string()), variant_node_id)
            .unwrap()
            .node_id;
        doc.node_mut(value_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(42)));

        let ctx = doc.parse_context(root_id);

        // External repr won't match (key is not string), but since it has 1 key,
        // it will still try External extraction which fails due to non-string key,
        // then fall back to Untagged parsing which also fails (no matching variant)
        let err: ParseError = ctx
            .parse_union(VariantRepr::External)
            .unwrap()
            .variant("a", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value: i64 = rec.parse_field("value")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::A { value })
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let name: String = rec.parse_field("name")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::B { name })
            })
            .parse()
            .unwrap_err();

        // Falls back to Untagged, variant "a" tried but "value" not at root level
        assert_eq!(err.kind, ParseErrorKind::MissingField("value".to_string()));
    }

    #[test]
    fn test_eure_mode_uses_variant_extension_over_repr() {
        // In Eure mode (default), $variant extension is used and repr is ignored
        // Internal repr would extract "a", but $variant = "b" takes precedence
        let mut doc = create_internal_repr_doc("a", 42);
        let root_id = doc.get_root_id();

        // Add $variant = "b" extension
        let variant_node_id = doc
            .add_extension(identifier("variant"), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("b".to_string())));

        let ctx = doc.parse_context(root_id);

        // In Eure mode, $variant = "b" is used (repr is ignored)
        // Since "b" expects a "name" field and doc has "value", this fails
        let err: ParseError = ctx
            .parse_union(VariantRepr::Internal {
                tag: "type".to_string(),
            })
            .unwrap()
            .variant("a", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let value: i64 = rec.parse_field("value")?;
                rec.allow_unknown_fields()?;
                Ok(ReprTestEnum::A { value })
            })
            .variant("b", |ctx: &ParseContext<'_>| {
                let mut rec = ctx.parse_record()?;
                let name: String = rec.parse_field("name")?;
                rec.deny_unknown_fields()?;
                Ok(ReprTestEnum::B { name })
            })
            .parse()
            .unwrap_err();

        // In Eure mode, $variant = "b" is used, which expects "name" field
        assert_eq!(err.kind, ParseErrorKind::MissingField("name".to_string()));
    }

    #[test]
    fn test_variant_path_empty_uses_untagged() {
        // When variant_path is Some but empty (consumed by parent), use Untagged
        // This is tested indirectly through nested unions after consuming the path
        let doc = create_text_doc("value");
        let root_id = doc.get_root_id();
        let ctx = doc.parse_context(root_id);

        // Simulate a context where variant_path was set but is now empty
        let child_ctx = ctx.with_variant_rest(Some(VariantPath::empty()));

        // With empty variant_path, should use Untagged parsing
        let result: String = child_ctx
            .parse_union(VariantRepr::default())
            .unwrap()
            .variant("first", String::parse)
            .variant("second", String::parse)
            .parse()
            .unwrap();

        // Priority variant "first" wins in Untagged mode
        assert_eq!(result, "value");
    }
}
