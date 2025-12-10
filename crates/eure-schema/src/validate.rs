//! Document schema validation
//!
//! # Architecture
//!
//! Validation is built on `DocumentParser` composition:
//! - `SchemaValidator`: Dispatches to type-specific validators based on `SchemaNodeContent`
//! - Type validators: Implement `DocumentParser<Output = (), Error = ValidatorError>`
//! - `ValidationContext`: Manages shared state (errors, warnings, path)
//!
//! # Error Handling
//!
//! Two categories of errors:
//! - `ValidationError`: Type mismatches accumulated in `ValidationContext` (non-fatal)
//! - `ValidatorError`: Internal validator errors causing fail-fast (e.g., undefined references)
//!
//! # Hole Values
//!
//! The hole value (`!`) represents an unfilled placeholder:
//! - Type checking: Holes match any schema (always pass)
//! - Completeness: Documents containing holes are valid but not complete

mod compound;
mod context;
mod error;
mod primitive;
mod record;
mod reference;
mod union;

pub use context::{ValidationContext, ValidationOutput, ValidationState};
pub use error::{ValidationError, ValidationWarning, ValidatorError};

use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::parse::{DocumentParser, ParseContext};

use crate::{SchemaDocument, SchemaNodeContent, SchemaNodeId};

use compound::{ArrayValidator, MapValidator, TupleValidator};
use primitive::{
    AnyValidator, BooleanValidator, FloatValidator, IntegerValidator, LiteralValidator,
    NullValidator, TextValidator,
};
use record::RecordValidator;
use reference::ReferenceValidator;
use union::UnionValidator;

// =============================================================================
// Public API
// =============================================================================

/// Validate a document against a schema.
///
/// # Example
///
/// ```ignore
/// let output = validate(&document, &schema);
/// if output.is_valid {
///     println!("Document is valid!");
/// } else {
///     for error in &output.errors {
///         println!("Error: {}", error);
///     }
/// }
/// ```
pub fn validate(document: &EureDocument, schema: &SchemaDocument) -> ValidationOutput {
    let root_id = document.get_root_id();
    validate_node(document, schema, root_id, schema.root)
}

/// Validate a specific node against a schema node.
pub fn validate_node(
    document: &EureDocument,
    schema: &SchemaDocument,
    node_id: NodeId,
    schema_id: SchemaNodeId,
) -> ValidationOutput {
    let ctx = ValidationContext::new(document, schema);
    let parse_ctx = document.parse_context(node_id);

    let validator = SchemaValidator {
        ctx: &ctx,
        schema_node_id: schema_id,
    };

    // Errors are accumulated in ctx, result is always Ok unless internal error
    let _ = parse_ctx.parse_with(validator);

    ctx.finish()
}

// =============================================================================
// SchemaValidator (main dispatcher)
// =============================================================================

/// Main validator that dispatches to type-specific validators.
///
/// Implements `DocumentParser` to enable composition with other parsers.
pub struct SchemaValidator<'a, 'doc> {
    pub ctx: &'a ValidationContext<'doc>,
    pub schema_node_id: SchemaNodeId,
}

