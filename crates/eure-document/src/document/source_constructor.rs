//! Source-aware document constructor.
//!
//! This module provides [`SourceConstructor`], which builds an [`EureDocument`]
//! while tracking source layout information for round-trip formatting.
//!
//! # Architecture
//!
//! `SourceConstructor` builds both the semantic document and an AST representation
//! of the source structure. The 6 patterns from the Eure grammar are:
//!
//! | # | Pattern | API calls |
//! |---|---------|-----------|
//! | 1 | `path = value` | `begin_binding` → navigate → `bind_*` → `end_binding_value` |
//! | 2 | `path { eure }` | `begin_binding` → navigate → `begin_eure_block` → ... → `end_eure_block` → `end_binding_block` |
//! | 3 | `path { = value eure }` | `begin_binding` → navigate → `begin_eure_block` → `bind_*` → `set_block_value` → ... → `end_eure_block` → `end_binding_block` |
//! | 4 | `@ section` (items) | `begin_section` → navigate → `begin_section_items` → ... → `end_section_items` |
//! | 5 | `@ section { eure }` | `begin_section` → navigate → `begin_eure_block` → ... → `end_eure_block` → `end_section_block` |
//! | 6 | `@ section { = value eure }` | `begin_section` → navigate → `begin_eure_block` → `bind_*` → `set_block_value` → ... → `end_eure_block` → `end_section_block` |

use crate::document::constructor::{DocumentConstructor, Scope};
use crate::document::interpreter_sink::InterpreterSink;
use crate::document::{ConstructorError, InsertError, NodeId};
use crate::path::PathSegment;
use crate::prelude_internal::*;
use crate::source::{
    BindingSource, Comment, EureSource, SectionSource, SourceDocument, SourceId, SourceKey,
    SourcePath, SourcePathSegment, Trivia,
};

/// Builder context for tracking nested structures.
#[derive(Debug)]
enum BuilderContext {
    /// Building an EureSource block (for `{ eure }` patterns)
    EureBlock {
        /// The SourceId for this block in the arena
        source_id: SourceId,
        /// Saved pending path from the enclosing binding/section
        saved_path: SourcePath,
        /// Saved pending trivia from the enclosing context
        saved_trivia: Vec<Trivia>,
    },
    /// Building section items (for `@ section` pattern #4)
    SectionItems {
        /// Trivia before this section header
        trivia_before: Vec<Trivia>,
        /// Path for the section header
        path: SourcePath,
        /// Optional initial value binding
        value: Option<NodeId>,
        /// Bindings collected so far
        bindings: Vec<BindingSource>,
    },
}

/// A document constructor that tracks source layout for round-trip formatting.
///
/// `SourceConstructor` wraps [`DocumentConstructor`] and records source structure
/// (sections, bindings, comments) as an AST. This enables converting from other
/// formats (like TOML) while preserving their structure.
///
/// # Example
///
/// ```ignore
/// let mut constructor = SourceConstructor::new();
///
/// // Build: name = "Alice" (pattern #1)
/// constructor.begin_binding();
/// let scope = constructor.begin_scope();
/// constructor.navigate(PathSegment::Ident("name".parse()?))?;
/// constructor.bind_primitive("Alice".into())?;
/// constructor.end_scope(scope)?;
/// constructor.end_binding_value().unwrap();
///
/// // Build: user { name = "Bob" } (pattern #2)
/// constructor.begin_binding();
/// let scope = constructor.begin_scope();
/// constructor.navigate(PathSegment::Ident("user".parse()?))?;
/// constructor.begin_eure_block();
///   constructor.begin_binding();
///   let inner_scope = constructor.begin_scope();
///   constructor.navigate(PathSegment::Ident("name".parse()?))?;
///   constructor.bind_primitive("Bob".into())?;
///   constructor.end_scope(inner_scope)?;
///   constructor.end_binding_value().unwrap();
/// constructor.end_eure_block().unwrap();
/// constructor.end_scope(scope)?;
/// constructor.end_binding_block().unwrap();
///
/// let source_doc = constructor.finish();
/// ```
pub struct SourceConstructor {
    /// The underlying document constructor
    inner: DocumentConstructor,

    /// Arena of EureSource blocks
    sources: Vec<EureSource>,

    /// Stack of builder contexts for nested structures
    builder_stack: Vec<BuilderContext>,

    /// Pending path segments for the current binding/section
    pending_path: Vec<SourcePathSegment>,

    /// Pending trivia (comments/blank lines) to attach to the next item
    pending_trivia: Vec<Trivia>,

    /// Node ID of the last bound value (for end_binding_value and set_block_value)
    last_bound_node: Option<NodeId>,

    /// SourceId of the last completed EureSource block (for end_binding_block/end_section_block)
    last_block_id: Option<SourceId>,
}

