//! Record type validator
//!
//! Validates records against RecordSchema constraints using parse_record() API.

use std::collections::HashSet;

use eure_document::identifier::Identifier;
use eure_document::parse::{DocumentParser, ParseContext, ParserScope};
use eure_document::value::ObjectKey;

use crate::{
    MapSchema, RecordSchema, SchemaNodeContent, SchemaNodeId, UnionSchema, UnknownFieldsPolicy,
};

use super::SchemaValidator;
use super::context::{ValidationContext, ValidationState};
use super::error::{ValidationError, ValidationWarning, ValidatorError, select_best_variant_match};
use super::key::key_matches_schema;
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

struct DeferredUnknownField {
    name: String,
}

struct FlattenVariantTrial {
    variant_name: String,
    schema_node_id: SchemaNodeId,
    hard_errors: Vec<ValidationError>,
    deferred_unknown_fields: Vec<DeferredUnknownField>,
    validation_state: ValidationState,
    accessed_state: eure_document::parse::AccessedSnapshot,
}

struct PendingFlattenedUnionError {
    schema_node_id: SchemaNodeId,
    parent_node_id: eure_document::document::NodeId,
    trials: Vec<FlattenVariantTrial>,
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
        let mut pending_flattened_unions = Vec::new();

        for &flatten_schema_id in &self.schema.flatten {
            let flatten_ctx = rec.flatten();
            if let Some(pending_union_error) =
                self.validate_flatten_target(&flatten_ctx, flatten_schema_id, node_id)?
            {
                pending_flattened_unions.push(pending_union_error);
            }
        }

        let mut unknown_string_fields = HashSet::new();
        let mut unknown_fields = Vec::new();
        let mut invalid_keys = Vec::new();
        for result in rec.unknown_fields() {
            match result {
                Ok((field_name, field_ctx)) => {
                    unknown_string_fields.insert(field_name.to_string());
                    unknown_fields.push((field_name.to_string(), field_ctx));
                }
                Err((key, ctx)) => invalid_keys.push((key.clone(), ctx)),
            }
        }

        for pending_union_error in pending_flattened_unions {
            self.record_pending_flattened_union_error(pending_union_error, &unknown_string_fields);
        }

