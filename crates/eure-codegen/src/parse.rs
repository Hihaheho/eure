//! ParseDocument implementations for codegen configuration types.
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
//! - [`CascadeExtTypeCodegen`] - Codegen settings for cascade-ext-types

use std::collections::HashMap;

use eure_document::identifier::Identifier;
use eure_document::parse::{ParseContext, ParseDocument, ParseError};

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
#[derive(Debug, Clone, Default)]
pub struct RootCodegen {
    /// The root type name for the generated code.
    pub type_name: Option<String>,
}

impl ParseDocument<'_> for RootCodegen {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let mut rec = ctx.parse_record()?;

        let type_name = rec.parse_field_optional::<String>("type")?;

        rec.deny_unknown_fields()?;

        Ok(RootCodegen { type_name })
    }
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
#[derive(Debug, Clone, Default)]
pub struct CodegenDefaults {
    /// Default derive macros for all generated types.
    pub derive: Option<Vec<String>>,
    /// Prefix for extension type field names (e.g., "ext_" -> ext_types).
    pub ext_types_field_prefix: Option<String>,
    /// Prefix for extension type names (e.g., "Ext" -> ExtTypes).
    pub ext_types_type_prefix: Option<String>,
    /// Field name for storing document node ID in generated types.
    pub document_node_id_field: Option<String>,
}

impl ParseDocument<'_> for CodegenDefaults {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let mut rec = ctx.parse_record()?;

        let derive = rec.parse_field_optional::<Vec<String>>("derive")?;
        let ext_types_field_prefix =
            rec.parse_field_optional::<String>("ext-types-field-prefix")?;
        let ext_types_type_prefix = rec.parse_field_optional::<String>("ext-types-type-prefix")?;
        let document_node_id_field =
            rec.parse_field_optional::<String>("document-node-id-field")?;

        rec.deny_unknown_fields()?;

        Ok(CodegenDefaults {
            derive,
            ext_types_field_prefix,
            ext_types_type_prefix,
            document_node_id_field,
        })
    }
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
#[derive(Debug, Clone, Default)]
pub struct UnionCodegen {
    /// Override the generated type name (from base-codegen).
    pub type_name: Option<String>,
    /// Override the list of derive macros (from base-codegen).
    pub derive: Option<Vec<String>>,
    /// Generate separate types for each variant instead of struct-like variants.
    pub variant_types: Option<bool>,
    /// Suffix for variant type names (e.g., "Type" -> TextType, IntegerType).
    pub variant_types_suffix: Option<String>,
}

impl ParseDocument<'_> for UnionCodegen {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let mut rec = ctx.parse_record()?;

        // Parse base-codegen fields (flattened)
        let type_name = rec.parse_field_optional::<String>("type")?;
        let derive = rec.parse_field_optional::<Vec<String>>("derive")?;

        // Parse union-specific fields
        let variant_types = rec.parse_field_optional::<bool>("variant-types")?;
        let variant_types_suffix = rec.parse_field_optional::<String>("variant-types-suffix")?;

        rec.deny_unknown_fields()?;

        Ok(UnionCodegen {
            type_name,
            derive,
            variant_types,
            variant_types_suffix,
        })
    }
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
#[derive(Debug, Clone, Default)]
pub struct RecordCodegen {
    /// Override the generated type name (from base-codegen).
    pub type_name: Option<String>,
    /// Override the list of derive macros (from base-codegen).
    pub derive: Option<Vec<String>>,
}

impl ParseDocument<'_> for RecordCodegen {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let mut rec = ctx.parse_record()?;

        // Parse base-codegen fields (flattened)
        let type_name = rec.parse_field_optional::<String>("type")?;
        let derive = rec.parse_field_optional::<Vec<String>>("derive")?;

        rec.deny_unknown_fields()?;

        Ok(RecordCodegen { type_name, derive })
    }
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
#[derive(Debug, Clone, Default)]
pub struct FieldCodegen {
    /// Override the field name in generated Rust code.
    pub name: Option<String>,
}