impl Default for SourceConstructor {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceConstructor {
    /// Create a new source constructor.
    #[must_use]
    pub fn new() -> Self {
        // Create root EureSource (index 0)
        let sources = vec![EureSource::default()];

        Self {
            inner: DocumentConstructor::new(),
            sources,
            builder_stack: vec![BuilderContext::EureBlock {
                source_id: SourceId(0),
                saved_path: Vec::new(),
                saved_trivia: Vec::new(),
            }],
            pending_path: Vec::new(),
            pending_trivia: Vec::new(),
            last_bound_node: None,
            last_block_id: None,
        }
    }

    /// Finish building and return the [`SourceDocument`].
    #[must_use]
    pub fn finish(mut self) -> SourceDocument {
        // Any remaining pending trivia becomes trailing trivia of the root source
        if !self.pending_trivia.is_empty() {
            self.sources[0].trailing_trivia = std::mem::take(&mut self.pending_trivia);
        }
        SourceDocument::new(self.inner.finish(), self.sources)
    }

    /// Get mutable reference to the current EureSource being built.
    ///
    /// Finds the nearest EureBlock context in the builder stack.
    fn current_source_mut(&mut self) -> &mut EureSource {
        for ctx in self.builder_stack.iter().rev() {
            if let BuilderContext::EureBlock { source_id, .. } = ctx {
                return &mut self.sources[source_id.0];
            }
        }
        // Root EureBlock should always be present
        &mut self.sources[0]
    }

    // ========================================================================
    // Inherent methods (mirror InterpreterSink trait for macro compatibility)
    //
    // These methods allow the eure! macro to work without importing the
    // InterpreterSink trait.
    // ========================================================================

    /// Begin a new scope. Returns a handle that must be passed to `end_scope`.
    pub fn begin_scope(&mut self) -> Scope {
        InterpreterSink::begin_scope(self)
    }

    /// End a scope, restoring to the state when `begin_scope` was called.
    pub fn end_scope(&mut self, scope: Scope) -> Result<(), InsertError> {
        InterpreterSink::end_scope(self, scope)
    }

    /// Navigate to a child node by path segment.
    pub fn navigate(&mut self, segment: PathSegment) -> Result<NodeId, InsertError> {
        InterpreterSink::navigate(self, segment)
    }

    /// Assert that the current node is unbound (a hole).
    pub fn require_hole(&self) -> Result<(), InsertError> {
        InterpreterSink::require_hole(self)
    }

    /// Bind a primitive value to the current node.
    pub fn bind_primitive(&mut self, value: PrimitiveValue) -> Result<(), InsertError> {
        InterpreterSink::bind_primitive(self, value)
    }

    /// Bind a hole (with optional label) to the current node.
    pub fn bind_hole(&mut self, label: Option<Identifier>) -> Result<(), InsertError> {
        InterpreterSink::bind_hole(self, label)
    }

    /// Bind an empty map to the current node.
    pub fn bind_empty_map(&mut self) -> Result<(), InsertError> {
        InterpreterSink::bind_empty_map(self)
    }

    /// Bind an empty array to the current node.
    pub fn bind_empty_array(&mut self) -> Result<(), InsertError> {
        InterpreterSink::bind_empty_array(self)
    }

    /// Bind an empty tuple to the current node.
    pub fn bind_empty_tuple(&mut self) -> Result<(), InsertError> {
        InterpreterSink::bind_empty_tuple(self)
    }

    /// Bind a value using `Into<PrimitiveValue>`.
    pub fn bind_from(&mut self, value: impl Into<PrimitiveValue>) -> Result<(), InsertError> {
        InterpreterSink::bind_from(self, value)
    }

    /// Get the current node ID.
    pub fn current_node_id(&self) -> NodeId {
        InterpreterSink::current_node_id(self)
    }

    /// Get the current path from root.
    pub fn current_path(&self) -> &[PathSegment] {
        InterpreterSink::current_path(self)
    }

    /// Get a reference to the document being built.
    pub fn document(&self) -> &EureDocument {
        InterpreterSink::document(self)
    }

    /// Get a mutable reference to the document being built.
    pub fn document_mut(&mut self) -> &mut EureDocument {
        InterpreterSink::document_mut(self)
    }

    // =========================================================================
    // Source Layout Markers (inherent methods for macro compatibility)
    // =========================================================================

    /// Enter a new EureSource block (for `{ eure }` patterns).
    pub fn begin_eure_block(&mut self) {
        InterpreterSink::begin_eure_block(self)
    }

    /// Set the value binding for current block (for `{ = value ... }` patterns).
    pub fn set_block_value(&mut self) -> Result<(), InsertError> {
        InterpreterSink::set_block_value(self)
    }

    /// End current EureSource block.
    pub fn end_eure_block(&mut self) -> Result<(), InsertError> {
        InterpreterSink::end_eure_block(self)
    }

