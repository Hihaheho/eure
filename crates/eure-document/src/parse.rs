//! ParseDocument trait for parsing Rust types from Eure documents.

extern crate alloc;

pub mod object_key;
pub mod record;
pub mod tuple;
pub mod union;
pub mod variant_path;

pub use object_key::ParseObjectKey;
pub use record::{ExtParser, RecordParser};
pub use tuple::TupleParser;
pub use union::UnionParser;
pub use variant_path::VariantPath;

use alloc::format;
use num_bigint::BigInt;

use crate::{
    document::node::{Node, NodeArray},
    identifier::IdentifierError,
    prelude_internal::*,
    value::ValueKind,
};

// =============================================================================
// ParseContext
// =============================================================================

/// Context for parsing Eure documents.
///
/// Encapsulates the document, current node, and variant path state.
/// Similar to `DocumentConstructor` for writing, but for reading.
#[derive(Clone, Debug)]
pub struct ParseContext<'doc> {
    doc: &'doc EureDocument,
    node_id: NodeId,
    variant_path: Option<VariantPath>,
}

impl<'doc> ParseContext<'doc> {
    /// Create a new parse context at the given node.
    pub fn new(doc: &'doc EureDocument, node_id: NodeId) -> Self {
        Self {
            doc,
            node_id,
            variant_path: None,
        }
    }

    /// Get the current node ID.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the document reference (internal use only).
    pub(crate) fn doc(&self) -> &'doc EureDocument {
        self.doc
    }

    /// Get the current node.
    pub fn node(&self) -> &'doc Node {
        self.doc.node(self.node_id)
    }

    /// Create a new context at a different node (clears variant path).
    pub(crate) fn at(&self, node_id: NodeId) -> Self {
        Self {
            doc: self.doc,
            node_id,
            variant_path: None,
        }
    }

    /// Parse the current node as type T.
    pub fn parse<T: ParseDocument<'doc>>(&self) -> Result<T, ParseError> {
        T::parse(self)
    }

    /// Get a union parser for the current node.
    ///
    /// Returns error if `$variant` extension has invalid type or syntax.
    pub fn parse_union<T>(&self) -> Result<UnionParser<'doc, '_, T>, ParseError> {
        UnionParser::new(self)
    }

    /// Parse the current node as a record.
    ///
    /// Returns error if variant path is not empty.
    pub fn parse_record(&self) -> Result<RecordParser<'doc>, ParseError> {
        self.ensure_no_variant_path()?;
        RecordParser::new(self)
    }

    /// Parse the current node as a tuple.
    ///
    /// Returns error if variant path is not empty.
    pub fn parse_tuple(&self) -> Result<TupleParser<'doc>, ParseError> {
        self.ensure_no_variant_path()?;
        TupleParser::new(self)
    }

    /// Parse the current node as a primitive value.
    ///
    /// Returns error if variant path is not empty.
    pub fn parse_primitive(&self) -> Result<&'doc PrimitiveValue, ParseError> {
        self.ensure_no_variant_path()?;
        match &self.node().content {
            NodeValue::Primitive(p) => Ok(p),
            value => Err(ParseError {
                node_id: self.node_id,
                kind: handle_unexpected_node_value(value),
            }),
        }
    }

    /// Get an ExtParser for parsing extension types on the current node.
    pub fn parse_extension(&self) -> ExtParser<'doc> {
        let node = self.node();
        ExtParser::new(self.doc, self.node_id, &node.extensions)
    }

    /// Check if the current node is null.
    pub fn is_null(&self) -> bool {
        matches!(
            &self.node().content,
            NodeValue::Primitive(PrimitiveValue::Null)
        )
    }

    /// Create a child context with the remaining variant path.
    pub(crate) fn with_variant_rest(&self, rest: Option<VariantPath>) -> Self {
        Self {
            doc: self.doc,
            node_id: self.node_id,
            variant_path: rest,
        }
    }

    /// Get the current variant path.
    pub(crate) fn variant_path(&self) -> Option<&VariantPath> {
        self.variant_path.as_ref()
    }

    /// Check that no variant path remains, error otherwise.
    fn ensure_no_variant_path(&self) -> Result<(), ParseError> {
        if let Some(vp) = &self.variant_path
            && !vp.is_empty()
        {
            return Err(ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::UnexpectedVariantPath(vp.clone()),
            });
        }
        Ok(())
    }
}

