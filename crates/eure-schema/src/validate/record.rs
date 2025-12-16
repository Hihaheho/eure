//! Record type validator
//!
//! Validates records against RecordSchema constraints using parse_record() API.

use eure_document::identifier::Identifier;
use eure_document::parse::{DocumentParser, ParseContext};
use eure_document::value::ObjectKey;

use crate::{RecordSchema, SchemaNodeId, UnknownFieldsPolicy};

use super::SchemaValidator;
use super::context::ValidationContext;
use super::error::{ValidationError, ValidationWarning, ValidatorError};
use super::primitive::actual_type_from_error;

// =============================================================================
// RecordValidator
// =============================================================================

/// Validates record values against RecordSchema constraints.
///
/// Uses `parse_ctx.parse_record()` to get a `RecordParser` and
/// validates each field using child `SchemaValidator`s.
pub struct RecordValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s RecordSchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for RecordValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Use parse_record() API
        let mut rec = match parse_ctx.parse_record() {
            Ok(r) => r,
            Err(e) => {
                self.ctx.record_error(ValidationError::TypeMismatch {
                    expected: "record".to_string(),
                    actual: actual_type_from_error(&e.kind),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
                return Ok(());
            }
        };

        // Validate each field in schema using field_optional() and parse_with()
        for (field_name, field_schema) in &self.schema.properties {
            if let Some(field_ctx) = rec.field_optional(field_name) {
                // Check deprecated
                let field_schema_node = self.ctx.schema.node(field_schema.schema);
                if field_schema_node.metadata.deprecated {
                    self.ctx.record_warning(ValidationWarning::DeprecatedField {
                        field: field_name.to_string(),
                        path: self.ctx.path(),
                    });
                }

                // Push path for error reporting
                if let Ok(ident) = field_name.parse::<Identifier>() {
                    self.ctx.push_path_ident(ident);
                } else {
                    self.ctx
                        .push_path_key(ObjectKey::String(field_name.to_string()));
                }

                // Validate field value using parse_with() and SchemaValidator
                let child_validator = SchemaValidator {
                    ctx: self.ctx,
                    schema_node_id: field_schema.schema,
                };
                // Note: errors are accumulated in ctx, not propagated
                let _ = field_ctx.parse_with(child_validator);

                self.ctx.pop_path();
            } else if !field_schema.optional {
                self.ctx
                    .record_error(ValidationError::MissingRequiredField {
                        field: field_name.to_string(),
                        path: self.ctx.path(),
                        node_id,
                        schema_node_id: self.schema_node_id,
                    });
            }
        }

        // Handle unknown fields using unknown_fields() iterator
        for (field_name, field_ctx) in rec.unknown_fields() {
            match &self.schema.unknown_fields {
                UnknownFieldsPolicy::Deny => {
                    self.ctx.record_error(ValidationError::UnknownField {
                        field: field_name.to_string(),
                        path: self.ctx.path(),
                        node_id,
                        schema_node_id: self.schema_node_id,
                    });
                }
                UnknownFieldsPolicy::Allow => {}
                UnknownFieldsPolicy::Schema(s) => {
                    if let Ok(ident) = field_name.parse::<Identifier>() {
                        self.ctx.push_path_ident(ident);
                    } else {
                        self.ctx
                            .push_path_key(ObjectKey::String(field_name.to_string()));
                    }

                    let child_validator = SchemaValidator {
                        ctx: self.ctx,
                        schema_node_id: *s,
                    };
                    let _ = field_ctx.parse_with(child_validator);

                    self.ctx.pop_path();
                }
            }
        }

        Ok(())
    }
}
