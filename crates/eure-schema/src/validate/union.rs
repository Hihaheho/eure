//! Union type validator
//!
//! Validates union values using parse_union() API pattern.

use std::collections::HashSet;

use eure_document::parse::{DocumentParser, ParseContext};

use crate::{SchemaNodeId, UnionSchema};

use super::SchemaValidator;
use super::context::ValidationContext;
use super::error::{ValidationError, ValidatorError};

// =============================================================================
// UnionValidator
// =============================================================================

/// Validates union values against UnionSchema.
///
/// Uses similar pattern to `UnionParser` but for validation:
/// - `$variant` extension for explicit variant tagging
/// - VariantRepr patterns (External, Internal, Adjacent)
/// - Untagged matching with priority resolution
pub struct UnionValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s UnionSchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for UnionValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        // Use parse_union() API to leverage the same variant resolution logic
        let union_parser =
            match parse_ctx.parse_union::<(), ValidatorError>(self.schema.repr.clone()) {
                Ok(p) => p,
                Err(e) => {
                    // Error extracting variant (e.g., invalid $variant type, conflicting tags)
                    if let Some(parse_error) = e.as_parse_error() {
                        // Wrap ParseError with schema context
                        self.ctx.record_error(ValidationError::ParseError {
                            path: self.ctx.path(),
                            node_id: parse_ctx.node_id(),
                            schema_node_id: self.schema_node_id,
                            error: parse_error.clone(),
                        });
                    } else {
                        // Fallback for other ValidatorErrors
                        self.ctx.record_error(ValidationError::InvalidVariantTag {
                            tag: format!("{e}"),
                            path: self.ctx.path(),
                            node_id: parse_ctx.node_id(),
                            schema_node_id: self.schema_node_id,
                        });
                    }
                    return Ok(());
                }
            };

        // Build validator closures for each variant
        let priority_names: HashSet<_> = self.schema.priority.iter().flatten().cloned().collect();

        // Create a validator that tries variants using UnionParser's pattern
        let mut builder = union_parser;

        // Determine if variant is tagged (determined by repr or $variant)
        // In tagged mode, propagate nested errors; in untagged mode, don't
        let is_tagged = !matches!(
            self.schema.repr,
            eure_document::data_model::VariantRepr::Untagged
        );

        // Register priority variants first
        if let Some(priority) = &self.schema.priority {
            for name in priority {
                if let Some(&variant_schema_id) = self.schema.variants.get(name) {
                    let ctx = self.ctx;
                    let schema_node_id = variant_schema_id;
                    let variant_name = name.clone();
                    builder = builder.variant(name, move |parse_ctx: &ParseContext<'_>| {
                        validate_variant(ctx, parse_ctx, schema_node_id, is_tagged, &variant_name)
                    });
                }
            }
        }

        // Register non-priority variants
        for (name, &variant_schema_id) in &self.schema.variants {
            if priority_names.contains(name) {
                continue;
            }
            let ctx = self.ctx;
            let schema_node_id = variant_schema_id;
            let variant_name = name.clone();
            builder = builder.other(name, move |parse_ctx: &ParseContext<'_>| {
                validate_variant(ctx, parse_ctx, schema_node_id, is_tagged, &variant_name)
            });
        }

        // Execute union parsing/validation
        match builder.parse() {
            Ok(()) => Ok(()),
            Err(e) => {
                // Skip adding error if inner errors were already propagated
                if matches!(e, ValidatorError::InnerErrorsPropagated) {
                    return Ok(());
                }

                // Convert ValidatorError to ValidationError with schema context
                if let Some(parse_error) = e.as_parse_error() {
                    // Wrap ParseError with schema context
                    self.ctx.record_error(ValidationError::ParseError {
                        path: self.ctx.path(),
                        node_id: parse_ctx.node_id(),
                        schema_node_id: self.schema_node_id,
                        error: parse_error.clone(),
                    });
                } else {
                    // Fallback: generic no-match error
                    self.ctx.record_error(ValidationError::NoVariantMatched {
                        path: self.ctx.path(),
                        variant_errors: Vec::new(),
                        node_id: parse_ctx.node_id(),
                        schema_node_id: self.schema_node_id,
                    });
                }
                Ok(())
            }
        }
    }
}

/// Validate a single variant.
///
/// Returns Ok(()) if validation succeeds (no errors accumulated).
/// Returns Err if validation fails (so UnionParser can try other variants).
///
/// `propagate_errors`: When true (tagged mode), propagate nested errors to parent context
/// so they are reported with correct node positions. When false (untagged mode), discard
/// errors from failed attempts to avoid confusing multiple error messages.
fn validate_variant<'doc>(
    ctx: &ValidationContext<'doc>,
    parse_ctx: &ParseContext<'doc>,
    schema_node_id: SchemaNodeId,
    propagate_errors: bool,
    variant_name: &str,
) -> Result<(), ValidatorError> {
    // Fork state for trial validation
    let forked_state = ctx.fork_state();
    let trial_ctx = ValidationContext::with_state_and_mode(
        ctx.document,
        ctx.schema,
        forked_state,
        ctx.union_tag_mode,
    );

    let child_validator = SchemaValidator {
        ctx: &trial_ctx,
        schema_node_id,
    };

    let result = parse_ctx.parse_with(child_validator);

    if result.is_ok() && !trial_ctx.has_errors() {
        // Success - merge any warnings/holes from trial
        ctx.merge_state(trial_ctx.state.into_inner());
        Ok(())
    } else {
        // Validation failed
        // In tagged mode, propagate nested errors so they are reported with correct positions
        // In untagged mode, discard errors to avoid confusing output from failed attempts
        if propagate_errors && trial_ctx.has_errors() {
            ctx.merge_state(trial_ctx.state.into_inner());
            // Signal that inner errors were propagated - no additional error needed
            Err(ValidatorError::InnerErrorsPropagated)
        } else {
            Err(ValidatorError::InvalidVariantTag {
                tag: variant_name.to_string(),
                reason: "type mismatch".to_string(),
            })
        }
    }
}
