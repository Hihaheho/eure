//! ParseDocument trait for parsing Rust types from Eure documents.

extern crate alloc;

pub mod object_key;
pub mod record;
pub mod tuple;
pub mod union;
pub mod variant_path;

pub use object_key::ParseObjectKey;
pub use record::RecordParser;
pub use tuple::TupleParser;
pub use union::UnionParser;
pub use variant_path::VariantPath;
// UnionTagMode is defined in this module and exported automatically

use alloc::format;
use alloc::rc::Rc;
use core::cell::RefCell;
use num_bigint::BigInt;

use core::marker::PhantomData;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    data_model::VariantRepr,
    document::node::{Node, NodeArray},
    identifier::IdentifierError,
    prelude_internal::*,
    value::ValueKind,
};

// =============================================================================
// UnionTagMode
// =============================================================================

/// Mode for union tag resolution.
///
/// This determines how variant tags are resolved during union parsing:
/// - `Eure`: Use `$variant` extension and untagged matching (for native Eure documents)
/// - `Repr`: Use only `VariantRepr` patterns (for JSON/YAML imports)
///
/// These modes are mutually exclusive to avoid false positives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UnionTagMode {
    /// Eure mode: Use `$variant` extension or untagged matching.
    ///
    /// This is the default mode for native Eure documents.
    /// - If `$variant` extension is present, use it to determine the variant
    /// - Otherwise, use untagged matching (try all variants)
    /// - `VariantRepr` is ignored in this mode
    #[default]
    Eure,

    /// Repr mode: Use only `VariantRepr` patterns.
    ///
    /// This mode is for documents imported from JSON/YAML.
    /// - Extract variant tag using `VariantRepr` (External, Internal, Adjacent)
    /// - `$variant` extension is ignored in this mode
    /// - If repr doesn't extract a tag, error (no untagged fallback)
    Repr,
}

// =============================================================================
// AccessedSet
// =============================================================================

/// Snapshot of accessed state (fields, extensions).
pub type AccessedSnapshot = (HashSet<String>, HashSet<Identifier>);

/// Tracks accessed fields and extensions with snapshot/rollback support for union parsing.
///
/// The internal state is a stack where the last item is always the current working state.
/// Items before it are snapshots (save points) for rollback.
///
/// Invariant: stack is never empty.
///
/// # Stack visualization
/// ```text
/// Initial:        [current]
/// push_snapshot:  [snapshot, current]  (snapshot = copy of current)
/// modify:         [snapshot, current'] (current' has changes)
/// restore:        [snapshot, snapshot] (current reset to snapshot)
/// pop_restore:    [snapshot]           (current removed, snapshot is new current)
/// pop_no_restore: [current']           (snapshot removed, keep modified current)
/// ```
#[derive(Debug, Clone)]
pub struct AccessedSet(Rc<RefCell<Vec<AccessedSnapshot>>>);

impl AccessedSet {
    /// Create a new empty set.
    pub fn new() -> Self {
        // Start with one empty state (the current working state)
        Self(Rc::new(RefCell::new(vec![(
            HashSet::new(),
            HashSet::new(),
        )])))
    }

    /// Add a field to the accessed set.
    pub fn add_field(&self, field: impl Into<String>) {
        self.0
            .borrow_mut()
            .last_mut()
            .unwrap()
            .0
            .insert(field.into());
    }

    /// Add an extension to the accessed set.
    pub fn add_ext(&self, ext: Identifier) {
        self.0.borrow_mut().last_mut().unwrap().1.insert(ext);
    }

    /// Check if a field has been accessed.
    pub fn has_field(&self, field: &str) -> bool {
        self.0.borrow().last().unwrap().0.contains(field)
    }

    /// Check if an extension has been accessed.
    pub fn has_ext(&self, ext: &Identifier) -> bool {
        self.0.borrow().last().unwrap().1.contains(ext)
    }

    /// Push a snapshot (call at start of union parsing).
    /// Inserts a copy of current BEFORE current, so current can be modified.
    pub fn push_snapshot(&self) {
        let mut stack = self.0.borrow_mut();
        let snapshot = stack.last().unwrap().clone();
        let len = stack.len();
        stack.insert(len - 1, snapshot);
        // Stack: [..., current] → [..., snapshot, current]
    }

    /// Restore current to the snapshot (call when variant fails).
    /// Resets current (last) to match the snapshot (second-to-last).
    pub fn restore_to_current_snapshot(&self) {
        let mut stack = self.0.borrow_mut();
        if stack.len() >= 2 {
            let snapshot = stack[stack.len() - 2].clone();
            *stack.last_mut().unwrap() = snapshot;
        }
    }

