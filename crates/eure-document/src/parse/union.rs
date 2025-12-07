//! UnionParser for parsing union types from Eure documents.
//!
//! Implements oneOf semantics with priority-based ambiguity resolution.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Display;
use core::str::FromStr;

use thisisplural::Plural;

use crate::document::{EureDocument, NodeId};
use crate::identifier::Identifier;

use super::{DocumentParser, ParseError, ParseErrorKind};

/// A path of variant names for nested union types.
///
/// Represents a dot-separated path like `ok.some.left` for nested unions
/// such as `Result<Option<Either<T, U>>>`.
///
/// # Example
///
/// ```
/// use eure_document::parse::VariantPath;
///
/// let path: VariantPath = "ok.some.left".parse().unwrap();
/// assert_eq!(path.len(), 3);
///
/// let segments: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
/// assert_eq!(segments, vec!["ok", "some", "left"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Plural)]
#[plural(len, is_empty, iter, into_iter, from_iter)]
pub struct VariantPath(Vec<String>);

impl VariantPath {
    /// Create a new VariantPath from a single segment.
    pub fn single(segment: impl Into<String>) -> Self {
        VariantPath(alloc::vec![segment.into()])
    }

    /// Get the first segment of the path, if any.
    pub fn first(&self) -> Option<&str> {
        self.0.first().map(String::as_str)
    }

    /// Split the path into the first segment and the remaining path.
    ///
    /// Returns `None` if the path is empty.
    /// Returns `(first, None)` if the path has only one segment.
    /// Returns `(first, Some(rest))` if the path has multiple segments.
    pub fn split_first(&self) -> Option<(&str, Option<VariantPath>)> {
        match self.0.as_slice() {
            [] => None,
            [first] => Some((first.as_str(), None)),
            [first, rest @ ..] => Some((
                first.as_str(),
                Some(VariantPath(rest.iter().cloned().collect())),
            )),
        }
    }

    /// Create a new path by appending a segment.
    pub fn push(&mut self, segment: impl Into<String>) {
        self.0.push(segment.into());
    }

    /// Create a new path by prepending a segment.
    pub fn prepend(mut self, segment: impl Into<String>) -> Self {
        self.0.insert(0, segment.into());
        self
    }
}

impl Display for VariantPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut first = true;
        for segment in &self.0 {
            if !first {
                write!(f, ".")?;
            }
            write!(f, "{}", segment)?;
            first = false;
        }
        Ok(())
    }
}

impl FromStr for VariantPath {
    type Err = VariantPathParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(VariantPathParseError::Empty);
        }

        let segments: Vec<String> = s.split('.').map(String::from).collect();

        // Validate each segment is a valid identifier
        for segment in &segments {
            if segment.is_empty() {
                return Err(VariantPathParseError::EmptySegment);
            }
            // Basic validation: first char must be XID_Start or '_'
            let mut chars = segment.chars();
            if let Some(first) = chars.next() {
                if !first.is_alphabetic() && first != '_' {
                    return Err(VariantPathParseError::InvalidSegment(segment.clone()));
                }
            }
        }

        Ok(VariantPath(segments))
    }
}

/// Error when parsing a variant path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantPathParseError {
    /// The path is empty.
    Empty,
    /// A segment in the path is empty (e.g., "ok..left").
    EmptySegment,
    /// A segment is not a valid identifier.
    InvalidSegment(String),
}

impl Display for VariantPathParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VariantPathParseError::Empty => write!(f, "variant path is empty"),
            VariantPathParseError::EmptySegment => {
                write!(f, "variant path contains empty segment")
            }
            VariantPathParseError::InvalidSegment(s) => {
                write!(f, "invalid variant path segment: {}", s)
            }
        }
    }
}

impl From<&str> for VariantPath {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_else(|_| VariantPath::single(s))
    }
}