// =============================================================================
// ParseDocument trait
// =============================================================================

/// Trait for parsing Rust types from Eure documents.
///
/// Types implementing this trait can be constructed from [`EureDocument`]
/// via [`ParseContext`].
///
/// # Lifetime Parameter
///
/// The `'doc` lifetime ties the parsed output to the document's lifetime,
/// allowing zero-copy parsing for reference types like `&'doc str`.
///
/// # Examples
///
/// ```ignore
/// // Reference type - borrows from document
/// impl<'doc> ParseDocument<'doc> for &'doc str { ... }
///
/// // Owned type - no lifetime dependency
/// impl ParseDocument<'_> for String { ... }
/// ```
pub trait ParseDocument<'doc>: Sized {
    /// Parse a value of this type from the given parse context.
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError>;
}

fn handle_unexpected_node_value(node_value: &NodeValue) -> ParseErrorKind {
    match node_value {
        NodeValue::Hole(_) => ParseErrorKind::UnexpectedHole,
        value => value
            .value_kind()
            .map(|actual| ParseErrorKind::TypeMismatch {
                expected: ValueKind::Text,
                actual,
            })
            .unwrap_or_else(|| ParseErrorKind::UnexpectedHole),
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
#[error("parse error: {kind}")]
pub struct ParseError {
    pub node_id: NodeId,
    pub kind: ParseErrorKind,
}

/// Error type for parsing failures.
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum ParseErrorKind {
    /// Unexpected uninitialized value.
    #[error("unexpected uninitialized value")]
    UnexpectedHole,

    /// Type mismatch between expected and actual value.
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: ValueKind,
        actual: ValueKind,
    },

    /// Required field is missing.
    #[error("missing field: {0}")]
    MissingField(String),

    /// Required extension is missing.
    #[error("missing extension: ${0}")]
    MissingExtension(String),

    /// Unknown variant in a union type.
    #[error("unknown variant: {0}")]
    UnknownVariant(String),

    /// Value is out of valid range.
    #[error("value out of range: {0}")]
    OutOfRange(String),

    /// Invalid string pattern.
    #[error("invalid pattern: expected {pattern}, got {value}")]
    InvalidPattern { pattern: String, value: String },

    /// Nested parse error with path context.
    #[error("at {path}: {source}")]
    Nested {
        path: String,
        #[source]
        source: Box<ParseErrorKind>,
    },

    /// Invalid identifier.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(#[from] IdentifierError),

    /// Unexpected tuple length.
    #[error("unexpected tuple length: expected {expected}, got {actual}")]
    UnexpectedTupleLength { expected: usize, actual: usize },

    /// Unknown field in record.
    #[error("unknown field: {0}")]
    UnknownField(String),

    /// Invalid key type in record (expected string).
    #[error("invalid key type in record: expected string key, got {0:?}")]
    InvalidKeyType(crate::value::ObjectKey),

    /// No variant matched in union type.
    #[error("no matching variant")]
    NoMatchingVariant,

    /// Multiple variants matched with no priority to resolve.
    #[error("ambiguous union: {0:?}")]
    AmbiguousUnion(Vec<String>),

    /// Literal value mismatch.
    #[error("literal value mismatch: expected {expected}, got {actual}")]
    // FIXME: Use EureDocument instead of String?
    LiteralMismatch { expected: String, actual: String },

    /// Variant path provided but type is not a union.
    #[error("unexpected variant path: {0}")]
    UnexpectedVariantPath(VariantPath),

    /// $variant extension has invalid type (not a string).
    #[error("$variant must be a string, got {0}")]
    InvalidVariantType(ValueKind),

    /// $variant extension has invalid path syntax.
    #[error("invalid $variant path syntax: {0}")]
    InvalidVariantPath(String),
}

impl ParseErrorKind {
    /// Wrap this error with path context.
    pub fn at(self, path: impl Into<String>) -> Self {
        ParseErrorKind::Nested {
            path: path.into(),
            source: Box::new(self),
        }
    }
}

impl<'doc> EureDocument {
    /// Parse a value of type T from the given node.
    pub fn parse<T: ParseDocument<'doc>>(&'doc self, node_id: NodeId) -> Result<T, ParseError> {
        let ctx = ParseContext::new(self, node_id);
        T::parse(&ctx)
    }

    /// Create a parse context at the given node.
    pub fn parse_context(&'doc self, node_id: NodeId) -> ParseContext<'doc> {
        ParseContext::new(self, node_id)
    }

    /// Parse a node as a record.
    ///
    /// Convenience method equivalent to `doc.parse_context(node_id).parse_record()`.
    pub fn parse_record(&'doc self, node_id: NodeId) -> Result<RecordParser<'doc>, ParseError> {
        RecordParser::from_doc_and_node(self, node_id)
    }

    /// Get an extension parser for a node.
    ///
    /// Convenience method equivalent to `doc.parse_context(node_id).parse_extension()`.
    pub fn parse_extension(&'doc self, node_id: NodeId) -> ExtParser<'doc> {
        let node = self.node(node_id);
        ExtParser::new(self, node_id, &node.extensions)
    }

    /// Parse a node as a tuple.
    ///
    /// Convenience method equivalent to `doc.parse_context(node_id).parse_tuple()`.
    pub fn parse_tuple(&'doc self, node_id: NodeId) -> Result<TupleParser<'doc>, ParseError> {
        TupleParser::from_doc_and_node(self, node_id)
    }
}

impl<'doc> ParseDocument<'doc> for &'doc str {
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        match ctx.parse_primitive()? {
            PrimitiveValue::Text(text) => Ok(text.as_str()),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::Text,
                    actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for String {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        ctx.parse::<&str>().map(String::from)
    }
}

impl ParseDocument<'_> for Text {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        match ctx.parse_primitive()? {
            PrimitiveValue::Text(text) => Ok(text.clone()),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::Text,
                    actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for bool {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        match ctx.parse_primitive()? {
            PrimitiveValue::Bool(b) => Ok(*b),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::Bool,
                    actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for BigInt {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        match ctx.parse_primitive()? {
            PrimitiveValue::Integer(i) => Ok(i.clone()),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::Integer,
                    actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for f32 {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        match ctx.parse_primitive()? {
            PrimitiveValue::F32(f) => Ok(*f),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::F32,
                    actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for f64 {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        match ctx.parse_primitive()? {
            // Accept both F32 (with conversion) and F64 if we add it later
            PrimitiveValue::F32(f) => Ok(*f as f64),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::F64,
                    actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for u32 {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let value: BigInt = ctx.parse()?;
        u32::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of u32 range", value)),
        })
    }
}

impl ParseDocument<'_> for i32 {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let value: BigInt = ctx.parse()?;
        i32::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of i32 range", value)),
        })
    }
}

impl ParseDocument<'_> for i64 {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let value: BigInt = ctx.parse()?;
        i64::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of i64 range", value)),
        })
    }
}