        // A flattened record participates in its parent's field space.
        // The parent validates any entries that remain unconsumed after all
        // flatten targets have run.
        if parse_ctx.parser_scope() != Some(ParserScope::Record) {
            for (key, ctx) in invalid_keys {
                // Non-string key in record
                // Only report if there are no Map flatten targets (which handle non-string keys)
                if !has_map_flatten {
                    self.ctx.record_error(ValidationError::InvalidKeyType {
                        key,
                        path: self.ctx.path(),
                        node_id: ctx.node_id(),
                        schema_node_id: self.schema_node_id,
                    });
                }
            }

            for (field_name, field_ctx) in unknown_fields {
                match &self.schema.unknown_fields {
                    UnknownFieldsPolicy::Deny => {
                        self.ctx.record_error(ValidationError::UnknownField {
                            field: field_name,
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
                                .push_path_key(ObjectKey::String(field_name.clone()));
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
    ) -> Result<Option<PendingFlattenedUnionError>, ValidatorError> {
        let flatten_node = self.ctx.schema.node(flatten_schema_id);

        match &flatten_node.content {
            SchemaNodeContent::Record(_) => {
                self.validate_flattened_record(flatten_ctx, flatten_schema_id)?;
                Ok(None)
            }
            SchemaNodeContent::Union(union_schema) => self.validate_flattened_union(
                flatten_ctx,
                union_schema,
                flatten_schema_id,
                parent_node_id,
            ),
            SchemaNodeContent::Reference(type_ref) => {
                // Resolve the reference and recurse
                if let Some(resolved_id) = self.ctx.schema.get_type(&type_ref.name) {
                    self.validate_flatten_target(flatten_ctx, resolved_id, parent_node_id)
                } else {
                    // Record error for undefined type reference
                    self.ctx
                        .record_error(ValidationError::UndefinedTypeReference {
                            name: type_ref.name.to_string(),
                            path: self.ctx.path(),
                            node_id: parent_node_id,
                            schema_node_id: flatten_schema_id,
                        });
                    Ok(None)
                }
            }
            SchemaNodeContent::Map(map_schema) => {
                self.validate_flattened_map(
                    flatten_ctx,
                    map_schema,
                    flatten_schema_id,
                    parent_node_id,
                )?;
                Ok(None)
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
                Ok(None)
            }
        }
    }

    /// Validate a flattened record's properties against the current record.
    ///
    /// The flattened child shares the parent's field space, and `rec.flatten()`
    /// encodes that in the parse context.
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
    ) -> Result<Option<PendingFlattenedUnionError>, ValidatorError> {
        let flatten_state = flatten_ctx
            .flatten_ctx()
            .expect("flattened union must run in flatten context");
        let base_accessed_state = flatten_state.capture_current_state();
        let union_owned_fields = self.collect_owned_field_names_for_union(union_schema);

        let mut matched = Vec::new();
        let mut failed_trials = Vec::new();

        for (variant_name, &variant_schema_id) in &union_schema.variants {
            flatten_state.restore_to_state(base_accessed_state.clone());
            let trial = self.try_variant(
                flatten_ctx,
                variant_name,
                variant_schema_id,
                &union_owned_fields,
            )?;
            if trial.hard_errors.is_empty() {
                matched.push(trial);
            } else {
                failed_trials.push(trial);
            }
        }

        match matched.len() {
            0 => {
                flatten_state.restore_to_state(base_accessed_state);
                Ok(Some(PendingFlattenedUnionError {
                    schema_node_id,
                    parent_node_id,
                    trials: failed_trials,
                }))
            }
            1 => {
                let trial = matched.into_iter().next().unwrap();
                flatten_state.restore_to_state(trial.accessed_state);
                self.ctx.merge_state(trial.validation_state);
                Ok(None)
            }
            _ => {
                let merged_accessed_state = merge_accessed_states(
                    &base_accessed_state,
                    matched.iter().map(|trial| &trial.accessed_state),
                );
                flatten_state.restore_to_state(merged_accessed_state);
                self.ctx.record_error(ValidationError::AmbiguousUnion {
                    variants: matched
                        .into_iter()
                        .map(|trial| trial.variant_name)
                        .collect(),
                    path: self.ctx.path(),
                    node_id: parent_node_id,
                    schema_node_id,
                });
                Ok(None)
            }
        }
    }

    /// Try validating a flattened-union variant.
    fn try_variant(
        &self,
        flatten_ctx: &ParseContext<'doc>,
        variant_name: &str,
        schema_node_id: SchemaNodeId,
        union_owned_fields: &HashSet<String>,
    ) -> Result<FlattenVariantTrial, ValidatorError> {
        let forked = self.ctx.fork_state();
        let trial = ValidationContext::with_state(self.ctx.document, self.ctx.schema, forked);
        let _ = flatten_ctx.parse_with(SchemaValidator {
            ctx: &trial,
            schema_node_id,
        });

        let validation_state = trial.state.into_inner();
        let mut hard_errors = validation_state.errors.clone();
        let deferred_unknown_fields = match flatten_ctx.parse_record() {
            Ok(record) => record
                .unknown_fields()
                .filter_map(|result| match result {
                    Ok((field_name, _)) => {
                        let field_name = field_name.to_string();
                        if union_owned_fields.contains(&field_name) {
                            hard_errors.push(ValidationError::UnknownField {
                                field: field_name,
                                path: self.ctx.path(),
                                node_id: flatten_ctx.node_id(),
                                schema_node_id,
                            });
                            None
                        } else {
                            Some(DeferredUnknownField { name: field_name })
                        }
                    }
                    Err(_) => None,
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        let accessed_state = flatten_ctx
            .flatten_ctx()
            .expect("flattened union must run in flatten context")
            .capture_current_state();

        Ok(FlattenVariantTrial {
            variant_name: variant_name.to_string(),
            schema_node_id,
            hard_errors,
            deferred_unknown_fields,
            validation_state,
            accessed_state,
        })
    }

    fn collect_owned_field_names_for_union(&self, union_schema: &UnionSchema) -> HashSet<String> {
        let mut field_names = HashSet::new();
        let mut visited = HashSet::new();
        for &variant_schema_id in union_schema.variants.values() {
            self.collect_owned_field_names(variant_schema_id, &mut field_names, &mut visited);
        }
        field_names
    }

    fn collect_owned_field_names(
        &self,
        schema_id: SchemaNodeId,
        field_names: &mut HashSet<String>,
        visited: &mut HashSet<SchemaNodeId>,
    ) {
        if !visited.insert(schema_id) {
            return;
        }

        let schema_node = self.ctx.schema.node(schema_id);
        match &schema_node.content {
            SchemaNodeContent::Record(record_schema) => {
                field_names.extend(record_schema.properties.keys().cloned());
                for &flatten_schema_id in &record_schema.flatten {
                    self.collect_owned_field_names(flatten_schema_id, field_names, visited);
                }
            }
            SchemaNodeContent::Union(union_schema) => {
                for &variant_schema_id in union_schema.variants.values() {
                    self.collect_owned_field_names(variant_schema_id, field_names, visited);
                }
            }
            SchemaNodeContent::Reference(type_ref) => {
                if let Some(resolved_id) = self.ctx.schema.get_type(&type_ref.name) {
                    self.collect_owned_field_names(resolved_id, field_names, visited);
                }
            }
            _ => {}
        }
    }

    fn record_pending_flattened_union_error(
        &self,
        pending: PendingFlattenedUnionError,
        final_unknown_fields: &HashSet<String>,
    ) {
        let PendingFlattenedUnionError {
            schema_node_id,
            parent_node_id,
            trials,
        } = pending;

        let variant_errors = trials
            .into_iter()
            .map(|trial| {
                let mut errors = trial.hard_errors;
                for deferred in trial.deferred_unknown_fields {
                    if final_unknown_fields.contains(&deferred.name) {
                        errors.push(ValidationError::UnknownField {
                            field: deferred.name,
                            path: self.ctx.path(),
                            node_id: parent_node_id,
                            schema_node_id: trial.schema_node_id,
                        });
                    }
                }
                (trial.variant_name, trial.schema_node_id, errors)
            })
            .collect();

        self.ctx.record_error(ValidationError::NoVariantMatched {
            path: self.ctx.path(),
            best_match: select_best_variant_match(variant_errors).map(Box::new),
            node_id: parent_node_id,
            schema_node_id,
        });
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
            (
                _,
                SchemaNodeContent::Boolean
                | SchemaNodeContent::Union(_)
                | SchemaNodeContent::Reference(_),
            ) if key_matches_schema(self.ctx, key, map_schema.key) => {
                // Boolean/union/reference key schema matched
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

fn merge_accessed_states<'a>(
    base: &eure_document::parse::AccessedSnapshot,
    states: impl IntoIterator<Item = &'a eure_document::parse::AccessedSnapshot>,
) -> eure_document::parse::AccessedSnapshot {
    let mut merged_fields = base.0.clone();
    let mut merged_extensions = base.1.clone();
    for state in states {
        merged_fields.extend(state.0.iter().cloned());
        merged_extensions.extend(state.1.iter().cloned());
    }
    (merged_fields, merged_extensions)
}