impl From<String> for VariantPath {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

/// The `$variant` extension identifier.
pub const VARIANT: Identifier = Identifier::new_unchecked("variant");

/// Helper for parsing union types from Eure documents.
///
/// Implements oneOf semantics:
/// - Exactly one variant must match
/// - Multiple matches resolved by registration order (priority)
/// - Short-circuits on first priority variant match
/// - When `$variant` extension is specified, matches by name directly
/// - Supports nested union types with dot-separated variant paths (e.g., `ok.some.left`)
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
///
/// # Nested Unions Example
///
/// ```ignore
/// // For Result<Option<Either<i32, String>>>
/// // Data: $variant = `ok.some.left`, value = 42
/// doc.parse_union(node_id)
///     .nested("ok", |doc, id, rest| {
///         doc.parse_union_with_path(id, rest)
///             .nested("some", |doc, id, rest| {
///                 doc.parse_union_with_path(id, rest)
///                     .nested("left", |doc, id, _| doc.parse::<i32>(id).map(Either::Left))
///                     .nested("right", |doc, id, _| doc.parse::<String>(id).map(Either::Right))
///                     .parse()
///                     .map(Some)
///             })
///             .variant("none", |_, _| Ok(None))
///             .parse()
///             .map(Ok)
///     })
///     .nested("err", |doc, id, _| doc.parse::<String>(id).map(Err))
///     .parse()
/// ```
pub struct UnionParser<'doc, T> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    /// Variant path from `$variant` extension (if specified)
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

impl<'doc, T> UnionParser<'doc, T> {
    /// Create a new UnionParser for the given node.
    pub(crate) fn new(doc: &'doc EureDocument, node_id: NodeId) -> Self {
        let variant_path = Self::extract_variant_path(doc, node_id);
        Self::with_path(doc, node_id, variant_path)
    }

    /// Create a new UnionParser with an explicit variant path.
    ///
    /// This is used for nested unions where the outer parser passes
    /// the remaining path to the inner parser.
    pub(crate) fn with_path(
        doc: &'doc EureDocument,
        node_id: NodeId,
        variant_path: Option<VariantPath>,
    ) -> Self {
        Self {
            doc,
            node_id,
            variant_path,
            variant_result: None,
            priority_result: None,
            other_results: Vec::new(),
            other_failures: Vec::new(),
        }
    }

    /// Extract the `$variant` extension value from the node as a VariantPath.
    fn extract_variant_path(doc: &EureDocument, node_id: NodeId) -> Option<VariantPath> {
        let node = doc.node(node_id);
        let variant_node_id = node.extensions.get(&VARIANT)?;
        let variant_str: &str = doc.parse(*variant_node_id).ok()?;
        Some(VariantPath::from(variant_str))
    }

    /// Register a priority variant.
    ///
    /// Priority variants are tried in registration order.
    /// When a priority variant matches, parsing short-circuits and returns immediately.
    pub fn variant<P>(mut self, name: &str, parser: P) -> Self
    where
        P: DocumentParser<'doc, Output = T> + 'doc,
    {
        if let Some(ref path) = self.variant_path {
            // $variant specified: only parse if first segment matches and no result yet
            if let Some((first, rest)) = path.split_first() {
                if first == name && rest.is_none() && self.variant_result.is_none() {
                    self.variant_result = Some(parser.parse(self.doc, self.node_id));
                }
            }
        } else if self.priority_result.is_none()
            && let Ok(value) = parser.parse(self.doc, self.node_id)
        {
            // No $variant: short-circuit on first match
            self.priority_result = Some(value);
        }
        self
    }

