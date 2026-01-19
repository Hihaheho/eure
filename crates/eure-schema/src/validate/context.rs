//! Validation context and output types
//!
//! `ValidationContext` manages state during validation:
//! - Schema reference
//! - Current path for error reporting
//! - Accumulated errors and warnings
//! - Hole tracking for completeness check

use std::cell::RefCell;

use eure_document::document::EureDocument;
use eure_document::identifier::Identifier;
use eure_document::parse::UnionTagMode;
use eure_document::path::{EurePath, PathSegment};
use eure_document::value::ObjectKey;

use crate::{SchemaDocument, SchemaNodeContent, SchemaNodeId};

use super::error::{ValidationError, ValidationWarning};

// =============================================================================
// ValidationOutput (final result for public API)
// =============================================================================

/// Final validation output returned to callers.
#[derive(Debug, Clone, Default)]
pub struct ValidationOutput {
    /// No type errors (holes are allowed)
    pub is_valid: bool,
    /// No type errors AND no holes
    pub is_complete: bool,
    /// Type errors encountered during validation
    pub errors: Vec<ValidationError>,
    /// Warnings (e.g., unknown extensions)
    pub warnings: Vec<ValidationWarning>,
}

// =============================================================================
// ValidationState (internal mutable state)
// =============================================================================

/// Internal mutable state during validation.
///
/// Wrapped in RefCell to allow interior mutability through shared references.
#[derive(Debug)]
pub struct ValidationState {
    /// Current path in the document (for error reporting)
    pub path: EurePath,
    /// Whether any holes have been encountered
    pub has_holes: bool,
    /// Accumulated validation errors
    pub errors: Vec<ValidationError>,
    /// Accumulated warnings
    pub warnings: Vec<ValidationWarning>,
    /// Temporary storage for variant errors during union validation.
    /// Each entry is (variant_name, variant_schema_id, errors_from_that_variant).
    pub(crate) variant_errors: Vec<(String, SchemaNodeId, Vec<ValidationError>)>,
}

impl Default for ValidationState {
    fn default() -> Self {
        Self {
            path: EurePath::root(),
            has_holes: false,
            errors: Vec::new(),
            warnings: Vec::new(),
            variant_errors: Vec::new(),
        }
    }
}

impl ValidationState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an error (validation continues).
    pub fn record_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Record a warning.
    pub fn record_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Mark that a hole was encountered.
    pub fn mark_has_holes(&mut self) {
        self.has_holes = true;
    }

    /// Check if any errors have been recorded.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    // -------------------------------------------------------------------------
    // Path management
    // -------------------------------------------------------------------------

    /// Push an identifier to the path.
    pub fn push_path_ident(&mut self, ident: Identifier) {
        self.path.0.push(PathSegment::Ident(ident));
    }

    /// Push a key to the path.
    pub fn push_path_key(&mut self, key: ObjectKey) {
        self.path.0.push(PathSegment::Value(key));
    }

    /// Push an array index to the path.
    pub fn push_path_index(&mut self, index: usize) {
        self.path.0.push(PathSegment::ArrayIndex(Some(index)));
    }

    /// Push a tuple index to the path.
    pub fn push_path_tuple_index(&mut self, index: u8) {
        self.path.0.push(PathSegment::TupleIndex(index));
    }

    /// Push an extension to the path.
    pub fn push_path_extension(&mut self, ident: Identifier) {
        self.path.0.push(PathSegment::Extension(ident));
    }

    /// Pop the last segment from the path.
    pub fn pop_path(&mut self) {
        self.path.0.pop();
    }

    /// Clone for fork (trial validation).
    pub fn fork(&self) -> Self {
        Self {
            path: self.path.clone(),
            has_holes: self.has_holes,
            errors: Vec::new(),
            warnings: Vec::new(),
            variant_errors: Vec::new(), // Don't inherit variant errors
        }
    }

    /// Merge results from a forked state.
    pub fn merge(&mut self, other: Self) {
        self.has_holes |= other.has_holes;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    /// Consume and produce final output.
    pub fn finish(self) -> ValidationOutput {
        ValidationOutput {
            is_valid: self.errors.is_empty(),
            is_complete: self.errors.is_empty() && !self.has_holes,
            errors: self.errors,
            warnings: self.warnings,
        }
    }

    // -------------------------------------------------------------------------
    // Variant error collection (for union validation)
    // -------------------------------------------------------------------------

    /// Record variant attempt errors (for union validation).
    pub fn record_variant_errors(
        &mut self,
        variant_name: String,
        schema_id: SchemaNodeId,
        errors: Vec<ValidationError>,
    ) {
        self.variant_errors.push((variant_name, schema_id, errors));
    }

    /// Take collected variant errors (for union error construction).
    pub fn take_variant_errors(&mut self) -> Vec<(String, SchemaNodeId, Vec<ValidationError>)> {
        std::mem::take(&mut self.variant_errors)
    }

    /// Clear variant errors without taking (for successful variant match).
    pub fn clear_variant_errors(&mut self) {
        self.variant_errors.clear();
    }
}

// =============================================================================
// ValidationContext (shared immutable + RefCell for state)
// =============================================================================

/// Validation context combining schema reference and mutable state.
///
/// Uses interior mutability (RefCell) to allow validators implementing
/// `DocumentParser` to record errors without requiring `&mut self`.
pub struct ValidationContext<'a> {
    /// Reference to the schema being validated against
    pub schema: &'a SchemaDocument,
    /// Reference to the document being validated
    pub document: &'a EureDocument,
    /// Mutable state (errors, warnings, path, holes)
    pub state: RefCell<ValidationState>,
    /// Union tag mode for this validation context.
    pub union_tag_mode: UnionTagMode,
}

