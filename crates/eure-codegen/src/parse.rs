//! FromEure implementations for codegen configuration types.
//!
//! These types represent the `$codegen` and `$codegen-defaults` extensions
//! defined in the Eure schema spec (`assets/schemas/eure-schema.schema.eure`).
//!
//! # Types
//!
//! - [`RootCodegen`] - Root-level `$codegen` extension
//! - [`CodegenDefaults`] - Root-level `$codegen-defaults` extension
//! - [`UnionCodegen`] - Codegen settings for union types
//! - [`RecordCodegen`] - Codegen settings for record types
//! - [`FieldCodegen`] - Codegen settings for individual record fields

use eure_macros::FromEure;

// ============================================================================
// Root-Level Codegen Types
// ============================================================================

/// Root-level codegen settings.
///
/// Corresponds to `$types.root-codegen` in the schema.
/// Used at document root via `$codegen` extension.
///
/// # Example
///
/// ```eure
/// $codegen {
///   type = "MyRootType"
/// }
/// ```
#[derive(Debug, Clone, Default, FromEure)]
#[eure(crate = eure_document)]
pub struct RootCodegen {
    /// The root type name for the generated code.
    #[eure(rename = "type", default)]
    pub type_name: Option<String>,
}

/// Default codegen settings applied to all types.
///
/// Corresponds to `$types.codegen-defaults` in the schema.
/// Used at document root via `$codegen-defaults` extension.
///
/// # Example
///
/// ```eure
/// $codegen-defaults {
///   derive = ["Debug", "Clone", "PartialEq"]
///   ext-types-field-prefix = "ext_"
///   ext-types-type-prefix = "Ext"
///   document-node-id-field = "doc_node"
/// }
/// ```
#[derive(Debug, Clone, Default, FromEure)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct CodegenDefaults {
    /// Default derive macros for all generated types.
    #[eure(default)]
    pub derive: Option<Vec<String>>,
    /// Prefix for extension type field names (e.g., "ext_" -> ext_types).
    #[eure(default)]
    pub ext_types_field_prefix: Option<String>,
    /// Prefix for extension type names (e.g., "Ext" -> ExtTypes).
    #[eure(default)]
    pub ext_types_type_prefix: Option<String>,
    /// Field name for storing document node ID in generated types.
    #[eure(default)]
    pub document_node_id_field: Option<String>,
}

// ============================================================================
// Type-Level Codegen Types
// ============================================================================

/// Codegen settings for union types.
///
/// Corresponds to `$types.union-codegen` in the schema.
/// Used via `$ext-type.codegen` on union type definitions.
/// Includes flattened fields from `$types.base-codegen`.
///
/// # Example
///
/// ```eure
/// @ $types.my-union {
///   $variant: union
///   $codegen {
///     type = "MyUnion"
///     variant-types = true
///     variant-types-suffix = "Variant"
///   }
///   variants.a = `text`
///   variants.b = `integer`
/// }
/// ```
#[derive(Debug, Clone, Default, FromEure)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct UnionCodegen {
    /// Override the generated type name (from base-codegen).
    #[eure(rename = "type", default)]
    pub type_name: Option<String>,
    /// Override the list of derive macros (from base-codegen).
    #[eure(default)]
    pub derive: Option<Vec<String>>,
    /// Generate separate types for each variant instead of struct-like variants.
    #[eure(default)]
    pub variant_types: Option<bool>,
    /// Suffix for variant type names (e.g., "Type" -> TextType, IntegerType).
    #[eure(default)]
    pub variant_types_suffix: Option<String>,
}

/// Codegen settings for record types.
///
/// Corresponds to `$types.record-codegen` in the schema.
/// Used via `$ext-type.codegen` on record type definitions.
/// Includes flattened fields from `$types.base-codegen`.
///
/// For field-level codegen settings, use [`FieldCodegen`] via
/// `value.$ext-type.codegen` on individual fields.
///
/// # Example
///
/// ```eure
/// @ $types.user {
///   $codegen {
///     type = "User"
///     derive = ["Debug", "Clone", "Serialize"]
///   }
///   name = `text`
///   age = `integer`
/// }
/// ```
#[derive(Debug, Clone, Default, FromEure)]
#[eure(crate = eure_document)]
pub struct RecordCodegen {
    /// Override the generated type name (from base-codegen).
    #[eure(rename = "type", default)]
    pub type_name: Option<String>,
    /// Override the list of derive macros (from base-codegen).
    #[eure(default)]
    pub derive: Option<Vec<String>>,
}

/// Codegen settings for individual record fields.
///
/// Corresponds to `$types.field-codegen` in the schema.
/// Used via `value.$ext-type.codegen` on record field values.
///
/// # Example
///
/// ```eure
/// @ $types.user {
///   user-name = `text`
///   user-name.$codegen.name = "username"  // Rename to `username` in Rust
/// }
/// ```
#[derive(Debug, Clone, Default, FromEure)]
#[eure(crate = eure_document)]
pub struct FieldCodegen {
    /// Override the field name in generated Rust code.
    #[eure(default)]
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::document::node::NodeValue;
    use eure_document::document::{EureDocument, NodeId};

    fn create_empty_map_node(doc: &mut EureDocument) -> NodeId {
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Map(Default::default());
        root_id
    }

    #[test]
    fn test_root_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: RootCodegen = doc.parse(node_id).unwrap();
        assert!(result.type_name.is_none());
    }

    #[test]
    fn test_codegen_defaults_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: CodegenDefaults = doc.parse(node_id).unwrap();
        assert!(result.derive.is_none());
        assert!(result.ext_types_field_prefix.is_none());
        assert!(result.ext_types_type_prefix.is_none());
        assert!(result.document_node_id_field.is_none());
    }

    #[test]
    fn test_union_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: UnionCodegen = doc.parse(node_id).unwrap();
        assert!(result.type_name.is_none());
        assert!(result.derive.is_none());
        assert!(result.variant_types.is_none());
        assert!(result.variant_types_suffix.is_none());
    }

    #[test]
    fn test_record_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: RecordCodegen = doc.parse(node_id).unwrap();
        assert!(result.type_name.is_none());
        assert!(result.derive.is_none());
    }

    #[test]
    fn test_field_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: FieldCodegen = doc.parse(node_id).unwrap();
        assert!(result.name.is_none());
    }
}