    /// Start a binding statement.
    pub fn begin_binding(&mut self) {
        InterpreterSink::begin_binding(self)
    }

    /// End binding #1: `path = value`.
    pub fn end_binding_value(&mut self) -> Result<(), InsertError> {
        InterpreterSink::end_binding_value(self)
    }

    /// End binding #2/#3: `path { eure }`.
    pub fn end_binding_block(&mut self) -> Result<(), InsertError> {
        InterpreterSink::end_binding_block(self)
    }

    /// Start a section header.
    pub fn begin_section(&mut self) {
        InterpreterSink::begin_section(self)
    }

    /// Begin section #4: `@ section` (items follow).
    pub fn begin_section_items(&mut self) {
        InterpreterSink::begin_section_items(self)
    }

    /// End section #4: finalize section with items body.
    pub fn end_section_items(&mut self) -> Result<(), InsertError> {
        InterpreterSink::end_section_items(self)
    }

    /// End section #5/#6: `@ section { eure }`.
    pub fn end_section_block(&mut self) -> Result<(), InsertError> {
        InterpreterSink::end_section_block(self)
    }

    /// Add a comment to the pending trivia.
    pub fn comment(&mut self, comment: Comment) {
        InterpreterSink::comment(self, comment)
    }

    /// Add a blank line to the pending trivia.
    pub fn blank_line(&mut self) {
        InterpreterSink::blank_line(self)
    }

    /// Add trivia (comment or blank line) to the pending trivia.
    pub fn add_trivia(&mut self, trivia: Trivia) {
        self.pending_trivia.push(trivia);
    }

    // =========================================================================
    // Helper methods
    // =========================================================================

    /// Convert a PathSegment to a SourcePathSegment.
    fn path_segment_to_source(segment: &PathSegment) -> SourcePathSegment {
        match segment {
            PathSegment::Ident(id) => SourcePathSegment::ident(id.clone()),
            PathSegment::Extension(id) => SourcePathSegment::extension(id.clone()),
            PathSegment::Value(key) => SourcePathSegment {
                key: Self::object_key_to_source_key(key),
                array: None,
            },
            PathSegment::TupleIndex(idx) => SourcePathSegment {
                key: SourceKey::TupleIndex(*idx),
                array: None,
            },
            PathSegment::ArrayIndex(_) => {
                // Array index should always be merged with the previous segment in navigate().
                // This conversion should never be called directly.
                unreachable!(
                    "ArrayIndex should be merged with previous segment, not converted directly"
                )
            }
        }
    }

    /// Convert an ObjectKey to a SourceKey.
    fn object_key_to_source_key(key: &ObjectKey) -> SourceKey {
        match key {
            ObjectKey::String(s) => {
                // Try to parse as identifier, otherwise use string
                if let Ok(id) = s.parse::<Identifier>() {
                    SourceKey::Ident(id)
                } else {
                    SourceKey::quoted(s.clone())
                }
            }
            ObjectKey::Number(n) => {
                // Try to convert BigInt to i64, fallback to string representation
                if let Ok(n64) = i64::try_from(n) {
                    SourceKey::Integer(n64)
                } else {
                    SourceKey::quoted(n.to_string())
                }
            }
            ObjectKey::Tuple(keys) => {
                SourceKey::Tuple(keys.iter().map(Self::object_key_to_source_key).collect())
            }
        }
    }

    /// Add a binding to the current context with pending trivia attached.
    fn push_binding(&mut self, mut binding: BindingSource) {
        // Attach pending trivia to this binding
        binding.trivia_before = std::mem::take(&mut self.pending_trivia);

        match self.builder_stack.last_mut() {
            Some(BuilderContext::SectionItems { bindings, .. }) => {
                bindings.push(binding);
            }
            Some(BuilderContext::EureBlock { source_id, .. }) => {
                self.sources[source_id.0].bindings.push(binding);
            }
            None => {
                // Should never happen - root context is always present
                self.sources[0].bindings.push(binding);
            }
        }
    }

    /// Add a section to the current EureSource with trivia attached.
    fn push_section(&mut self, mut section: SectionSource, trivia: Vec<Trivia>) {
        // Attach trivia to this section
        section.trivia_before = trivia;
        self.current_source_mut().sections.push(section);
    }
}

impl InterpreterSink for SourceConstructor {
    type Error = InsertError;
    type Scope = Scope;

    fn begin_scope(&mut self) -> Self::Scope {
        self.inner.begin_scope()
    }

    fn end_scope(&mut self, scope: Self::Scope) -> Result<(), Self::Error> {
        InterpreterSink::end_scope(&mut self.inner, scope)
    }

