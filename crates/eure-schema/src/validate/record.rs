//! Record type validator
//!
//! Validates records against RecordSchema constraints using parse_record() API.

use eure_document::identifier::Identifier;
use eure_document::parse::{DocumentParser, ParseContext};
use eure_document::value::ObjectKey;

use crate::{
    MapSchema, RecordSchema, SchemaNodeContent, SchemaNodeId, UnionSchema, UnknownFieldsPolicy,
};

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
        // Collect missing required fields to report in a single error
        let mut missing_required_fields: Vec<String> = Vec::new();

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
                missing_required_fields.push(field_name.to_string());
            }
        }

        // Report all missing required fields in a single error
        if !missing_required_fields.is_empty() {
            self.ctx
                .record_error(ValidationError::MissingRequiredField {
                    fields: missing_required_fields,
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
        }

        // Process flatten targets
        // Each flatten target shares the field access tracking via rec.flatten()
        // Track if any flatten target resolves to a Map (those handle non-string keys)
        let has_map_flatten = self
            .schema
            .flatten
            .iter()
            .any(|&id| self.flatten_target_is_map(id));

        for &flatten_schema_id in &self.schema.flatten {
            let flatten_ctx = rec.flatten();
            self.validate_flatten_target(&flatten_ctx, flatten_schema_id, node_id)?;
        }

        // Handle unknown fields using unknown_fields() iterator
        // This happens after all flatten targets have been processed
        for result in rec.unknown_fields() {
            let (field_name, field_ctx) = match result {
                Ok(field) => field,
                Err((key, ctx)) => {
                    // Non-string key in record
                    // Only report if there are no Map flatten targets (which handle non-string keys)
                    if !has_map_flatten {
                        self.ctx.record_error(ValidationError::InvalidKeyType {
                            key: key.clone(),
                            path: self.ctx.path(),
                            node_id: ctx.node_id(),
                            schema_node_id: self.schema_node_id,
                        });
                    }
                    continue;
                }
            };
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
    /// Check if a flatten target resolves to a Map schema.
    ///
    /// Follows references to determine the underlying schema type.
    fn flatten_target_is_map(&self, schema_id: SchemaNodeId) -> bool {
        let node = self.ctx.schema.node(schema_id);
        match &node.content {
            SchemaNodeContent::Map(_) => true,
            SchemaNodeContent::Reference(type_ref) => {
                // Resolve the reference and recurse
                if let Some(resolved_id) = self.ctx.schema.get_type(&type_ref.name) {
                    self.flatten_target_is_map(resolved_id)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

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
            SchemaNodeContent::Map(map_schema) => {
                self.validate_flattened_map(
                    flatten_ctx,
                    map_schema,
                    flatten_schema_id,
                    parent_node_id,
                )?;
            }
            _ => {
                // Only Record, Union, Map, and Reference can be flattened
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
            .map(|(name, &id)| (name.clone(), id, self.try_variant(flatten_ctx, id)))
            .partition(|(_, _, result)| result.is_ok());

        let matched: Vec<_> = matched.into_iter().map(|(name, _, _)| name).collect();
        let errors: Vec<_> = errors
            .into_iter()
            .filter_map(|(name, id, r)| r.err().map(|e| (name, id, e)))
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

    /// Validate a flattened map - unknown entries are validated against the map schema.
    ///
    /// When a map is flattened into a record:
    /// 1. Explicit record properties are matched first (handled before flatten processing)
    /// 2. The flattened map consumes ALL remaining unknown entries (both string and non-string keys)
    /// 3. Each key is validated against the map's key schema
    /// 4. Each value is validated against the map's value schema
    /// 5. Map size constraints (min-size/max-size) apply to the count of consumed entries
    fn validate_flattened_map(
        &self,
        flatten_ctx: &ParseContext<'doc>,
        map_schema: &MapSchema,
        schema_node_id: SchemaNodeId,
        parent_node_id: eure_document::document::NodeId,
    ) -> Result<(), ValidatorError> {
        // Parse as a record to get access to unknown_entries()
        let rec = match flatten_ctx.parse_record() {
            Ok(r) => r,
            Err(_) => {
                // Not a record - this shouldn't happen in flatten context
                return Ok(());
            }
        };

        // First, collect all unknown entries (both string and non-string keys)
        let unknown_entries: Vec<(ObjectKey, eure_document::document::NodeId)> = rec
            .unknown_entries()
            .map(|(key, ctx)| (key.clone(), ctx.node_id()))
            .collect();

        // Now consume each entry and validate
        let mut field_count = 0usize;
        for (key, value_node_id) in &unknown_entries {
            field_count += 1;

            // For string keys, mark as consumed via field_optional
            if let ObjectKey::String(name) = key {
                let _ = rec.field_optional(name);
            }

            // Validate key against map's key schema
            self.validate_flattened_map_key(key, map_schema, schema_node_id, parent_node_id);

            // Validate value against map's value schema
            self.ctx.push_path_key(key.clone());

            let value_ctx = ParseContext::new(self.ctx.document, *value_node_id);
            let child_validator = SchemaValidator {
                ctx: self.ctx,
                schema_node_id: map_schema.value,
            };
            let _ = value_ctx.parse_with(child_validator);

            self.ctx.pop_path();
        }

        // Validate size constraints
        if let Some(min) = map_schema.min_size
            && field_count < min as usize
        {
            self.ctx.record_error(ValidationError::MapSizeOutOfBounds {
                size: field_count,
                min: Some(min),
                max: map_schema.max_size,
                path: self.ctx.path(),
                node_id: parent_node_id,
                schema_node_id,
            });
        }
        if let Some(max) = map_schema.max_size
            && field_count > max as usize
        {
            self.ctx.record_error(ValidationError::MapSizeOutOfBounds {
                size: field_count,
                min: map_schema.min_size,
                max: Some(max),
                path: self.ctx.path(),
                node_id: parent_node_id,
                schema_node_id,
            });
        }

        Ok(())
    }

    /// Validate a key against the map's key schema.
    ///
    /// Handles both string and integer keys, validating them against the expected
    /// key schema type. For text schemas, also validates pattern and length constraints.
    fn validate_flattened_map_key(
        &self,
        key: &ObjectKey,
        map_schema: &MapSchema,
        schema_node_id: SchemaNodeId,
        record_node_id: eure_document::document::NodeId,
    ) {
        let key_content = self.ctx.resolve_schema_content(map_schema.key);

        match (key, key_content) {
            // String key with text schema - validate pattern and length constraints
            (ObjectKey::String(field_name), SchemaNodeContent::Text(text_schema)) => {
                // Validate pattern constraint
                if let Some(regex) = &text_schema.pattern
                    && !regex.is_match(field_name)
                {
                    self.ctx
                        .record_error(ValidationError::FlattenMapKeyMismatch {
                            key: field_name.to_string(),
                            pattern: Some(regex.as_str().to_string()),
                            path: self.ctx.path(),
                            node_id: record_node_id,
                            schema_node_id,
                        });
                }

                // Validate length constraints
                let len = field_name.chars().count();
                if let Some(min) = text_schema.min_length
                    && len < min as usize
                {
                    self.ctx
                        .record_error(ValidationError::StringLengthOutOfBounds {
                            length: len,
                            min: Some(min),
                            max: text_schema.max_length,
                            path: self.ctx.path(),
                            node_id: record_node_id,
                            schema_node_id,
                        });
                }
                if let Some(max) = text_schema.max_length
                    && len > max as usize
                {
                    self.ctx
                        .record_error(ValidationError::StringLengthOutOfBounds {
                            length: len,
                            min: text_schema.min_length,
                            max: Some(max),
                            path: self.ctx.path(),
                            node_id: record_node_id,
                            schema_node_id,
                        });
                }
            }
            // Number key with integer schema - valid (could add range validation if needed)
            (ObjectKey::Number(_), SchemaNodeContent::Integer(_)) => {
                // Number key matches integer schema - valid
                // TODO: Could add range validation here if needed
            }
            // Any schema accepts any key type
            (_, SchemaNodeContent::Any) => {
                // Any accepts any key
            }
            // Type mismatch - record an error
            (_, _) => {
                self.ctx.record_error(ValidationError::InvalidKeyType {
                    key: key.clone(),
                    path: self.ctx.path(),
                    node_id: record_node_id,
                    schema_node_id,
                });
            }
        }
    }
}
