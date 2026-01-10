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

// Re-export UnionTagMode for convenience
pub use eure_document::parse::UnionTagMode;

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
/// Uses the default `Eure` union tag mode.
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
    validate_with_mode(document, schema, UnionTagMode::default())
}

/// Validate a document against a schema with the specified union tag mode.
///
/// # Arguments
///
/// * `document` - The document to validate
/// * `schema` - The schema to validate against
/// * `mode` - The union tag mode to use:
///   - `UnionTagMode::Eure`: Use `$variant` extension or untagged matching (native Eure documents)
///   - `UnionTagMode::Repr`: Use only `VariantRepr` patterns (JSON/YAML imports)
pub fn validate_with_mode(
    document: &EureDocument,
    schema: &SchemaDocument,
    mode: UnionTagMode,
) -> ValidationOutput {
    let root_id = document.get_root_id();
    validate_node_with_mode(document, schema, root_id, schema.root, mode)
}

/// Validate a specific node against a schema node.
///
/// Uses the default `Eure` union tag mode.
pub fn validate_node(
    document: &EureDocument,
    schema: &SchemaDocument,
    node_id: NodeId,
    schema_id: SchemaNodeId,
) -> ValidationOutput {
    validate_node_with_mode(
        document,
        schema,
        node_id,
        schema_id,
        UnionTagMode::default(),
    )
}