    /// Register a priority variant for nested unions.
    ///
    /// The parser receives the remaining variant path after consuming the first segment.
    /// Use this when the variant contains another union type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For Result<Option<T>>
    /// doc.parse_union(node_id)
    ///     .nested("ok", |doc, id, rest| {
    ///         doc.parse_union_with_path(id, rest)
    ///             .variant("some", |doc, id| doc.parse::<T>(id).map(Some))
    ///             .variant("none", |_, _| Ok(None))
    ///             .parse()
    ///             .map(Ok)
    ///     })
    ///     .nested("err", |doc, id, _| doc.parse::<E>(id).map(Err))
    ///     .parse()
    /// ```
    pub fn nested<P>(mut self, name: &str, parser: P) -> Self
    where
        P: FnOnce(&'doc EureDocument, NodeId, Option<VariantPath>) -> Result<T, ParseError>,
    {
        if let Some(ref path) = self.variant_path {
            // $variant specified: only parse if first segment matches and no result yet
            if let Some((first, rest)) = path.split_first() {
                if first == name && self.variant_result.is_none() {
                    self.variant_result = Some(parser(self.doc, self.node_id, rest));
                }
            }
        } else if self.priority_result.is_none()
            && let Ok(value) = parser(self.doc, self.node_id, None)
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
        if let Some(ref path) = self.variant_path {
            // $variant specified: only parse if first segment matches and no result yet
            if let Some((first, rest)) = path.split_first() {
                if first == name && rest.is_none() && self.variant_result.is_none() {
                    self.variant_result = Some(parser.parse(self.doc, self.node_id));
                }
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

    /// Register a non-priority variant for nested unions.
    ///
    /// Similar to `nested`, but for non-priority variants.
    pub fn other_nested<P>(mut self, name: &str, parser: P) -> Self
    where
        P: FnOnce(&'doc EureDocument, NodeId, Option<VariantPath>) -> Result<T, ParseError>,
    {
        if let Some(ref path) = self.variant_path {
            // $variant specified: only parse if first segment matches and no result yet
            if let Some((first, rest)) = path.split_first() {
                if first == name && self.variant_result.is_none() {
                    self.variant_result = Some(parser(self.doc, self.node_id, rest));
                }
            }
        } else {
            // No $variant: try all for ambiguity detection (only if no priority match)
            if self.priority_result.is_none() {
                match parser(self.doc, self.node_id, None) {
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

    /// Get a UnionParser with an explicit variant path.
    ///
    /// This is used for nested unions where the outer parser passes
    /// the remaining path to the inner parser.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For Result<Option<T>> with $variant = "ok.some"
    /// doc.parse_union(node_id)
    ///     .nested("ok", |doc, id, rest| {
    ///         doc.parse_union_with_path(id, rest)
    ///             .variant("some", |doc, id| doc.parse::<T>(id).map(Some))
    ///             .variant("none", |_, _| Ok(None))
    ///             .parse()
    ///             .map(Ok)
    ///     })
    ///     .nested("err", |doc, id, _| doc.parse::<E>(id).map(Err))
    ///     .parse()
    /// ```
    pub fn parse_union_with_path<T>(
        &self,
        node_id: NodeId,
        path: Option<VariantPath>,
    ) -> UnionParser<'_, T> {
        UnionParser::with_path(self, node_id, path)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

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

    fn create_int_doc_with_variant(content: i64, variant: &str) -> EureDocument {
        use num_bigint::BigInt;

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Set content as integer
        doc.node_mut(root_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(content)));

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
        // $variant = "baz" specified, matches other("baz")
        // All parsers always succeed
        let doc = create_doc_with_variant("anything", "baz");
        let root_id = doc.get_root_id();

        let result: TestEnum = doc
            .parse_union(root_id)
            .variant("foo", |_, _| Ok(TestEnum::Foo))
            .other("baz", |_, _| Ok(TestEnum::Bar))
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

        let err = doc
            .parse_union::<TestEnum>(root_id)
            .variant("foo", |_, _| Ok(TestEnum::Foo))
            .other("baz", |_, _| Ok(TestEnum::Bar))
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

        let err = doc
            .parse_union::<TestEnum>(root_id)
            .variant("foo", |_, _| Ok(TestEnum::Foo))
            .other("baz", |_, id| {
                Err(ParseError {
                    node_id: id,
                    kind: ParseErrorKind::MissingField("test".to_string()),
                })
            })
            .parse()
            .unwrap_err();

        // Parser's error is returned directly
        assert_eq!(err.node_id, root_id);
        assert_eq!(err.kind, ParseErrorKind::MissingField("test".to_string()));
    }

    // --- VariantPath tests ---

    #[test]
    fn test_variant_path_parse_single() {
        let path: VariantPath = "ok".parse().unwrap();
        assert_eq!(path.len(), 1);
        assert_eq!(path.first(), Some("ok"));

        let (first, rest) = path.split_first().unwrap();
        assert_eq!(first, "ok");
        assert!(rest.is_none());
    }

    #[test]
    fn test_variant_path_parse_multiple() {
        let path: VariantPath = "ok.some.left".parse().unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path.first(), Some("ok"));

        let (first, rest) = path.split_first().unwrap();
        assert_eq!(first, "ok");

        let rest = rest.unwrap();
        assert_eq!(rest.len(), 2);
        assert_eq!(rest.first(), Some("some"));

        let (second, rest2) = rest.split_first().unwrap();
        assert_eq!(second, "some");

        let rest2 = rest2.unwrap();
        assert_eq!(rest2.len(), 1);
        assert_eq!(rest2.first(), Some("left"));
    }

    #[test]
    fn test_variant_path_display() {
        let path: VariantPath = "ok.some.left".parse().unwrap();
        assert_eq!(path.to_string(), "ok.some.left");

        let single: VariantPath = "ok".parse().unwrap();
        assert_eq!(single.to_string(), "ok");
    }

    #[test]
    fn test_variant_path_iter() {
        let path: VariantPath = "ok.some.left".parse().unwrap();
        let segments: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        assert_eq!(segments, vec!["ok", "some", "left"]);
    }

    #[test]
    fn test_variant_path_from_iter() {
        let segments = vec!["ok", "some", "left"];
        let path: VariantPath = segments.into_iter().map(String::from).collect();
        assert_eq!(path.to_string(), "ok.some.left");
    }

    #[test]
    fn test_variant_path_parse_errors() {
        assert!("".parse::<VariantPath>().is_err());
        assert!("ok..left".parse::<VariantPath>().is_err());
        assert!("123invalid".parse::<VariantPath>().is_err());
    }

    // --- Nested union tests ---

    #[derive(Debug, PartialEq)]
    enum ResultOption {
        Ok(Option<i32>),
        Err(String),
    }

    impl crate::parse::ParseDocument<'_> for ResultOption {
        fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
            doc.parse_union(node_id)
                .nested("ok", |doc, id, rest| {
                    doc.parse_union_with_path(id, rest)
                        .variant("some", |doc: &EureDocument, id| {
                            doc.parse::<i32>(id).map(Some)
                        })
                        .variant("none", |_, _| Ok(None))
                        .parse()
                        .map(ResultOption::Ok)
                })
                .nested("err", |doc, id, _| {
                    doc.parse::<String>(id).map(ResultOption::Err)
                })
                .parse()
        }
    }

    #[test]
    fn test_nested_union_ok_some() {
        // $variant = "ok.some", value = 42
        let doc = create_int_doc_with_variant(42, "ok.some");
        let result: ResultOption = doc.parse(doc.get_root_id()).unwrap();
        assert_eq!(result, ResultOption::Ok(Some(42)));
    }

    #[test]
    fn test_nested_union_ok_none() {
        // $variant = "ok.none"
        let doc = create_doc_with_variant("ignored", "ok.none");
        let result: ResultOption = doc.parse(doc.get_root_id()).unwrap();
        assert_eq!(result, ResultOption::Ok(None));
    }

    #[test]
    fn test_nested_union_err() {
        // $variant = "err"
        let doc = create_doc_with_variant("error message", "err");
        let result: ResultOption = doc.parse(doc.get_root_id()).unwrap();
        assert_eq!(result, ResultOption::Err("error message".to_string()));
    }

    #[test]
    fn test_nested_union_unknown_inner_variant() {
        // $variant = "ok.invalid" - inner variant doesn't exist
        let doc = create_int_doc_with_variant(42, "ok.invalid");
        let err = doc.parse::<ResultOption>(doc.get_root_id()).unwrap_err();
        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("invalid".to_string())
        );
    }

    #[test]
    fn test_nested_union_unknown_outer_variant() {
        // $variant = "unknown.some"
        let doc = create_int_doc_with_variant(42, "unknown.some");
        let err = doc.parse::<ResultOption>(doc.get_root_id()).unwrap_err();
        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("unknown.some".to_string())
        );
    }

    #[derive(Debug, PartialEq)]
    enum Either<L, R> {
        Left(L),
        Right(R),
    }

    #[derive(Debug, PartialEq)]
    struct TripleNested(Result<Option<Either<i32, String>>, String>);

    impl crate::parse::ParseDocument<'_> for TripleNested {
        fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
            doc.parse_union(node_id)
                .nested("ok", |doc, id, rest| {
                    doc.parse_union_with_path(id, rest)
                        .nested("some", |doc, id, rest| {
                            doc.parse_union_with_path(id, rest)
                                .variant("left", |doc: &EureDocument, id| {
                                    doc.parse::<i32>(id).map(Either::Left)
                                })
                                .variant("right", |doc: &EureDocument, id| {
                                    doc.parse::<String>(id).map(Either::Right)
                                })
                                .parse()
                                .map(Some)
                        })
                        .variant("none", |_, _| Ok(None))
                        .parse()
                        .map(Ok)
                })
                .nested("err", |doc, id, _| doc.parse::<String>(id).map(Err))
                .parse()
                .map(TripleNested)
        }
    }

    #[test]
    fn test_triple_nested_union() {
        // Result<Option<Either<i32, String>>>
        // $variant = "ok.some.left", value = 42
        let doc = create_int_doc_with_variant(42, "ok.some.left");
        let result: TripleNested = doc.parse(doc.get_root_id()).unwrap();
        assert_eq!(result, TripleNested(Ok(Some(Either::Left(42)))));
    }

    #[test]
    fn test_triple_nested_union_right() {
        // Result<Option<Either<i32, String>>>
        // $variant = "ok.some.right"
        let doc = create_doc_with_variant("hello", "ok.some.right");
        let result: TripleNested = doc.parse(doc.get_root_id()).unwrap();
        assert_eq!(
            result,
            TripleNested(Ok(Some(Either::Right("hello".to_string()))))
        );
    }
}