    /// Capture the current state (for non-priority variants).
    pub fn capture_current_state(&self) -> AccessedSnapshot {
        self.0.borrow().last().unwrap().clone()
    }

    /// Restore to a previously captured state.
    pub fn restore_to_state(&self, state: AccessedSnapshot) {
        *self.0.borrow_mut().last_mut().unwrap() = state;
    }

    /// Pop and restore (call when union fails/ambiguous).
    /// Removes current, snapshot becomes new current.
    pub fn pop_and_restore(&self) {
        let mut stack = self.0.borrow_mut();
        if stack.len() >= 2 {
            stack.pop(); // Remove current, snapshot is now current
        }
    }

    /// Pop without restore (call when union succeeds).
    /// Removes the snapshot, keeps current.
    pub fn pop_without_restore(&self) {
        let mut stack = self.0.borrow_mut();
        if stack.len() >= 2 {
            let snapshot_idx = stack.len() - 2;
            stack.remove(snapshot_idx); // Remove snapshot, keep current
        }
    }
}

impl Default for AccessedSet {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ParserScope
// =============================================================================

/// Scope for flatten parsing - indicates whether we're in record or extension mode.
///
/// This determines what `parse_record_or_ext()` iterates over for catch-all types like HashMap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserScope {
    /// Record scope - iterates record fields (from `rec.flatten()`)
    Record,
    /// Extension scope - iterates extensions (from `ext.flatten_ext()`)
    Extension,
}

// =============================================================================
// FlattenContext
// =============================================================================

/// Context for flatten parsing - wraps AccessedSet with snapshot/rollback support.
///
/// When parsing flattened types, all levels share a single `AccessedSet` owned
/// by the root parser. Child parsers add to this shared set, and only the root
/// parser actually validates with `deny_unknown_fields()`.
///
/// # Example
///
/// ```ignore
/// // Root parser owns the accessed set
/// let mut rec1 = ctx.parse_record()?;  // accessed = {} (owned)
/// rec1.field("a");  // accessed = {a}
///
/// // Child parser shares and adds to the same set
/// let ctx2 = rec1.flatten();
/// let mut rec2 = ctx2.parse_record()?;  // shares accessed via Rc
/// rec2.field("b");  // accessed = {a, b}
/// rec2.deny_unknown_fields()?;  // NO-OP (child)
///
/// rec1.field("c");  // accessed = {a, b, c}
/// rec1.deny_unknown_fields()?;  // VALIDATES (root)
/// ```
#[derive(Debug, Clone)]
pub struct FlattenContext {
    accessed: AccessedSet,
    scope: ParserScope,
}

impl FlattenContext {
    /// Create a FlattenContext from an existing AccessedSet with the given scope.
    pub fn new(accessed: AccessedSet, scope: ParserScope) -> Self {
        Self { accessed, scope }
    }

    /// Get the parser scope.
    pub fn scope(&self) -> ParserScope {
        self.scope
    }

    /// Get the underlying AccessedSet (for sharing with RecordParser).
    pub fn accessed_set(&self) -> &AccessedSet {
        &self.accessed
    }

    /// Add a field to the accessed set.
    pub fn add_field(&self, field: impl Into<String>) {
        self.accessed.add_field(field);
    }

    /// Add an extension to the accessed set.
    pub fn add_ext(&self, ext: Identifier) {
        self.accessed.add_ext(ext);
    }

    /// Check if a field has been accessed.
    pub fn has_field(&self, field: &str) -> bool {
        self.accessed.has_field(field)
    }

    /// Check if an extension has been accessed.
    pub fn has_ext(&self, ext: &Identifier) -> bool {
        self.accessed.has_ext(ext)
    }

    /// Push snapshot (at start of union parsing).
    pub fn push_snapshot(&self) {
        self.accessed.push_snapshot();
    }

    /// Restore to current snapshot (when variant fails).
    pub fn restore_to_current_snapshot(&self) {
        self.accessed.restore_to_current_snapshot();
    }

    /// Capture current state (for non-priority variants).
    pub fn capture_current_state(&self) -> AccessedSnapshot {
        self.accessed.capture_current_state()
    }

    /// Restore to a captured state (when selecting a non-priority variant).
    pub fn restore_to_state(&self, state: AccessedSnapshot) {
        self.accessed.restore_to_state(state);
    }