    fn navigate(&mut self, segment: PathSegment) -> Result<NodeId, Self::Error> {
        // Handle array markers: merge with previous segment
        if let PathSegment::ArrayIndex(idx) = &segment {
            let last = self.pending_path.last_mut().ok_or_else(|| InsertError {
                kind: ConstructorError::StandaloneArrayIndex.into(),
                path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
            })?;
            last.array = Some(*idx);
        } else {
            let source_segment = Self::path_segment_to_source(&segment);
            self.pending_path.push(source_segment);
        }

        InterpreterSink::navigate(&mut self.inner, segment)
    }

    fn require_hole(&self) -> Result<(), Self::Error> {
        self.inner.require_hole()
    }

    fn bind_primitive(&mut self, value: PrimitiveValue) -> Result<(), Self::Error> {
        self.last_bound_node = Some(self.inner.current_node_id());
        InterpreterSink::bind_primitive(&mut self.inner, value)
    }

    fn bind_hole(&mut self, label: Option<Identifier>) -> Result<(), Self::Error> {
        self.last_bound_node = Some(self.inner.current_node_id());
        InterpreterSink::bind_hole(&mut self.inner, label)
    }

    fn bind_empty_map(&mut self) -> Result<(), Self::Error> {
        self.last_bound_node = Some(self.inner.current_node_id());
        InterpreterSink::bind_empty_map(&mut self.inner)
    }

    fn bind_empty_array(&mut self) -> Result<(), Self::Error> {
        self.last_bound_node = Some(self.inner.current_node_id());
        InterpreterSink::bind_empty_array(&mut self.inner)
    }

    fn bind_empty_tuple(&mut self) -> Result<(), Self::Error> {
        self.last_bound_node = Some(self.inner.current_node_id());
        InterpreterSink::bind_empty_tuple(&mut self.inner)
    }

    fn current_node_id(&self) -> NodeId {
        self.inner.current_node_id()
    }

    fn current_path(&self) -> &[PathSegment] {
        self.inner.current_path()
    }

    fn document(&self) -> &EureDocument {
        self.inner.document()
    }

    fn document_mut(&mut self) -> &mut EureDocument {
        self.inner.document_mut()
    }

    // =========================================================================
    // Source Layout Markers (overrides with actual implementations)
    // =========================================================================

    fn begin_eure_block(&mut self) {
        // Create a new EureSource in the arena
        let source_id = SourceId(self.sources.len());
        self.sources.push(EureSource::default());

        // Save the pending path and trivia, clear them for the inner block
        let saved_path = std::mem::take(&mut self.pending_path);
        let saved_trivia = std::mem::take(&mut self.pending_trivia);

        // Push context
        self.builder_stack.push(BuilderContext::EureBlock {
            source_id,
            saved_path,
            saved_trivia,
        });
    }

    fn set_block_value(&mut self) -> Result<(), Self::Error> {
        // Set the value field of the current EureSource
        let node_id = self.last_bound_node.take().ok_or_else(|| InsertError {
            kind: ConstructorError::MissingBindBeforeSetBlockValue.into(),
            path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
        })?;
        self.current_source_mut().value = Some(node_id);
        Ok(())
    }

    fn end_eure_block(&mut self) -> Result<(), Self::Error> {
        // Any remaining pending trivia becomes trailing trivia of this block
        if !self.pending_trivia.is_empty() {
            let source_id = match self.builder_stack.last() {
                Some(BuilderContext::EureBlock { source_id, .. }) => *source_id,
                _ => {
                    return Err(InsertError {
                        kind: ConstructorError::InvalidBuilderStackForEndEureBlock.into(),
                        path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
                    });
                }
            };
            self.sources[source_id.0].trailing_trivia = std::mem::take(&mut self.pending_trivia);
        }

        // Pop the EureBlock context and record its SourceId
        match self.builder_stack.pop() {
            Some(BuilderContext::EureBlock {
                source_id,
                saved_path,
                saved_trivia,
            }) => {
                self.last_block_id = Some(source_id);
                // Restore the saved path and trivia for the enclosing binding/section
                self.pending_path = saved_path;
                self.pending_trivia = saved_trivia;
                Ok(())
            }
            _ => Err(InsertError {
                kind: ConstructorError::InvalidBuilderStackForEndEureBlock.into(),
                path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
            }),
        }
    }

    fn begin_binding(&mut self) {
        self.pending_path.clear();
    }

    fn end_binding_value(&mut self) -> Result<(), Self::Error> {
        // Pattern #1: path = value
        let path = std::mem::take(&mut self.pending_path);
        let node_id = self.last_bound_node.take().ok_or_else(|| InsertError {
            kind: ConstructorError::MissingBindBeforeEndBindingValue.into(),
            path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
        })?;

        let binding = BindingSource::value(path, node_id);
        self.push_binding(binding);
        Ok(())
    }

    fn end_binding_block(&mut self) -> Result<(), Self::Error> {
        // Pattern #2/#3: path { eure }
        let path = std::mem::take(&mut self.pending_path);
        let source_id = self.last_block_id.take().ok_or_else(|| InsertError {
            kind: ConstructorError::MissingEndEureBlockBeforeEndBindingBlock.into(),
            path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
        })?;

        let binding = BindingSource::block(path, source_id);
        self.push_binding(binding);
        Ok(())
    }

