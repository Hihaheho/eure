//! Reference type validator
//!
//! Resolves type references and delegates to SchemaValidator.

use eure_document::parse::{DocumentParser, ParseContext};

use crate::{SchemaNodeId, TypeReference};

use super::SchemaValidator;
use super::context::ValidationContext;
use super::error::{ValidationError, ValidatorError};

// =============================================================================
// ReferenceValidator
// =============================================================================

/// Validates values by resolving type references.
///
/// Looks up the referenced type in schema.types and delegates
/// validation to SchemaValidator with the resolved schema.
pub struct ReferenceValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub type_ref: &'s TypeReference,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for ReferenceValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        if let Some(resolved_id) = self.ctx.schema.resolve_reference(self.type_ref) {
            // Delegate to SchemaValidator with resolved type
            let child_validator = SchemaValidator {
                ctx: self.ctx,
                schema_node_id: resolved_id,
            };
            parse_ctx.parse_with(child_validator)
        } else {
            self.ctx
                .record_error(ValidationError::UndefinedTypeReference {
                    name: self.ctx.schema.display_reference(self.type_ref),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
            Ok(())
        }
    }
}