    /// Pop and restore (when union fails/ambiguous).
    pub fn pop_and_restore(&self) {
        self.accessed.pop_and_restore();
    }

    /// Pop without restore (when union succeeds).
    pub fn pop_without_restore(&self) {
        self.accessed.pop_without_restore();
    }
}

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
    /// Flatten context for tracking shared accessed fields/extensions.
    /// If Some, this context is a flattened child - deny_unknown_* is no-op.
    /// If None, this is a root context.
    flatten_ctx: Option<FlattenContext>,
    /// Mode for union tag resolution.
    union_tag_mode: UnionTagMode,
    /// Tracks accessed fields and extensions.
    accessed: AccessedSet,
}

impl<'doc> ParseContext<'doc> {
    /// Create a new parse context at the given node.
    pub fn new(doc: &'doc EureDocument, node_id: NodeId) -> Self {
        Self {
            doc,
            node_id,
            variant_path: None,
            flatten_ctx: None,
            union_tag_mode: UnionTagMode::default(),
            accessed: AccessedSet::new(),
        }
    }

    /// Create a new parse context with the specified union tag mode.
    pub fn with_union_tag_mode(
        doc: &'doc EureDocument,
        node_id: NodeId,
        mode: UnionTagMode,
    ) -> Self {
        Self {
            doc,
            node_id,
            variant_path: None,
            flatten_ctx: None,
            union_tag_mode: mode,
            accessed: AccessedSet::new(),
        }
    }

    /// Create a context with a flatten context (for flatten parsing).
    ///
    /// When a flatten context is present:
    /// - `deny_unknown_fields()` and `deny_unknown_extensions()` become no-ops
    /// - All field/extension accesses are recorded in the shared `FlattenContext`
    pub fn with_flatten_ctx(
        doc: &'doc EureDocument,
        node_id: NodeId,
        flatten_ctx: FlattenContext,
        mode: UnionTagMode,
    ) -> Self {
        // Share accessed set from flatten context
        let accessed = flatten_ctx.accessed_set().clone();
        Self {
            doc,
            node_id,
            variant_path: None,
            flatten_ctx: Some(flatten_ctx),
            union_tag_mode: mode,
            accessed,
        }
    }

    /// Get the flatten context, if present.
    pub fn flatten_ctx(&self) -> Option<&FlattenContext> {
        self.flatten_ctx.as_ref()
    }

    /// Check if this context is flattened (has a flatten context).
    ///
    /// When flattened, `deny_unknown_fields()` and `deny_unknown_extensions()` are no-ops.
    pub fn is_flattened(&self) -> bool {
        self.flatten_ctx.is_some()
    }

    /// Get the parser scope, if in a flatten context.
    ///
    /// Returns `Some(ParserScope::Record)` if from `rec.flatten()`,
    /// `Some(ParserScope::Extension)` if from `ext.flatten_ext()`,
    /// `None` if not in a flatten context.
    pub fn parser_scope(&self) -> Option<ParserScope> {
        self.flatten_ctx.as_ref().map(|fc| fc.scope())
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

    /// Create a new context at a different node (clears variant path and flatten context).
    pub(crate) fn at(&self, node_id: NodeId) -> Self {
        Self {
            doc: self.doc,
            node_id,
            variant_path: None,
            flatten_ctx: None,
            union_tag_mode: self.union_tag_mode,
            accessed: AccessedSet::new(),
        }
    }

    /// Create a flattened version of this context for shared access tracking.
    ///
    /// When you need both record parsing and extension parsing to share
    /// access tracking (so deny_unknown_* works correctly), use this method
    /// to create a shared context first.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Both record and extension parsers share the same AccessedSet
    /// let ctx = ctx.flatten();
    /// // ... parse extensions with ctx.parse_ext(...) ...
    /// let mut rec = ctx.parse_record()?;
    /// // ... parse fields ...
    /// rec.deny_unknown_fields()?;  // Validates both fields and extensions
    /// ```
    pub fn flatten(&self) -> Self {
        // If already has flatten context, reuse it
        if let Some(fc) = &self.flatten_ctx {
            return Self {
                doc: self.doc,
                node_id: self.node_id,
                variant_path: self.variant_path.clone(),
                flatten_ctx: Some(fc.clone()),
                union_tag_mode: self.union_tag_mode,
                accessed: self.accessed.clone(),
            };
        }

        // Create new flatten context with shared AccessedSet (Record scope for general use)
        let flatten_ctx = FlattenContext::new(self.accessed.clone(), ParserScope::Record);
        Self {
            doc: self.doc,
            node_id: self.node_id,
            variant_path: self.variant_path.clone(),
            flatten_ctx: Some(flatten_ctx),
            union_tag_mode: self.union_tag_mode,
            accessed: self.accessed.clone(),
        }
    }

    /// Get the union tag mode.
    pub fn union_tag_mode(&self) -> UnionTagMode {
        self.union_tag_mode
    }

    /// Parse the current node as type T.
    pub fn parse<T: ParseDocument<'doc>>(&self) -> Result<T, T::Error> {
        T::parse(self)
    }