/// Validate a specific node against a schema node with the specified union tag mode.
pub fn validate_node_with_mode(
    document: &EureDocument,
    schema: &SchemaDocument,
    node_id: NodeId,
    schema_id: SchemaNodeId,
    mode: UnionTagMode,
) -> ValidationOutput {
    let ctx = ValidationContext::with_mode(document, schema, mode);
    let parse_ctx = ctx.parse_context(node_id);

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

        // Create a flattened context so extensions and content validation share AccessedSet
        let parse_ctx = parse_ctx.flatten();

        // Validate extensions (accesses tracked in flatten context)
        self.validate_extensions(&parse_ctx)?;

        // Dispatch to type-specific validator
        match &schema_node.content {
            SchemaNodeContent::Any => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = AnyValidator;
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Text(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = TextValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Integer(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = IntegerValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Float(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = FloatValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Boolean => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = BooleanValidator {
                    ctx: self.ctx,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Null => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = NullValidator {
                    ctx: self.ctx,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Literal(expected) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = LiteralValidator {
                    ctx: self.ctx,
                    expected,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Array(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = ArrayValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Map(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = MapValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Record(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = RecordValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Tuple(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = TupleValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Union(s) => {
                self.warn_unknown_extensions(&parse_ctx);
                let mut v = UnionValidator {
                    ctx: self.ctx,
                    schema: s,
                    schema_node_id: self.schema_node_id,
                };
                v.parse(&parse_ctx)
            }
            SchemaNodeContent::Reference(r) => {
                // Reference: recurse with resolved schema using the same flattened context
                // This ensures extension tracking is shared through Reference indirection
                let mut child_validator = ReferenceValidator {
                    ctx: self.ctx,
                    type_ref: r,
                    schema_node_id: self.schema_node_id,
                };
                child_validator.parse(&parse_ctx)
            }
        }
    }
}

impl<'a, 'doc> SchemaValidator<'a, 'doc> {
    /// Validate extensions on the current node.
    ///
    /// This validates required and present extensions. Accesses are tracked
    /// in the flatten context's AccessedSet.
    fn validate_extensions(&self, parse_ctx: &ParseContext<'doc>) -> Result<(), ValidatorError> {
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

        // Validate present extensions - accesses are tracked in the shared flatten context
        for (ext_ident, ext_schema) in ext_types {
            if let Some(ext_ctx) = parse_ctx.ext_optional(ext_ident.as_ref()) {
                self.ctx.push_path_extension(ext_ident.clone());

                let child_validator = SchemaValidator {
                    ctx: self.ctx,
                    schema_node_id: ext_schema.schema,
                };
                let _ = ext_ctx.parse_with(child_validator);

                self.ctx.pop_path();
            }
        }

        Ok(())
    }

    /// Warn about unknown extensions at terminal types.
    ///
    /// Extensions that are:
    /// - Not accessed (not in schema's ext_types)
    /// - Not built-in ($variant, $schema, $ext-type, etc.)
    ///
    /// Uses the shared AccessedSet from the flatten context to determine
    /// which extensions have been accessed.
    fn warn_unknown_extensions(&self, parse_ctx: &ParseContext<'doc>) {
        for (ext_ident, _) in parse_ctx.unknown_extensions() {
            // Skip built-in extensions used by the schema system
            if Self::is_builtin_extension(ext_ident) {
                continue;
            }
            self.ctx
                .record_warning(ValidationWarning::UnknownExtension {
                    name: ext_ident.to_string(),
                    path: self.ctx.path(),
                });
        }
    }

    /// Check if an extension is a built-in schema system extension.
    ///
    /// Built-in extensions are always allowed and not warned about:
    /// - $variant: used by union types
    /// - $schema: used to specify the schema for a document
    /// - $ext-type: used to define extension types in schemas
    /// - $codegen: used for code generation hints
    /// - $codegen-defaults: used for default codegen settings
    /// - $flatten: used for record field flattening
    fn is_builtin_extension(ident: &eure_document::identifier::Identifier) -> bool {
        use crate::identifiers;

        // Core schema extensions
        ident == &identifiers::VARIANT
            || ident == &identifiers::SCHEMA
            || ident == &identifiers::EXT_TYPE
            // Codegen extensions
            || ident.as_ref() == "codegen"
            || ident.as_ref() == "codegen-defaults"
            // FIXME: This seems not builtin so must be properly handled.
            || ident.as_ref() == "flatten"
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArraySchema, Bound, IntegerSchema, RangeStyle, RecordFieldSchema, RecordSchema, TextSchema,
        UnionSchema, UnknownFieldsPolicy,
    };
    use eure_document::data_model::VariantRepr;
    use eure_document::text::Text;
    use eure_document::value::{ObjectKey, PrimitiveValue};
    use indexmap::{IndexMap, IndexSet};
    use num_bigint::BigInt;

    fn create_simple_schema(content: SchemaNodeContent) -> (SchemaDocument, SchemaNodeId) {
        let mut schema = SchemaDocument {
            nodes: Vec::new(),
            root: SchemaNodeId(0),
            types: IndexMap::new(),
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
            range_style: RangeStyle::default(),
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
            prefer_inline: None,
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

        let mut properties = IndexMap::new();
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
            flatten: vec![],
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

    /// Helper to create a literal schema from an EureDocument
    fn create_literal_schema(
        schema: &mut SchemaDocument,
        literal_doc: EureDocument,
    ) -> SchemaNodeId {
        schema.create_node(SchemaNodeContent::Literal(literal_doc))
    }

    #[test]
    fn test_validate_union_deny_untagged_without_tag() {
        use eure_document::eure;

        // Create a union with a literal variant that has deny_untagged = true
        let (mut schema, _) = create_simple_schema(SchemaNodeContent::Any);

        // Create literal schema for "active"
        let literal_schema_id = create_literal_schema(&mut schema, eure!({ = "active" }));

        // Create union with literal variant that requires explicit tagging
        let mut variants = IndexMap::new();
        variants.insert("literal".to_string(), literal_schema_id);

        let mut deny_untagged = IndexSet::new();
        deny_untagged.insert("literal".to_string());

        schema.node_mut(schema.root).content = SchemaNodeContent::Union(UnionSchema {
            variants,
            unambiguous: IndexSet::new(),
            repr: VariantRepr::Untagged,
            repr_explicit: false,
            deny_untagged,
        });

        // Create document with literal value but NO $variant tag
        let doc = eure!({ = "active" });

        // Validation should fail with RequiresExplicitVariant error
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(
            e,
            ValidationError::RequiresExplicitVariant { variant, .. } if variant == "literal"
        )));
    }

    #[test]
    fn test_validate_union_deny_untagged_with_tag() {
        use eure_document::eure;

        // Create a union with a literal variant that has deny_untagged = true
        let (mut schema, _) = create_simple_schema(SchemaNodeContent::Any);

        // Create literal schema for "active"
        let literal_schema_id = create_literal_schema(&mut schema, eure!({ = "active" }));

        // Create union with literal variant that requires explicit tagging
        let mut variants = IndexMap::new();
        variants.insert("literal".to_string(), literal_schema_id);

        let mut deny_untagged = IndexSet::new();
        deny_untagged.insert("literal".to_string());

        schema.node_mut(schema.root).content = SchemaNodeContent::Union(UnionSchema {
            variants,
            unambiguous: IndexSet::new(),
            repr: VariantRepr::Untagged,
            repr_explicit: false,
            deny_untagged,
        });

        // Create document with literal value WITH $variant tag
        let doc = eure!({
            = "active"
            %variant = "literal"
        });

        // Validation should succeed
        let result = validate(&doc, &schema);
        assert!(
            result.is_valid,
            "Expected valid, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_union_mixed_deny_untagged() {
        use eure_document::eure;

        // Test that non-deny-untagged variants can still match via untagged
        let (mut schema, _) = create_simple_schema(SchemaNodeContent::Any);

        // Create literal schema for "active" (deny_untagged)
        let literal_active_id = create_literal_schema(&mut schema, eure!({ = "active" }));

        // Create text schema (not deny_untagged)
        let text_schema_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));

        // Create union where literal requires explicit tag but text doesn't
        let mut variants = IndexMap::new();
        variants.insert("literal".to_string(), literal_active_id);
        variants.insert("text".to_string(), text_schema_id);

        let mut deny_untagged = IndexSet::new();
        deny_untagged.insert("literal".to_string());

        schema.node_mut(schema.root).content = SchemaNodeContent::Union(UnionSchema {
            variants,
            unambiguous: IndexSet::new(),
            repr: VariantRepr::Untagged,
            repr_explicit: false,
            deny_untagged,
        });

        // Create document with value "active" but no tag
        // This should fail because "literal" matches but requires explicit tag
        let doc = eure!({ = "active" });

        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(
            e,
            ValidationError::RequiresExplicitVariant { variant, .. } if variant == "literal"
        )));

        // Create document with value "other text" - should match text variant via untagged
        let doc2 = eure!({ = "other text" });

        let result2 = validate(&doc2, &schema);
        assert!(
            result2.is_valid,
            "Expected valid for text match, got errors: {:?}",
            result2.errors
        );
    }

    #[test]
    fn test_validate_literal_with_inline_code() {
        use eure_document::eure;

        // Test that Literal comparison works correctly with inline code (Language::Implicit)
        let mut schema = SchemaDocument::new();

        // Create literal schema using inline code (like meta-schema does)
        let literal_doc = eure!({ = @code("boolean") });

        schema.node_mut(schema.root).content = SchemaNodeContent::Literal(literal_doc);

        // Create document with inline code "boolean"
        let doc = eure!({ = @code("boolean") });

        // Validation should succeed
        let result = validate(&doc, &schema);
        assert!(
            result.is_valid,
            "Expected valid, got errors: {:?}",
            result.errors
        );
    }
}
