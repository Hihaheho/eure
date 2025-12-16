//! Union type validator
//!
//! Validates union values using parse_union() API pattern.

use std::collections::HashSet;

use eure_document::parse::union::{VARIANT, extract_repr_variant};
use eure_document::parse::{DocumentParser, ParseContext};

use crate::{SchemaNodeId, UnionSchema};

use super::SchemaValidator;
use super::context::ValidationContext;
use super::error::{BestVariantMatch, ValidationError, ValidatorError};

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

        // Check if this value has explicit variant tagging ($variant extension or repr pattern)
        // This is used to enforce deny_untagged: variants in deny_untagged must have explicit tags
        let has_explicit_tag = {
            // Check for $variant extension
            let has_variant_ext = parse_ctx.node().extensions.contains_key(&VARIANT);
            // Check if repr pattern matches (for non-Untagged reprs)
            let has_repr_tag =
                extract_repr_variant(self.ctx.document, parse_ctx.node_id(), &self.schema.repr)
                    .ok()
                    .flatten()
                    .is_some();
            has_variant_ext || has_repr_tag
        };

        let deny_untagged = &self.schema.deny_untagged;

        // Register priority variants first
        if let Some(priority) = &self.schema.priority {
            for name in priority {
                if let Some(&variant_schema_id) = self.schema.variants.get(name) {
                    let ctx = self.ctx;
                    let schema_node_id = variant_schema_id;
                    let variant_name = name.clone();
                    let requires_explicit = deny_untagged.contains(name);
                    builder = builder.variant(name, move |parse_ctx: &ParseContext<'_>| {
                        validate_variant(
                            ctx,
                            parse_ctx,
                            schema_node_id,
                            is_tagged,
                            &variant_name,
                            requires_explicit,
                            has_explicit_tag,
                        )
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
            let requires_explicit = deny_untagged.contains(name);
            builder = builder.other(name, move |parse_ctx: &ParseContext<'_>| {
                validate_variant(
                    ctx,
                    parse_ctx,
                    schema_node_id,
                    is_tagged,
                    &variant_name,
                    requires_explicit,
                    has_explicit_tag,
                )
            });
        }

        // Execute union parsing/validation
        match builder.parse() {
            Ok(()) => {
                // Success - clear any accumulated variant errors
                self.ctx.clear_variant_errors();
                Ok(())
            }
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
                    // Untagged union failed - create NoVariantMatched with best match info
                    let variant_errors = self.ctx.take_variant_errors();
                    let best_match = select_best_variant_match(variant_errors).map(Box::new);

                    self.ctx.record_error(ValidationError::NoVariantMatched {
                        path: self.ctx.path(),
                        best_match,
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
/// so they are reported with correct node positions. When false (untagged mode), store
/// errors for later analysis to find the best matching variant.
///
/// `requires_explicit_tag`: When true, this variant is in deny_untagged and requires explicit tagging.
/// `has_explicit_tag`: Whether the value has an explicit variant tag ($variant or repr pattern).
fn validate_variant<'doc>(
    ctx: &ValidationContext<'doc>,
    parse_ctx: &ParseContext<'doc>,
    schema_node_id: SchemaNodeId,
    propagate_errors: bool,
    variant_name: &str,
    requires_explicit_tag: bool,
    has_explicit_tag: bool,
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
        // Check deny_untagged constraint: variant requires explicit tag but none was provided
        if requires_explicit_tag && !has_explicit_tag {
            ctx.record_error(ValidationError::RequiresExplicitVariant {
                variant: variant_name.to_string(),
                path: ctx.path(),
                node_id: parse_ctx.node_id(),
                schema_node_id,
            });
            // Signal that inner errors were propagated - no additional error needed
            return Err(ValidatorError::InnerErrorsPropagated);
        }

        // Success - merge any warnings/holes from trial
        ctx.merge_state(trial_ctx.state.into_inner());
        Ok(())
    } else {
        // Validation failed
        let trial_state = trial_ctx.state.into_inner();

        if propagate_errors && !trial_state.errors.is_empty() {
            // Tagged mode: propagate errors to parent context
            ctx.merge_state(trial_state);
            // Signal that inner errors were propagated - no additional error needed
            Err(ValidatorError::InnerErrorsPropagated)
        } else {
            // Untagged mode: store errors for later analysis
            if !trial_state.errors.is_empty() {
                ctx.record_variant_errors(variant_name.to_string(), trial_state.errors);
            }
            Err(ValidatorError::InvalidVariantTag {
                tag: variant_name.to_string(),
                reason: "type mismatch".to_string(),
            })
        }
    }
}

/// Select the best matching variant from collected errors.
///
/// The "best" variant is selected based on:
/// 1. **Depth** (primary): Errors deeper in the structure indicate better match
/// 2. **Error count** (secondary): Fewer errors indicate closer match
/// 3. **Error priority** (tertiary): Higher priority errors indicate clearer mismatch
///
/// Returns None if no variants were tried or all had empty errors.
fn select_best_variant_match(
    variant_errors: Vec<(String, Vec<ValidationError>)>,
) -> Option<BestVariantMatch> {
    if variant_errors.is_empty() {
        return None;
    }

    // Find the best match based on metrics
    let best = variant_errors
        .into_iter()
        .filter(|(_, errors)| !errors.is_empty())
        .max_by_key(|(_, errors)| {
            // Calculate metrics
            let max_depth = errors.iter().map(|e| e.depth()).max().unwrap_or(0);
            let error_count = errors.len();
            let max_priority = errors.iter().map(|e| e.priority_score()).max().unwrap_or(0);

            // Return tuple for comparison: (depth, -count, priority)
            // Higher depth = better (got further)
            // Lower count = better (fewer issues), so we use MAX - count
            // Higher priority = better (more significant mismatch)
            (max_depth, usize::MAX - error_count, max_priority)
        });

    best.map(|(variant_name, mut errors)| {
        let depth = errors.iter().map(|e| e.depth()).max().unwrap_or(0);
        let error_count = errors.len();

        // Select primary error (highest priority, or deepest if tied)
        errors.sort_by_key(|e| {
            (
                std::cmp::Reverse(e.priority_score()),
                std::cmp::Reverse(e.depth()),
            )
        });
        let primary_error = errors.first().cloned().unwrap();

        BestVariantMatch {
            variant_name,
            error: Box::new(primary_error),
            all_errors: errors,
            depth,
            error_count,
        }
    })
}