    pub fn parse_with<T: DocumentParser<'doc>>(
        &self,
        mut parser: T,
    ) -> Result<T::Output, T::Error> {
        parser.parse(self)
    }

    /// Get a union parser for the current node with the specified variant representation.
    ///
    /// Returns error if `$variant` extension has invalid type or syntax.
    ///
    /// # Arguments
    ///
    /// * `repr` - The variant representation to use. Use `VariantRepr::default()` for Untagged.
    pub fn parse_union<T, E>(&self, repr: VariantRepr) -> Result<UnionParser<'doc, '_, T, E>, E>
    where
        E: From<ParseError>,
    {
        UnionParser::new(self, repr).map_err(Into::into)
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

    // =========================================================================
    // Extension parsing methods
    // =========================================================================

    /// Get the AccessedSet for this context.
    pub(crate) fn accessed(&self) -> &AccessedSet {
        &self.accessed
    }

    /// Mark an extension as accessed.
    fn mark_ext_accessed(&self, ident: Identifier) {
        self.accessed.add_ext(ident);
    }

    /// Get a required extension field.
    ///
    /// Returns `ParseErrorKind::MissingExtension` if the extension is not present.
    pub fn parse_ext<T>(&self, name: &str) -> Result<T, T::Error>
    where
        T: ParseDocument<'doc>,
        T::Error: From<ParseError>,
    {
        self.parse_ext_with(name, T::parse)
    }

    /// Get a required extension field with a custom parser.
    pub fn parse_ext_with<T>(&self, name: &str, mut parser: T) -> Result<T::Output, T::Error>
    where
        T: DocumentParser<'doc>,
        T::Error: From<ParseError>,
    {
        let ident: Identifier = name.parse().map_err(|e| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::InvalidIdentifier(e),
        })?;
        self.mark_ext_accessed(ident.clone());
        let ext_node_id = self
            .node()
            .extensions
            .get(&ident)
            .ok_or_else(|| ParseError {
                node_id: self.node_id,
                kind: ParseErrorKind::MissingExtension(name.to_string()),
            })?;
        let ctx = ParseContext::with_union_tag_mode(self.doc, *ext_node_id, self.union_tag_mode);
        parser.parse(&ctx)
    }

    /// Get an optional extension field.
    ///
    /// Returns `Ok(None)` if the extension is not present.
    pub fn parse_ext_optional<T>(&self, name: &str) -> Result<Option<T>, T::Error>
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
        &self,
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
        self.mark_ext_accessed(ident.clone());
        match self.node().extensions.get(&ident) {
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
    pub fn ext(&self, name: &str) -> Result<ParseContext<'doc>, ParseError> {
        let ident: Identifier = name.parse().map_err(|e| ParseError {
            node_id: self.node_id,
            kind: ParseErrorKind::InvalidIdentifier(e),
        })?;
        self.mark_ext_accessed(ident.clone());
        let ext_node_id =
            self.node()
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
    pub fn ext_optional(&self, name: &str) -> Option<ParseContext<'doc>> {
        let ident: Identifier = name.parse().ok()?;
        self.mark_ext_accessed(ident.clone());
        self.node().extensions.get(&ident).map(|&node_id| {
            ParseContext::with_union_tag_mode(self.doc, node_id, self.union_tag_mode)
        })
    }

    /// Finish parsing with Deny policy (error if unknown extensions exist).
    ///
    /// **Flatten behavior**: If this context is in a flatten chain (has flatten_ctx),
    /// this is a no-op. Only root parsers validate.
    pub fn deny_unknown_extensions(&self) -> Result<(), ParseError> {
        // If child (in any flatten context), no-op - parent will validate
        if self.flatten_ctx.is_some() {
            return Ok(());
        }

        // Root parser - validate using accessed set
        for (ident, _) in self.node().extensions.iter() {
            if !self.accessed.has_ext(ident) {
                return Err(ParseError {
                    node_id: self.node_id,
                    kind: ParseErrorKind::UnknownExtension(ident.clone()),
                });
            }
        }
        Ok(())
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
        self.node()
            .extensions
            .iter()
            .filter_map(move |(ident, &node_id)| {
                if !accessed.has_ext(ident) {
                    Some((ident, ParseContext::with_union_tag_mode(doc, node_id, mode)))
                } else {
                    None
                }
            })
    }

    /// Create a flatten context for child parsers in Extension scope.
    ///
    /// This creates a FlattenContext initialized with the current accessed extensions,
    /// and returns a ParseContext that children can use. Children created from this
    /// context will:
    /// - Add their accessed extensions to the shared FlattenContext
    /// - Have deny_unknown_extensions() be a no-op
    ///
    /// The root parser should call deny_unknown_extensions() after all children are done.
    pub fn flatten_ext(&self) -> ParseContext<'doc> {
        // Create or reuse flatten context with Extension scope
        let flatten_ctx = match &self.flatten_ctx {
            Some(fc) => fc.clone(),
            None => {
                // Root: create new FlattenContext wrapping our AccessedSet with Extension scope
                FlattenContext::new(self.accessed.clone(), ParserScope::Extension)
            }
        };

        ParseContext::with_flatten_ctx(self.doc, self.node_id, flatten_ctx, self.union_tag_mode)
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
            flatten_ctx: self.flatten_ctx.clone(),
            union_tag_mode: self.union_tag_mode,
            accessed: self.accessed.clone(),
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
    /// The error type returned by parsing.
    type Error;

    /// Parse a value of this type from the given parse context.
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error>;
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

    /// Invalid value pattern or format.
    ///
    /// Used for validation errors in types like regex, URL, UUID, etc.
    /// - `kind`: Type of validation (e.g., "regex", "url", "uuid", "pattern: <expected>")
    /// - `reason`: Human-readable error message explaining the failure
    #[error("invalid {kind}: {reason}")]
    InvalidPattern { kind: String, reason: String },

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

    /// Unknown extension on node.
    #[error("unknown extension: ${0}")]
    UnknownExtension(Identifier),

    /// Invalid key type in record (expected string).
    #[error("invalid key type in record: expected string key, got {0:?}")]
    InvalidKeyType(crate::value::ObjectKey),

    /// No variant matched in union type.
    #[error("no matching variant{}", variant.as_ref().map(|v| format!(" (variant: {})", v)).unwrap_or_default())]
    NoMatchingVariant {
        /// Variant name extracted (if any).
        variant: Option<String>,
    },

    /// Conflicting variant tags: $variant and repr extracted different variant names.
    #[error("conflicting variant tags: $variant = {explicit}, repr = {repr}")]
    ConflictingVariantTags { explicit: String, repr: String },

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

    /// Tried to parse record fields while in extension flatten scope.
    /// This happens when using #[eure(flatten_ext)] with a type that calls parse_record().
    #[error(
        "cannot parse record in extension scope: use #[eure(flatten)] instead of #[eure(flatten_ext)]"
    )]
    RecordInExtensionScope,
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
    pub fn parse<T: ParseDocument<'doc>>(&'doc self, node_id: NodeId) -> Result<T, T::Error> {
        self.parse_with(node_id, T::parse)
    }

    pub fn parse_with<T: DocumentParser<'doc>>(
        &'doc self,
        node_id: NodeId,
        mut parser: T,
    ) -> Result<T::Output, T::Error> {
        parser.parse(&self.parse_context(node_id))
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

    /// Create a parse context for extension parsing.
    ///
    /// Convenience method equivalent to `doc.parse_context(node_id)`.
    /// Use the returned context's `parse_ext()`, `ext()`, etc. methods.
    pub fn parse_extension_context(&'doc self, node_id: NodeId) -> ParseContext<'doc> {
        ParseContext::new(self, node_id)
    }

    /// Parse a node as a tuple.
    ///
    /// Convenience method equivalent to `doc.parse_context(node_id).parse_tuple()`.
    pub fn parse_tuple(&'doc self, node_id: NodeId) -> Result<TupleParser<'doc>, ParseError> {
        TupleParser::from_doc_and_node(self, node_id)
    }
}

impl<'doc> ParseDocument<'doc> for EureDocument {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        Ok(ctx.doc().node_subtree_to_document(ctx.node_id()))
    }
}

impl<'doc> ParseDocument<'doc> for &'doc str {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
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
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        ctx.parse::<&str>().map(String::from)
    }
}