    fn begin_section(&mut self) {
        self.pending_path.clear();
    }

    fn begin_section_items(&mut self) {
        // Pattern #4: @ section (items follow)
        let path = std::mem::take(&mut self.pending_path);
        let trivia_before = std::mem::take(&mut self.pending_trivia);

        // Check if there was a value binding before this
        let value = self.last_bound_node.take();

        self.builder_stack.push(BuilderContext::SectionItems {
            trivia_before,
            path,
            value,
            bindings: Vec::new(),
        });
    }

    fn end_section_items(&mut self) -> Result<(), Self::Error> {
        // Finalize pattern #4
        match self.builder_stack.pop() {
            Some(BuilderContext::SectionItems {
                trivia_before,
                path,
                value,
                bindings,
            }) => {
                let section = SectionSource::items(path, value, bindings);
                self.push_section(section, trivia_before);
                Ok(())
            }
            _ => Err(InsertError {
                kind: ConstructorError::InvalidBuilderStackForEndSectionItems.into(),
                path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
            }),
        }
    }

    fn end_section_block(&mut self) -> Result<(), Self::Error> {
        // Pattern #5/#6: @ section { eure }
        let path = std::mem::take(&mut self.pending_path);
        let trivia_before = std::mem::take(&mut self.pending_trivia);
        let source_id = self.last_block_id.take().ok_or_else(|| InsertError {
            kind: ConstructorError::MissingEndEureBlockBeforeEndSectionBlock.into(),
            path: EurePath::from_iter(self.inner.current_path().iter().cloned()),
        })?;

        let section = SectionSource::block(path, source_id);
        self.push_section(section, trivia_before);
        Ok(())
    }

    fn comment(&mut self, comment: Comment) {
        self.pending_trivia.push(Trivia::Comment(comment));
    }

    fn blank_line(&mut self) {
        self.pending_trivia.push(Trivia::BlankLine);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::InsertErrorKind;
    use crate::source::{BindSource, SectionBody};

    fn ident(s: &str) -> Identifier {
        s.parse().unwrap()
    }

    // =========================================================================
    // Pattern #1: path = value
    // =========================================================================

    #[test]
    fn test_pattern1_simple_binding() {
        let mut constructor = SourceConstructor::new();

        // Build: name = "Alice"
        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("name")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("Alice")))
            .unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_value().unwrap();

        let source_doc = constructor.finish();

        // Check source structure
        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 1);
        assert!(root.sections.is_empty());
        assert!(root.value.is_none());

