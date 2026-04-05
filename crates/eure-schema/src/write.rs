//! Write Eure documents/sources from `SchemaDocument` using generic write API composition.

use crate::identifiers::{CONTENT, EXT_TYPE, OPTIONAL, TAG, VARIANT, VARIANT_REPR};
use crate::interop::VariantRepr;
use crate::{
    ArraySchema, BindingStyle, Bound, CodegenDefaults, Description, ExtTypeSchema, FieldCodegen,
    FloatPrecision, FloatSchema, IntegerSchema, MapSchema, RecordCodegen, RecordFieldSchema,
    RecordSchema, RootCodegen, SchemaDocument, SchemaMetadata, SchemaNodeContent, SchemaNodeId,
    TupleSchema, TypeCodegen, TypeReference, UnionCodegen, UnionSchema, UnknownFieldsPolicy,
};
use eure_document::document::constructor::DocumentConstructor;
use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::identifier::Identifier;
use eure_document::layout::{DocLayout, project_with_layout};
use eure_document::path::PathSegment;
use eure_document::source::SourceDocument;
use eure_document::text::Text;
use eure_document::value::{ObjectKey, PrimitiveValue};
use eure_document::write::{IntoEure, WriteError};
use num_bigint::BigInt;
use thiserror::Error;

const IDENT_TYPES: Identifier = Identifier::new_unchecked("types");
const IDENT_BINDING_STYLE: Identifier = Identifier::new_unchecked("binding-style");
const IDENT_UNKNOWN_FIELDS: Identifier = Identifier::new_unchecked("unknown-fields");
const IDENT_FLATTEN: Identifier = Identifier::new_unchecked("flatten");
const IDENT_DESCRIPTION: Identifier = Identifier::new_unchecked("description");
const IDENT_DEPRECATED: Identifier = Identifier::new_unchecked("deprecated");
const IDENT_DEFAULT: Identifier = Identifier::new_unchecked("default");
const IDENT_EXAMPLES: Identifier = Identifier::new_unchecked("examples");
const IDENT_DENY_UNTAGGED: Identifier = Identifier::new_unchecked("deny-untagged");
const IDENT_UNAMBIGUOUS: Identifier = Identifier::new_unchecked("unambiguous");
const IDENT_INTEROP: Identifier = Identifier::new_unchecked("interop");
const IDENT_CODEGEN: Identifier = Identifier::new_unchecked("codegen");
const IDENT_CODEGEN_DEFAULTS: Identifier = Identifier::new_unchecked("codegen-defaults");

const KEY_VARIANTS: &str = "variants";

/// Errors that can occur during schema writing.
#[derive(Debug, Error, Clone)]
pub enum SchemaWriteError {
    #[error("write error: {0}")]
    Write(#[from] WriteError),
    #[error("literal root cannot be a hole")]
    LiteralRootIsHole,
    #[error(
        "conflicting root $codegen type names: root={root_type_name}, type_codegen={type_codegen_type_name}"
    )]
    ConflictingRootCodegenTypeName {
        root_type_name: String,
        type_codegen_type_name: String,
    },
}

/// Emit an [`EureDocument`] from a [`SchemaDocument`].
pub fn schema_to_document(schema: &SchemaDocument) -> Result<EureDocument, SchemaWriteError> {
    validate_schema_for_write(schema)?;

    let mut c = DocumentConstructor::new();
    c.write(schema.clone())?;
    Ok(c.finish())
}

/// Project a schema document to source using a caller-provided generic layout plan.
pub fn schema_to_source_document(
    schema: &SchemaDocument,
    layout: &DocLayout,
) -> Result<SourceDocument, SchemaWriteError> {
    let doc = schema_to_document(schema)?;
    Ok(project_with_layout(&doc, layout))
}

impl IntoEure for SchemaDocument {
    type Error = WriteError;

    fn write(value: Self, c: &mut DocumentConstructor) -> Result<(), Self::Error> {
        write_schema_document(&value, c)
    }
}

fn validate_schema_for_write(schema: &SchemaDocument) -> Result<(), SchemaWriteError> {
    for node in &schema.nodes {
        if let SchemaNodeContent::Literal(literal_doc) = &node.content
            && matches!(literal_doc.root().content, NodeValue::Hole(_))
        {
            return Err(SchemaWriteError::LiteralRootIsHole);
        }
    }

    if let Some(root_type_name) = schema.root_codegen.type_name.as_deref()
        && let Some(type_codegen_type_name) = root_type_codegen_type_name(schema)
        && root_type_name != type_codegen_type_name
    {
        return Err(SchemaWriteError::ConflictingRootCodegenTypeName {
            root_type_name: root_type_name.to_string(),
            type_codegen_type_name: type_codegen_type_name.to_string(),
        });
    }

    Ok(())
}

