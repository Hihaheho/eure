use crate::ids::{QualifiedTypeName, SchemaNodeIrId, TypeId};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IrBuildError {
    #[error("type `{type_id}` declares both proxy and opaque targets")]
    ProxyOpaqueConflict { type_id: String },

    #[error(
        "variant `{variant}` in type `{type_id}` sets allow_unknown_fields on a non-record variant"
    )]
    VariantAllowUnknownFieldsInvalid { type_id: String, variant: String },

    #[error("field `{field}` in type `{type_id}` has conflicting mode attrs: {detail}")]
    FieldModeConflict {
        type_id: String,
        field: String,
        detail: String,
    },

    #[error("field `{field}` in type `{type_id}` cannot use `via` with flatten/flatten_ext")]
    ViaWithFlatten { type_id: String, field: String },

    #[error("field `{field}` in type `{type_id}` cannot use default with flatten/flatten_ext")]
    DefaultWithFlatten { type_id: String, field: String },

    #[error(
        "field `{field}` in type `{type_id}` uses flatten in parse_ext container; use flatten_ext"
    )]
    FlattenInParseExt { type_id: String, field: String },

    #[error("name_index entry `{name:?}` references missing type `{missing}`")]
    NameIndexMissingType {
        name: QualifiedTypeName,
        missing: String,
    },

    #[error(
        "name_index entry `{name:?}` points to type `{pointed}` but type carries schema name `{actual:?}`"
    )]
    NameIndexMismatch {
        name: QualifiedTypeName,
        pointed: String,
        actual: Option<QualifiedTypeName>,
    },

    #[error("type `{type_id}` root node `{node:?}` does not exist")]
    MissingSemanticRoot {
        type_id: String,
        node: SchemaNodeIrId,
    },

    #[error(
        "type `{type_id}` node `{node:?}` references missing schema node `{target:?}` at {path}"
    )]
    MissingSchemaNodeReference {
        type_id: String,
        node: SchemaNodeIrId,
        target: SchemaNodeIrId,
        path: String,
    },

    #[error(
        "type `{type_id}` union node `{node:?}` has policy entry `{variant}` not present in variants"
    )]
    UnionPolicyUnknownVariant {
        type_id: String,
        node: SchemaNodeIrId,
        variant: String,
    },

    #[error(
        "type `{type_id}` exists in name_index but is duplicated for schema name `{schema_name:?}`"
    )]
    DuplicateSchemaName {
        type_id: String,
        schema_name: QualifiedTypeName,
    },

    #[error("type `{type_id}` is missing from module roots while declared as root")]
    RootMissingType { type_id: String },

    #[error("codegen override at `{path}` in type `{type_id}` cannot be empty")]
    EmptyCodegenOverride { type_id: String, path: String },

    #[error("root codegen override at `{path}` cannot be empty")]
    EmptyRootCodegenOverride { path: String },

    #[error(
        "root codegen type name `{root_type_name}` conflicts with root type `{type_id}` codegen type name `{type_type_name}`"
    )]
    RootTypeNameConflict {
        type_id: String,
        root_type_name: String,
        type_type_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("structural mismatch at {path}: {message}")]
pub struct StructuralDiff {
    pub path: String,
    pub message: String,
}

impl StructuralDiff {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

pub fn type_id_string(id: &TypeId) -> String {
    id.0.clone()
}