        let binding = &root.bindings[0];
        assert_eq!(binding.path.len(), 1);
        assert_eq!(binding.path[0].key, SourceKey::Ident(ident("name")));
        match &binding.bind {
            BindSource::Value(node_id) => {
                assert!(node_id.0 > 0); // Not root
            }
            _ => panic!("Expected BindSource::Value"),
        }
    }

    #[test]
    fn test_pattern1_nested_path() {
        let mut constructor = SourceConstructor::new();

        // Build: a.b.c = 42
        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("a")))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(ident("b")))
            .unwrap();
        constructor
            .navigate(PathSegment::Ident(ident("c")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(42.into()))
            .unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_value().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 1);

        let binding = &root.bindings[0];
        assert_eq!(binding.path.len(), 3);
        assert_eq!(binding.path[0].key, SourceKey::Ident(ident("a")));
        assert_eq!(binding.path[1].key, SourceKey::Ident(ident("b")));
        assert_eq!(binding.path[2].key, SourceKey::Ident(ident("c")));
    }

    // =========================================================================
    // Pattern #2: path { eure }
    // =========================================================================

    #[test]
    fn test_pattern2_binding_block() {
        let mut constructor = SourceConstructor::new();

        // Build: user { name = "Bob" }
        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("user")))
            .unwrap();
        constructor.begin_eure_block();

        // Inner binding: name = "Bob"
        constructor.begin_binding();
        let inner_scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("name")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("Bob")))
            .unwrap();
        constructor.end_scope(inner_scope).unwrap();
        constructor.end_binding_value().unwrap();

        constructor.end_eure_block().unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_block().unwrap();

        let source_doc = constructor.finish();

        // Check root
        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 1);

        let binding = &root.bindings[0];
        assert_eq!(binding.path.len(), 1);
        assert_eq!(binding.path[0].key, SourceKey::Ident(ident("user")));

        match &binding.bind {
            BindSource::Block(source_id) => {
                let inner_source = source_doc.source(*source_id);
                assert!(inner_source.value.is_none());
                assert_eq!(inner_source.bindings.len(), 1);
                assert_eq!(
                    inner_source.bindings[0].path[0].key,
                    SourceKey::Ident(ident("name"))
                );
            }
            _ => panic!("Expected BindSource::Block"),
        }
    }

    // =========================================================================
    // Pattern #3: path { = value eure }
    // =========================================================================

    #[test]
    fn test_pattern3_binding_value_block() {
        let mut constructor = SourceConstructor::new();

        // Build: data { = [] $schema = "array" }
        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("data")))
            .unwrap();
        constructor.begin_eure_block();

        // Value: = []
        constructor.bind_empty_array().unwrap();
        constructor.set_block_value().unwrap();

        // Inner binding: $schema = "array"
        constructor.begin_binding();
        let inner_scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Extension(ident("schema")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("array")))
            .unwrap();
        constructor.end_scope(inner_scope).unwrap();
        constructor.end_binding_value().unwrap();

        constructor.end_eure_block().unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_block().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 1);

        let binding = &root.bindings[0];
        match &binding.bind {
            BindSource::Block(source_id) => {
                let inner_source = source_doc.source(*source_id);
                // Should have a value
                assert!(inner_source.value.is_some());
                // And one binding
                assert_eq!(inner_source.bindings.len(), 1);
            }
            _ => panic!("Expected BindSource::Block"),
        }
    }

    // =========================================================================
    // Pattern #4: @ section (items follow)
    // =========================================================================

    #[test]
    fn test_pattern4_section_items() {
        let mut constructor = SourceConstructor::new();

        // Build:
        // @ server
        // host = "localhost"
        // port = 8080

        constructor.begin_section();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("server")))
            .unwrap();
        constructor.begin_section_items();

        // Binding 1: host = "localhost"
        constructor.begin_binding();
        let inner_scope1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("host")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("localhost")))
            .unwrap();
        constructor.end_scope(inner_scope1).unwrap();
        constructor.end_binding_value().unwrap();

        // Binding 2: port = 8080
        constructor.begin_binding();
        let inner_scope2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("port")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(8080.into()))
            .unwrap();
        constructor.end_scope(inner_scope2).unwrap();
        constructor.end_binding_value().unwrap();

        constructor.end_section_items().unwrap();
        constructor.end_scope(scope).unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert!(root.bindings.is_empty());
        assert_eq!(root.sections.len(), 1);

        let section = &root.sections[0];
        assert_eq!(section.path.len(), 1);
        assert_eq!(section.path[0].key, SourceKey::Ident(ident("server")));

        match &section.body {
            SectionBody::Items { value, bindings } => {
                assert!(value.is_none());
                assert_eq!(bindings.len(), 2);
                assert_eq!(bindings[0].path[0].key, SourceKey::Ident(ident("host")));
                assert_eq!(bindings[1].path[0].key, SourceKey::Ident(ident("port")));
            }
            _ => panic!("Expected SectionBody::Items"),
        }
    }

    // =========================================================================
    // Pattern #5: @ section { eure }
    // =========================================================================

    #[test]
    fn test_pattern5_section_block() {
        let mut constructor = SourceConstructor::new();

        // Build: @ server { host = "localhost" }
        constructor.begin_section();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("server")))
            .unwrap();
        constructor.begin_eure_block();

        // Inner binding: host = "localhost"
        constructor.begin_binding();
        let inner_scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("host")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("localhost")))
            .unwrap();
        constructor.end_scope(inner_scope).unwrap();
        constructor.end_binding_value().unwrap();

        constructor.end_eure_block().unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_section_block().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert!(root.bindings.is_empty());
        assert_eq!(root.sections.len(), 1);

        let section = &root.sections[0];
        match &section.body {
            SectionBody::Block(source_id) => {
                let inner_source = source_doc.source(*source_id);
                assert!(inner_source.value.is_none());
                assert_eq!(inner_source.bindings.len(), 1);
            }
            _ => panic!("Expected SectionBody::Block"),
        }
    }

    // =========================================================================
    // Pattern #6: @ section { = value eure }
    // =========================================================================

    #[test]
    fn test_pattern6_section_value_block() {
        let mut constructor = SourceConstructor::new();

        // Build: @ data { = [] $schema = "array" }
        constructor.begin_section();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("data")))
            .unwrap();
        constructor.begin_eure_block();

        // Value: = []
        constructor.bind_empty_array().unwrap();
        constructor.set_block_value().unwrap();

        // Inner binding: $schema = "array"
        constructor.begin_binding();
        let inner_scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Extension(ident("schema")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("array")))
            .unwrap();
        constructor.end_scope(inner_scope).unwrap();
        constructor.end_binding_value().unwrap();

        constructor.end_eure_block().unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_section_block().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.sections.len(), 1);

        let section = &root.sections[0];
        match &section.body {
            SectionBody::Block(source_id) => {
                let inner_source = source_doc.source(*source_id);
                // Should have a value
                assert!(inner_source.value.is_some());
                // And one binding
                assert_eq!(inner_source.bindings.len(), 1);
            }
            _ => panic!("Expected SectionBody::Block"),
        }
    }

    // =========================================================================
    // Array index tests
    // =========================================================================

    #[test]
    fn test_array_index_with_key() {
        // Build: items[0] = "first"
        let mut constructor = SourceConstructor::new();

        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("items")))
            .unwrap();
        constructor
            .navigate(PathSegment::ArrayIndex(Some(0)))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("first")))
            .unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_value().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 1);

        let binding = &root.bindings[0];
        // Path should have one segment with array marker
        assert_eq!(binding.path.len(), 1);
        assert_eq!(binding.path[0].key, SourceKey::Ident(ident("items")));
        assert_eq!(binding.path[0].array, Some(Some(0)));
    }

    #[test]
    fn test_array_push_marker() {
        // Build: items[] = "new"
        let mut constructor = SourceConstructor::new();

        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("items")))
            .unwrap();
        constructor.navigate(PathSegment::ArrayIndex(None)).unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("new")))
            .unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_value().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        let binding = &root.bindings[0];
        assert_eq!(binding.path.len(), 1);
        assert_eq!(binding.path[0].key, SourceKey::Ident(ident("items")));
        // None means array push (no index specified)
        assert_eq!(binding.path[0].array, Some(None));
    }

    #[test]
    fn test_standalone_array_index_returns_error() {
        // Standalone [] is not valid in Eure syntax
        let mut constructor = SourceConstructor::new();

        constructor.begin_binding();
        let _scope = constructor.begin_scope();
        // This should return an error - ArrayIndex without a preceding key segment
        let result = constructor.navigate(PathSegment::ArrayIndex(Some(0)));
        assert!(matches!(
            result,
            Err(InsertError {
                kind: InsertErrorKind::ConstructorError(ConstructorError::StandaloneArrayIndex),
                ..
            })
        ));
    }

    // =========================================================================
    // Error case tests
    // =========================================================================

    #[test]
    fn test_end_binding_value_without_bind_returns_error() {
        let mut constructor = SourceConstructor::new();

        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("name")))
            .unwrap();
        // Missing: bind operation here
        constructor.end_scope(scope).unwrap();
        let result = constructor.end_binding_value();
        assert!(matches!(
            result,
            Err(InsertError {
                kind: InsertErrorKind::ConstructorError(
                    ConstructorError::MissingBindBeforeEndBindingValue
                ),
                ..
            })
        ));
    }

    #[test]
    fn test_set_block_value_without_bind_returns_error() {
        let mut constructor = SourceConstructor::new();

        constructor.begin_binding();
        let _scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("data")))
            .unwrap();
        constructor.begin_eure_block();
        // Missing: bind operation here
        let result = constructor.set_block_value();
        assert!(matches!(
            result,
            Err(InsertError {
                kind: InsertErrorKind::ConstructorError(
                    ConstructorError::MissingBindBeforeSetBlockValue
                ),
                ..
            })
        ));
    }

    #[test]
    fn test_end_binding_block_without_end_eure_block_returns_error() {
        let mut constructor = SourceConstructor::new();

        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("data")))
            .unwrap();
        // Missing: begin_eure_block, end_eure_block
        constructor.end_scope(scope).unwrap();
        let result = constructor.end_binding_block();
        assert!(matches!(
            result,
            Err(InsertError {
                kind: InsertErrorKind::ConstructorError(
                    ConstructorError::MissingEndEureBlockBeforeEndBindingBlock
                ),
                ..
            })
        ));
    }

    // =========================================================================
    // Complex nested structure tests
    // =========================================================================

    #[test]
    fn test_multiple_bindings() {
        let mut constructor = SourceConstructor::new();

        // Build: a = 1, b = 2
        for (name, value) in [("a", 1), ("b", 2)] {
            constructor.begin_binding();
            let scope = constructor.begin_scope();
            constructor
                .navigate(PathSegment::Ident(ident(name)))
                .unwrap();
            constructor
                .bind_primitive(PrimitiveValue::Integer(value.into()))
                .unwrap();
            constructor.end_scope(scope).unwrap();
            constructor.end_binding_value().unwrap();
        }

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 2);
        assert_eq!(root.bindings[0].path[0].key, SourceKey::Ident(ident("a")));
        assert_eq!(root.bindings[1].path[0].key, SourceKey::Ident(ident("b")));
    }

    #[test]
    fn test_nested_blocks() {
        let mut constructor = SourceConstructor::new();

        // Build: outer { inner { value = 1 } }
        constructor.begin_binding();
        let scope1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("outer")))
            .unwrap();
        constructor.begin_eure_block();

        constructor.begin_binding();
        let scope2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("inner")))
            .unwrap();
        constructor.begin_eure_block();

        constructor.begin_binding();
        let scope3 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("value")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(1.into()))
            .unwrap();
        constructor.end_scope(scope3).unwrap();
        constructor.end_binding_value().unwrap();

        constructor.end_eure_block().unwrap();
        constructor.end_scope(scope2).unwrap();
        constructor.end_binding_block().unwrap();

        constructor.end_eure_block().unwrap();
        constructor.end_scope(scope1).unwrap();
        constructor.end_binding_block().unwrap();

        let source_doc = constructor.finish();

        // Verify structure: root -> outer -> inner -> value
        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 1);

        if let BindSource::Block(outer_id) = &root.bindings[0].bind {
            let outer = source_doc.source(*outer_id);
            assert_eq!(outer.bindings.len(), 1);

            if let BindSource::Block(inner_id) = &outer.bindings[0].bind {
                let inner = source_doc.source(*inner_id);
                assert_eq!(inner.bindings.len(), 1);
                assert!(matches!(inner.bindings[0].bind, BindSource::Value(_)));
            } else {
                panic!("Expected inner block");
            }
        } else {
            panic!("Expected outer block");
        }
    }

    // =========================================================================
    // Trivia (comments and blank lines) tests
    // =========================================================================

    #[test]
    fn test_trivia_before_binding() {
        let mut constructor = SourceConstructor::new();

        // Add comment and blank line before first binding
        constructor.comment(Comment::Line("This is a comment".to_string()));
        constructor.blank_line();

        // Build: name = "Alice"
        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("name")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("Alice")))
            .unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_value().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 1);

        // Check trivia attached to the binding
        let binding = &root.bindings[0];
        assert_eq!(binding.trivia_before.len(), 2);
        assert!(matches!(
            &binding.trivia_before[0],
            Trivia::Comment(Comment::Line(s)) if s == "This is a comment"
        ));
        assert!(matches!(&binding.trivia_before[1], Trivia::BlankLine));
    }

    #[test]
    fn test_trivia_before_section() {
        let mut constructor = SourceConstructor::new();

        // Add blank line before section
        constructor.blank_line();

        // Build: @ server
        constructor.begin_section();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("server")))
            .unwrap();
        constructor.begin_section_items();

        // Binding inside section
        constructor.begin_binding();
        let inner_scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("host")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("localhost")))
            .unwrap();
        constructor.end_scope(inner_scope).unwrap();
        constructor.end_binding_value().unwrap();

        constructor.end_section_items().unwrap();
        constructor.end_scope(scope).unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.sections.len(), 1);

        // Check trivia attached to the section
        let section = &root.sections[0];
        assert_eq!(section.trivia_before.len(), 1);
        assert!(matches!(&section.trivia_before[0], Trivia::BlankLine));
    }

    #[test]
    fn test_trailing_trivia() {
        let mut constructor = SourceConstructor::new();

        // Build: name = "Alice"
        constructor.begin_binding();
        let scope = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("name")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Text(Text::plaintext("Alice")))
            .unwrap();
        constructor.end_scope(scope).unwrap();
        constructor.end_binding_value().unwrap();

        // Add trailing comment/blank line after all items
        constructor.blank_line();
        constructor.comment(Comment::Line("end of file".to_string()));

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.trailing_trivia.len(), 2);
        assert!(matches!(&root.trailing_trivia[0], Trivia::BlankLine));
        assert!(matches!(
            &root.trailing_trivia[1],
            Trivia::Comment(Comment::Line(s)) if s == "end of file"
        ));
    }

    #[test]
    fn test_trivia_between_bindings() {
        let mut constructor = SourceConstructor::new();

        // Build: a = 1
        constructor.begin_binding();
        let scope1 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("a")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(1.into()))
            .unwrap();
        constructor.end_scope(scope1).unwrap();
        constructor.end_binding_value().unwrap();

        // Add blank line and comment between bindings
        constructor.blank_line();
        constructor.comment(Comment::Line("Second binding".to_string()));

        // Build: b = 2
        constructor.begin_binding();
        let scope2 = constructor.begin_scope();
        constructor
            .navigate(PathSegment::Ident(ident("b")))
            .unwrap();
        constructor
            .bind_primitive(PrimitiveValue::Integer(2.into()))
            .unwrap();
        constructor.end_scope(scope2).unwrap();
        constructor.end_binding_value().unwrap();

        let source_doc = constructor.finish();

        let root = source_doc.root_source();
        assert_eq!(root.bindings.len(), 2);

        // First binding should have no trivia
        assert!(root.bindings[0].trivia_before.is_empty());

        // Second binding should have the trivia
        assert_eq!(root.bindings[1].trivia_before.len(), 2);
        assert!(matches!(
            &root.bindings[1].trivia_before[0],
            Trivia::BlankLine
        ));
        assert!(matches!(
            &root.bindings[1].trivia_before[1],
            Trivia::Comment(Comment::Line(s)) if s == "Second binding"
        ));
    }
}