impl ParseDocument<'_> for u64 {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let value: BigInt = ctx.parse()?;
        u64::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of u64 range", value)),
        })
    }
}

impl ParseDocument<'_> for usize {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let value: BigInt = ctx.parse()?;
        usize::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of usize range", value)),
        })
    }
}

impl<'doc> ParseDocument<'doc> for &'doc PrimitiveValue {
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        ctx.parse_primitive()
    }
}

impl ParseDocument<'_> for PrimitiveValue {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        ctx.parse::<&PrimitiveValue>().cloned()
    }
}

impl ParseDocument<'_> for Identifier {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        match ctx.parse_primitive()? {
            PrimitiveValue::Text(text) => text
                .content
                .parse()
                .map_err(ParseErrorKind::InvalidIdentifier)
                .map_err(|kind| ParseError {
                    node_id: ctx.node_id(),
                    kind,
                }),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::TypeMismatch {
                    expected: ValueKind::Text,
                    actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                },
            }),
        }
    }
}

impl<'doc> ParseDocument<'doc> for &'doc NodeArray {
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        ctx.ensure_no_variant_path()?;
        match &ctx.node().content {
            NodeValue::Array(array) => Ok(array),
            value => Err(ParseError {
                node_id: ctx.node_id(),
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

impl<'doc, T> ParseDocument<'doc> for Vec<T>
where
    T: ParseDocument<'doc>,
{
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        ctx.ensure_no_variant_path()?;
        match &ctx.node().content {
            NodeValue::Array(array) => array
                .iter()
                .map(|item| T::parse(&ctx.at(*item)))
                .collect::<Result<Vec<_>, _>>(),
            value => Err(ParseError {
                node_id: ctx.node_id(),
                kind: handle_unexpected_node_value(value),
            }),
        }
    }
}

macro_rules! parse_tuple {
    ($n:expr, $($var:ident),*) => {
        impl<'doc, $($var: ParseDocument<'doc>),*> ParseDocument<'doc> for ($($var),*,) {
            fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
                ctx.ensure_no_variant_path()?;
                let tuple = match &ctx.node().content {
                    NodeValue::Tuple(tuple) => tuple,
                    value => return Err(ParseError { node_id: ctx.node_id(), kind: handle_unexpected_node_value(value) }),
                };
                if tuple.len() != $n {
                    return Err(ParseError { node_id: ctx.node_id(), kind: ParseErrorKind::UnexpectedTupleLength { expected: $n, actual: tuple.len() } });
                }
                let mut iter = tuple.iter();
                Ok(($($var::parse(&ctx.at(*iter.next().unwrap()))?),*,))
            }
        }
    }
}