impl<'a, 'doc> DocumentParser<'doc> for SchemaValidator<'a, 'doc> {
    type Output = ();
    type Error = ValidatorError;

    fn parse(&mut self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        let node = parse_ctx.node();

        // Check for hole - holes match any schema
        if matches!(&node.content, NodeValue::Hole(_)) {
            self.ctx.mark_has_holes();
            return Ok(());
        }

        let schema_node = self.ctx.schema.node(self.schema_node_id);

        // Validate extensions first
        self.validate_extensions(parse_ctx)?;

        // Dispatch to type-specific validator
        match &schema_node.content {
            SchemaNodeContent::Any => {
                let mut v = AnyValidator;
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Text(s) => {
                let mut v = TextValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Integer(s) => {
                let mut v = IntegerValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Float(s) => {
                let mut v = FloatValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Boolean => {
                let mut v = BooleanValidator {
                    ctx: self.ctx,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Null => {
                let mut v = NullValidator {
                    ctx: self.ctx,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Literal(expected) => {
                let mut v = LiteralValidator {
                    ctx: self.ctx,
                    expected,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Array(s) => {
                let mut v = ArrayValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Map(s) => {
                let mut v = MapValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Record(s) => {
                let mut v = RecordValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Tuple(s) => {
                let mut v = TupleValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Union(s) => {
                let mut v = UnionValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
            SchemaNodeContent::Reference(type_ref) => {
                let mut v = ReferenceValidator {
                    ctx: self.ctx,
                    type_ref,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(parse_ctx)
            }
        }
    }
}

impl<'a, 'doc> SchemaValidator<'a, 'doc> {
    /// Validate extensions on the current node.
    fn validate_extensions(&self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
        use crate::identifiers;

        let schema_node = self.ctx.schema.node(self.schema_node_id);
        let ext_types = &schema_node.ext_types;
        let node = parse_ctx.node();
        let node_id = parse_ctx.node_id();

        // Check for missing required extensions
        for (ext_ident, ext_schema) in ext_types {
            if !ext_schema.optional && !node.extensions.contains_key(ext_ident) {
                self.ctx
                    .record_error(ValidationError::MissingRequiredExtension {
                        extension: ext_ident.to_string(),
                        path: self.ctx.path(),
                        node_id,
                        schema_node_id: self.schema_node_id,
                    });
            }
        }

        // Validate present extensions using parse_extension() API
        let mut ext_parser = parse_ctx.parse_extension();

        for (ext_ident, ext_schema) in ext_types {
            if let Some(ext_ctx) = ext_parser.ext_optional(ext_ident.as_ref()) {
                self.ctx.push_path_extension(ext_ident.clone());

                let child_validator = SchemaValidator {
                    ctx: self.ctx,
                    schema_node_id: ext_schema.schema,
                };
                let _ = ext_ctx.parse_with(child_validator);

                self.ctx.pop_path();
            }
        }

        // Warn about unknown extensions (except $variant)
        for (ext_ident, _) in &node.extensions {
            if ext_ident == &identifiers::VARIANT {
                continue;
            }
            if !ext_types.contains_key(ext_ident) {
                self.ctx
                    .record_warning(ValidationWarning::UnknownExtension {
                        name: ext_ident.to_string(),
                        path: self.ctx.path(),
                    });
            }
        }

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArraySchema, Bound, IntegerSchema, RecordFieldSchema, RecordSchema, TextSchema,
        UnknownFieldsPolicy,
    };
    use eure_document::text::Text;
    use eure_document::value::{ObjectKey, PrimitiveValue};
    use num_bigint::BigInt;
    use std::collections::HashMap;

    fn create_simple_schema(content: SchemaNodeContent) -> (SchemaDocument, SchemaNodeId) {
        let mut schema = SchemaDocument {
            nodes: Vec::new(),
            root: SchemaNodeId(0),
            types: HashMap::new(),
        };
        let id = schema.create_node(content);
        schema.root = id;
        (schema, id)
    }

    fn create_doc_with_primitive(value: PrimitiveValue) -> EureDocument {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Primitive(value);
        doc
    }

    #[test]
    fn test_validate_text_basic() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema::default()));
        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_text_pattern() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema {
            pattern: Some(regex::Regex::new("^[a-z]+$").unwrap()),
            ..Default::default()
        }));

        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext(
            "Hello123".to_string(),
        )));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_integer() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Integer(IntegerSchema {
            min: Bound::Inclusive(BigInt::from(0)),
            max: Bound::Inclusive(BigInt::from(100)),
            multiple_of: None,
        }));

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(50)));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(150)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_boolean() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Boolean);

        let doc = create_doc_with_primitive(PrimitiveValue::Bool(true));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(1)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_array() {
        let (mut schema, _) = create_simple_schema(SchemaNodeContent::Any);
        let item_schema_id =
            schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        schema.node_mut(schema.root).content = SchemaNodeContent::Array(ArraySchema {
            item: item_schema_id,
            min_length: Some(1),
            max_length: Some(3),
            unique: false,
            contains: None,
            binding_style: None,
        });

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Array(Default::default());
        let child1 = doc.add_array_element(None, root_id).unwrap().node_id;
        doc.node_mut(child1).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(1)));
        let child2 = doc.add_array_element(None, root_id).unwrap().node_id;
        doc.node_mut(child2).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(2)));

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_record() {
        let (mut schema, _) = create_simple_schema(SchemaNodeContent::Any);
        let name_schema_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let age_schema_id =
            schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));

        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            RecordFieldSchema {
                schema: name_schema_id,
                optional: false,
                binding_style: None,
            },
        );
        properties.insert(
            "age".to_string(),
            RecordFieldSchema {
                schema: age_schema_id,
                optional: true,
                binding_style: None,
            },
        );

        schema.node_mut(schema.root).content = SchemaNodeContent::Record(RecordSchema {
            properties,
            unknown_fields: UnknownFieldsPolicy::Deny,
        });

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let name_id = doc
            .add_map_child(ObjectKey::String("name".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(name_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("Alice".to_string())));

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_hole() {
        let (schema, _) =
            create_simple_schema(SchemaNodeContent::Integer(IntegerSchema::default()));

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Hole(None);

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
        assert!(!result.is_complete);
    }
}