fn write_schema_document(
    schema: &SchemaDocument,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    write_schema_node_internal(schema, schema.root, false, c)?;
    write_types_extension(schema, c)?;
    write_root_codegen_extension(schema, c)?;
    write_codegen_defaults_extension(&schema.codegen_defaults, c)?;
    Ok(())
}

fn write_schema_node(
    schema: &SchemaDocument,
    schema_id: SchemaNodeId,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    write_schema_node_internal(schema, schema_id, true, c)
}

fn write_schema_node_internal(
    schema: &SchemaDocument,
    schema_id: SchemaNodeId,
    write_type_codegen: bool,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    let node = schema.node(schema_id);
    write_schema_content(schema, &node.content, c)?;
    write_ext_types(schema, &node.ext_types, c)?;
    write_metadata(&node.metadata, c)?;
    if write_type_codegen {
        write_type_codegen_extension(&node.type_codegen, c)?;
    }
    Ok(())
}

fn write_schema_content(
    schema_doc: &SchemaDocument,
    content: &SchemaNodeContent,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    match content {
        SchemaNodeContent::Any => c.write(Text::inline_implicit("any")),
        SchemaNodeContent::Boolean => c.write(Text::inline_implicit("boolean")),
        SchemaNodeContent::Null => c.write(Text::inline_implicit("null")),
        SchemaNodeContent::Integer(schema) => schema.write(c),
        SchemaNodeContent::Float(schema) => schema.write(c),
        SchemaNodeContent::Text(schema) => schema.write(c),
        SchemaNodeContent::Array(schema) => write_array_schema(schema_doc, schema, c),
        SchemaNodeContent::Map(schema) => write_map_schema(schema_doc, schema, c),
        SchemaNodeContent::Record(schema) => write_record_schema(schema_doc, schema, c),
        SchemaNodeContent::Tuple(schema) => write_tuple_schema(schema_doc, schema, c),
        SchemaNodeContent::Union(schema) => write_union_schema(schema_doc, schema, c),
        SchemaNodeContent::Reference(reference) => reference.write(c),
        SchemaNodeContent::Literal(doc) => write_literal(doc, c),
    }
}

impl IntegerSchema {
    pub fn is_shorthand_compatible(&self) -> bool {
        matches!(self.min, Bound::Unbounded)
            && matches!(self.max, Bound::Unbounded)
            && self.multiple_of.is_none()
    }

    pub fn shorthand(&self) -> Option<Text> {
        self.is_shorthand_compatible()
            .then(|| Text::inline_implicit("integer"))
    }

    pub fn write(&self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        if let Some(shorthand) = self.shorthand() {
            return c.write(shorthand);
        }

        c.record(|rec| {
            rec.constructor().set_variant("integer")?;
            rec.field_optional(
                "range",
                format_bound_range(&self.min, &self.max, format_bigint),
            )?;
            rec.field_optional("multiple-of", self.multiple_of.clone())?;
            Ok(())
        })
    }
}

impl FloatSchema {
    pub fn is_shorthand_compatible(&self) -> bool {
        matches!(self.min, Bound::Unbounded)
            && matches!(self.max, Bound::Unbounded)
            && self.multiple_of.is_none()
            && matches!(self.precision, FloatPrecision::F64)
    }

    pub fn shorthand(&self) -> Option<Text> {
        self.is_shorthand_compatible()
            .then(|| Text::inline_implicit("float"))
    }

    pub fn write(&self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        if let Some(shorthand) = self.shorthand() {
            return c.write(shorthand);
        }

        c.record(|rec| {
            rec.constructor().set_variant("float")?;
            rec.field_optional(
                "range",
                format_bound_range(&self.min, &self.max, format_f64),
            )?;
            rec.field_optional("multiple-of", self.multiple_of)?;
            if matches!(self.precision, FloatPrecision::F32) {
                rec.field("precision", "f32")?;
            }
            Ok(())
        })
    }
}

fn write_array_schema(
    schema_doc: &SchemaDocument,
    schema: &ArraySchema,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    let use_shorthand = schema.min_length.is_none()
        && schema.max_length.is_none()
        && !schema.unique
        && schema.contains.is_none()
        && schema.binding_style.is_none()
        && can_emit_as_single_inline_text(schema_doc, schema.item);

    if use_shorthand {
        c.bind_empty_array()?;
        let scope = c.begin_scope();
        c.navigate(PathSegment::ArrayIndex(None))?;
        write_schema_node(schema_doc, schema.item, c)?;
        c.end_scope(scope)?;
        return Ok(());
    }

    c.record(|rec| {
        rec.constructor().set_variant("array")?;
        rec.field_with("item", |c| write_schema_node(schema_doc, schema.item, c))?;
        rec.field_optional("min-length", schema.min_length)?;
        rec.field_optional("max-length", schema.max_length)?;
        if schema.unique {
            rec.field("unique", true)?;
        }
        if let Some(contains) = schema.contains {
            rec.field_with("contains", |c| write_schema_node(schema_doc, contains, c))?;
        }
        if let Some(style) = schema.binding_style {
            write_binding_style_extension(style, rec.constructor())?;
        }
        Ok(())
    })
}