parse_tuple!(1, A);
parse_tuple!(2, A, B);
parse_tuple!(3, A, B, C);
parse_tuple!(4, A, B, C, D);
parse_tuple!(5, A, B, C, D, E);
parse_tuple!(6, A, B, C, D, E, F);
parse_tuple!(7, A, B, C, D, E, F, G);
parse_tuple!(8, A, B, C, D, E, F, G, H);
parse_tuple!(9, A, B, C, D, E, F, G, H, I);
parse_tuple!(10, A, B, C, D, E, F, G, H, I, J);
parse_tuple!(11, A, B, C, D, E, F, G, H, I, J, K);
parse_tuple!(12, A, B, C, D, E, F, G, H, I, J, K, L);
parse_tuple!(13, A, B, C, D, E, F, G, H, I, J, K, L, M);
parse_tuple!(14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
parse_tuple!(15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
parse_tuple!(16, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

impl<'doc, K, T> ParseDocument<'doc> for Map<K, T>
where
    K: ParseObjectKey<'doc>,
    T: ParseDocument<'doc>,
{
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        ctx.ensure_no_variant_path()?;
        let map = match &ctx.node().content {
            NodeValue::Map(map) => map,
            value => {
                return Err(ParseError {
                    node_id: ctx.node_id(),
                    kind: handle_unexpected_node_value(value),
                });
            }
        };
        map.iter()
            .map(|(key, value)| {
                Ok((
                    K::from_object_key(key).map_err(|kind| ParseError {
                        node_id: ctx.node_id(),
                        kind,
                    })?,
                    T::parse(&ctx.at(*value))?,
                ))
            })
            .collect::<Result<Map<_, _>, _>>()
    }
}

/// `Option<T>` is a union with variants `some` and `none`.
///
/// - `$variant: some` -> parse T
/// - `$variant: none` -> None
/// - No `$variant` and value is null -> None
/// - No `$variant` and value is not null -> try parsing as T (Some)
impl<'doc, T> ParseDocument<'doc> for Option<T>
where
    T: ParseDocument<'doc>,
{
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        ctx.parse_union()?
            .variant("some", |ctx| ctx.parse::<T>().map(Some))
            .variant("none", |ctx| {
                if ctx.is_null() {
                    Ok(None)
                } else {
                    Err(ParseError {
                        node_id: ctx.node_id(),
                        kind: ParseErrorKind::TypeMismatch {
                            expected: ValueKind::Null,
                            actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                        },
                    })
                }
            })
            .parse()
    }
}

/// `Result<T, E>` is a union with variants `ok` and `err`.
///
/// - `$variant: ok` -> parse T as Ok
/// - `$variant: err` -> parse E as Err
/// - No `$variant` -> try Ok first, then Err (priority-based)
impl<'doc, T, E> ParseDocument<'doc> for Result<T, E>
where
    T: ParseDocument<'doc>,
    E: ParseDocument<'doc>,
{
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, ParseError> {
        ctx.parse_union()?
            .variant("ok", |ctx| ctx.parse::<T>().map(Ok))
            .variant("err", |ctx| ctx.parse::<E>().map(Err))
            .parse()
    }
}

impl ParseDocument<'_> for crate::data_model::VariantRepr {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        use crate::data_model::VariantRepr;

        // Check if it's a simple string value
        if let Ok(value) = ctx.parse::<&str>() {
            return match value {
                "external" => Ok(VariantRepr::External),
                "untagged" => Ok(VariantRepr::Untagged),
                _ => Err(ParseError {
                    node_id: ctx.node_id(),
                    kind: ParseErrorKind::UnknownVariant(value.to_string()),
                }),
            };
        }

        // Otherwise, it should be a record with tag/content fields
        let mut rec = ctx.parse_record()?;

        let tag = rec.field_optional::<String>("tag")?;
        let content = rec.field_optional::<String>("content")?;

        rec.allow_unknown_fields()?;

        match (tag, content) {
            (Some(tag), Some(content)) => Ok(VariantRepr::Adjacent { tag, content }),
            (Some(tag), None) => Ok(VariantRepr::Internal { tag }),
            (None, None) => Ok(VariantRepr::External),
            (None, Some(_)) => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::MissingField(
                    "tag (required when content is present)".to_string(),
                ),
            }),
        }
    }
}