impl ParseDocument<'_> for Text {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
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
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
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
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
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
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
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
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        match ctx.parse_primitive()? {
            // Accept both F32 (with conversion) and F64
            PrimitiveValue::F32(f) => Ok(*f as f64),
            PrimitiveValue::F64(f) => Ok(*f),
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
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let value: BigInt = ctx.parse()?;
        u32::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of u32 range", value)),
        })
    }
}

impl ParseDocument<'_> for i32 {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let value: BigInt = ctx.parse()?;
        i32::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of i32 range", value)),
        })
    }
}

impl ParseDocument<'_> for i64 {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let value: BigInt = ctx.parse()?;
        i64::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of i64 range", value)),
        })
    }
}

impl ParseDocument<'_> for u64 {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let value: BigInt = ctx.parse()?;
        u64::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of u64 range", value)),
        })
    }
}

impl ParseDocument<'_> for usize {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let value: BigInt = ctx.parse()?;
        usize::try_from(&value).map_err(|_| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::OutOfRange(format!("value {} out of usize range", value)),
        })
    }
}

impl<'doc> ParseDocument<'doc> for &'doc PrimitiveValue {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        ctx.parse_primitive()
    }
}

impl ParseDocument<'_> for PrimitiveValue {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        ctx.parse::<&PrimitiveValue>().cloned()
    }
}

