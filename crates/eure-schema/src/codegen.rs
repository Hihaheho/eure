//! Schema-model code generation metadata types.

use eure_macros::{FromEure, IntoEure};

/// Root-level codegen settings (`$codegen`).
#[derive(Debug, Clone, Default, PartialEq, Eq, FromEure, IntoEure)]
#[eure(crate = eure_document)]
pub struct RootCodegen {
    /// Override root generated type name.
    #[eure(rename = "type", default)]
    pub type_name: Option<String>,
}

/// Root-level default codegen settings (`$codegen-defaults`).
#[derive(Debug, Clone, Default, PartialEq, Eq, FromEure, IntoEure)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct CodegenDefaults {
    /// Default derives for emitted Rust types.
    #[eure(default)]
    pub derive: Option<Vec<String>>,
    /// Default derives for generated inline companion types.
    #[eure(default)]
    pub inline_derive: Option<Vec<String>>,
    /// Default derives for generated `variant_types` companion types.
    #[eure(default)]
    pub variant_type_derive: Option<Vec<String>>,
    /// Prefix for generated extension field names.
    #[eure(default)]
    pub ext_types_field_prefix: Option<String>,
    /// Prefix for generated extension type names.
    #[eure(default)]
    pub ext_types_type_prefix: Option<String>,
    /// Optional document node id field name.
    #[eure(default)]
    pub document_node_id_field: Option<String>,
}

/// Record type-level codegen metadata (`$codegen` on record schema nodes).
#[derive(Debug, Clone, Default, PartialEq, Eq, FromEure, IntoEure)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct RecordCodegen {
    /// Override generated Rust type name.
    #[eure(rename = "type", default)]
    pub type_name: Option<String>,
    /// Override derives for this type.
    #[eure(default)]
    pub derive: Option<Vec<String>>,
    /// Override derives for generated inline companion types.
    #[eure(default)]
    pub inline_derive: Option<Vec<String>>,
}

/// Union type-level codegen metadata (`$codegen` on union schema nodes).
#[derive(Debug, Clone, Default, PartialEq, Eq, FromEure, IntoEure)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct UnionCodegen {
    /// Override generated Rust type name.
    #[eure(rename = "type", default)]
    pub type_name: Option<String>,
    /// Override derives for this type.
    #[eure(default)]
    pub derive: Option<Vec<String>>,
    /// Override derives for generated inline companion types.
    #[eure(default)]
    pub inline_derive: Option<Vec<String>>,
    /// Generate dedicated types for variants.
    #[eure(default)]
    pub variant_types: Option<bool>,
    /// Suffix for generated variant types.
    #[eure(default)]
    pub variant_types_suffix: Option<String>,
    /// Override derives for generated `variant_types` companion types.
    #[eure(default)]
    pub variant_type_derive: Option<Vec<String>>,
}

/// Field-level codegen metadata (`$codegen` on record field entries).
#[derive(Debug, Clone, Default, PartialEq, Eq, FromEure, IntoEure)]
#[eure(crate = eure_document)]
pub struct FieldCodegen {
    /// Override generated Rust field name.
    #[eure(default)]
    pub name: Option<String>,
}

/// Type-level codegen metadata for schema nodes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TypeCodegen {
    #[default]
    None,
    Record(RecordCodegen),
    Union(UnionCodegen),
}