impl ParseDocument<'_> for FieldCodegen {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let mut rec = ctx.parse_record()?;

        let name = rec.parse_field_optional::<String>("name")?;

        rec.deny_unknown_fields()?;

        Ok(FieldCodegen { name })
    }
}

// ============================================================================
// Cascade Ext-Type Codegen Types
// ============================================================================

/// Struct configuration for grouping cascade-ext-type fields.
///
/// Used within [`CascadeExtTypeCodegen`] to define composite structs
/// that group multiple extension fields together.
///
/// # Example
///
/// ```eure
/// $cascade-ext-types {
///   $codegen {
///     structs.schema-metadata.fields = ["description", "deprecated", "default", "examples"]
///   }
///   description = `text`
///   deprecated = `boolean`
///   default = `any`
///   examples = [`text`]
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CodegenStruct {
    /// List of field identifiers to include in this struct.
    pub fields: Vec<Identifier>,
}

impl ParseDocument<'_> for CodegenStruct {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let mut rec = ctx.parse_record()?;

        let fields = rec.parse_field::<Vec<Identifier>>("fields")?;

        rec.deny_unknown_fields()?;

        Ok(CodegenStruct { fields })
    }
}

/// Codegen settings for cascade-ext-types sections.
///
/// Corresponds to `$types.cascade-ext-type-codegen` in the schema.
/// Used via `$codegen` on `$cascade-ext-types` definitions.
/// Includes flattened fields from `$types.base-codegen`.
///
/// # Example
///
/// ```eure
/// $cascade-ext-types {
///   $codegen {
///     type = "SchemaMetadata"
///     structs.metadata.fields = ["description", "deprecated"]
///   }
///   description = `text`
///   description.$optional = true
///   deprecated = `boolean`
///   deprecated.$optional = true
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct CascadeExtTypeCodegen {
    /// Override the generated type name (from base-codegen).
    pub type_name: Option<String>,
    /// Override the list of derive macros (from base-codegen).
    pub derive: Option<Vec<String>>,
    /// Group multiple fields into composite structs.
    /// Key is struct name (kebab-case), value specifies fields to include.
    pub structs: Option<HashMap<Identifier, CodegenStruct>>,
}

impl ParseDocument<'_> for CascadeExtTypeCodegen {
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        use eure_document::parse::ParseErrorKind;

        let mut rec = ctx.parse_record()?;

        // Parse base-codegen fields (flattened)
        let type_name = rec.parse_field_optional::<String>("type")?;
        let derive = rec.parse_field_optional::<Vec<String>>("derive")?;

        // Parse cascade-ext-type-specific fields
        // Parse structs map manually since Identifier doesn't implement ParseObjectKey
        let structs: Option<HashMap<Identifier, CodegenStruct>> =
            match rec.field_record_optional("structs")? {
                Some(structs_rec) => {
                    let mut result = HashMap::new();

                    for (name, field_ctx) in structs_rec.unknown_fields() {
                        let ident: Identifier = name.parse().map_err(|e| ParseError {
                            node_id: structs_rec.node_id(),
                            kind: ParseErrorKind::InvalidIdentifier(e),
                        })?;
                        let codegen_struct: CodegenStruct = field_ctx.parse()?;
                        result.insert(ident, codegen_struct);
                    }

                    structs_rec.allow_unknown_fields()?;
                    Some(result)
                }
                None => None,
            };

        rec.deny_unknown_fields()?;

        Ok(CascadeExtTypeCodegen {
            type_name,
            derive,
            structs,
        })
    }
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

    #[test]
    fn test_cascade_ext_type_codegen_empty() {
        let mut doc = EureDocument::new();
        let node_id = create_empty_map_node(&mut doc);

        let result: CascadeExtTypeCodegen = doc.parse(node_id).unwrap();
        assert!(result.type_name.is_none());
        assert!(result.derive.is_none());
        assert!(result.structs.is_none());
    }
}