impl<'a> ValidationContext<'a> {
    /// Create a new validation context with default (Eure) mode.
    pub fn new(document: &'a EureDocument, schema: &'a SchemaDocument) -> Self {
        Self::with_mode(document, schema, UnionTagMode::default())
    }

    /// Create a new validation context with specified union tag mode.
    pub fn with_mode(
        document: &'a EureDocument,
        schema: &'a SchemaDocument,
        union_tag_mode: UnionTagMode,
    ) -> Self {
        Self {
            schema,
            document,
            state: RefCell::new(ValidationState::new()),
            union_tag_mode,
        }
    }

    /// Create a context with existing state (for fork/merge).
    pub fn with_state(
        document: &'a EureDocument,
        schema: &'a SchemaDocument,
        state: ValidationState,
    ) -> Self {
        Self::with_state_and_mode(document, schema, state, UnionTagMode::default())
    }

    /// Create a context with existing state and mode (for fork/merge).
    pub fn with_state_and_mode(
        document: &'a EureDocument,
        schema: &'a SchemaDocument,
        state: ValidationState,
        union_tag_mode: UnionTagMode,
    ) -> Self {
        Self {
            schema,
            document,
            state: RefCell::new(state),
            union_tag_mode,
        }
    }

    /// Record an error.
    pub fn record_error(&self, error: ValidationError) {
        self.state.borrow_mut().record_error(error);
    }

    /// Record a warning.
    pub fn record_warning(&self, warning: ValidationWarning) {
        self.state.borrow_mut().record_warning(warning);
    }

    /// Mark that a hole was encountered.
    pub fn mark_has_holes(&self) {
        self.state.borrow_mut().mark_has_holes();
    }

    /// Check if any errors have been recorded.
    pub fn has_errors(&self) -> bool {
        self.state.borrow().has_errors()
    }

    /// Get the current error count.
    pub fn error_count(&self) -> usize {
        self.state.borrow().error_count()
    }

    /// Get a clone of the current path.
    pub fn path(&self) -> EurePath {
        self.state.borrow().path.clone()
    }

    /// Push an identifier to the path.
    pub fn push_path_ident(&self, ident: Identifier) {
        self.state.borrow_mut().push_path_ident(ident);
    }

    /// Push a key to the path.
    pub fn push_path_key(&self, key: ObjectKey) {
        self.state.borrow_mut().push_path_key(key);
    }

    /// Push an array index to the path.
    pub fn push_path_index(&self, index: usize) {
        self.state.borrow_mut().push_path_index(index);
    }

    /// Push a tuple index to the path.
    pub fn push_path_tuple_index(&self, index: u8) {
        self.state.borrow_mut().push_path_tuple_index(index);
    }

    /// Push an extension to the path.
    pub fn push_path_extension(&self, ident: Identifier) {
        self.state.borrow_mut().push_path_extension(ident);
    }

    /// Pop the last segment from the path.
    pub fn pop_path(&self) {
        self.state.borrow_mut().pop_path();
    }

    /// Fork for trial validation (returns forked state).
    pub fn fork_state(&self) -> ValidationState {
        self.state.borrow().fork()
    }

    /// Merge forked state back.
    pub fn merge_state(&self, other: ValidationState) {
        self.state.borrow_mut().merge(other);
    }

    /// Resolve type references to get the actual schema content.
    pub fn resolve_schema_content(&self, schema_id: SchemaNodeId) -> &SchemaNodeContent {
        let mut current_id = schema_id;
        // Limit iterations to prevent infinite loops with circular refs
        for _ in 0..100 {
            let content = &self.schema.node(current_id).content;
            match content {
                SchemaNodeContent::Reference(type_ref) => {
                    if type_ref.namespace.is_some() {
                        return content; // Cross-schema refs not resolved
                    }
                    if let Some(&resolved_id) = self.schema.types.get(&type_ref.name) {
                        current_id = resolved_id;
                    } else {
                        return content; // Unresolved reference
                    }
                }
                _ => return content,
            }
        }
        &self.schema.node(current_id).content
    }

    /// Create a ParseContext for the given node with this context's union tag mode.
    pub fn parse_context(
        &self,
        node_id: eure_document::document::NodeId,
    ) -> eure_document::parse::ParseContext<'a> {
        eure_document::parse::ParseContext::with_union_tag_mode(
            self.document,
            node_id,
            self.union_tag_mode,
        )
    }

    // -------------------------------------------------------------------------
    // Variant error collection (for union validation)
    // -------------------------------------------------------------------------

    /// Record variant attempt errors during union validation.
    pub fn record_variant_errors(
        &self,
        variant_name: String,
        schema_id: SchemaNodeId,
        errors: Vec<ValidationError>,
    ) {
        self.state
            .borrow_mut()
            .record_variant_errors(variant_name, schema_id, errors);
    }

    /// Take collected variant errors.
    pub fn take_variant_errors(&self) -> Vec<(String, SchemaNodeId, Vec<ValidationError>)> {
        self.state.borrow_mut().take_variant_errors()
    }

    /// Clear variant errors.
    pub fn clear_variant_errors(&self) {
        self.state.borrow_mut().clear_variant_errors();
    }

    /// Consume context and produce final output.
    pub fn finish(self) -> ValidationOutput {
        self.state.into_inner().finish()
    }
}
