//! Primitive type validators
//!
//! Validators for: Text, Integer, Float, Boolean, Null, Literal, Any
//! All implement `DocumentParser<Output = (), Error = ValidatorError>`.

use eure_document::document::EureDocument;
use eure_document::document::node::NodeValue;
use eure_document::parse::{DocumentParser, ParseContext};
use eure_document::value::PrimitiveValue;
use num_bigint::BigInt;

use crate::{Bound, FloatSchema, IntegerSchema, SchemaNodeId, TextSchema};

use super::context::ValidationContext;
use super::error::{ValidationError, ValidatorError};

// =============================================================================
// Helper: node type name for error messages
// =============================================================================

pub(crate) fn node_type_name(content: &NodeValue) -> String {
    match content {
        NodeValue::Hole(_) => "hole".to_string(),
        NodeValue::Primitive(p) => match p {
            PrimitiveValue::Null => "null".to_string(),
            PrimitiveValue::Bool(_) => "boolean".to_string(),
            PrimitiveValue::Integer(_) => "integer".to_string(),
            PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => "float".to_string(),
            PrimitiveValue::Text(_) => "text".to_string(),
        },
        NodeValue::Array(_) => "array".to_string(),
        NodeValue::Tuple(_) => "tuple".to_string(),
        NodeValue::Map(_) => "map".to_string(),
    }
}

/// Extract actual type name from ParseErrorKind for error messages.
pub(crate) fn actual_type_from_error(kind: &eure_document::parse::ParseErrorKind) -> String {
    use eure_document::parse::ParseErrorKind;

    match kind {
        ParseErrorKind::TypeMismatch { actual, .. } => value_kind_to_name(*actual),
        ParseErrorKind::UnexpectedHole => "hole".to_string(),
        _ => format!("{}", kind),
    }
}

/// Convert ValueKind to user-friendly type name.
fn value_kind_to_name(kind: eure_document::value::ValueKind) -> String {
    use eure_document::value::ValueKind;
    match kind {
        ValueKind::Null => "null".to_string(),
        ValueKind::Bool => "boolean".to_string(),
        ValueKind::Integer => "integer".to_string(),
        ValueKind::F32 | ValueKind::F64 => "float".to_string(),
        ValueKind::Text => "text".to_string(),
        ValueKind::Array => "array".to_string(),
        ValueKind::Tuple => "tuple".to_string(),
        ValueKind::Map => "map".to_string(),
        ValueKind::Hole => "hole".to_string(),
    }
}

// =============================================================================
// AnyValidator
// =============================================================================

/// Validates any value (always succeeds).
///
/// Note: Hole checking is done in `SchemaValidator` before dispatching,
/// so by the time `AnyValidator` runs, the node is guaranteed non-hole.
pub struct AnyValidator;

impl<'doc> DocumentParser<'doc> for AnyValidator {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, _parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        Ok(())
    }
}

// =============================================================================
// TextValidator
// =============================================================================

