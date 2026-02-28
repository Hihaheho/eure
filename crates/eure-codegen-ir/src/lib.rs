mod build_check;
mod codegen;
mod emission;
mod error;
mod ids;
mod module;
mod rust_binding;
mod schema;
mod structural;
mod value;

pub use codegen::{
    CodegenDefaultsIr, DEFAULT_VARIANT_TYPES_SUFFIX, FieldCodegenIr, InheritableCodegenValueIr,
    RecordCodegenIr, RootCodegenIr, TypeCodegenIr, UnionCodegenIr,
};
pub use emission::{
    EffectiveEmissionIr, EmissionDefaultsIr, TypeEmissionConfigIr, effective_emission,
    filter_desired_derives,
};
pub use error::{IrBuildError, StructuralDiff};
pub use ids::{QualifiedTypeName, RustPathIr, SchemaNodeIrId, TypeId};
pub use module::{IrModule, TypeDefIr, TypeNamesIr, TypeOriginIr};
pub use rust_binding::{
    ConstParamIr, ContainerAttrsIr, DefaultValueIr, FieldModeIr, FieldSourceAttrsIr,
    LifetimeParamIr, MapImplTypeIr, PrimitiveRustTypeIr, ProxyModeIr, RenameRuleIr, RustBindingIr,
    RustFieldIr, RustGenericsIr, RustTypeExprIr, RustTypeKindIr, RustVariantIr, TupleElementIr,
    TypeParamIr, VariantShapeIr, WhereClauseIr, WrapperKindIr,
};
pub use schema::{
    ArraySchemaIr, BindingStyleIr, BoundIr, DescriptionIr, ExtTypeIr, FloatPrecisionIr,
    FloatSchemaIr, IntegerSchemaIr, MapSchemaIr, RecordFieldSchemaIr, RecordSchemaIr,
    SchemaMetadataIr, SchemaNodeContentIr, SchemaNodeIr, TextSchemaIr, TupleSchemaIr,
    UnionInteropIr, UnionSchemaIr, UnknownFieldsPolicyIr, VariantReprIr,
};
pub use structural::{assert_structural_eq, structural_eq};
pub use value::{DecimalInt, ObjectKeyIr, TextLanguageIr, TextValueIr, ValueIr};