impl ParseDocument<'_> for Identifier {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
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
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
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
    T::Error: From<ParseError>,
{
    type Error = T::Error;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        ctx.ensure_no_variant_path()?;
        match &ctx.node().content {
            NodeValue::Array(array) => array
                .iter()
                .map(|item| T::parse(&ctx.at(*item)))
                .collect::<Result<Vec<_>, _>>(),
            value => Err(ParseError {
                node_id: ctx.node_id(),
                kind: handle_unexpected_node_value(value),
            }
            .into()),
        }
    }
}

macro_rules! parse_tuple {
    ($n:expr, $($var:ident),*) => {
        impl<'doc, $($var),*, Err> ParseDocument<'doc> for ($($var),*,)
            where $($var: ParseDocument<'doc, Error = Err>),*,
            Err: From<ParseError>,
        {
            type Error = Err;

            fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
                ctx.ensure_no_variant_path()?;
                let tuple = match &ctx.node().content {
                    NodeValue::Tuple(tuple) => tuple,
                    value => return Err(ParseError { node_id: ctx.node_id(), kind: handle_unexpected_node_value(value) }.into()),
                };
                if tuple.len() != $n {
                    return Err(ParseError { node_id: ctx.node_id(), kind: ParseErrorKind::UnexpectedTupleLength { expected: $n, actual: tuple.len() } }.into());
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

macro_rules! parse_map {
    ($ctx:ident) => {{
        $ctx.ensure_no_variant_path()?;

        // Check scope: Extension scope iterates extensions, otherwise record fields
        if $ctx.parser_scope() == Some(ParserScope::Extension) {
            // Extension scope: iterate UNACCESSED extensions only
            let node = $ctx.node();
            let accessed = $ctx.flatten_ctx().map(|fc| fc.accessed_set());
            node.extensions
                .iter()
                .filter(|(ident, _)| {
                    // Only include extensions not already accessed
                    accessed.map_or(true, |a| !a.has_ext(ident))
                })
                .map(|(ident, &node_id)| {
                    Ok((
                        K::from_extension_ident(ident).map_err(|kind| ParseError {
                            node_id: $ctx.node_id(),
                            kind,
                        })?,
                        T::parse(&$ctx.at(node_id))?,
                    ))
                })
                .collect::<Result<_, _>>()
        } else {
            // Record scope or no scope: iterate record fields
            let map = match &$ctx.node().content {
                NodeValue::Map(map) => map,
                value => {
                    return Err(ParseError {
                        node_id: $ctx.node_id(),
                        kind: handle_unexpected_node_value(value),
                    }
                    .into());
                }
            };
            // If in flatten context with Record scope, only iterate UNACCESSED fields
            let accessed = $ctx
                .flatten_ctx()
                .filter(|fc| fc.scope() == ParserScope::Record)
                .map(|fc| fc.accessed_set().clone());
            map.iter()
                .filter(|(key, _)| {
                    match &accessed {
                        Some(acc) => match key {
                            ObjectKey::String(s) => !acc.has_field(s),
                            _ => true, // Non-string keys are always included
                        },
                        None => true, // No flatten context means include all
                    }
                })
                .map(|(key, value)| {
                    Ok((
                        K::from_object_key(key).map_err(|kind| ParseError {
                            node_id: $ctx.node_id(),
                            kind,
                        })?,
                        T::parse(&$ctx.at(*value))?,
                    ))
                })
                .collect::<Result<_, _>>()
        }
    }};
}

impl<'doc, K, T> ParseDocument<'doc> for Map<K, T>
where
    K: ParseObjectKey<'doc>,
    T: ParseDocument<'doc>,
    T::Error: From<ParseError>,
{
    type Error = T::Error;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        parse_map!(ctx)
    }
}

impl<'doc, K, T> ParseDocument<'doc> for BTreeMap<K, T>
where
    K: ParseObjectKey<'doc>,
    T: ParseDocument<'doc>,
    T::Error: From<ParseError>,
{
    type Error = T::Error;
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        parse_map!(ctx)
    }
}

impl<'doc, K, T> ParseDocument<'doc> for HashMap<K, T>
where
    K: ParseObjectKey<'doc>,
    T: ParseDocument<'doc>,
    T::Error: From<ParseError>,
{
    type Error = T::Error;
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        parse_map!(ctx)
    }
}

impl ParseDocument<'_> for regex::Regex {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let pattern: &str = ctx.parse()?;
        regex::Regex::new(pattern).map_err(|e| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::InvalidPattern {
                kind: format!("regex '{}'", pattern),
                reason: e.to_string(),
            },
        })
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
    T::Error: From<ParseError>,
{
    type Error = T::Error;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        ctx.parse_union::<Option<T>, T::Error>(VariantRepr::default())?
            .variant("some", (T::parse).map(Some))
            .variant("none", |ctx: &ParseContext<'_>| {
                if ctx.is_null() {
                    Ok(None)
                } else {
                    Err(ParseError {
                        node_id: ctx.node_id(),
                        kind: ParseErrorKind::TypeMismatch {
                            expected: ValueKind::Null,
                            actual: ctx.node().content.value_kind().unwrap_or(ValueKind::Null),
                        },
                    }
                    .into())
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
impl<'doc, T, E, Err> ParseDocument<'doc> for Result<T, E>
where
    T: ParseDocument<'doc, Error = Err>,
    E: ParseDocument<'doc, Error = Err>,
    Err: From<ParseError>,
{
    type Error = Err;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        ctx.parse_union::<Self, Self::Error>(VariantRepr::default())?
            .variant("ok", (T::parse).map(Ok))
            .variant("err", (E::parse).map(Err))
            .parse()
    }
}

impl ParseDocument<'_> for crate::data_model::VariantRepr {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
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
        let rec = ctx.parse_record()?;

        let tag = rec.parse_field_optional::<String>("tag")?;
        let content = rec.parse_field_optional::<String>("content")?;

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

impl<'doc> ParseDocument<'doc> for () {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        ctx.parse_tuple()?.finish()
    }
}

impl<'doc> ParseDocument<'doc> for NodeId {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        Ok(ctx.node_id())
    }
}

pub trait DocumentParser<'doc> {
    type Output;
    type Error;
    fn parse(&mut self, ctx: &ParseContext<'doc>) -> Result<Self::Output, Self::Error>;
}

pub struct AlwaysParser<T, E>(T, PhantomData<E>);

impl<T, E> AlwaysParser<T, E> {
    pub fn new(value: T) -> AlwaysParser<T, E> {
        Self(value, PhantomData)
    }
}

impl<'doc, T, E> DocumentParser<'doc> for AlwaysParser<T, E>
where
    T: Clone,
{
    type Output = T;
    type Error = E;
    fn parse(&mut self, _ctx: &ParseContext<'doc>) -> Result<Self::Output, Self::Error> {
        Ok(self.0.clone())
    }
}

impl<'doc, T, F, E> DocumentParser<'doc> for F
where
    F: FnMut(&ParseContext<'doc>) -> Result<T, E>,
{
    type Output = T;
    type Error = E;
    fn parse(&mut self, ctx: &ParseContext<'doc>) -> Result<Self::Output, Self::Error> {
        (*self)(ctx)
    }
}

pub struct LiteralParser<T>(pub T);

impl<'doc, T, E> DocumentParser<'doc> for LiteralParser<T>
where
    T: ParseDocument<'doc, Error = E> + PartialEq + core::fmt::Debug,
    E: From<ParseError>,
{
    type Output = T;
    type Error = E;
    fn parse(&mut self, ctx: &ParseContext<'doc>) -> Result<Self::Output, Self::Error> {
        let value: T = ctx.parse::<T>()?;
        if value == self.0 {
            Ok(value)
        } else {
            Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::LiteralMismatch {
                    expected: format!("{:?}", self.0),
                    actual: format!("{:?}", value),
                },
            }
            .into())
        }
    }
}

