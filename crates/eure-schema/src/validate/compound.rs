//! Compound type validators
//!
//! Validators for: Array, Map, Tuple
//! Uses ParseContext APIs and SchemaValidator for child validation.

use eure_document::document::node::NodeValue;
use eure_document::parse::{DocumentParser, ParseContext};
use eure_document::value::ObjectKey;

use crate::{ArraySchema, MapSchema, SchemaNodeContent, SchemaNodeId, TupleSchema};

use super::SchemaValidator;
use super::context::ValidationContext;
use super::error::{ValidationError, ValidatorError};

// =============================================================================
// ArrayValidator
// =============================================================================

/// Validates array values against ArraySchema constraints.
pub struct ArrayValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s ArraySchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for ArrayValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Use parse::<Vec<...>>() pattern - but we need raw array access for validation
        let arr = match &parse_ctx.node().content {
            NodeValue::Array(a) => a,
            _ => {
                self.ctx.record_error(ValidationError::TypeMismatch {
                    expected: "array".to_string(),
                    actual: super::primitive::node_type_name(&parse_ctx.node().content),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
                return Ok(());
            }
        };

        let len = arr.len();

        // Validate length constraints
        if let Some(min) = self.schema.min_length
            && len < min as usize
        {
            self.ctx
                .record_error(ValidationError::ArrayLengthOutOfBounds {
                    length: len,
                    min: Some(min),
                    max: self.schema.max_length,
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
        }
        if let Some(max) = self.schema.max_length
            && len > max as usize
        {
            self.ctx
                .record_error(ValidationError::ArrayLengthOutOfBounds {
                    length: len,
                    min: self.schema.min_length,
                    max: Some(max),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
        }

        // Collect items to avoid borrowing issues
        let items: Vec<_> = arr.iter().copied().collect();

        // Validate each item using SchemaValidator
        for (i, item_id) in items.iter().enumerate() {
            self.ctx.push_path_index(i);

            let item_ctx = self.ctx.parse_context(*item_id);
            let child_validator = SchemaValidator {
                ctx: self.ctx,
                schema_node_id: self.schema.item,
            };
            let _ = item_ctx.parse_with(child_validator);

            self.ctx.pop_path();
        }

        // Validate unique constraint
        if self.schema.unique {
            self.validate_unique(&items, node_id);
        }

        // Validate contains constraint
        if let Some(contains_schema) = self.schema.contains {
            self.validate_contains(&items, contains_schema, node_id);
        }

        Ok(())
    }
}

impl<'a, 'doc, 's> ArrayValidator<'a, 'doc, 's> {
    fn validate_unique(
        &self,
        items: &[eure_document::document::NodeId],
        array_node_id: eure_document::document::NodeId,
    ) {
        // O(n²) comparison - could be optimized with hashing
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                let doc_i = self.ctx.document.node_subtree_to_document(items[i]);
                let doc_j = self.ctx.document.node_subtree_to_document(items[j]);
                if doc_i == doc_j {
                    self.ctx.record_error(ValidationError::ArrayNotUnique {
                        path: self.ctx.path(),
                        node_id: array_node_id,
                        schema_node_id: self.schema_node_id,
                    });
                    return;
                }
            }
        }
    }

    fn validate_contains(
        &self,
        items: &[eure_document::document::NodeId],
        contains_schema: SchemaNodeId,
        array_node_id: eure_document::document::NodeId,
    ) {
        for &item_id in items {
            // Fork state for trial validation
            let forked_state = self.ctx.fork_state();
            let trial_ctx = ValidationContext::with_state_and_mode(
                self.ctx.document,
                self.ctx.schema,
                forked_state,
                self.ctx.union_tag_mode,
            );

            let item_parse_ctx = trial_ctx.parse_context(item_id);
            let child_validator = SchemaValidator {
                ctx: &trial_ctx,
                schema_node_id: contains_schema,
            };

            if item_parse_ctx.parse_with(child_validator).is_ok() && !trial_ctx.has_errors() {
                return; // Found a matching element
            }
        }

        self.ctx
            .record_error(ValidationError::ArrayMissingContains {
                path: self.ctx.path(),
                node_id: array_node_id,
                schema_node_id: self.schema_node_id,
            });
    }
}

// =============================================================================
// MapValidator
// =============================================================================

/// Validates map values against MapSchema constraints.
pub struct MapValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s MapSchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for MapValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        let map = match &parse_ctx.node().content {
            NodeValue::Map(m) => m,
            _ => {
                self.ctx.record_error(ValidationError::TypeMismatch {
                    expected: "map".to_string(),
                    actual: super::primitive::node_type_name(&parse_ctx.node().content),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
                return Ok(());
            }
        };

        let size = map.len();

        // Validate size constraints
        if let Some(min) = self.schema.min_size
            && size < min as usize
        {
            self.ctx.record_error(ValidationError::MapSizeOutOfBounds {
                size,
                min: Some(min),
                max: self.schema.max_size,
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }
        if let Some(max) = self.schema.max_size
            && size > max as usize
        {
            self.ctx.record_error(ValidationError::MapSizeOutOfBounds {
                size,
                min: self.schema.min_size,
                max: Some(max),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        // Collect entries to avoid borrowing issues
        let entries: Vec<_> = map.iter().map(|(k, &v)| (k.clone(), v)).collect();

        // Validate each entry
        for (key, val_id) in entries {
            // Validate key type
            self.validate_key_type(&key, node_id);

            // Validate value using SchemaValidator
            self.ctx.push_path_key(key);

            let val_ctx = self.ctx.parse_context(val_id);
            let child_validator = SchemaValidator {
                ctx: self.ctx,
                schema_node_id: self.schema.value,
            };
            let _ = val_ctx.parse_with(child_validator);

            self.ctx.pop_path();
        }

        Ok(())
    }
}

impl<'a, 'doc, 's> MapValidator<'a, 'doc, 's> {
    fn validate_key_type(&self, key: &ObjectKey, map_node_id: eure_document::document::NodeId) {
        let schema_content = self.ctx.resolve_schema_content(self.schema.key);
        let valid = match (key, schema_content) {
            (ObjectKey::String(_), SchemaNodeContent::Text(_)) => true,
            (ObjectKey::Number(_), SchemaNodeContent::Integer(_)) => true,
            (ObjectKey::Tuple(_), SchemaNodeContent::Tuple(_)) => true,
            (_, SchemaNodeContent::Any) => true, // Any accepts any key type
            _ => false,
        };

        if !valid {
            self.ctx.record_error(ValidationError::InvalidKeyType {
                key: key.clone(),
                path: self.ctx.path(),
                node_id: map_node_id,
                schema_node_id: self.schema_node_id,
            });
        }
    }
}

// =============================================================================
// TupleValidator
// =============================================================================

/// Validates tuple values against TupleSchema constraints.
pub struct TupleValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s TupleSchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for TupleValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Get tuple directly from node content
        let tuple = match &parse_ctx.node().content {
            NodeValue::Tuple(t) => t,
            _ => {
                self.ctx.record_error(ValidationError::TypeMismatch {
                    expected: "tuple".to_string(),
                    actual: super::primitive::node_type_name(&parse_ctx.node().content),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
                return Ok(());
            }
        };

        // Check length
        if tuple.len() != self.schema.elements.len() {
            self.ctx.record_error(ValidationError::TupleLengthMismatch {
                expected: self.schema.elements.len(),
                actual: tuple.len(),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
            return Ok(());
        }

        // Collect items to avoid borrowing issues
        let items: Vec<_> = tuple.iter().copied().collect();

        // Validate each element
        for (i, (item_id, &elem_schema)) in
            items.iter().zip(self.schema.elements.iter()).enumerate()
        {
            self.ctx.push_path_tuple_index(i as u8);

            let elem_ctx = self.ctx.parse_context(*item_id);
            let child_validator = SchemaValidator {
                ctx: self.ctx,
                schema_node_id: elem_schema,
            };
            let _ = elem_ctx.parse_with(child_validator);

            self.ctx.pop_path();
        }

        Ok(())
    }
}