/// Validates text values against TextSchema constraints.
pub struct TextValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s TextSchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for TextValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Use parse_primitive() API
        let primitive = match parse_ctx.parse_primitive() {
            Ok(p) => p,
            Err(e) => {
                self.ctx.record_error(ValidationError::TypeMismatch {
                    expected: "text".to_string(),
                    actual: actual_type_from_error(&e.kind),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
                return Ok(());
            }
        };

        let text = match primitive {
            PrimitiveValue::Text(t) => t,
            other => {
                self.ctx.record_error(ValidationError::TypeMismatch {
                    expected: "text".to_string(),
                    actual: primitive_type_name(other),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
                return Ok(());
            }
        };

        // Validate language
        if let Some(expected_lang) = &self.schema.language {
            use eure_document::text::Language;
            let actual_lang = &text.language;
            let matches = match actual_lang {
                Language::Plaintext => expected_lang == "plaintext" || expected_lang == "text",
                Language::Implicit => true, // Implicit can match any
                Language::Other(lang) => lang == expected_lang,
            };
            if !matches {
                self.ctx.record_error(ValidationError::LanguageMismatch {
                    expected: expected_lang.clone(),
                    actual: format!("{:?}", actual_lang),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
            }
        }

        // Validate length
        let len = text.as_str().chars().count();
        if let Some(min) = self.schema.min_length
            && len < min as usize
        {
            self.ctx
                .record_error(ValidationError::StringLengthOutOfBounds {
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
                .record_error(ValidationError::StringLengthOutOfBounds {
                    length: len,
                    min: self.schema.min_length,
                    max: Some(max),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
        }

        // Validate pattern
        if let Some(regex) = &self.schema.pattern
            && !regex.is_match(text.as_str())
        {
            self.ctx.record_error(ValidationError::PatternMismatch {
                pattern: regex.as_str().to_string(),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        Ok(())
    }
}

fn primitive_type_name(p: &PrimitiveValue) -> String {
    match p {
        PrimitiveValue::Null => "null".to_string(),
        PrimitiveValue::Bool(_) => "boolean".to_string(),
        PrimitiveValue::Integer(_) => "integer".to_string(),
        PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => "float".to_string(),
        PrimitiveValue::Text(_) => "text".to_string(),
    }
}

// =============================================================================
// IntegerValidator
// =============================================================================

/// Validates integer values against IntegerSchema constraints.
pub struct IntegerValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s IntegerSchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for IntegerValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Use parse::<BigInt>() API
        let int_val: BigInt = match parse_ctx.parse() {
            Ok(v) => v,
            Err(e) => {
                self.ctx.record_error(ValidationError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: actual_type_from_error(&e.kind),
                    path: self.ctx.path(),
                    node_id,
                    schema_node_id: self.schema_node_id,
                });
                return Ok(());
            }
        };

        // Validate range
        let in_range = match (&self.schema.min, &self.schema.max) {
            (Bound::Unbounded, Bound::Unbounded) => true,
            (Bound::Inclusive(min), Bound::Unbounded) => &int_val >= min,
            (Bound::Exclusive(min), Bound::Unbounded) => &int_val > min,
            (Bound::Unbounded, Bound::Inclusive(max)) => &int_val <= max,
            (Bound::Unbounded, Bound::Exclusive(max)) => &int_val < max,
            (Bound::Inclusive(min), Bound::Inclusive(max)) => &int_val >= min && &int_val <= max,
            (Bound::Inclusive(min), Bound::Exclusive(max)) => &int_val >= min && &int_val < max,
            (Bound::Exclusive(min), Bound::Inclusive(max)) => &int_val > min && &int_val <= max,
            (Bound::Exclusive(min), Bound::Exclusive(max)) => &int_val > min && &int_val < max,
        };

        if !in_range {
            self.ctx.record_error(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        // Validate multiple-of
        if let Some(divisor) = &self.schema.multiple_of
            && &int_val % divisor != BigInt::from(0)
        {
            self.ctx.record_error(ValidationError::NotMultipleOf {
                divisor: divisor.to_string(),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        Ok(())
    }
}

// =============================================================================
// FloatValidator
// =============================================================================

/// Validates float values against FloatSchema constraints.
pub struct FloatValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema: &'s FloatSchema,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for FloatValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Use parse::<f64>() API - note: this also accepts f32 with conversion
        let float_val: f64 = match parse_ctx.parse() {
            Ok(v) => v,
            Err(e) => {
                // Try integer coercion
                if let Ok(int_val) = parse_ctx.parse::<BigInt>() {
                    int_val.to_string().parse::<f64>().unwrap_or(f64::NAN)
                } else {
                    self.ctx.record_error(ValidationError::TypeMismatch {
                        expected: "float".to_string(),
                        actual: actual_type_from_error(&e.kind),
                        path: self.ctx.path(),
                        node_id,
                        schema_node_id: self.schema_node_id,
                    });
                    return Ok(());
                }
            }
        };

        // Validate range
        let in_range = match (&self.schema.min, &self.schema.max) {
            (Bound::Unbounded, Bound::Unbounded) => true,
            (Bound::Inclusive(min), Bound::Unbounded) => float_val >= *min,
            (Bound::Exclusive(min), Bound::Unbounded) => float_val > *min,
            (Bound::Unbounded, Bound::Inclusive(max)) => float_val <= *max,
            (Bound::Unbounded, Bound::Exclusive(max)) => float_val < *max,
            (Bound::Inclusive(min), Bound::Inclusive(max)) => {
                float_val >= *min && float_val <= *max
            }
            (Bound::Inclusive(min), Bound::Exclusive(max)) => float_val >= *min && float_val < *max,
            (Bound::Exclusive(min), Bound::Inclusive(max)) => float_val > *min && float_val <= *max,
            (Bound::Exclusive(min), Bound::Exclusive(max)) => float_val > *min && float_val < *max,
        };

        if !in_range {
            self.ctx.record_error(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        // Validate multiple-of
        if let Some(divisor) = self.schema.multiple_of
            && (float_val % divisor).abs() > f64::EPSILON
        {
            self.ctx.record_error(ValidationError::NotMultipleOf {
                divisor: divisor.to_string(),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        Ok(())
    }
}

// =============================================================================
// BooleanValidator
// =============================================================================

/// Validates boolean values.
pub struct BooleanValidator<'a, 'doc> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc> DocumentParser<'doc> for BooleanValidator<'a, 'doc> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Use parse::<bool>() API
        if let Err(e) = parse_ctx.parse::<bool>() {
            self.ctx.record_error(ValidationError::TypeMismatch {
                expected: "boolean".to_string(),
                actual: actual_type_from_error(&e.kind),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        Ok(())
    }
}

// =============================================================================
// NullValidator
// =============================================================================

/// Validates null values.
pub struct NullValidator<'a, 'doc> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc> DocumentParser<'doc> for NullValidator<'a, 'doc> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();

        // Check if null using is_null() API
        if !parse_ctx.is_null() {
            self.ctx.record_error(ValidationError::TypeMismatch {
                expected: "null".to_string(),
                actual: node_type_name(&parse_ctx.node().content),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }

        Ok(())
    }
}

// =============================================================================
// LiteralValidator
// =============================================================================

/// Validates literal values (exact match).
///
/// Note: When validating inside a union, accessed extensions (like `$variant`)
/// are excluded from comparison because they were consumed during parsing/validation.
pub struct LiteralValidator<'a, 'doc, 's> {
    pub ctx: &'a ValidationContext<'doc>,
    pub expected: &'s EureDocument,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc, 's> DocumentParser<'doc> for LiteralValidator<'a, 'doc, 's> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node_id = parse_ctx.node_id();
        // Get subtree excluding accessed extensions (like $variant consumed by union)
        let actual = parse_ctx.node_subtree_to_document_excluding_accessed();

        if actual != *self.expected {
            self.ctx.record_error(ValidationError::LiteralMismatch {
                expected: format!("{:?}", self.expected),
                actual: format!("{:?}", actual),
                path: self.ctx.path(),
                node_id,
                schema_node_id: self.schema_node_id,
            });
        }
        Ok(())
    }
}