/// A parser that matches a specific string literal as an enum variant name.
///
/// Similar to [`LiteralParser`], but returns [`ParseErrorKind::UnknownVariant`]
/// on mismatch instead of [`ParseErrorKind::LiteralMismatch`]. This provides
/// better error messages when parsing unit enum variants as string literals.
pub struct VariantLiteralParser(pub &'static str);

impl<'doc> DocumentParser<'doc> for VariantLiteralParser {
    type Output = &'static str;
    type Error = ParseError;
    fn parse(&mut self, ctx: &ParseContext<'doc>) -> Result<Self::Output, Self::Error> {
        let value: &str = ctx.parse()?;
        if value == self.0 {
            Ok(self.0)
        } else {
            Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::UnknownVariant(value.to_string()),
            })
        }
    }
}

pub struct MapParser<T, F> {
    parser: T,
    mapper: F,
}

impl<'doc, T, O, F> DocumentParser<'doc> for MapParser<T, F>
where
    T: DocumentParser<'doc>,
    F: FnMut(T::Output) -> O,
{
    type Output = O;
    type Error = T::Error;
    fn parse(&mut self, ctx: &ParseContext<'doc>) -> Result<Self::Output, Self::Error> {
        self.parser.parse(ctx).map(|value| (self.mapper)(value))
    }
}

