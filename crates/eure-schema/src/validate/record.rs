//! Record type validator
//!
//! Validates records against RecordSchema constraints using parse_record() API.

use eure_document::identifier::Identifier;
use eure_document::parse::{DocumentParser, ParseContext};
use eure_document::value::ObjectKey;

use crate::{RecordSchema, SchemaNodeContent, SchemaNodeId, UnionSchema, UnknownFieldsPolicy};

use super::SchemaValidator;
use super::context::ValidationContext;
use super::error::{ValidationError, ValidationWarning, ValidatorError, select_best_variant_match};
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
        let rec = match parse_ctx.parse_record() {
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

        // Process flatten targets
        // Each flatten target shares the field access tracking via rec.flatten()
        for &flatten_schema_id in &self.schema.flatten {
            let flatten_ctx = rec.flatten();
            self.validate_flatten_target(&flatten_ctx, flatten_schema_id, node_id)?;
        }

        // Handle unknown fields using unknown_fields() iterator
        // This happens after all flatten targets have been processed
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

impl<'a, 'doc, 's> RecordValidator<'a, 'doc, 's> {
    /// Validate a flatten target schema against the current record.
    ///
    /// The flatten_ctx shares field access tracking with the parent record,
    /// so fields consumed by the flatten target won't appear as unknown.
    fn validate_flatten_target(
        &self,
        flatten_ctx: &ParseContext<'doc>,
        flatten_schema_id: SchemaNodeId,
        parent_node_id: eure_document::document::NodeId,
    ) -> Result<(), ValidatorError> {
        let flatten_node = self.ctx.schema.node(flatten_schema_id);

        match &flatten_node.content {
            SchemaNodeContent::Record(_) => {
                self.validate_flattened_record(flatten_ctx, flatten_schema_id)?;
            }
            SchemaNodeContent::Union(union_schema) => {
                self.validate_flattened_union(
                    flatten_ctx,
                    union_schema,
                    flatten_schema_id,
                    parent_node_id,
                )?;
            }
            SchemaNodeContent::Reference(type_ref) => {
                // Resolve the reference and recurse
                if let Some(resolved_id) = self.ctx.schema.get_type(&type_ref.name) {
                    self.validate_flatten_target(flatten_ctx, resolved_id, parent_node_id)?;
                } else {
                    // Record error for undefined type reference
                    self.ctx
                        .record_error(ValidationError::UndefinedTypeReference {
                            name: type_ref.name.to_string(),
                            path: self.ctx.path(),
                            node_id: parent_node_id,
                            schema_node_id: flatten_schema_id,
                        });
                }
            }
            _ => {
                // Only Record, Union, and Reference can be flattened
                self.ctx
                    .record_error(ValidationError::InvalidFlattenTarget {
                        actual_kind: flatten_node.content.kind(),
                        path: self.ctx.path(),
                        node_id: parent_node_id,
                        schema_node_id: flatten_schema_id,
                    });
            }
        }

        Ok(())
    }

    /// Validate a flattened record's properties against the current record.
    ///
    /// Simply delegates to SchemaValidator - the `unknown_fields()` iterator
    /// returns empty for flattened contexts, so the parent handles unknown field checks.
    fn validate_flattened_record(
        &self,
        flatten_ctx: &ParseContext<'doc>,
        schema_node_id: SchemaNodeId,
    ) -> Result<(), ValidatorError> {
        let validator = SchemaValidator {
            ctx: self.ctx,
            schema_node_id,
        };
        let _ = flatten_ctx.parse_with(validator);
        Ok(())
    }

    /// Validate a flattened union - exactly one variant must match.
    fn validate_flattened_union(
        &self,
        flatten_ctx: &ParseContext<'doc>,
        union_schema: &UnionSchema,
        schema_node_id: SchemaNodeId,
        parent_node_id: eure_document::document::NodeId,
    ) -> Result<(), ValidatorError> {
        let (matched, errors): (Vec<_>, Vec<_>) = union_schema
            .variants
            .iter()
            .map(|(name, &id)| (name.clone(), self.try_variant(flatten_ctx, id)))
            .partition(|(_, result)| result.is_ok());

        let matched: Vec<_> = matched.into_iter().map(|(name, _)| name).collect();
        let errors: Vec<_> = errors
            .into_iter()
            .filter_map(|(name, r)| r.err().map(|e| (name, e)))
            .collect();

        match matched.len() {
            0 => self.ctx.record_error(ValidationError::NoVariantMatched {
                path: self.ctx.path(),
                best_match: select_best_variant_match(errors).map(Box::new),
                node_id: parent_node_id,
                schema_node_id,
            }),
            1 => {}
            _ => self.ctx.record_error(ValidationError::AmbiguousUnion {
                variants: matched,
                path: self.ctx.path(),
                node_id: parent_node_id,
                schema_node_id,
            }),
        }
        Ok(())
    }

    /// Try validating a variant, returning Ok(()) if it matches, Err(errors) if not.
    fn try_variant(
        &self,
        flatten_ctx: &ParseContext<'doc>,
        schema_node_id: SchemaNodeId,
    ) -> Result<(), Vec<ValidationError>> {
        let forked = self.ctx.fork_state();
        let trial = ValidationContext::with_state_and_mode(
            self.ctx.document,
            self.ctx.schema,
            forked,
            self.ctx.union_tag_mode,
        );
        let _ = flatten_ctx.parse_with(SchemaValidator {
            ctx: &trial,
            schema_node_id,
        });
        if trial.has_errors() {
            Err(trial.state.into_inner().errors)
        } else {
            self.ctx.merge_state(trial.state.into_inner());
            Ok(())
        }
    }
}