fn write_tuple_schema(
    schema_doc: &SchemaDocument,
    schema: &TupleSchema,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if schema.binding_style.is_none() {
        c.bind_empty_tuple()?;
        for (index, schema_id) in schema.elements.iter().enumerate() {
            let scope = c.begin_scope();
            c.navigate(PathSegment::TupleIndex(index as u8))?;
            write_schema_node(schema_doc, *schema_id, c)?;
            c.end_scope(scope)?;
        }
        return Ok(());
    }

    c.record(|rec| {
        rec.constructor().set_variant("tuple")?;
        rec.field_with("elements", |c| {
            c.bind_empty_array()?;
            for schema_id in &schema.elements {
                let scope = c.begin_scope();
                c.navigate(PathSegment::ArrayIndex(None))?;
                write_schema_node(schema_doc, *schema_id, c)?;
                c.end_scope(scope)?;
            }
            Ok(())
        })?;
        if let Some(style) = schema.binding_style {
            write_binding_style_extension(style, rec.constructor())?;
        }
        Ok(())
    })
}

fn write_map_schema(
    schema_doc: &SchemaDocument,
    schema: &MapSchema,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    c.record(|rec| {
        rec.constructor().set_variant("map")?;
        rec.field_with("key", |c| write_schema_node(schema_doc, schema.key, c))?;
        rec.field_with("value", |c| write_schema_node(schema_doc, schema.value, c))?;
        rec.field_optional("min-size", schema.min_size)?;
        rec.field_optional("max-size", schema.max_size)?;
        Ok(())
    })
}

fn write_record_schema(
    schema_doc: &SchemaDocument,
    schema: &RecordSchema,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    c.record(|rec| {
        write_unknown_fields_policy(schema_doc, &schema.unknown_fields, rec.constructor())?;
        write_flatten(schema_doc, &schema.flatten, rec.constructor())?;

        for (name, field_schema) in &schema.properties {
            rec.field_with(name, |c| {
                write_schema_node(schema_doc, field_schema.schema, c)?;
                write_record_field_extensions(field_schema, c)?;
                Ok(())
            })?;
        }

        Ok(())
    })
}

fn write_record_field_extensions(
    schema: &RecordFieldSchema,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if schema.optional {
        c.set_extension(OPTIONAL.as_ref(), true)?;
    }
    if let Some(style) = schema.binding_style {
        write_binding_style_extension(style, c)?;
    }
    write_field_codegen_extension(&schema.field_codegen, c)?;
    Ok(())
}

fn write_root_codegen_extension(
    schema: &SchemaDocument,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    match &schema.node(schema.root).type_codegen {
        TypeCodegen::None => {
            if schema.root_codegen == RootCodegen::default() {
                return Ok(());
            }
            write_extension(c, IDENT_CODEGEN, |c| c.write(schema.root_codegen.clone()))
        }
        TypeCodegen::Record(record_codegen) => {
            let merged = RecordCodegen {
                type_name: merge_root_type_name(
                    schema.root_codegen.type_name.as_deref(),
                    record_codegen.type_name.as_deref(),
                )?,
                derive: record_codegen.derive.clone(),
            };
            if merged == RecordCodegen::default() {
                return Ok(());
            }
            write_extension(c, IDENT_CODEGEN, |c| c.write(merged))
        }
        TypeCodegen::Union(union_codegen) => {
            let merged = UnionCodegen {
                type_name: merge_root_type_name(
                    schema.root_codegen.type_name.as_deref(),
                    union_codegen.type_name.as_deref(),
                )?,
                derive: union_codegen.derive.clone(),
                variant_types: union_codegen.variant_types,
                variant_types_suffix: union_codegen.variant_types_suffix.clone(),
            };
            if merged == UnionCodegen::default() {
                return Ok(());
            }
            write_extension(c, IDENT_CODEGEN, |c| c.write(merged))
        }
    }
}

fn write_codegen_defaults_extension(
    defaults: &CodegenDefaults,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if defaults == &CodegenDefaults::default() {
        return Ok(());
    }
    write_extension(c, IDENT_CODEGEN_DEFAULTS, |c| c.write(defaults.clone()))
}

fn write_type_codegen_extension(
    codegen: &TypeCodegen,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    match codegen {
        TypeCodegen::None => Ok(()),
        TypeCodegen::Record(record) => {
            write_extension(c, IDENT_CODEGEN, |c| c.write(record.clone()))
        }
        TypeCodegen::Union(union) => write_extension(c, IDENT_CODEGEN, |c| c.write(union.clone())),
    }
}

