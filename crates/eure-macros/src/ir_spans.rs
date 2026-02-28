use proc_macro2::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct FieldSpanInfo {
    pub(crate) field_span: Span,
    pub(crate) ty_span: Span,
    pub(crate) attr_spans: HashMap<String, Span>,
}

#[derive(Debug, Clone)]
pub(crate) struct VariantSpanInfo {
    pub(crate) variant_span: Span,
    pub(crate) fields: HashMap<String, FieldSpanInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct DeriveSpanTable {
    pub(crate) derive_span: Span,
    pub(crate) container_attr_spans: HashMap<String, Span>,
    pub(crate) fields: HashMap<String, FieldSpanInfo>,
    pub(crate) variants: HashMap<String, VariantSpanInfo>,
}

impl DeriveSpanTable {
    pub(crate) fn new(derive_span: Span, container_attr_spans: HashMap<String, Span>) -> Self {
        Self {
            derive_span,
            container_attr_spans,
            fields: HashMap::new(),
            variants: HashMap::new(),
        }
    }

    pub(crate) fn container_attr_span(&self, attr: &str) -> Option<Span> {
        self.container_attr_spans.get(attr).copied()
    }

    pub(crate) fn field_span(&self, name: &str) -> Option<Span> {
        self.fields.get(name).map(|entry| entry.field_span)
    }

    pub(crate) fn field_ty_span(&self, name: &str) -> Option<Span> {
        self.fields.get(name).map(|entry| entry.ty_span)
    }

    pub(crate) fn field_attr_span(&self, name: &str, attr: &str) -> Option<Span> {
        self.fields
            .get(name)
            .and_then(|entry| entry.attr_spans.get(attr).copied())
    }

    pub(crate) fn variant_span(&self, variant: &str) -> Option<Span> {
        self.variants.get(variant).map(|entry| entry.variant_span)
    }

    pub(crate) fn variant_field_span(&self, variant: &str, field: &str) -> Option<Span> {
        self.variants
            .get(variant)
            .and_then(|entry| entry.fields.get(field).map(|field| field.field_span))
    }

    pub(crate) fn variant_field_ty_span(&self, variant: &str, field: &str) -> Option<Span> {
        self.variants
            .get(variant)
            .and_then(|entry| entry.fields.get(field).map(|field| field.ty_span))
    }

    pub(crate) fn variant_field_attr_span(
        &self,
        variant: &str,
        field: &str,
        attr: &str,
    ) -> Option<Span> {
        self.variants
            .get(variant)
            .and_then(|entry| entry.fields.get(field))
            .and_then(|entry| entry.attr_spans.get(attr).copied())
    }

    pub(crate) fn upsert_field(
        &mut self,
        field_name: String,
        field_span: Span,
        ty_span: Span,
        attr_spans: HashMap<String, Span>,
    ) {
        self.fields.insert(
            field_name,
            FieldSpanInfo {
                field_span,
                ty_span,
                attr_spans,
            },
        );
    }

    pub(crate) fn upsert_variant(
        &mut self,
        variant_name: String,
        variant_span: Span,
        _attr_spans: HashMap<String, Span>,
    ) {
        self.variants
            .entry(variant_name)
            .or_insert(VariantSpanInfo {
                variant_span,
                fields: HashMap::new(),
            });
    }

    pub(crate) fn upsert_variant_field(
        &mut self,
        variant_name: &str,
        field_name: String,
        field_span: Span,
        ty_span: Span,
        attr_spans: HashMap<String, Span>,
    ) {
        let variant = self
            .variants
            .entry(variant_name.to_string())
            .or_insert_with(|| VariantSpanInfo {
                variant_span: self.derive_span,
                fields: HashMap::new(),
            });
        variant.fields.insert(
            field_name,
            FieldSpanInfo {
                field_span,
                ty_span,
                attr_spans,
            },
        );
    }
}