pub struct AndThenParser<T, F> {
    parser: T,
    mapper: F,
}

impl<'doc, T, O, F, E> DocumentParser<'doc> for AndThenParser<T, F>
where
    T: DocumentParser<'doc, Error = E>,
    F: Fn(T::Output) -> Result<O, E>,
{
    type Output = O;
    type Error = E;
    fn parse(&mut self, ctx: &ParseContext<'doc>) -> Result<Self::Output, Self::Error> {
        let value = self.parser.parse(ctx)?;
        (self.mapper)(value)
    }
}

pub trait DocumentParserExt<'doc>: DocumentParser<'doc> + Sized {
    fn map<O, F>(self, mapper: F) -> MapParser<Self, F>
    where
        F: Fn(Self::Output) -> O,
    {
        MapParser {
            parser: self,
            mapper,
        }
    }

    fn and_then<O, F>(self, mapper: F) -> AndThenParser<Self, F>
    where
        F: Fn(Self::Output) -> Result<O, Self::Error>,
    {
        AndThenParser {
            parser: self,
            mapper,
        }
    }
}

impl<'doc, T> DocumentParserExt<'doc> for T where T: DocumentParser<'doc> {}

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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.parse_field("value").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_option_none_tagged() {
        let doc =
            create_record_with_variant("value", NodeValue::Primitive(PrimitiveValue::Null), "none");
        let root_id = doc.get_root_id();
        let rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.parse_field("value").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_option_some_untagged() {
        // Without $variant, non-null value is Some
        let doc = eure!({ value = 42 });
        let root_id = doc.get_root_id();
        let rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.parse_field("value").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_option_none_untagged() {
        // Without $variant, null is None
        let doc = eure!({ value = null });
        let root_id = doc.get_root_id();
        let rec = doc.parse_record(root_id).unwrap();
        let value: Option<i32> = rec.parse_field("value").unwrap();
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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Result<i32, String> = rec.parse_field("value").unwrap();
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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Result<i32, String> = rec.parse_field("value").unwrap();
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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Result<Option<i32>, String> = rec.parse_field("value").unwrap();
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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Result<Option<i32>, String> = rec.parse_field("value").unwrap();
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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Result<Option<i32>, String> = rec.parse_field("value").unwrap();
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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Option<Option<i32>> = rec.parse_field("value").unwrap();
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
        let rec = doc.parse_record(root_id).unwrap();
        let value: Option<Option<i32>> = rec.parse_field("value").unwrap();
        assert_eq!(value, Some(None));
    }

    #[test]
    fn test_outer_none() {
        // $variant: none - Option<Option<i32>> outer None
        let doc =
            create_record_with_variant("value", NodeValue::Primitive(PrimitiveValue::Null), "none");
        let root_id = doc.get_root_id();
        let rec = doc.parse_record(root_id).unwrap();
        let value: Option<Option<i32>> = rec.parse_field("value").unwrap();
        assert_eq!(value, None);
    }
}
