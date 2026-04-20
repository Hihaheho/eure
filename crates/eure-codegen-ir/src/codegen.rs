pub const DEFAULT_VARIANT_TYPES_SUFFIX: &str = "Data";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InheritableCodegenValueIr<T> {
    #[default]
    InheritCodegenDefaults,
    Value(T),
}

impl<T> InheritableCodegenValueIr<T> {
    pub fn inherit() -> Self {
        Self::InheritCodegenDefaults
    }

    pub fn explicit(value: T) -> Self {
        Self::Value(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, bon::Builder)]
pub struct RootCodegenIr {
    pub type_name_override: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, bon::Builder)]
pub struct CodegenDefaultsIr {
    #[builder(default)]
    pub derive: Vec<String>,
    #[builder(default)]
    pub inline_derive: Vec<String>,
    #[builder(default)]
    pub variant_type_derive: Vec<String>,
    #[builder(default)]
    pub ext_types_field_prefix: String,
    #[builder(default)]
    pub ext_types_type_prefix: String,
    #[builder(default)]
    pub document_node_id_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TypeCodegenIr {
    #[default]
    None,
    Record(RecordCodegenIr),
    Union(UnionCodegenIr),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, bon::Builder)]
pub struct RecordCodegenIr {
    pub type_name_override: Option<String>,
    #[builder(default)]
    pub derive: InheritableCodegenValueIr<Vec<String>>,
    #[builder(default)]
    pub inline_derive: InheritableCodegenValueIr<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, bon::Builder)]
pub struct UnionCodegenIr {
    pub type_name_override: Option<String>,
    #[builder(default)]
    pub derive: InheritableCodegenValueIr<Vec<String>>,
    #[builder(default)]
    pub inline_derive: InheritableCodegenValueIr<Vec<String>>,
    #[builder(default)]
    pub variant_types: bool,
    pub variant_types_suffix_override: Option<String>,
    #[builder(default)]
    pub variant_type_derive: InheritableCodegenValueIr<Vec<String>>,
}

impl Default for UnionCodegenIr {
    fn default() -> Self {
        Self {
            type_name_override: None,
            derive: InheritableCodegenValueIr::InheritCodegenDefaults,
            inline_derive: InheritableCodegenValueIr::InheritCodegenDefaults,
            variant_types: false,
            variant_types_suffix_override: None,
            variant_type_derive: InheritableCodegenValueIr::InheritCodegenDefaults,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, bon::Builder)]
pub struct FieldCodegenIr {
    pub name_override: Option<String>,
}