fn write_field_codegen_extension(
    codegen: &FieldCodegen,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if codegen == &FieldCodegen::default() {
        return Ok(());
    }
    write_extension(c, IDENT_CODEGEN, |c| c.write(codegen.clone()))
}

fn write_union_schema(
    schema_doc: &SchemaDocument,
    schema: &UnionSchema,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    c.record(|rec| {
        rec.constructor().set_variant("union")?;

        write_interop_extension(&schema.interop.variant_repr, rec.constructor())?;

        rec.field_with(KEY_VARIANTS, |c| {
            c.record(|variants_rec| {
                for (name, schema_id) in &schema.variants {
                    variants_rec.field_with(name, |c| {
                        write_schema_node(schema_doc, *schema_id, c)?;
                        if schema.deny_untagged.contains(name) {
                            c.set_extension(IDENT_DENY_UNTAGGED.as_ref(), true)?;
                        }
                        if schema.unambiguous.contains(name) {
                            c.set_extension(IDENT_UNAMBIGUOUS.as_ref(), true)?;
                        }
                        Ok(())
                    })?;
                }
                Ok(())
            })
        })?;

        Ok(())
    })
}

impl TypeReference {
    pub fn write(&self, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        let mut path = String::from("$types.");
        if let Some(namespace) = &self.namespace {
            path.push_str(namespace);
            path.push('.');
        }
        path.push_str(self.name.as_ref());

        c.write(Text::inline_implicit(path))
    }
}

fn write_literal(
    literal_doc: &EureDocument,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    let root_id = literal_doc.get_root_id();
    let root = literal_doc.node(root_id);
    if matches!(root.content, NodeValue::Hole(_)) {
        return Err(WriteError::InvalidIdentifier(
            "literal root cannot be a hole".to_string(),
        ));
    }

    copy_subtree(literal_doc, root_id, c, true)?;

    if literal_needs_variant(root) {
        c.set_variant("literal")?;
    }

    Ok(())
}

fn write_types_extension(
    schema: &SchemaDocument,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if schema.types.is_empty() {
        return Ok(());
    }

    write_extension(c, IDENT_TYPES, |c| {
        c.record(|rec| {
            for (name, schema_id) in &schema.types {
                rec.field_with(name.as_ref(), |c| write_schema_node(schema, *schema_id, c))?;
            }
            Ok(())
        })
    })
}

