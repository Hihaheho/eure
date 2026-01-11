//! Interpreter sink trait for document construction.
//!
//! This module defines the abstract interface for document construction,
//! matching the semantics of spec Section 8 (Document Interpretation).
//!
//! # Architecture
//!
//! The [`InterpreterSink`] trait provides:
//! - Core actions from spec Section 8 (scope, navigation, binding)
//! - Source layout markers with default no-op implementations
//!
//! # Implementations
//!
//! - [`DocumentConstructor`](super::constructor::DocumentConstructor): Low-level, uses default no-ops
//! - [`SourceConstructor`](super::source_constructor::SourceConstructor): Overrides to track layout

use crate::document::NodeId;
use crate::path::PathSegment;
use crate::prelude_internal::*;
use crate::source::Comment;

/// Core trait for document construction (spec section 8 semantics).
///
/// The 5 primitive actions from the spec:
/// - `begin_scope()` / `end_scope()` -> scope management
/// - `navigate(segment)` -> tree navigation
/// - `assert_unbound()` -> `require_hole()`
/// - `bind(value)` -> `bind_*()` methods
///
/// # Example
///
/// ```ignore
/// // Equivalent to: a.b = true
/// let scope = sink.begin_scope();
/// sink.navigate(PathSegment::Ident("a".parse()?))?;
/// sink.navigate(PathSegment::Ident("b".parse()?))?;
/// sink.require_hole()?;
/// sink.bind_primitive(PrimitiveValue::Bool(true))?;
/// sink.end_scope(scope)?;
/// ```
pub trait InterpreterSink {
    /// The error type for operations.
    type Error;
    /// The scope handle type returned by `begin_scope`.
    type Scope;

    // === Scope Management ===

    /// Begin a new scope. Returns a handle that must be passed to `end_scope`.
    /// Scopes must be ended in LIFO order (most recent first).
    fn begin_scope(&mut self) -> Self::Scope;

    /// End a scope, restoring the sink to the state when `begin_scope` was called.
    fn end_scope(&mut self, scope: Self::Scope) -> Result<(), Self::Error>;

    // === Navigation ===

    /// Navigate to a child node by path segment.
    /// Creates the node if it doesn't exist.
    fn navigate(&mut self, segment: PathSegment) -> Result<NodeId, Self::Error>;

    // === Validation ===

    /// Assert that the current node is unbound (a hole).
    /// Use this before binding a value to ensure the node hasn't already been assigned.
    fn require_hole(&self) -> Result<(), Self::Error>;

    // === Binding ===

    /// Bind a primitive value to the current node.
    fn bind_primitive(&mut self, value: PrimitiveValue) -> Result<(), Self::Error>;

    /// Bind a hole (with optional label) to the current node.
    fn bind_hole(&mut self, label: Option<Identifier>) -> Result<(), Self::Error>;

    /// Bind an empty map to the current node.
    fn bind_empty_map(&mut self) -> Result<(), Self::Error>;

    /// Bind an empty array to the current node.
    fn bind_empty_array(&mut self) -> Result<(), Self::Error>;

    /// Bind an empty tuple to the current node.
    fn bind_empty_tuple(&mut self) -> Result<(), Self::Error>;

    /// Bind a value using `Into<PrimitiveValue>`.
    /// Convenience method for use with the `eure!` macro.
    fn bind_from(&mut self, value: impl Into<PrimitiveValue>) -> Result<(), Self::Error> {
        self.bind_primitive(value.into())
    }

    // === Access ===

    /// Get the current node ID.
    fn current_node_id(&self) -> NodeId;

    /// Get the current path from root.
    fn current_path(&self) -> &[PathSegment];

    /// Get a reference to the document being built.
    fn document(&self) -> &EureDocument;

    /// Get a mutable reference to the document being built.
    fn document_mut(&mut self) -> &mut EureDocument;

    // =========================================================================
    // Source Layout Markers (default no-op implementations)
    //
    // These methods track source structure for round-trip formatting.
    // Override in SourceConstructor; DocumentConstructor uses the default no-ops.
    //
    // The 6 source patterns:
    // #1: path = value           -> begin_binding, end_binding_value
    // #2: path { eure }          -> begin_binding, begin_eure_block, ..., end_eure_block, end_binding_block
    // #3: path { = value eure }  -> begin_binding, begin_eure_block, set_block_value, ..., end_eure_block, end_binding_block
    // #4: @ path (items)         -> begin_section, begin_section_items, ..., end_section_items
    // #5: @ path { eure }        -> begin_section, begin_eure_block, ..., end_eure_block, end_section_block
    // #6: @ path { = value eure }-> begin_section, begin_eure_block, set_block_value, ..., end_eure_block, end_section_block
    // =========================================================================

    // === EureSource block management ===

    /// Enter a new EureSource block (for `{ eure }` patterns).
    /// Pushes a new EureSource onto the builder stack.
    /// Default: no-op.
    fn begin_eure_block(&mut self) {}

    /// Set the value binding for current EureSource (for `{ = value ... }` patterns).
    /// Called after `bind_*` to record the value node.
    /// Returns error if called without a preceding bind operation.
    /// Default: no-op (returns Ok).
    fn set_block_value(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// End current EureSource block.
    /// Pops from the builder stack.
    /// Returns error if the builder stack is in an invalid state.
    /// Default: no-op (returns Ok).
    fn end_eure_block(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    // === Binding patterns (#1-#3) ===

    /// Start a binding statement (`path ...`).
    /// Default: no-op.
    fn begin_binding(&mut self) {}

    /// End binding #1: `path = value`.
    /// Adds BindingSource with BindSource::Value to current EureSource.
    /// Returns error if called without a preceding bind operation.
    /// Default: no-op (returns Ok).
    fn end_binding_value(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// End binding #2/#3: `path { eure }`.
    /// Adds BindingSource with BindSource::Block to current EureSource.
    /// The block's EureSource was built between begin_eure_block/end_eure_block.
    /// Returns error if called without a preceding end_eure_block.
    /// Default: no-op (returns Ok).
    fn end_binding_block(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    // === Section patterns (#4-#6) ===

    /// Start a section header (`@ path ...`).
    /// Default: no-op.
    fn begin_section(&mut self) {}

    /// Begin section #4: `@ section` (items follow).
    /// Begins collecting items into SectionBody::Items.
    /// Default: no-op.
    fn begin_section_items(&mut self) {}

    /// End section #4: finalize section with items body.
    /// Adds SectionSource with SectionBody::Items to current EureSource.
    /// Returns error if the builder stack is in an invalid state.
    /// Default: no-op (returns Ok).
    fn end_section_items(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// End section #5/#6: `@ section { eure }`.
    /// Adds SectionSource with SectionBody::Block to current EureSource.
    /// The block's EureSource was built between begin_eure_block/end_eure_block.
    /// Returns error if called without a preceding end_eure_block.
    /// Default: no-op (returns Ok).
    fn end_section_block(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    // === Other ===

    /// Add a comment to the layout.
    /// Default: no-op.
    fn comment(&mut self, _comment: Comment) {}

    /// Add a blank line to the layout.
    /// Default: no-op.
    fn blank_line(&mut self) {}
}
