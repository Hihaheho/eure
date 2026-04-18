use indexmap::{IndexMap, IndexSet};

use crate::codegen::FieldCodegenIr;
use crate::ids::{QualifiedTypeName, SchemaNodeIrId};
use crate::value::{DecimalInt, ValueIr};

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaNodeIr {
    content: SchemaNodeContentIr,
    metadata: SchemaMetadataIr,
    ext_types: IndexMap<String, ExtTypeIr>,
}

impl SchemaNodeIr {
    pub fn new(
        content: SchemaNodeContentIr,
        metadata: SchemaMetadataIr,
        ext_types: IndexMap<String, ExtTypeIr>,
    ) -> Self {
        Self {
            content,
            metadata,
            ext_types,
        }
    }

    pub fn content(&self) -> &SchemaNodeContentIr {
        &self.content
    }

    pub fn content_mut(&mut self) -> &mut SchemaNodeContentIr {
        &mut self.content
    }

    pub fn metadata(&self) -> &SchemaMetadataIr {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut SchemaMetadataIr {
        &mut self.metadata
    }

    pub fn ext_types(&self) -> &IndexMap<String, ExtTypeIr> {
        &self.ext_types
    }

    pub fn ext_types_mut(&mut self) -> &mut IndexMap<String, ExtTypeIr> {
        &mut self.ext_types
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaNodeContentIr {
    Any,
    Text(TextSchemaIr),
    Integer(IntegerSchemaIr),
    Float(FloatSchemaIr),
    Boolean,
    Null,
    Literal(ValueIr),
    Array(ArraySchemaIr),
    Map(MapSchemaIr),
    Record(RecordSchemaIr),
    Tuple(TupleSchemaIr),
    Union(UnionSchemaIr),
    Reference(QualifiedTypeName),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSchemaIr {
    pub language: Option<String>,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub pattern: Option<String>,
    pub unknown_fields: IndexMap<String, ValueIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerSchemaIr {
    pub min: BoundIr<DecimalInt>,
    pub max: BoundIr<DecimalInt>,
    pub multiple_of: Option<DecimalInt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FloatSchemaIr {
    pub min: BoundIr<f64>,
    pub max: BoundIr<f64>,
    pub multiple_of: Option<f64>,
    pub precision: FloatPrecisionIr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArraySchemaIr {
    pub item: SchemaNodeIrId,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub unique: bool,
    pub contains: Option<SchemaNodeIrId>,
    pub binding_style: Option<BindingStyleIr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapSchemaIr {
    pub key: SchemaNodeIrId,
    pub value: SchemaNodeIrId,
    pub min_size: Option<u32>,
    pub max_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordSchemaIr {
    properties: IndexMap<String, RecordFieldSchemaIr>,
    flatten: Vec<SchemaNodeIrId>,
    unknown_fields: UnknownFieldsPolicyIr,
}

impl RecordSchemaIr {
    pub fn new(
        properties: IndexMap<String, RecordFieldSchemaIr>,
        flatten: Vec<SchemaNodeIrId>,
        unknown_fields: UnknownFieldsPolicyIr,
    ) -> Self {
        Self {
            properties,
            flatten,
            unknown_fields,
        }
    }

    pub fn properties(&self) -> &IndexMap<String, RecordFieldSchemaIr> {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut IndexMap<String, RecordFieldSchemaIr> {
        &mut self.properties
    }

    pub fn flatten(&self) -> &[SchemaNodeIrId] {
        &self.flatten
    }

    pub fn flatten_mut(&mut self) -> &mut Vec<SchemaNodeIrId> {
        &mut self.flatten
    }

    pub fn unknown_fields(&self) -> &UnknownFieldsPolicyIr {
        &self.unknown_fields
    }

    pub fn unknown_fields_mut(&mut self) -> &mut UnknownFieldsPolicyIr {
        &mut self.unknown_fields
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordFieldSchemaIr {
    schema: SchemaNodeIrId,
    optional: bool,
    binding_style: Option<BindingStyleIr>,
    field_codegen: FieldCodegenIr,
}

impl RecordFieldSchemaIr {
    pub fn new(
        schema: SchemaNodeIrId,
        optional: bool,
        binding_style: Option<BindingStyleIr>,
        field_codegen: FieldCodegenIr,
    ) -> Self {
        Self {
            schema,
            optional,
            binding_style,
            field_codegen,
        }
    }

    pub fn schema(&self) -> SchemaNodeIrId {
        self.schema
    }

    pub fn optional(&self) -> bool {
        self.optional
    }

    pub fn binding_style(&self) -> Option<BindingStyleIr> {
        self.binding_style
    }

    pub fn field_codegen(&self) -> &FieldCodegenIr {
        &self.field_codegen
    }

    pub fn field_codegen_mut(&mut self) -> &mut FieldCodegenIr {
        &mut self.field_codegen
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnknownFieldsPolicyIr {
    Deny,
    Allow,
    Schema(SchemaNodeIrId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleSchemaIr {
    pub elements: Vec<SchemaNodeIrId>,
    pub binding_style: Option<BindingStyleIr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionSchemaIr {
    variants: IndexMap<String, SchemaNodeIrId>,
    unambiguous: IndexSet<String>,
    deny_untagged: IndexSet<String>,
    interop: UnionInteropIr,
}

impl UnionSchemaIr {
    pub fn new(
        variants: IndexMap<String, SchemaNodeIrId>,
        unambiguous: IndexSet<String>,
        deny_untagged: IndexSet<String>,
        interop: UnionInteropIr,
    ) -> Self {
        Self {
            variants,
            unambiguous,
            deny_untagged,
            interop,
        }
    }

    pub fn variants(&self) -> &IndexMap<String, SchemaNodeIrId> {
        &self.variants
    }

    pub fn variants_mut(&mut self) -> &mut IndexMap<String, SchemaNodeIrId> {
        &mut self.variants
    }

    pub fn unambiguous(&self) -> &IndexSet<String> {
        &self.unambiguous
    }

    pub fn unambiguous_mut(&mut self) -> &mut IndexSet<String> {
        &mut self.unambiguous
    }

    pub fn deny_untagged(&self) -> &IndexSet<String> {
        &self.deny_untagged
    }

    pub fn deny_untagged_mut(&mut self) -> &mut IndexSet<String> {
        &mut self.deny_untagged
    }

    pub fn interop(&self) -> &UnionInteropIr {
        &self.interop
    }

    pub fn interop_mut(&mut self) -> &mut UnionInteropIr {
        &mut self.interop
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnionInteropIr {
    pub variant_repr: Option<VariantReprIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantReprIr {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloatPrecisionIr {
    F32,
    F64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStyleIr {
    Inline,
    BindingBlock,
    BindingValueBlock,
    Section,
    SectionBlock,
    SectionValueBlock,
    Flatten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayBindingStyleIr {
    /// Single inline binding: `path = [...]`.
    Inline,
    /// Per-element emission with `[]` push marker.
    PerElement(BindingStyleIr),
    /// Per-element emission with explicit `[i]` indices.
    PerElementIndexed(BindingStyleIr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtTypeIr {
    pub schema: SchemaNodeIrId,
    pub optional: bool,
    pub binding_style: Option<BindingStyleIr>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SchemaMetadataIr {
    pub description: Option<DescriptionIr>,
    pub deprecated: bool,
    pub default: Option<ValueIr>,
    pub examples: Option<Vec<ValueIr>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptionIr {
    String(String),
    Markdown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BoundIr<T> {
    #[default]
    Unbounded,
    Inclusive(T),
    Exclusive(T),
}