fn write_ext_types(
    schema_doc: &SchemaDocument,
    ext_types: &indexmap::IndexMap<Identifier, ExtTypeSchema>,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if ext_types.is_empty() {
        return Ok(());
    }

    write_extension(c, EXT_TYPE, |c| {
        c.record(|rec| {
            for (name, ext_schema) in ext_types {
                rec.field_with(name.as_ref(), |c| {
                    write_schema_node(schema_doc, ext_schema.schema, c)?;
                    if ext_schema.optional {
                        c.set_extension(OPTIONAL.as_ref(), true)?;
                    }
                    if let Some(style) = ext_schema.binding_style {
                        write_binding_style_extension(style, c)?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    })
}

fn write_metadata(
    metadata: &SchemaMetadata,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if let Some(description) = &metadata.description {
        match description {
            Description::String(v) => c.set_extension(IDENT_DESCRIPTION.as_ref(), v.clone())?,
            Description::Markdown(v) => {
                let text = if v.contains('\n') {
                    Text::block(v, "markdown")
                } else {
                    Text::inline(v, "markdown")
                };
                c.set_extension(IDENT_DESCRIPTION.as_ref(), text)?;
            }
        }
    }

    if metadata.deprecated {
        c.set_extension(IDENT_DEPRECATED.as_ref(), true)?;
    }

    if let Some(default_doc) = &metadata.default {
        write_extension(c, IDENT_DEFAULT, |c| {
            copy_subtree(default_doc, default_doc.get_root_id(), c, false)
        })?;
    }

    if let Some(examples) = &metadata.examples {
        write_extension(c, IDENT_EXAMPLES, |c| {
            c.bind_empty_array()?;
            for example in examples {
                let scope = c.begin_scope();
                c.navigate(PathSegment::ArrayIndex(None))?;
                copy_subtree(example, example.get_root_id(), c, false)?;
                c.end_scope(scope)?;
            }
            Ok(())
        })?;
    }

    Ok(())
}

fn write_unknown_fields_policy(
    schema_doc: &SchemaDocument,
    policy: &UnknownFieldsPolicy,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    match policy {
        UnknownFieldsPolicy::Deny => Ok(()),
        UnknownFieldsPolicy::Allow => c.set_extension(IDENT_UNKNOWN_FIELDS.as_ref(), "allow"),
        UnknownFieldsPolicy::Schema(schema_id) => write_extension(c, IDENT_UNKNOWN_FIELDS, |c| {
            write_schema_node(schema_doc, *schema_id, c)
        }),
    }
}

fn write_flatten(
    schema_doc: &SchemaDocument,
    flatten: &[SchemaNodeId],
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    if flatten.is_empty() {
        return Ok(());
    }

    write_extension(c, IDENT_FLATTEN, |c| {
        c.bind_empty_array()?;
        for schema_id in flatten {
            let scope = c.begin_scope();
            c.navigate(PathSegment::ArrayIndex(None))?;
            write_schema_node(schema_doc, *schema_id, c)?;
            c.end_scope(scope)?;
        }
        Ok(())
    })
}

fn write_interop_extension(
    repr: &Option<VariantRepr>,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    let Some(repr) = repr else {
        return Ok(());
    };

    let scope = c.begin_scope();
    c.navigate(PathSegment::Extension(IDENT_INTEROP))?;
    c.navigate(PathSegment::Value(ObjectKey::String(
        VARIANT_REPR.as_ref().to_string(),
    )))?;
    write_variant_repr_value(repr, c)?;
    c.end_scope(scope)?;
    Ok(())
}

fn write_variant_repr_value(
    repr: &VariantRepr,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    match repr {
        VariantRepr::External => c.write("external"),
        VariantRepr::Untagged => c.write("untagged"),
        VariantRepr::Internal { tag } => c.record(|rec| {
            rec.field(TAG.as_ref(), tag.clone())?;
            Ok(())
        }),
        VariantRepr::Adjacent { tag, content } => c.record(|rec| {
            rec.field(TAG.as_ref(), tag.clone())?;
            rec.field(CONTENT.as_ref(), content.clone())?;
            Ok(())
        }),
    }
}

fn write_binding_style_extension(
    style: BindingStyle,
    c: &mut DocumentConstructor,
) -> Result<(), WriteError> {
    c.set_extension(
        IDENT_BINDING_STYLE.as_ref(),
        Text::plaintext(binding_style_as_str(style)),
    )
}

fn binding_style_as_str(style: BindingStyle) -> &'static str {
    match style {
        BindingStyle::Auto => "auto",
        BindingStyle::Passthrough => "passthrough",
        BindingStyle::Section => "section",
        BindingStyle::Nested => "nested",
        BindingStyle::Binding => "binding",
        BindingStyle::SectionBinding => "section-binding",
        BindingStyle::SectionRootBinding => "section-root-binding",
    }
}

fn root_type_codegen_type_name(schema: &SchemaDocument) -> Option<&str> {
    match &schema.node(schema.root).type_codegen {
        TypeCodegen::None => None,
        TypeCodegen::Record(codegen) => codegen.type_name.as_deref(),
        TypeCodegen::Union(codegen) => codegen.type_name.as_deref(),
    }
}

fn merge_root_type_name(
    root_type_name: Option<&str>,
    type_codegen_type_name: Option<&str>,
) -> Result<Option<String>, WriteError> {
    match (root_type_name, type_codegen_type_name) {
        (Some(root), Some(ty)) if root != ty => Err(WriteError::InvalidIdentifier(format!(
            "conflicting root $codegen type names: root={root}, type_codegen={ty}"
        ))),
        (Some(root), _) => Ok(Some(root.to_string())),
        (None, Some(ty)) => Ok(Some(ty.to_string())),
        (None, None) => Ok(None),
    }
}

fn write_extension<F>(
    c: &mut DocumentConstructor,
    ident: Identifier,
    writer: F,
) -> Result<(), WriteError>
where
    F: FnOnce(&mut DocumentConstructor) -> Result<(), WriteError>,
{
    let scope = c.begin_scope();
    c.navigate(PathSegment::Extension(ident))?;
    writer(c)?;
    c.end_scope(scope)?;
    Ok(())
}

fn copy_subtree(
    src_doc: &EureDocument,
    src_node_id: NodeId,
    c: &mut DocumentConstructor,
    skip_variant_extension: bool,
) -> Result<(), WriteError> {
    let src_node = src_doc.node(src_node_id);

    match &src_node.content {
        NodeValue::Hole(label) => {
            c.bind_hole(label.clone())?;
        }
        NodeValue::Primitive(prim) => {
            c.bind_primitive(prim.clone())?;
        }
        NodeValue::Array(array) => {
            c.bind_empty_array()?;
            for &child_id in array.iter() {
                let scope = c.begin_scope();
                c.navigate(PathSegment::ArrayIndex(None))?;
                copy_subtree(src_doc, child_id, c, skip_variant_extension)?;
                c.end_scope(scope)?;
            }
        }
        NodeValue::Tuple(tuple) => {
            c.bind_empty_tuple()?;
            for (index, &child_id) in tuple.iter().enumerate() {
                let scope = c.begin_scope();
                c.navigate(PathSegment::TupleIndex(index as u8))?;
                copy_subtree(src_doc, child_id, c, skip_variant_extension)?;
                c.end_scope(scope)?;
            }
        }
        NodeValue::Map(map) => {
            c.bind_empty_map()?;
            for (key, &child_id) in map.iter() {
                let scope = c.begin_scope();
                c.navigate(PathSegment::Value(key.clone()))?;
                copy_subtree(src_doc, child_id, c, skip_variant_extension)?;
                c.end_scope(scope)?;
            }
        }
        NodeValue::PartialMap(map) => {
            c.bind_empty_partial_map()?;
            for (key, &child_id) in map.iter() {
                let scope = c.begin_scope();
                c.navigate_partial_map_entry(key.clone())?;
                copy_subtree(src_doc, child_id, c, skip_variant_extension)?;
                c.end_scope(scope)?;
            }
        }
    }

    for (ident, &ext_node_id) in src_node.extensions.iter() {
        if skip_variant_extension && ident == &VARIANT {
            continue;
        }
        let scope = c.begin_scope();
        c.navigate(PathSegment::Extension(ident.clone()))?;
        copy_subtree(src_doc, ext_node_id, c, skip_variant_extension)?;
        c.end_scope(scope)?;
    }

    Ok(())
}

fn literal_needs_variant(node: &eure_document::document::node::Node) -> bool {
    match &node.content {
        NodeValue::Primitive(PrimitiveValue::Text(t)) => {
            t.language.is_implicit() || t.language.is_other("eure-path")
        }
        NodeValue::Primitive(_) => false,
        NodeValue::Array(_)
        | NodeValue::Tuple(_)
        | NodeValue::Map(_)
        | NodeValue::PartialMap(_) => true,
        NodeValue::Hole(_) => true,
    }
}

fn can_emit_as_single_inline_text(schema: &SchemaDocument, schema_id: SchemaNodeId) -> bool {
    let schema_node = schema.node(schema_id);
    if !schema_node.ext_types.is_empty() || schema_node.metadata != SchemaMetadata::default() {
        return false;
    }

    match &schema_node.content {
        SchemaNodeContent::Any
        | SchemaNodeContent::Boolean
        | SchemaNodeContent::Null
        | SchemaNodeContent::Reference(_) => true,
        SchemaNodeContent::Integer(s) => {
            matches!(s.min, Bound::Unbounded)
                && matches!(s.max, Bound::Unbounded)
                && s.multiple_of.is_none()
        }
        SchemaNodeContent::Float(s) => {
            matches!(s.min, Bound::Unbounded)
                && matches!(s.max, Bound::Unbounded)
                && s.multiple_of.is_none()
                && matches!(s.precision, FloatPrecision::F64)
        }
        SchemaNodeContent::Text(s) => {
            s.min_length.is_none()
                && s.max_length.is_none()
                && s.pattern.is_none()
                && s.unknown_fields.is_empty()
        }
        _ => false,
    }
}

fn format_bound_range<T>(
    min: &Bound<T>,
    max: &Bound<T>,
    format_value: fn(&T) -> String,
) -> Option<String> {
    if matches!(min, Bound::Unbounded) && matches!(max, Bound::Unbounded) {
        return None;
    }

    let left = match min {
        Bound::Inclusive(_) => '[',
        Bound::Exclusive(_) | Bound::Unbounded => '(',
    };
    let right = match max {
        Bound::Inclusive(_) => ']',
        Bound::Exclusive(_) | Bound::Unbounded => ')',
    };

    let min_str = match min {
        Bound::Unbounded => String::new(),
        Bound::Inclusive(v) | Bound::Exclusive(v) => format_value(v),
    };
    let max_str = match max {
        Bound::Unbounded => String::new(),
        Bound::Inclusive(v) | Bound::Exclusive(v) => format_value(v),
    };

    Some(format!("{left}{min_str}, {max_str}{right}"))
}

fn format_bigint(value: &BigInt) -> String {
    value.to_string()
}

fn format_f64(value: &f64) -> String {
    let s = value.to_string();
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        format!("{s}.0")
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::document_to_schema;
    use crate::interop::UnionInterop;
    use crate::{
        CodegenDefaults, FieldCodegen, RecordCodegen, RootCodegen, TextSchema, TypeCodegen,
        UnknownFieldsPolicy,
    };
    use eure_document::document::node::NodeMap;
    use eure_document::value::ObjectKey;

    fn make_union_schema(repr: Option<VariantRepr>) -> SchemaDocument {
        let mut schema = SchemaDocument::new();
        let variant_node = schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let mut variants = indexmap::IndexMap::new();
        variants.insert("v".to_string(), variant_node);

        schema.root = schema.create_node(SchemaNodeContent::Union(UnionSchema {
            variants,
            unambiguous: Default::default(),
            interop: UnionInterop { variant_repr: repr },
            deny_untagged: Default::default(),
        }));
        schema
    }

    #[test]
    fn schema_to_document_delegates_to_into_eure_path() {
        let schema = make_union_schema(Some(VariantRepr::Untagged));

        let mut c = DocumentConstructor::new();
        c.write(schema.clone()).expect("into-eure write");
        let expected = c.finish();

        let actual = schema_to_document(&schema).expect("schema_to_document");
        assert_eq!(actual, expected);
    }

    #[test]
    fn emits_union_repr_when_untagged_was_explicit() {
        let schema = make_union_schema(Some(VariantRepr::Untagged));
        let doc = schema_to_document(&schema).expect("schema emit");

        let interop_id = doc
            .root()
            .extensions
            .get(&IDENT_INTEROP)
            .expect("interop extension should be emitted");
        let interop_ctx = doc.parse_context(*interop_id);
        let interop_rec = interop_ctx.parse_record().expect("interop record");
        let repr_ctx = interop_rec
            .field(VARIANT_REPR.as_ref())
            .expect("variant-repr field");
        let repr = repr_ctx.parse::<&str>().expect("repr parse");
        assert_eq!(repr, "untagged");
    }

    #[test]
    fn omits_union_repr_when_untagged_is_implicit() {
        let schema = make_union_schema(None);
        let doc = schema_to_document(&schema).expect("schema emit");

        assert!(!doc.root().extensions.contains_key(&IDENT_INTEROP));
    }

    #[test]
    fn array_shorthand_requires_single_inline_type_token() {
        let mut inline_schema = SchemaDocument::new();
        let int_id =
            inline_schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        inline_schema.root = inline_schema.create_node(SchemaNodeContent::Array(ArraySchema {
            item: int_id,
            min_length: None,
            max_length: None,
            unique: false,
            contains: None,
            binding_style: None,
        }));
        let inline_doc = schema_to_document(&inline_schema).expect("inline array");
        assert!(matches!(inline_doc.root().content, NodeValue::Array(_)));
        assert!(!inline_doc.root().extensions.contains_key(&VARIANT));

        let mut complex_schema = SchemaDocument::new();
        let x_schema =
            complex_schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let item_id = complex_schema.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: indexmap::IndexMap::from([(
                "x".to_string(),
                RecordFieldSchema {
                    schema: x_schema,
                    optional: false,
                    binding_style: None,
                    field_codegen: Default::default(),
                },
            )]),
            flatten: Vec::new(),
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        complex_schema.root = complex_schema.create_node(SchemaNodeContent::Array(ArraySchema {
            item: item_id,
            min_length: None,
            max_length: None,
            unique: false,
            contains: None,
            binding_style: None,
        }));

        let complex_doc = schema_to_document(&complex_schema).expect("complex array");
        assert!(matches!(complex_doc.root().content, NodeValue::Map(_)));
        let variant_id = complex_doc
            .root()
            .extensions
            .get(&VARIANT)
            .expect("non-inline array should emit explicit array variant");
        let variant = complex_doc
            .parse::<&str>(*variant_id)
            .expect("variant parse");
        assert_eq!(variant, "array");
    }

    #[test]
    fn literal_preserves_extensions_except_variant() {
        let mut literal = EureDocument::new();
        let root_id = literal.get_root_id();
        literal.node_mut(root_id).content = NodeValue::Map(NodeMap::default());

        let child_id = literal
            .add_map_child(ObjectKey::String("x".to_string()), root_id)
            .expect("insert child")
            .node_id;
        literal.node_mut(child_id).content =
            NodeValue::Primitive(PrimitiveValue::Integer(1.into()));

        let root_variant_id = literal
            .add_extension(VARIANT, root_id)
            .expect("root variant ext")
            .node_id;
        literal.node_mut(root_variant_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("old-root")));

        let foo_ext_id = literal
            .add_extension("foo".parse().unwrap(), root_id)
            .expect("root foo ext")
            .node_id;
        literal.node_mut(foo_ext_id).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        let child_variant_id = literal
            .add_extension(VARIANT, child_id)
            .expect("child variant ext")
            .node_id;
        literal.node_mut(child_variant_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("old-child")));

        let child_baz_id = literal
            .add_extension("baz".parse().unwrap(), child_id)
            .expect("child baz ext")
            .node_id;
        literal.node_mut(child_baz_id).content = NodeValue::Primitive(PrimitiveValue::Bool(true));

        let mut schema = SchemaDocument::new();
        schema.root = schema.create_node(SchemaNodeContent::Literal(literal));

        let doc = schema_to_document(&schema).expect("schema emit");

        let root = doc.root();
        let variant_id = root
            .extensions
            .get(&VARIANT)
            .expect("literal map should emit $variant = literal");
        let root_variant = doc.parse::<&str>(*variant_id).expect("variant parse");
        assert_eq!(root_variant, "literal");

        assert!(root.extensions.contains_key(&"foo".parse().unwrap()));

        let root_map = match &root.content {
            NodeValue::Map(map) => map,
            other => panic!("expected map root, got {other:?}"),
        };
        let child = doc.node(*root_map.get(&ObjectKey::String("x".to_string())).unwrap());
        assert!(child.extensions.contains_key(&"baz".parse().unwrap()));
        assert!(!child.extensions.contains_key(&VARIANT));
    }

    #[test]
    fn text_schema_uses_shorthand_when_compatible() {
        let mut schema = SchemaDocument::new();
        schema.root = schema.create_node(SchemaNodeContent::Text(TextSchema {
            language: Some("uuid".to_string()),
            min_length: None,
            max_length: None,
            pattern: None,
            unknown_fields: Default::default(),
        }));

        let doc = schema_to_document(&schema).expect("schema emit");
        match &doc.root().content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => {
                assert!(t.language.is_implicit());
                assert_eq!(t.as_str(), "text.uuid");
            }
            other => panic!("expected shorthand text token, got {other:?}"),
        }

        let mut schema_constrained = SchemaDocument::new();
        schema_constrained.root =
            schema_constrained.create_node(SchemaNodeContent::Text(TextSchema {
                language: None,
                min_length: Some(1),
                max_length: None,
                pattern: None,
                unknown_fields: Default::default(),
            }));
        let constrained_doc = schema_to_document(&schema_constrained).expect("schema emit");
        assert!(matches!(constrained_doc.root().content, NodeValue::Map(_)));
        let variant_id = constrained_doc
            .root()
            .extensions
            .get(&VARIANT)
            .expect("constrained text should emit explicit text variant");
        let variant = constrained_doc
            .parse::<&str>(*variant_id)
            .expect("variant parse");
        assert_eq!(variant, "text");
    }

    #[test]
    fn roundtrips_root_type_and_field_codegen_metadata() {
        let mut schema = SchemaDocument::new();
        schema.root_codegen = RootCodegen {
            type_name: Some("User".to_string()),
        };
        schema.codegen_defaults = CodegenDefaults {
            derive: Some(vec!["Debug".to_string(), "Clone".to_string()]),
            ext_types_field_prefix: Some("ext_".to_string()),
            ext_types_type_prefix: Some("Ext".to_string()),
            document_node_id_field: Some("node_id".to_string()),
        };

        let text_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        schema.root = schema.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: indexmap::IndexMap::from([(
                "user-name".to_string(),
                RecordFieldSchema {
                    schema: text_id,
                    optional: false,
                    binding_style: None,
                    field_codegen: FieldCodegen {
                        name: Some("user_name".to_string()),
                    },
                },
            )]),
            flatten: Vec::new(),
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        schema.node_mut(schema.root).type_codegen = TypeCodegen::Record(RecordCodegen {
            type_name: Some("User".to_string()),
            derive: Some(vec!["Debug".to_string()]),
        });

        let doc = schema_to_document(&schema).expect("write schema");
        let (roundtrip, _) = document_to_schema(&doc).expect("parse schema");

        assert_eq!(roundtrip.root_codegen.type_name.as_deref(), Some("User"));
        assert_eq!(
            roundtrip.codegen_defaults.document_node_id_field.as_deref(),
            Some("node_id")
        );
        let TypeCodegen::Record(record_codegen) = &roundtrip.node(roundtrip.root).type_codegen
        else {
            panic!("expected record codegen")
        };
        assert_eq!(record_codegen.type_name.as_deref(), Some("User"));
        let record = match &roundtrip.node(roundtrip.root).content {
            SchemaNodeContent::Record(record) => record,
            _ => panic!("expected record root"),
        };
        assert_eq!(
            record.properties["user-name"].field_codegen.name.as_deref(),
            Some("user_name")
        );
    }

    #[test]
    fn rejects_conflicting_root_codegen_type_names() {
        let mut schema = SchemaDocument::new();
        schema.root_codegen = RootCodegen {
            type_name: Some("Root".to_string()),
        };
        schema.root = schema.create_node(SchemaNodeContent::Record(RecordSchema::default()));
        schema.node_mut(schema.root).type_codegen = TypeCodegen::Record(RecordCodegen {
            type_name: Some("User".to_string()),
            derive: None,
        });

        let error = schema_to_document(&schema).expect_err("conflict must be rejected");
        assert!(matches!(
            error,
            SchemaWriteError::ConflictingRootCodegenTypeName { .. }
        ));
    }
}