pub trait DocumentParser<'doc> {
    type Output: 'doc;
    fn parse(self, doc: &'doc EureDocument, node_id: NodeId) -> Result<Self::Output, ParseError>;
}

impl<'doc, T: 'doc, F> DocumentParser<'doc> for F
where
    F: FnOnce(&'doc EureDocument, NodeId) -> Result<T, ParseError>,
{
    type Output = T;
    fn parse(self, doc: &'doc EureDocument, node_id: NodeId) -> Result<Self::Output, ParseError> {
        self(doc, node_id)
    }
}

pub struct LiteralParser<T>(T);

impl<'doc, T> DocumentParser<'doc> for LiteralParser<T>
where
    T: 'doc + ParseDocument<'doc> + PartialEq + core::fmt::Debug,
{
    type Output = T;
    fn parse(self, doc: &'doc EureDocument, node_id: NodeId) -> Result<Self::Output, ParseError> {
        let value: T = doc.parse(node_id)?;
        if value == self.0 {
            Ok(value)
        } else {
            Err(ParseError {
                node_id,
                kind: ParseErrorKind::LiteralMismatch {
                    expected: format!("{:?}", self.0),
                    actual: format!("{:?}", value),
                },
            })
        }
    }
}

pub struct MappedParser<T, F> {
    parser: T,
    mapper: F,
}

impl<'doc, T, O, F> DocumentParser<'doc> for MappedParser<T, F>
where
    T: DocumentParser<'doc>,
    F: Fn(T::Output) -> Result<O, ParseError>,
    O: 'doc,
{
    type Output = O;
    fn parse(self, doc: &'doc EureDocument, node_id: NodeId) -> Result<Self::Output, ParseError> {
        let value = self.parser.parse(doc, node_id)?;
        (self.mapper)(value)
    }
}

pub trait DocumentParserExt<'doc>: DocumentParser<'doc> + Sized {
    fn map<O, F>(self, mapper: F) -> MappedParser<Self, F>
    where
        F: Fn(Self::Output) -> Result<O, ParseError>,
        O: 'doc,
    {
        MappedParser {
            parser: self,
            mapper,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::node::NodeValue;
    use crate::eure;
    use crate::identifier::Identifier;
    use crate::text::Text;
    use crate::value::ObjectKey;
    use num_bigint::BigInt;

    fn identifier(s: &str) -> Identifier {
        s.parse().unwrap()
    }

    /// Create a document with a single field that has a $variant extension
    fn create_record_with_variant(
        field_name: &str,
        value: NodeValue,
        variant: &str,
    ) -> EureDocument {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();

        // Add field
        let field_id = doc
            .add_map_child(ObjectKey::String(field_name.to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(field_id).content = value;

        // Add $variant extension
        let variant_node_id = doc
            .add_extension(identifier("variant"), field_id)
            .unwrap()
            .node_id;
        doc.node_mut(variant_node_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(variant.to_string())));

        doc
    }

    #[test]
    fn test_option_some_tagged() {
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(42))),
            "some",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.field("value").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_option_none_tagged() {
        let doc =
            create_record_with_variant("value", NodeValue::Primitive(PrimitiveValue::Null), "none");
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.field("value").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_option_some_untagged() {
        // Without $variant, non-null value is Some
        let doc = eure!({ value = 42 });
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.field("value").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_option_none_untagged() {
        // Without $variant, null is None
        let doc = eure!({ value = null });
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.field("value").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_result_ok_tagged() {
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(42))),
            "ok",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Result<i32, String> = rec.field("value").unwrap();
        assert_eq!(value, Ok(42));
    }

    #[test]
    fn test_result_err_tagged() {
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(
                "error message".to_string(),
            ))),
            "err",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Result<i32, String> = rec.field("value").unwrap();
        assert_eq!(value, Err("error message".to_string()));
    }

    #[test]
    fn test_nested_result_option_ok_some() {
        // $variant: ok.some - Result<Option<i32>, String>
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(42))),
            "ok.some",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Result<Option<i32>, String> = rec.field("value").unwrap();
        assert_eq!(value, Ok(Some(42)));
    }

    #[test]
    fn test_nested_result_option_ok_none() {
        // $variant: ok.none - Result<Option<i32>, String>
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Null),
            "ok.none",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Result<Option<i32>, String> = rec.field("value").unwrap();
        assert_eq!(value, Ok(None));
    }

    #[test]
    fn test_nested_result_option_err() {
        // $variant: err - Result<Option<i32>, String>
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("error".to_string()))),
            "err",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Result<Option<i32>, String> = rec.field("value").unwrap();
        assert_eq!(value, Err("error".to_string()));
    }

    #[test]
    fn test_deeply_nested_option_option() {
        // $variant: some.some - Option<Option<i32>>
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(42))),
            "some.some",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Option<Option<i32>> = rec.field("value").unwrap();
        assert_eq!(value, Some(Some(42)));
    }

    #[test]
    fn test_deeply_nested_option_none() {
        // $variant: some.none - Option<Option<i32>> inner None
        let doc = create_record_with_variant(
            "value",
            NodeValue::Primitive(PrimitiveValue::Null),
            "some.none",
        );
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Option<Option<i32>> = rec.field("value").unwrap();
        assert_eq!(value, Some(None));
    }

    #[test]
    fn test_outer_none() {
        // $variant: none - Option<Option<i32>> outer None
        let doc =
            create_record_with_variant("value", NodeValue::Primitive(PrimitiveValue::Null), "none");
        let root_id = doc.get_root_id();
        let mut rec = doc.parse_record(root_id).unwrap();
        let value: Option<Option<i32>> = rec.field("value").unwrap();
        assert_eq!(value, None);
    }
}
